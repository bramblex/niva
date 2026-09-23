//! 前端资源打包：复刻 `packages/devtools/src/build-scripts/base.ts`。
//!
//! 包格式（与读侧 `crates/niva/src/app/resource_manager/mod.rs` 对应）：
//! - 首包永远是配置文件，包内 key 固定为 `"niva.json"`；
//! - 其余文件按相对路径为 key，`\` 统一换成 `/`（与前端
//!   `name.replace(/\\/g, "/")` 一致）；
//! - `INDEXES = JSON { path: [offset, length] }`，
//!   `DATA = deflate_raw(文件拼接)`（`pako.deflateRaw` 对应
//!   `flate2` 的 `DeflateEncoder`，读侧 `DeflateDecoder`）。
//!
//! 与前端的两处有意差异（均不影响读侧，`serde_json` 对空白不敏感）：
//! 1. 遍历顺序：前端是 `readdir` 原生顺序（不确定），这里排序后打包，
//!    产物确定、可复现；`offset` 都记在 index 里，顺序不影响读取。
//! 2. `INDEXES` 用紧凑 JSON（前端是 2 空格 pretty），包更小。

use anyhow::Result;
use std::collections::BTreeMap;
use std::io::Write;
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
    let mut index: BTreeMap<String, (usize, usize)> = BTreeMap::new();
    let mut raw: Vec<u8> = Vec::new();

    let mut push = |key: String, bytes: &[u8]| {
        index.insert(key, (raw.len(), bytes.len()));
        raw.extend_from_slice(bytes);
    };

    push(
        CONFIG_KEY.to_string(),
        &std::fs::read(config_file)
            .map_err(|e| anyhow::anyhow!("read config {}: {e}", config_file.display()))?,
    );

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
        push(rel.clone(), &bytes);
    }

    let indexes_json = serde_json::to_vec(&index)?;
    let mut enc = flate2::write::DeflateEncoder::new(Vec::new(), flate2::Compression::default());
    enc.write_all(&raw)?;
    let data_deflated = enc.finish()?;

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
