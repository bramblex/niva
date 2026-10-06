#[cfg(target_os = "windows")]
mod win_utils;

pub(crate) mod image_utils;

use anyhow::{Result, ensure};
use std::{
    collections::HashMap,
    io::{Read, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};
use tao::window::Icon;

use crate::lock;

use super::utils::{ArcMut, arc, arc_mut};

type IconCache = HashMap<String, Icon>;

pub trait ResourceManager: std::fmt::Debug + Send + Sync {
    /// Filesystem root for resources backed by a directory. Embedded resource
    /// stores return `None`; callers must not infer a path from the executable.
    fn filesystem_root(&self) -> Option<PathBuf> {
        None
    }
    fn exists(&self, path: &str) -> bool;
    fn load(&self, path: &str) -> Result<Vec<u8>>;
    /// Return the size of a regular resource without loading its contents.
    fn resource_size(&self, path: &str) -> Result<Option<u64>> {
        if !self.exists(path) {
            return Ok(None);
        }
        Ok(Some(self.load(path)?.len() as u64))
    }
    /// Read a bounded slice and report the length of the same resource handle.
    /// File-backed implementations override this so large media is never
    /// materialized as a complete `Vec` just to serve a byte range.
    fn read_range(&self, path: &str, offset: u64, length: usize) -> Result<ResourceRead> {
        let data = self.load(path)?;
        let total_len = data.len() as u64;
        let start = usize::try_from(offset).unwrap_or(usize::MAX);
        let end = start.saturating_add(length).min(data.len());
        let bytes = if start <= data.len() {
            data[start..end].to_vec()
        } else {
            Vec::new()
        };
        Ok(ResourceRead { total_len, bytes })
    }
    /// `None` means the resource exceeds the response limit. Implementations
    /// backed by files or an index should reject it before cloning/reading.
    fn load_capped(&self, path: &str, max_bytes: usize) -> Result<Option<Vec<u8>>> {
        let bytes = self.load(path)?;
        Ok((bytes.len() <= max_bytes).then_some(bytes))
    }
    fn extract(&self, from: &str, to: &Path) -> Result<()>;
    /// Materialize an embedded resource tree into a caller-owned empty root.
    /// The caller owns the root's temporary lifetime and cleanup.
    fn extract_all(&self, _to: &Path) -> Result<()> {
        Err(anyhow::anyhow!(
            "This resource backend is already filesystem-backed."
        ))
    }
    fn load_icon(&self, path: &str) -> Result<Icon>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResourceRead {
    pub total_len: u64,
    pub bytes: Vec<u8>,
}

#[derive(Debug)]
pub struct FileSystemResource {
    root_dir: PathBuf,
    icon_cache: ArcMut<IconCache>,
}

impl FileSystemResource {
    pub fn new(root_dir: &Path) -> Result<Arc<FileSystemResource>> {
        if std::fs::symlink_metadata(root_dir)?
            .file_type()
            .is_symlink()
        {
            return Err(anyhow::anyhow!("Resource directory cannot be a symlink."));
        }
        let canonical_root = root_dir
            .canonicalize()
            .ok()
            .filter(|path| path.is_dir())
            .ok_or(anyhow::anyhow!("Invalid resource directory."))?;
        Ok(arc(FileSystemResource {
            root_dir: canonical_root,
            icon_cache: arc_mut(HashMap::new()),
        }))
    }

    fn open_file(&self, path: &str) -> Result<std::fs::File> {
        // HTTP resource paths may still be percent-encoded when they reach
        // this layer. Decode before checking components so `%2e%2e` cannot
        // bypass the root boundary.
        let decoded = percent_decode_path(path)?;
        self.open_file_unescaped(&decoded)
    }

    /// The custom protocol already decodes and validates URL paths in
    /// `asset_path`; resolving a literal percent a second time would make
    /// external files differ from embedded resources (and can double-decode).
    fn open_file_unescaped(&self, path: &str) -> Result<std::fs::File> {
        let relative = Path::new(path);
        if relative.is_absolute()
            || path.contains('\\')
            || relative
                .components()
                .any(|component| !matches!(component, std::path::Component::Normal(_)))
        {
            return Err(anyhow::anyhow!("Invalid resource path."));
        }
        let candidate = self.root_dir.join(relative);
        let canonical = candidate.canonicalize()?;
        if !canonical.starts_with(&self.root_dir) || !canonical.is_file() {
            return Err(anyhow::anyhow!("Invalid resource path."));
        }
        let file = std::fs::File::open(&canonical)?;
        let opened = file.metadata()?;
        if !opened.is_file() {
            return Err(anyhow::anyhow!("Invalid resource path."));
        }
        // Open the already-canonical path, then re-check both containment and
        // file identity. If a symlink or path component changed while opening,
        // reject the handle rather than trusting the pre-open path check.
        let after = canonical.canonicalize()?;
        if !after.starts_with(&self.root_dir) || !path_matches_open_file(&file, &after)? {
            return Err(anyhow::anyhow!("Resource changed while opening."));
        }
        Ok(file)
    }
}

#[cfg(unix)]
fn path_matches_open_file(file: &std::fs::File, path: &Path) -> Result<bool> {
    use std::os::unix::fs::MetadataExt;
    let left = file.metadata()?;
    let right = std::fs::metadata(path)?;
    Ok(left.dev() == right.dev() && left.ino() == right.ino())
}

#[cfg(windows)]
fn path_matches_open_file(file: &std::fs::File, path: &Path) -> Result<bool> {
    win_utils::opened_file_matches_path(file, path)
}

#[cfg(not(any(unix, windows)))]
fn path_matches_open_file(file: &std::fs::File, path: &Path) -> Result<bool> {
    let left = file.metadata()?;
    let right = std::fs::metadata(path)?;
    Ok(left.len() == right.len() && left.modified().ok() == right.modified().ok())
}

#[cfg(test)]
mod file_system_resource_tests {
    use super::{AppResourceManager, FileSystemResource, ResourceManager};
    use std::{fs, path::PathBuf, time::SystemTime};

    struct TempDir(PathBuf);

    impl TempDir {
        fn new() -> Self {
            let suffix = SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let path = std::env::temp_dir().join(format!("niva-resource-test-{suffix}"));
            fs::create_dir_all(&path).unwrap();
            Self(path)
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn allows_nested_file_and_rejects_traversal_paths() {
        let temp = TempDir::new();
        let root = temp.0.join("root");
        fs::create_dir_all(root.join("nested")).unwrap();
        fs::write(root.join("nested/file.txt"), b"inside").unwrap();
        fs::write(temp.0.join("outside.txt"), b"outside").unwrap();
        let resources = FileSystemResource::new(&root).unwrap();

        assert_eq!(resources.load("nested/file.txt").unwrap(), b"inside");
        assert!(
            resources
                .load_capped("nested/file.txt", 2)
                .unwrap()
                .is_none()
        );
        assert_eq!(
            resources.load_capped("nested/file.txt", 6).unwrap(),
            Some(b"inside".to_vec())
        );
        assert!(!resources.exists("../outside.txt"));
        assert!(!resources.exists("%2e%2e/outside.txt"));
        assert!(!resources.exists(&temp.0.join("outside.txt").to_string_lossy()));
        assert!(resources.load("../outside.txt").is_err());
    }

    #[test]
    fn directory_resources_expose_the_canonical_root_and_read_bounded_ranges() {
        let temp = TempDir::new();
        let root = temp.0.join("resources");
        fs::create_dir_all(root.join("media")).unwrap();
        fs::write(root.join("media/video.bin"), b"0123456789").unwrap();
        let resources = FileSystemResource::new(&root).unwrap();

        assert_eq!(
            resources.filesystem_root(),
            Some(root.canonicalize().unwrap())
        );
        assert_eq!(
            resources.resource_size("media/video.bin").unwrap(),
            Some(10)
        );
        assert_eq!(
            resources.read_range("media/video.bin", 3, 4).unwrap(),
            super::ResourceRead {
                total_len: 10,
                bytes: b"3456".to_vec(),
            }
        );
        assert!(resources.read_range("media/video.bin", 11, 2).is_err());
    }

    #[test]
    fn rejects_invalid_packed_index_without_panicking() {
        let resources = AppResourceManager {
            indexes: std::collections::HashMap::from([("bad".into(), (2, 10))]),
            compressed_data: vec![1, 2],
            icon_cache: std::sync::Mutex::new(std::collections::HashMap::new()),
        };
        assert!(resources.load("bad").is_err());
        assert!(resources.load_capped("bad", 2).unwrap().is_none());
    }

    #[test]
    fn embedded_resources_materialize_through_streaming_indexed_ranges() {
        use std::io::Write;

        let temp = TempDir::new();
        let output = temp.0.join("resources");
        fs::create_dir(&output).unwrap();
        let raw = b"{}index html bytesvideo bytes";
        let mut encoder =
            flate2::write::DeflateEncoder::new(Vec::new(), flate2::Compression::default());
        encoder.write_all(raw).unwrap();
        let resources = AppResourceManager {
            indexes: std::collections::HashMap::from([
                ("niva.json".into(), (0, 2)),
                ("index.html".into(), (2, 16)),
                ("media/video.bin".into(), (18, 11)),
            ]),
            compressed_data: encoder.finish().unwrap(),
            icon_cache: std::sync::Mutex::new(std::collections::HashMap::new()),
        };

        resources.extract_all(&output).unwrap();
        assert_eq!(fs::read(output.join("niva.json")).unwrap(), b"{}");
        assert_eq!(
            fs::read(output.join("index.html")).unwrap(),
            b"index html bytes"
        );
        assert_eq!(
            fs::read(output.join("media/video.bin")).unwrap(),
            b"video bytes"
        );
    }

    #[test]
    fn embedded_materialization_rejects_encoded_traversal_indices() {
        let temp = TempDir::new();
        let output = temp.0.join("resources");
        fs::create_dir(&output).unwrap();
        let resources = AppResourceManager {
            indexes: std::collections::HashMap::from([("%2e%2e/outside".into(), (0, 1))]),
            compressed_data: vec![1, 2],
            icon_cache: std::sync::Mutex::new(std::collections::HashMap::new()),
        };
        assert!(resources.extract_all(&output).is_err());
        assert_eq!(fs::read_dir(&output).unwrap().count(), 0);
    }

    #[cfg(unix)]
    #[test]
    fn rejects_symlink_escape() {
        use std::os::unix::fs::symlink;

        let temp = TempDir::new();
        let root = temp.0.join("root");
        fs::create_dir_all(&root).unwrap();
        fs::write(temp.0.join("outside.txt"), b"outside").unwrap();
        symlink(temp.0.join("outside.txt"), root.join("escape.txt")).unwrap();
        let resources = FileSystemResource::new(&root).unwrap();

        assert!(!resources.exists("escape.txt"));
        assert!(resources.load("escape.txt").is_err());
        assert!(
            resources
                .extract("escape.txt", &temp.0.join("copy.txt"))
                .is_err()
        );
    }
}

pub(crate) fn percent_decode_path(path: &str) -> Result<String> {
    let bytes = path.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%' {
            if index + 2 >= bytes.len() {
                return Err(anyhow::anyhow!("Invalid resource path encoding."));
            }
            let hex = std::str::from_utf8(&bytes[index + 1..index + 3])?;
            decoded.push(u8::from_str_radix(hex, 16)?);
            index += 3;
        } else {
            decoded.push(bytes[index]);
            index += 1;
        }
    }
    Ok(String::from_utf8(decoded)?)
}

impl ResourceManager for FileSystemResource {
    fn filesystem_root(&self) -> Option<PathBuf> {
        Some(self.root_dir.clone())
    }

    fn exists(&self, path: &str) -> bool {
        self.open_file(path).is_ok()
    }

    fn load(&self, path: &str) -> Result<Vec<u8>> {
        let mut file = self.open_file(path)?;
        let before = file.metadata()?;
        let mut bytes = Vec::with_capacity(usize::try_from(before.len()).unwrap_or(0));
        file.read_to_end(&mut bytes)?;
        ensure_unchanged(&before, &file.metadata()?)?;
        Ok(bytes)
    }

    fn resource_size(&self, path: &str) -> Result<Option<u64>> {
        let file = self.open_file_unescaped(path)?;
        Ok(Some(file.metadata()?.len()))
    }

    fn read_range(&self, path: &str, offset: u64, length: usize) -> Result<ResourceRead> {
        let mut file = self.open_file_unescaped(path)?;
        let before = file.metadata()?;
        if offset > before.len() || length > 32 * 1024 * 1024 {
            return Err(anyhow::anyhow!("Invalid resource range."));
        }
        file.seek(SeekFrom::Start(offset))?;
        let remaining = before.len() - offset;
        let wanted = usize::try_from(remaining.min(length as u64))?;
        let mut bytes = vec![0; wanted];
        file.read_exact(&mut bytes)?;
        ensure_unchanged(&before, &file.metadata()?)?;
        Ok(ResourceRead {
            total_len: before.len(),
            bytes,
        })
    }

    fn load_capped(&self, path: &str, max_bytes: usize) -> Result<Option<Vec<u8>>> {
        let mut file = self.open_file(path)?;
        let before = file.metadata()?;
        if before.len() > max_bytes as u64 {
            return Ok(None);
        }
        let mut bytes = Vec::new();
        Read::take(&mut file, (max_bytes as u64).saturating_add(1)).read_to_end(&mut bytes)?;
        ensure_unchanged(&before, &file.metadata()?)?;
        Ok((bytes.len() <= max_bytes).then_some(bytes))
    }

    fn extract(&self, from: &str, to: &Path) -> Result<()> {
        let mut source = self.open_file(from)?;
        let before = source.metadata()?;
        let decoded = percent_decode_path(from)?;
        let destination = if to.is_dir() {
            to.join(
                Path::new(&decoded)
                    .file_name()
                    .ok_or(anyhow::anyhow!("Invalid resource path."))?,
            )
        } else {
            to.to_path_buf()
        };
        if destination.exists() {
            return Err(anyhow::anyhow!("Destination already exists."));
        }
        if let Some(parent) = destination.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let mut destination_file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&destination)?;
        let copied = (|| -> Result<()> {
            std::io::copy(&mut source, &mut destination_file)?;
            destination_file.flush()?;
            ensure_unchanged(&before, &source.metadata()?)?;
            let _ = destination_file.set_permissions(before.permissions());
            Ok(())
        })();
        if let Err(error) = copied {
            let _ = std::fs::remove_file(&destination);
            return Err(error);
        }
        Ok(())
    }

    fn load_icon(&self, path: &str) -> Result<Icon> {
        // Fast path under lock; file IO + decode happen lock-free, then a
        // double-checked insert (last writer wins, icons are deterministic).
        if let Some(icon) = lock!(self.icon_cache)?.get(path).cloned() {
            return Ok(icon);
        }
        let data = self.load(path)?;
        if !path.ends_with("png") {
            return Err(anyhow::anyhow!("Unsupported icon format."));
        }
        let icon = image_utils::png_to_icon(&data)?;
        lock!(self.icon_cache)?.insert(path.to_string(), icon.clone());
        Ok(icon)
    }
}

fn ensure_unchanged(before: &std::fs::Metadata, after: &std::fs::Metadata) -> Result<()> {
    if before.len() != after.len() || before.modified().ok() != after.modified().ok() {
        return Err(anyhow::anyhow!("Resource changed while reading."));
    }
    Ok(())
}

pub struct AppResourceManager {
    indexes: HashMap<String, (usize, usize)>,
    compressed_data: Vec<u8>,
    icon_cache: Mutex<IconCache>,
}

impl std::fmt::Debug for AppResourceManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MacOSAppResourceManager")
            .field("indexes", &self.indexes)
            .field("compressed_data_bytes", &self.compressed_data.len())
            .finish()
    }
}

