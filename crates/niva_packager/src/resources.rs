use anyhow::{Context, Result, ensure};
use flate2::{Compression, write::DeflateEncoder};
use serde_json::Value;
use std::{
    collections::BTreeMap,
    fs,
    io::Write,
    path::{Component, Path, PathBuf},
};

#[derive(Debug)]
pub struct Package {
    pub indexes: Vec<u8>,
    pub data: Vec<u8>,
    /// Populated only for an external layout. Files are first copied through
    /// checked handles into this immutable per-build staging snapshot, then
    /// streamed from it into each target package.
    pub external_root: Option<PathBuf>,
}
// Reject symlinks rather than accidentally packaging files outside the project.
// Checking each component also prevents a symlinked directory from escaping.
pub fn safe_file(root: &Path, relative: &str) -> Result<PathBuf> {
    ensure!(
        !fs::symlink_metadata(root)?.file_type().is_symlink(),
        "resource root cannot be a symlink"
    );
    ensure!(
        !relative.is_empty() && !relative.contains('\\') && !relative.contains(':'),
        "invalid relative resource path: {relative}"
    );
    let path = Path::new(relative);
    ensure!(
        path.components().all(|c| matches!(c, Component::Normal(_))),
        "resource path must stay inside its root: {relative}"
    );
    let mut current = root.to_path_buf();
    for part in path.components() {
        current.push(part);
        ensure!(
            !fs::symlink_metadata(&current)
                .with_context(|| format!("missing resource: {}", current.display()))?
                .file_type()
                .is_symlink(),
            "symlink resources are not supported: {}",
            current.display()
        );
    }
    ensure!(
        current.is_file(),
        "resource is not a file: {}",
        current.display()
    );
    Ok(current)
}
fn collect(root: &Path, dir: &Path, out: &mut BTreeMap<String, PathBuf>) -> Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let kind = entry.file_type()?;
        ensure!(
            !kind.is_symlink(),
            "symlink resources are not supported: {}",
            path.display()
        );
        if kind.is_dir() {
            collect(root, &path, out)?;
        } else {
            ensure!(kind.is_file(), "resource must be a regular file");
            let relative = path
                .strip_prefix(root)?
                .components()
                .map(|c| {
                    c.as_os_str()
                        .to_str()
                        .context("resource name must be UTF-8")
                })
                .collect::<Result<Vec<_>>>()?
                .join("/");
            safe_file(root, &relative)?;
            out.insert(relative, path);
        }
    }
    Ok(())
}
pub fn prepare(
    root: &Path,
    config_bytes: &[u8],
    _config: &Value,
    external: bool,
    staging_root: &Path,
    excluded_files: &[String],
    license_root: Option<&Path>,
) -> Result<Package> {
    ensure!(root.is_dir(), "resource directory does not exist");
    ensure!(
        !fs::symlink_metadata(root)?.file_type().is_symlink(),
        "resource root cannot be a symlink"
    );
    let mut files = BTreeMap::new();
    collect(root, root, &mut files)?;
    files.remove("niva.json");
    for excluded in excluded_files {
        let matching = files
            .keys()
            .find(|path| path.eq_ignore_ascii_case(excluded))
            .cloned();
        if let Some(matching) = matching {
            files.remove(&matching);
        }
    }
    let mut source_roots: BTreeMap<String, PathBuf> = files
        .keys()
        .map(|path| (path.clone(), root.to_path_buf()))
        .collect();
    let mut source_relatives: BTreeMap<String, String> = files
        .keys()
        .map(|path| (path.clone(), path.clone()))
        .collect();
    if let Some(license_root) = license_root {
        let license_files = collect_license_files(license_root)?;
        ensure!(
            !files
                .keys()
                .any(|path| path.to_ascii_lowercase().starts_with("meta-inf/niva/")),
            "META-INF/niva is reserved for Niva runtime license notices"
        );
        for (path, source) in license_files {
            let source_relative = source
                .strip_prefix(license_root)?
                .to_str()
                .context("license path must be UTF-8")?
                .replace(std::path::MAIN_SEPARATOR, "/");
            source_roots.insert(path.clone(), license_root.to_path_buf());
            source_relatives.insert(path.clone(), source_relative);
            files.insert(path, source);
        }
    }
    ensure!(
        !files
            .keys()
            .any(|path| path.to_ascii_lowercase().starts_with("__niva_runtime/")),
        "__niva_runtime is reserved for the embedded JavaScript runtime"
    );
    if external {
        let external_root = staging_root.join("resource-snapshot");
        ensure!(!external_root.exists(), "resource snapshot already exists");
        fs::create_dir_all(&external_root)?;
        fs::write(external_root.join("niva.json"), config_bytes)?;
        for (relative, source) in files {
            let source_root = source_roots
                .get(&relative)
                .context("resource source root missing")?;
            let source_relative = source_relatives
                .get(&relative)
                .context("resource source path missing")?;
            copy_checked_resource(
                source_root,
                source_relative,
                &source,
                &external_root.join(&relative),
            )?;
        }
        return Ok(Package {
            indexes: Vec::new(),
            data: Vec::new(),
            external_root: Some(external_root),
        });
    }

    let mut indexes = BTreeMap::new();
    indexes.insert("niva.json".to_owned(), (0usize, config_bytes.len()));
    let mut raw = config_bytes.to_vec();
    for (key, path) in files {
        let source_root = source_roots
            .get(&key)
            .context("resource source root missing")?;
        let source_relative = source_relatives
            .get(&key)
            .context("resource source path missing")?;
        let bytes = read_checked_resource(source_root, source_relative, &path)?;
        indexes.insert(key, (raw.len(), bytes.len()));
        raw.extend(bytes);
    }
    let mut encoder = DeflateEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(&raw)?;
    Ok(Package {
        indexes: serde_json::to_vec(&indexes)?,
        data: encoder.finish()?,
        external_root: None,
    })
}

