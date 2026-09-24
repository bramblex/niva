use anyhow::{Context, Result, ensure};
use flate2::{Compression, write::DeflateEncoder};
use serde_json::Value;
use std::{
    collections::{BTreeMap, BTreeSet},
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
pub fn prepare(
    root: &Path,
    config_bytes: &[u8],
    config: &Value,
    compat: Option<&Path>,
) -> Result<Package> {
    ensure!(root.is_dir(), "resource directory does not exist");
    ensure!(
        !fs::symlink_metadata(root)?.file_type().is_symlink(),
        "resource root cannot be a symlink"
    );
    let mut files = BTreeMap::new();
    collect(root, root, &mut files)?;
    files.remove("niva.json");
    if let Some(option) = config
        .get("nodeCompat")
        .filter(|v| !v.is_null() && **v != Value::Bool(false))
    {
        ensure!(
            !files.keys().any(|p| p.starts_with("__niva_compat/")),
            "__niva_compat conflicts with enabled NodeCompat; provide adapters via --node-compat-dir"
        );
        let compat =
            compat.context("nodeCompat is enabled: supply --node-compat-dir or use Devtools")?;
        for relative in compat_files(option)? {
            files.insert(
                format!("__niva_compat/{relative}"),
                safe_file(compat, &relative)?,
            );
        }
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
fn compat_files(option: &Value) -> Result<BTreeSet<String>> {
    let mapping: BTreeMap<String, Vec<String>> =
        serde_json::from_str(include_str!("node-compat-files.json"))?;
    ensure!(
        option == &Value::Bool(true) || option.is_object(),
        "nodeCompat must be true, false or an object"
    );
    if let Some(importmap) = option.get("importmap") {
        ensure!(
            importmap.is_boolean(),
            "nodeCompat.importmap must be boolean"
        );
    }
    let modules = if let Some(modules) = option.get("modules") {
        modules
            .as_array()
            .context("nodeCompat.modules must be an array")?
            .iter()
            .map(|v| {
                v.as_str()
                    .context("module name must be a string")
                    .map(str::to_owned)
            })
            .collect::<Result<Vec<_>>>()?
    } else {
        mapping.keys().cloned().collect()
    };
    let mut files = BTreeSet::from(["node-compat.js".to_owned()]);
    for module in modules {
        files.extend(
            mapping
                .get(&module)
                .with_context(|| format!("unknown NodeCompat module: {module}"))?
                .iter()
                .cloned(),
        );
    }
    Ok(files)
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
        let pkg = prepare(tmp.path(), b"{}", &serde_json::json!({}), None).unwrap();
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
    fn selected_modules_have_import_closure() {
        let files = compat_files(&serde_json::json!({"modules":["fs"]})).unwrap();
        assert!(files.contains("src/runtime/buffer.js"));
        assert!(!files.contains("src/http.js"));
        assert!(compat_files(&serde_json::json!({"modules":["bogus"]})).is_err());
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
        assert!(prepare(&alias, b"{}", &serde_json::json!({}), None).is_err());
        assert!(safe_file(&alias, "index.html").is_err());
    }

    #[cfg(unix)]
    #[test]
    fn symlink_escape_rejected() {
        let tmp = tempfile::tempdir().unwrap();
        std::os::unix::fs::symlink("/etc/hosts", tmp.path().join("escape")).unwrap();
        assert!(prepare(tmp.path(), b"{}", &serde_json::json!({}), None).is_err());
    }
}