impl AppResourceManager {
    #[cfg(target_os = "macos")]
    pub fn new() -> Result<Arc<dyn ResourceManager>> {
        let resources_dir = std::env::current_exe()?
            .parent()
            .ok_or(anyhow::anyhow!("Invalid resource directory."))?
            .join("../Resources/");
        match read_resource_mode(&resources_dir.join("RESOURCE_MODE"))?.as_deref() {
            Some(marker) if marker == b"external" => {
                return FileSystemResource::new(&resources_dir.join("app"))
                    .map(|resources| resources as Arc<dyn ResourceManager>);
            }
            Some(_) => return Err(anyhow::anyhow!("Invalid RESOURCE_MODE marker.")),
            None => {}
        }
        let indexes_data = std::fs::read(resources_dir.join("RESOURCE_INDEXES"))?;
        let indexes = serde_json::from_slice::<HashMap<String, (usize, usize)>>(&indexes_data)?;
        let compressed_data = std::fs::read(resources_dir.join("RESOURCE_DATA"))?;
        Ok(arc(AppResourceManager {
            indexes,
            compressed_data,
            icon_cache: Mutex::new(HashMap::new()),
        }) as Arc<dyn ResourceManager>)
    }

    #[cfg(target_os = "windows")]
    pub fn new() -> Result<Arc<dyn ResourceManager>> {
        use win_utils::load_resource;
        match win_utils::load_resource_optional("RESOURCE_MODE")?.as_deref() {
            Some(marker) if marker == b"external" => {
                let directory = std::env::current_exe()?
                    .parent()
                    .ok_or(anyhow::anyhow!("Invalid resource directory."))?
                    .join("resources");
                return FileSystemResource::new(&directory)
                    .map(|resources| resources as Arc<dyn ResourceManager>);
            }
            Some(_) => return Err(anyhow::anyhow!("Invalid RESOURCE_MODE marker.")),
            None => {}
        }
        let indexes_data = load_resource("RESOURCE_INDEXES")?;
        let indexes = serde_json::from_slice::<HashMap<String, (usize, usize)>>(&indexes_data)?;
        let compressed_data = load_resource("RESOURCE_DATA")?;
        Ok(arc(AppResourceManager {
            indexes,
            compressed_data,
            icon_cache: Mutex::new(HashMap::new()),
        }) as Arc<dyn ResourceManager>)
    }
}

