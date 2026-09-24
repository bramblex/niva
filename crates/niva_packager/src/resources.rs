use anyhow::{Context, Result, ensure};
use flate2::{Compression, write::DeflateEncoder};
use serde_json::Value;
use std::{
    collections::BTreeMap,
    fs,
    io::Write,
    path::{Component, Path, PathBuf},
};

pub struct Package {
    pub indexes: Vec<u8>,
    pub data: Vec<u8>,
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
pub fn prepare(root: &Path, config_bytes: &[u8], config: &Value) -> Result<Package> {
    ensure!(root.is_dir(), "resource directory does not exist");
    ensure!(
        !fs::symlink_metadata(root)?.file_type().is_symlink(),
        "resource root cannot be a symlink"
    );
    let mut files = BTreeMap::new();
    collect(root, root, &mut files)?;
    files.remove("niva.json");
    if config.get("nodeCompat") != Some(&Value::Bool(false)) {
        ensure!(
            !files.keys().any(|path| path.starts_with("__niva_compat/")),
            "__niva_compat is reserved for the embedded NodeCompat runtime"
        );
    }
    let mut indexes = BTreeMap::new();
    indexes.insert("niva.json".to_owned(), (0usize, config_bytes.len()));
    let mut raw = config_bytes.to_vec();
    for (key, path) in files {
        let bytes = fs::read(path)?;
        indexes.insert(key, (raw.len(), bytes.len()));
        raw.extend(bytes);
    }
    let mut encoder = DeflateEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(&raw)?;
    Ok(Package {
        indexes: serde_json::to_vec(&indexes)?,
        data: encoder.finish()?,
    })
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
        let pkg = prepare(tmp.path(), b"{}", &serde_json::json!({})).unwrap();
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
    fn node_compat_config_is_preserved_without_external_assets() {
        let tmp = tempfile::tempdir().unwrap();
        fs::write(tmp.path().join("index.html"), b"hello").unwrap();

        for config_bytes in [
            br#"{"name":"Default"}"#.as_slice(),
            br#"{"name":"Disabled","nodeCompat":false}"#.as_slice(),
            br#"{"name":"Filtered","nodeCompat":{"modules":["fs","tls"],"importmap":false}}"#
                .as_slice(),
        ] {
            let config: Value = serde_json::from_slice(config_bytes).unwrap();
            let package = prepare(tmp.path(), config_bytes, &config).unwrap();
            let indexes: BTreeMap<String, (usize, usize)> =
                serde_json::from_slice(&package.indexes).unwrap();
            assert!(indexes.contains_key("niva.json"));
            assert!(indexes.contains_key("index.html"));
            assert!(
                !indexes
                    .keys()
                    .any(|path| path.starts_with("__niva_compat/")),
                "NodeCompat assets are provided by the embedded runtime"
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
        assert!(prepare(&alias, b"{}", &serde_json::json!({})).is_err());
        assert!(safe_file(&alias, "index.html").is_err());
    }

    #[cfg(unix)]
    #[test]
    fn symlink_escape_rejected() {
        let tmp = tempfile::tempdir().unwrap();
        std::os::unix::fs::symlink("/etc/hosts", tmp.path().join("escape")).unwrap();
        assert!(prepare(tmp.path(), b"{}", &serde_json::json!({})).is_err());
    }

    #[test]
    fn enabled_node_compat_reserves_its_embedded_resource_prefix() {
        let tmp = tempfile::tempdir().unwrap();
        fs::create_dir(tmp.path().join("__niva_compat")).unwrap();
        fs::write(tmp.path().join("__niva_compat/path.js"), b"wrong").unwrap();

        assert!(prepare(tmp.path(), b"{}", &serde_json::json!({})).is_err());
        let disabled = br#"{"nodeCompat":false}"#;
        let config: Value = serde_json::from_slice(disabled).unwrap();
        assert!(prepare(tmp.path(), disabled, &config).is_ok());
    }
}