pub fn validate_licenses(root: &Path) -> Result<()> {
    let files = collect_license_files(root)?;
    ensure!(!files.is_empty(), "build kit has no license materials");
    Ok(())
}

fn collect_license_files(root: &Path) -> Result<BTreeMap<String, PathBuf>> {
    for name in ["LICENSE", "THIRD_PARTY.txt"] {
        safe_file(root, name).with_context(|| format!("build kit is missing {name}"))?;
    }
    let license_dir = root.join("licenses");
    ensure!(
        !fs::symlink_metadata(&license_dir)?.file_type().is_symlink() && license_dir.is_dir(),
        "build kit licenses must be a regular directory"
    );
    let mut files = BTreeMap::new();
    files.insert("META-INF/niva/LICENSE".to_owned(), root.join("LICENSE"));
    files.insert(
        "META-INF/niva/THIRD_PARTY.txt".to_owned(),
        root.join("THIRD_PARTY.txt"),
    );
    let mut dependency_files = BTreeMap::new();
    collect(root, &license_dir, &mut dependency_files)?;
    ensure!(
        !dependency_files.is_empty(),
        "build kit licenses directory is empty"
    );
    for (relative, source) in dependency_files {
        files.insert(format!("META-INF/niva/{relative}"), source);
    }
    Ok(files)
}

/// Copy a completed external resource snapshot into a fresh package directory.
/// Files are copied with bounded I/O; large media is never collected into a
/// process-sized `Vec` and remains stored uncompressed by the outer ZIP.
pub fn copy_external_resources(package: &Package, destination: &Path) -> Result<()> {
    let source_root = package
        .external_root
        .as_deref()
        .context("external resources were not prepared")?;
    ensure!(
        !destination.exists(),
        "external resource destination already exists"
    );
    fs::create_dir_all(destination)?;
    let mut files = BTreeMap::new();
    collect(source_root, source_root, &mut files)?;
    for (relative, source) in files {
        let target = destination.join(&relative);
        let parent = target.parent().context("resource target has no parent")?;
        fs::create_dir_all(parent)?;
        let input = fs::File::open(source)?;
        let mut output = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(target)?;
        let mut input = input;
        std::io::copy(&mut input, &mut output)?;
        output.sync_all()?;
    }
    Ok(())
}

fn read_checked_resource(root: &Path, relative: &str, source: &Path) -> Result<Vec<u8>> {
    let mut file = open_checked_resource(root, relative, source)?;
    let before = file.metadata()?;
    let mut bytes = Vec::with_capacity(usize::try_from(before.len()).unwrap_or(0));
    std::io::Read::read_to_end(&mut file, &mut bytes)?;
    ensure_source_unchanged(root, relative, source, &file, &before, &file.metadata()?)?;
    Ok(bytes)
}

fn copy_checked_resource(root: &Path, relative: &str, source: &Path, target: &Path) -> Result<()> {
    let mut file = open_checked_resource(root, relative, source)?;
    let before = file.metadata()?;
    let parent = target.parent().context("resource target has no parent")?;
    fs::create_dir_all(parent)?;
    let mut output = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(target)?;
    let copied = std::io::copy(&mut file, &mut output)?;
    output.sync_all()?;
    ensure!(
        copied == before.len(),
        "resource changed while copying: {relative}"
    );
    ensure_source_unchanged(root, relative, source, &file, &before, &file.metadata()?)?;
    Ok(())
}