#[cfg(target_os = "macos")]
fn read_resource_mode(path: &Path) -> Result<Option<Vec<u8>>> {
    match std::fs::read(path) {
        std::result::Result::Ok(bytes) => Ok(Some(bytes)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error.into()),
    }
}

impl ResourceManager for AppResourceManager {
    fn exists(&self, path: &str) -> bool {
        self.indexes.contains_key(path)
    }

    fn load(&self, path: &str) -> Result<Vec<u8>> {
        let (archive_offset, length) = self.index(path)?;
        self.read_uncompressed_range(archive_offset, length)
    }

    fn resource_size(&self, path: &str) -> Result<Option<u64>> {
        self.indexes
            .get(path)
            .map(|(_, length)| Ok(*length as u64))
            .transpose()
    }

    fn read_range(&self, path: &str, offset: u64, length: usize) -> Result<ResourceRead> {
        if length > 32 * 1024 * 1024 {
            return Err(anyhow::anyhow!("Resource range exceeds limit."));
        }
        let (archive_offset, resource_length) = self.index(path)?;
        if offset > resource_length {
            return Err(anyhow::anyhow!("Invalid resource range."));
        }
        let length = u64::try_from(length)?.min(resource_length - offset);
        let start = archive_offset
            .checked_add(offset)
            .ok_or(anyhow::anyhow!("Invalid resource index."))?;
        Ok(ResourceRead {
            total_len: resource_length,
            bytes: self.read_uncompressed_range(start, length)?,
        })
    }

