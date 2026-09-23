#[cfg(target_os = "windows")]
mod win_utils;

pub(crate) mod image_utils;

use anyhow::{Ok, Result};
use std::{
    collections::HashMap,
    io::Read,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};
use tao::window::Icon;

use crate::lock;

use super::utils::{ArcMut, arc, arc_mut};

type IconCache = HashMap<String, Icon>;

pub trait ResourceManager: std::fmt::Debug + Send + Sync {
    fn exists(&self, path: &str) -> bool;
    fn load(&self, path: &str) -> Result<Vec<u8>>;
    /// `None` means the resource exceeds the response limit. Implementations
    /// backed by files or an index should reject it before cloning/reading.
    fn load_capped(&self, path: &str, max_bytes: usize) -> Result<Option<Vec<u8>>> {
        let bytes = self.load(path)?;
        Ok((bytes.len() <= max_bytes).then_some(bytes))
    }
    fn extract(&self, from: &str, to: &Path) -> Result<()>;
    fn load_icon(&self, path: &str) -> Result<Icon>;
}

#[derive(Debug)]
pub struct FileSystemResource {
    root_dir: PathBuf,
    icon_cache: ArcMut<IconCache>,
}

impl FileSystemResource {
    pub fn new(root_dir: &Path) -> Result<Arc<FileSystemResource>> {
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

    fn resolve_path(&self, path: &str) -> Result<PathBuf> {
        // HTTP resource paths may still be percent-encoded when they reach
        // this layer. Decode before checking components so `%2e%2e` cannot
        // bypass the root boundary.
        let decoded = percent_decode_path(path)?;
        let relative = Path::new(&decoded);
        if relative.is_absolute()
            || decoded.contains('\\')
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
        Ok(canonical)
    }
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
    fn rejects_invalid_packed_index_without_panicking() {
        let resources = AppResourceManager {
            indexes: std::collections::HashMap::from([("bad".into(), (2, 10))]),
            data: vec![1, 2],
            icon_cache: std::sync::Mutex::new(std::collections::HashMap::new()),
        };
        assert!(resources.load("bad").is_err());
        assert!(resources.load_capped("bad", 2).unwrap().is_none());
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
    fn exists(&self, path: &str) -> bool {
        self.resolve_path(path).is_ok()
    }

    fn load(&self, path: &str) -> Result<Vec<u8>> {
        Ok(std::fs::read(self.resolve_path(path)?)?)
    }

    fn load_capped(&self, path: &str, max_bytes: usize) -> Result<Option<Vec<u8>>> {
        let resolved = self.resolve_path(path)?;
        let file = std::fs::File::open(resolved)?;
        if file.metadata()?.len() > max_bytes as u64 {
            return Ok(None);
        }
        let mut bytes = Vec::new();
        file.take((max_bytes as u64).saturating_add(1))
            .read_to_end(&mut bytes)?;
        Ok((bytes.len() <= max_bytes).then_some(bytes))
    }

    fn extract(&self, from: &str, to: &Path) -> Result<()> {
        use super::fs_ops;
        fs_ops::copy_file(
            &self.resolve_path(from)?,
            to,
            &fs_ops::CopyOptions::default(),
        )?;
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

pub struct AppResourceManager {
    indexes: HashMap<String, (usize, usize)>,
    data: Vec<u8>,
    icon_cache: Mutex<IconCache>,
}

impl std::fmt::Debug for AppResourceManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MacOSAppResourceManager")
            .field("indexes", &self.indexes)
            .field("data", &"Vec<u8>")
            .finish()
    }
}

impl AppResourceManager {
    #[cfg(target_os = "macos")]
    pub fn new() -> Result<Arc<AppResourceManager>> {
        let resources_dir = std::env::current_exe()?
            .parent()
            .ok_or(anyhow::anyhow!("Invalid resource directory."))?
            .join("../Resources/");
        let indexes_data = std::fs::read(resources_dir.join("RESOURCE_INDEXES"))?;
        let indexes = serde_json::from_slice::<HashMap<String, (usize, usize)>>(&indexes_data)?;
        let compressed_data = std::fs::read(resources_dir.join("RESOURCE_DATA"))?;
        let mut decoder = flate2::read::DeflateDecoder::new(&compressed_data[..]);
        let mut data = Vec::new();
        decoder.read_to_end(&mut data)?;
        Ok(arc(AppResourceManager {
            indexes,
            data,
            icon_cache: Mutex::new(HashMap::new()),
        }))
    }

    #[cfg(target_os = "windows")]
    pub fn new() -> Result<Arc<AppResourceManager>> {
        use win_utils::load_resource;
        let indexes_data = load_resource("RESOURCE_INDEXES")?;
        let indexes = serde_json::from_slice::<HashMap<String, (usize, usize)>>(&indexes_data)?;
        let compressed_data = load_resource("RESOURCE_DATA")?;
        let mut decoder = flate2::read::DeflateDecoder::new(&compressed_data[..]);
        let mut data = Vec::new();
        decoder.read_to_end(&mut data)?;
        Ok(arc(AppResourceManager {
            indexes,
            data,
            icon_cache: Mutex::new(HashMap::new()),
        }))
    }
}

impl ResourceManager for AppResourceManager {
    fn exists(&self, path: &str) -> bool {
        self.indexes.contains_key(path)
    }

    fn load(&self, path: &str) -> Result<Vec<u8>> {
        let (offset, length) = *self
            .indexes
            .get(path)
            .ok_or(anyhow::anyhow!("File not found."))?;
        let end = offset
            .checked_add(length)
            .ok_or(anyhow::anyhow!("Invalid resource index."))?;
        Ok(self
            .data
            .get(offset..end)
            .ok_or(anyhow::anyhow!("Invalid resource index."))?
            .to_vec())
    }

    fn load_capped(&self, path: &str, max_bytes: usize) -> Result<Option<Vec<u8>>> {
        let (_, length) = *self
            .indexes
            .get(path)
            .ok_or(anyhow::anyhow!("File not found."))?;
        if length > max_bytes {
            return Ok(None);
        }
        self.load(path).map(Some)
    }

    fn extract(&self, from: &str, to: &Path) -> Result<()> {
        let content = self.load(from)?;
        std::fs::write(to, content)?;
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
