//! Version 2 block-indexed resources, shared with the production packager and runtime.
use anyhow::Result;
use std::path::{Path, PathBuf};

/// exe 内 `RCDATA` 资源名：索引表。
pub const INDEXES_NAME: &str = "RESOURCE_INDEXES";
/// exe 内 `RCDATA` 资源名：压缩数据。
pub const DATA_NAME: &str = "RESOURCE_DATA";
/// 配置文件在包内的固定 key。
pub const CONFIG_KEY: &str = "niva.json";

pub struct ResourcePackage {
    pub indexes_json: Vec<u8>,
    pub data_deflated: Vec<u8>,
}

/// 打包一次：`config_file` 首包为 `"niva.json"`，再递归收 `resource_dir` 下所有文件。
pub fn bundle_resource_package(resource_dir: &Path, config_file: &Path) -> Result<ResourcePackage> {
    let mut index = crate::resource_archive::Index::new();
    let mut data_deflated = Vec::new();
    let mut push = |key: String, bytes: &[u8]| -> Result<()> {
        index.insert(
            key,
            crate::resource_archive::append(&mut data_deflated, bytes)?,
        );
        Ok(())
    };

    push(
        CONFIG_KEY.to_string(),
        &std::fs::read(config_file)
            .map_err(|e| anyhow::anyhow!("read config {}: {e}", config_file.display()))?,
    )?;

    let mut files = Vec::new();
    collect_files(resource_dir, resource_dir, &mut files)?;
    files.sort();
    for rel in &files {
        // The explicit config_file is authoritative. A build directory may
        // also contain niva.json; packing it again would replace the index
        // entry with an unrelated file while silently wasting the first copy.
        if rel == CONFIG_KEY {
            continue;
        }
        let bytes = std::fs::read(resource_dir.join(rel))
            .map_err(|e| anyhow::anyhow!("read resource {rel}: {e}"))?;
        push(rel.clone(), &bytes)?;
    }

    let indexes_json = crate::resource_archive::encode_index(&index)?;

    Ok(ResourcePackage {
        indexes_json,
        data_deflated,
    })
}

fn collect_files(root: &Path, dir: &Path, out: &mut Vec<String>) -> Result<()> {
    for entry in
        std::fs::read_dir(dir).map_err(|e| anyhow::anyhow!("read dir {}: {e}", dir.display()))?
    {
        let path = entry?.path();
        if path.is_dir() {
            collect_files(root, &path, out)?;
        } else if path.is_file() {
            let rel: PathBuf = path
                .strip_prefix(root)
                .map_err(|e| anyhow::anyhow!("{e}"))?
                .into();
            out.push(rel.to_string_lossy().replace('\\', "/"));
        }
    }
    Ok(())
}