    fn load_capped(&self, path: &str, max_bytes: usize) -> Result<Option<Vec<u8>>> {
        let (_, length) = self.index(path)?;
        if length > max_bytes as u64 {
            return Ok(None);
        }
        self.load(path).map(Some)
    }

    fn extract(&self, from: &str, to: &Path) -> Result<()> {
        let (offset, length) = self.index(from)?;
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(to)?;
        copy_uncompressed_range(
            &mut flate2::read::DeflateDecoder::new(&self.compressed_data[..]),
            offset,
            length,
            &mut file,
        )?;
        Ok(())
    }

    fn extract_all(&self, to: &Path) -> Result<()> {
        match std::fs::symlink_metadata(to) {
            Ok(metadata) => {
                ensure!(
                    !metadata.file_type().is_symlink() && metadata.is_dir(),
                    "resource extraction root must be a regular directory"
                );
                ensure!(
                    std::fs::read_dir(to)?.next().is_none(),
                    "resource extraction root must be empty"
                );
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                std::fs::create_dir_all(to)?;
            }
            Err(error) => return Err(error.into()),
        }
        let root = to.canonicalize()?;
        let mut entries = self
            .indexes
            .iter()
            .map(|(path, (offset, length))| {
                let relative = safe_embedded_path(path)?;
                let offset = u64::try_from(*offset)?;
                let length = u64::try_from(*length)?;
                let end = offset
                    .checked_add(length)
                    .ok_or(anyhow::anyhow!("Invalid resource index."))?;
                Ok((offset, end, relative))
            })
            .collect::<Result<Vec<_>>>()?;
        entries.sort_by_key(|(offset, _, _)| *offset);

        let mut decoder = flate2::read::DeflateDecoder::new(&self.compressed_data[..]);
        let mut cursor = 0u64;
        for (offset, end, relative) in entries {
            ensure!(offset >= cursor, "Overlapping resource index entries.");
            skip_uncompressed(&mut decoder, offset - cursor)?;
            let target = root.join(&relative);
            let parent = relative.parent().unwrap_or(Path::new(""));
            let parent = create_safe_parent(&root, parent)?;
            let filename = relative
                .file_name()
                .ok_or(anyhow::anyhow!("Invalid resource index path."))?;
            let destination = parent.join(filename);
            let mut file = std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&destination)?;
            if let Err(error) = copy_uncompressed_range(&mut decoder, 0, end - offset, &mut file) {
                let _ = std::fs::remove_file(&destination);
                return Err(error);
            }
            cursor = end;
            ensure!(
                destination.starts_with(&root) && target == destination,
                "Resource extraction escaped its root."
            );
        }
        let mut trailing = [0u8; 1];
        ensure!(
            decoder.read(&mut trailing)? == 0,
            "Embedded resource data has unindexed trailing bytes."
        );
        Ok(())
    }

    fn load_icon(&self, path: &str) -> Result<Icon> {
        // Fast path under lock; file IO + decode happen lock-free, then a
        // double-checked insert (last writer wins, icons are deterministic).
        if let Some(icon) = lock!(self.icon_cache)?.get(path).cloned() {
            return Ok(icon);
        }
        let data = self.load(path)?;
        if !path.ends_with("png") {
            return Err(anyhow::anyhow!("Unsupported icon format."));
        }
        let icon = image_utils::png_to_icon(&data)?;
        lock!(self.icon_cache)?.insert(path.to_string(), icon.clone());
        Ok(icon)
    }
}