pub fn snapshot_file(root: &Path, relative: &str, destination: &Path) -> Result<()> {
    let source = safe_file(root, relative)?;
    copy_checked_resource(root, relative, &source, destination)
}

fn open_checked_resource(root: &Path, relative: &str, source: &Path) -> Result<fs::File> {
    let canonical_root = root.canonicalize()?;
    let canonical = safe_file(root, relative)?.canonicalize()?;
    ensure!(
        canonical.starts_with(&canonical_root),
        "resource escaped its root: {relative}"
    );
    let file = fs::File::open(&canonical)?;
    let opened = file.metadata()?;
    ensure!(
        opened.is_file(),
        "resource is not a regular file: {relative}"
    );
    ensure_source_unchanged(
        root,
        relative,
        source,
        &file,
        &opened,
        &fs::metadata(&canonical)?,
    )?;
    Ok(file)
}

fn ensure_source_unchanged(
    root: &Path,
    relative: &str,
    source: &Path,
    file: &fs::File,
    opened: &fs::Metadata,
    current: &fs::Metadata,
) -> Result<()> {
    ensure!(
        opened.len() == current.len() && opened.modified().ok() == current.modified().ok(),
        "resource changed while reading: {relative}"
    );
    let canonical_root = root.canonicalize()?;
    let current_path = source.canonicalize()?;
    ensure!(
        current_path.starts_with(canonical_root),
        "resource escaped its root while reading: {relative}"
    );
    ensure!(
        win_packager::opened_file_matches_path(file, &current_path)?,
        "resource path changed while reading: {relative}"
    );
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Read;
    #[test]
    fn explicit_config_wins_and_resources_roundtrip() {
        let tmp = tempfile::tempdir().unwrap();
        fs::write(tmp.path().join("niva.json"), b"wrong").unwrap();
        fs::write(tmp.path().join("page.html"), b"hello").unwrap();
        let pkg = prepare(
            tmp.path(),
            b"{}",
            &serde_json::json!({}),
            false,
            tmp.path(),
            &[],
            None,
        )
        .unwrap();
        let indexes: BTreeMap<String, (usize, usize)> =
            serde_json::from_slice(&pkg.indexes).unwrap();
        let mut raw = Vec::new();
        flate2::read::DeflateDecoder::new(pkg.data.as_slice())
            .read_to_end(&mut raw)
            .unwrap();
        assert_eq!(&raw[..2], b"{}");
        let (offset, len) = indexes["page.html"];
        assert_eq!(&raw[offset..offset + len], b"hello");
    }
    #[test]
    fn traversal_rejected() {
        for p in ["../a", "/tmp/a", "a\\b", "C:/x", "a/../b"] {
            assert!(safe_file(Path::new("."), p).is_err());
        }
    }
    #[test]
    fn runtime_injection_config_is_preserved_without_external_assets() {
        let tmp = tempfile::tempdir().unwrap();
        fs::write(tmp.path().join("index.html"), b"hello").unwrap();

        for config_bytes in [
            br#"{"name":"Default"}"#.as_slice(),
            br#"{"name":"Disabled","injectCommonJs":false,"injectEsm":false}"#.as_slice(),
            br#"{"name":"Injected","injectCommonJs":true,"injectEsm":true}"#.as_slice(),
        ] {
            let config: Value = serde_json::from_slice(config_bytes).unwrap();
            let package = prepare(
                tmp.path(),
                config_bytes,
                &config,
                false,
                tmp.path(),
                &[],
                None,
            )
            .unwrap();
            let indexes: BTreeMap<String, (usize, usize)> =
                serde_json::from_slice(&package.indexes).unwrap();
            assert!(indexes.contains_key("niva.json"));
            assert!(indexes.contains_key("index.html"));
            assert!(
                !indexes
                    .keys()
                    .any(|path| path.starts_with("__niva_runtime/")),
                "JavaScript runtime assets are provided by the embedded runtime"
            );
            let (offset, length) = indexes["niva.json"];
            let mut raw = Vec::new();
            flate2::read::DeflateDecoder::new(package.data.as_slice())
                .read_to_end(&mut raw)
                .unwrap();
            assert_eq!(&raw[offset..offset + length], config_bytes);
        }
    }
    #[cfg(unix)]
    #[test]
    fn symlink_root_rejected() {
        let tmp = tempfile::tempdir().unwrap();
        let real = tmp.path().join("real");
        fs::create_dir(&real).unwrap();
        fs::write(real.join("index.html"), b"hello").unwrap();
        let alias = tmp.path().join("alias");
        std::os::unix::fs::symlink(&real, &alias).unwrap();
        assert!(
            prepare(
                &alias,
                b"{}",
                &serde_json::json!({}),
                false,
                tmp.path(),
                &[],
                None
            )
            .is_err()
        );
        assert!(safe_file(&alias, "index.html").is_err());
    }

    #[cfg(unix)]
    #[test]
    fn symlink_escape_rejected() {
        let tmp = tempfile::tempdir().unwrap();
        std::os::unix::fs::symlink("/etc/hosts", tmp.path().join("escape")).unwrap();
        assert!(
            prepare(
                tmp.path(),
                b"{}",
                &serde_json::json!({}),
                false,
                tmp.path(),
                &[],
                None
            )
            .is_err()
        );
    }

    #[test]
    fn runtime_assets_always_reserve_their_embedded_resource_prefix() {
        let tmp = tempfile::tempdir().unwrap();
        fs::create_dir_all(tmp.path().join("__niva_runtime/dist")).unwrap();
        fs::write(
            tmp.path().join("__niva_runtime/dist/bootstrap.js"),
            b"wrong",
        )
        .unwrap();

        assert!(
            prepare(
                tmp.path(),
                b"{}",
                &serde_json::json!({}),
                false,
                tmp.path(),
                &[],
                None
            )
            .is_err()
        );
    }

    #[test]
    fn external_snapshot_keeps_large_resources_out_of_the_package_vectors_and_carries_licenses() {
        let temp = tempfile::tempdir().unwrap();
        let project = temp.path().join("project");
        let kit = temp.path().join("kit");
        let staging = temp.path().join("staging");
        fs::create_dir_all(&project).unwrap();
        fs::create_dir_all(kit.join("licenses/runtime")).unwrap();
        fs::create_dir_all(&staging).unwrap();
        fs::write(project.join("niva.json"), b"stale config").unwrap();
        fs::write(project.join("index.html"), b"page").unwrap();
        let large = fs::File::create(project.join("media.bin")).unwrap();
        large.set_len(16 * 1024 * 1024).unwrap();
        fs::write(kit.join("LICENSE"), b"niva").unwrap();
        fs::write(kit.join("THIRD_PARTY.txt"), b"notices").unwrap();
        fs::write(kit.join("licenses/runtime/LICENSE"), b"runtime dependency").unwrap();

        let package = prepare(
            &project,
            br#"{"name":"Explicit"}"#,
            &serde_json::json!({"name":"Explicit"}),
            true,
            &staging,
            &[],
            Some(&kit),
        )
        .unwrap();
        assert!(package.indexes.is_empty());
        assert!(package.data.is_empty());
        let snapshot = package.external_root.as_ref().unwrap();
        assert_eq!(
            fs::metadata(snapshot.join("media.bin")).unwrap().len(),
            16 * 1024 * 1024
        );
        assert_eq!(
            fs::read(snapshot.join("niva.json")).unwrap(),
            br#"{"name":"Explicit"}"#
        );
        assert_eq!(
            fs::read(snapshot.join("META-INF/niva/THIRD_PARTY.txt")).unwrap(),
            b"notices"
        );

        let output = temp.path().join("external");
        copy_external_resources(&package, &output).unwrap();
        assert_eq!(
            fs::metadata(output.join("media.bin")).unwrap().len(),
            16 * 1024 * 1024
        );
        assert_eq!(
            fs::read(output.join("META-INF/niva/licenses/runtime/LICENSE")).unwrap(),
            b"runtime dependency"
        );
    }

    #[test]
    fn build_kit_requires_and_reserves_license_material_paths() {
        let temp = tempfile::tempdir().unwrap();
        let project = temp.path().join("project");
        let kit = temp.path().join("kit");
        fs::create_dir_all(project.join("META-INF/niva")).unwrap();
        fs::create_dir_all(kit.join("licenses")).unwrap();
        fs::write(project.join("META-INF/niva/LICENSE"), b"user").unwrap();
        fs::write(kit.join("LICENSE"), b"niva").unwrap();
        fs::write(kit.join("THIRD_PARTY.txt"), b"notices").unwrap();
        fs::write(kit.join("licenses/LICENSE"), b"dependency").unwrap();
        let error = prepare(
            &project,
            b"{}",
            &serde_json::json!({}),
            false,
            temp.path(),
            &[],
            Some(&kit),
        )
        .unwrap_err();
        assert!(error.to_string().contains("META-INF/niva is reserved"));
        assert!(validate_licenses(&kit).is_ok());
        assert!(validate_licenses(&project).is_err());
    }
}
