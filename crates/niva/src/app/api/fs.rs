use anyhow::Result;
use glob::Pattern;
use serde::Deserialize;
use serde_json::{Value, json};

use std::{path::Path, time::UNIX_EPOCH};

use crate::app::NivaApp;
use crate::app::api_manager::{ApiManager, ApiRequest, CallContext};
use crate::app::window_manager::window::NivaWindow;
use std::sync::Arc;

pub fn register_api_instances(api_manager: &mut ApiManager) {
    api_manager.register_blocking_api("fs.stat", stat);
    api_manager.register_blocking_api("fs.exists", exists);
    api_manager.register_stream_api("fs.readStream", read_stream);
    api_manager.register_blocking_api("fs.copy", copy);
    api_manager.register_stream_api("fs.writeStream", write_stream);
    api_manager.register_blocking_api("fs.move", move_);
    api_manager.register_blocking_api("fs.remove", remove);
    api_manager.register_blocking_api("fs.createDir", create_dir);
    api_manager.register_blocking_api("fs.createDirAll", create_dir_all);
    api_manager.register_blocking_api("fs.readDir", read_dir);
    api_manager.register_blocking_api("fs.readDirAll", read_dir_all);
}

fn stat(_app: Arc<NivaApp>, _window: Arc<NivaWindow>, request: ApiRequest) -> Result<Value> {
    let (path,) = request.args().get::<(String,)>()?;
    let meta = std::fs::metadata(path)?;

    Ok(json!({
        "isDir": meta.is_dir(),
        "isFile": meta.is_file(),
        "isSymlink": meta.file_type().is_symlink(),
        "size": meta.len(),
        "modified": meta.modified()?.duration_since(UNIX_EPOCH)?.as_millis(),
        "accessed": meta.accessed()?.duration_since(UNIX_EPOCH)?.as_millis(),
        "created": meta.created()?.duration_since(UNIX_EPOCH)?.as_millis(),
    }))
}

fn exists(_app: Arc<NivaApp>, _window: Arc<NivaWindow>, request: ApiRequest) -> Result<bool> {
    let (path,) = request.args().get::<(String,)>()?;
    let path = std::path::Path::new(&path);
    Ok(path.exists())
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CopyOptions {
    pub overwrite: Option<bool>,
    pub skip_exist: Option<bool>,
    pub buffer_size: Option<usize>,
    pub copy_inside: Option<bool>,
    pub content_only: Option<bool>,
    pub depth: Option<u64>,
}

fn _resolve_copy_options(options: Option<CopyOptions>) -> crate::app::fs_ops::CopyOptions {
    use crate::app::fs_ops::CopyOptions;
    match options {
        Some(options) => CopyOptions {
            overwrite: options.overwrite.unwrap_or(false),
            skip_exist: options.skip_exist.unwrap_or(false),
            copy_inside: options.copy_inside.unwrap_or(false),
            content_only: options.content_only.unwrap_or(false),
            depth: options.depth.unwrap_or(0),
        },
        None => CopyOptions::default(),
    }
}

fn move_(_app: Arc<NivaApp>, _window: Arc<NivaWindow>, request: ApiRequest) -> Result<()> {
    use crate::app::fs_ops;
    let (from, to, options) = request
        .args()
        .optional::<(String, String, Option<CopyOptions>)>(3)?;
    let from = std::path::Path::new(&from);
    let to = std::path::Path::new(&to);
    let options = _resolve_copy_options(options);

    if from.is_dir() {
        fs_ops::move_dir(from, to, &options)?;
    } else {
        fs_ops::move_file(from, to, &options)?;
    }
    Ok(())
}

fn copy(_app: Arc<NivaApp>, _window: Arc<NivaWindow>, request: ApiRequest) -> Result<()> {
    use crate::app::fs_ops;
    let (from, to, options) = request
        .args()
        .optional::<(String, String, Option<CopyOptions>)>(3)?;
    let from = std::path::Path::new(&from);
    let to = std::path::Path::new(&to);
    let options = _resolve_copy_options(options);

    if from.is_dir() {
        fs_ops::copy_dir(from, to, &options)?;
    } else {
        fs_ops::copy_file(from, to, &options)?;
    }

    Ok(())
}

fn remove(_app: Arc<NivaApp>, _window: Arc<NivaWindow>, request: ApiRequest) -> Result<()> {
    let (path,) = request.args().get::<(String,)>()?;
    let path = std::path::Path::new(&path);

    crate::app::fs_ops::remove_path(path)?;

    Ok(())
}

fn create_dir(_app: Arc<NivaApp>, _window: Arc<NivaWindow>, request: ApiRequest) -> Result<()> {
    let (path,) = request.args().get::<(String,)>()?;
    std::fs::create_dir(path)?;
    Ok(())
}

fn create_dir_all(_app: Arc<NivaApp>, _window: Arc<NivaWindow>, request: ApiRequest) -> Result<()> {
    let (path,) = request.args().get::<(String,)>()?;
    std::fs::create_dir_all(path)?;
    Ok(())
}

fn read_dir(
    _app: Arc<NivaApp>,
    _window: Arc<NivaWindow>,
    request: ApiRequest,
) -> Result<Vec<String>> {
    let (path,) = request.args().optional::<(Option<String>,)>(1)?;
    let path = path.unwrap_or(".".to_string());

    let mut entries = Vec::new();
    for entry in std::fs::read_dir(path)? {
        let entry = entry?;
        let path = entry.path();
        let file_name = path.file_name().unwrap().to_str().unwrap().to_string();
        entries.push(file_name);
    }
    Ok(entries)
}

fn _visit_dirs(
    dir: &Path,
    prefix: &Path,
    files: &mut Vec<String>,
    excludes: &Vec<Pattern>,
) -> Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let rel_path = path.strip_prefix(prefix).unwrap();

        // 检查是否需要排除当前文件或文件夹
        let skip_entry = excludes.iter().any(|pattern| {
            let path_str = path.to_str().unwrap();
            pattern.matches(path_str)
        });

        if skip_entry {
            continue;
        }

        if path.is_dir() {
            _visit_dirs(&path, prefix, files, excludes)?;
        } else {
            files.push(rel_path.to_str().unwrap().to_string());
        }
    }

    Ok(())
}