impl AppResourceManager {
    fn index(&self, path: &str) -> Result<(u64, u64)> {
        let (offset, length) = *self
            .indexes
            .get(path)
            .ok_or(anyhow::anyhow!("File not found."))?;
        let offset = u64::try_from(offset)?;
        let length = u64::try_from(length)?;
        offset
            .checked_add(length)
            .ok_or(anyhow::anyhow!("Invalid resource index."))?;
        Ok((offset, length))
    }

    fn read_uncompressed_range(&self, offset: u64, length: u64) -> Result<Vec<u8>> {
        let mut decoder = flate2::read::DeflateDecoder::new(&self.compressed_data[..]);
        let length = usize::try_from(length)?;
        let mut bytes = Vec::with_capacity(length);
        copy_uncompressed_range(&mut decoder, offset, length as u64, &mut bytes)?;
        Ok(bytes)
    }
}

fn copy_uncompressed_range(
    reader: &mut impl Read,
    offset: u64,
    length: u64,
    output: &mut impl Write,
) -> Result<()> {
    skip_uncompressed(reader, offset)?;
    let mut limited = reader.take(length);
    let copied = std::io::copy(&mut limited, output)?;
    ensure!(copied == length, "Embedded resource data is truncated.");
    Ok(())
}

fn skip_uncompressed(reader: &mut impl Read, length: u64) -> Result<()> {
    let mut limited = reader.take(length);
    let skipped = std::io::copy(&mut limited, &mut std::io::sink())?;
    ensure!(skipped == length, "Embedded resource data is truncated.");
    Ok(())
}

fn safe_embedded_path(path: &str) -> Result<PathBuf> {
    let safe = |value: &str| {
        !value.is_empty()
            && !value.contains('\\')
            && !value.contains(':')
            && !Path::new(value).is_absolute()
            && value
                .split('/')
                .all(|part| !part.is_empty() && part != "." && part != "..")
    };
    ensure!(safe(path), "Invalid embedded resource path.");
    let decoded = percent_decode_path(path)?;
    ensure!(
        safe(&decoded),
        "Encoded embedded resource path escapes its root."
    );
    Ok(PathBuf::from(path))
}

fn create_safe_parent(root: &Path, relative: &Path) -> Result<PathBuf> {
    let mut current = root.to_path_buf();
    for component in relative.components() {
        let std::path::Component::Normal(part) = component else {
            return Err(anyhow::anyhow!("Invalid embedded resource parent path."));
        };
        current.push(part);
        match std::fs::symlink_metadata(&current) {
            Ok(metadata) => ensure!(
                !metadata.file_type().is_symlink() && metadata.is_dir(),
                "Resource extraction parent is not a regular directory."
            ),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                std::fs::create_dir(&current)?;
            }
            Err(error) => return Err(error.into()),
        }
    }
    Ok(current)
}