fn read_dir_all(
    _app: Arc<NivaApp>,
    _window: Arc<NivaWindow>,
    request: ApiRequest,
) -> Result<Vec<String>> {
    let (path, excludes) = request
        .args()
        .optional::<(String, Option<Vec<String>>)>(2)?;
    let path = Path::new(&path);
    let mut files: Vec<String> = Vec::new();

    let mut exclude_patterns: Vec<Pattern> = vec![];
    if let Some(excludes) = excludes {
        for exclude in excludes {
            let pattern = Pattern::new(&exclude)?;
            exclude_patterns.push(pattern);
        }
    }

    _visit_dirs(path, path, &mut files, &exclude_patterns)?;
    Ok(files)
}

/// Chunked file read: binary chunks stream out (64KB default), terminal
/// result carries `{size}`. For files too big to fit a JSON string.
async fn read_stream(ctx: CallContext, request: ApiRequest) -> Result<()> {
    use std::io::Read;

    let (path, chunk_size): (String, Option<u64>) = request.args().optional(2)?;
    let chunk_size = chunk_size.unwrap_or(65536).clamp(1024, 1 << 20) as usize;

    // Whole loop on the elastic pool: blocking reads, cooperative cancel.
    crate::blocking!({
        let mut file = std::fs::File::open(&path)?;
        let size = file.metadata()?.len();
        let mut buf = vec![0u8; chunk_size];
        loop {
            if ctx.is_cancelled() {
                return Ok(());
            }
            let n = file.read(&mut buf)?;
            if n == 0 {
                break;
            }
            ctx.chunk(&buf[..n], false);
        }
        ctx.chunk(&[], true);
        ctx.respond(Ok(json!({ "size": size })));
        Ok(())
    })
    .await?;
    Ok(())
}

/// Streaming file write: binary chunks arrive from the client, END commits.
/// Terminal result carries `{bytes}`. Replaces unary write/append.
async fn write_stream(ctx: CallContext, request: ApiRequest) -> Result<()> {
    let (path, append): (String, Option<bool>) = request.args().optional(2)?;
    let append = append.unwrap_or(false);

    crate::blocking!({
        use std::io::Write;

        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(!append)
            .append(append)
            .open(&path)?;
        let mut bytes = 0u64;
        while let Some(chunk) = ctx.next_chunk_blocking() {
            if ctx.is_cancelled() {
                return Ok(());
            }
            file.write_all(&chunk.data)?;
            bytes += chunk.data.len() as u64;
            if chunk.end {
                break;
            }
        }
        file.flush()?;
        ctx.respond(Ok(json!({ "bytes": bytes })));
        Ok(())
    })
    .await?;
    Ok(())
}
