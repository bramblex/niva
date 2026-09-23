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
    stat_path(Path::new(&path))
}

fn stat_path(path: &Path) -> Result<Value> {
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
    Ok(path_exists(Path::new(&path)))
}

fn path_exists(path: &Path) -> bool {
    path.exists()
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
    let (from, to, options) = request
        .args()
        .optional::<(String, String, Option<CopyOptions>)>(3)?;
    move_paths(Path::new(&from), Path::new(&to), options)
}

fn move_paths(from: &Path, to: &Path, options: Option<CopyOptions>) -> Result<()> {
    use crate::app::fs_ops;
    let options = _resolve_copy_options(options);
    if from.is_dir() {
        fs_ops::move_dir(from, to, &options)?;
    } else {
        fs_ops::move_file(from, to, &options)?;
    }
    Ok(())
}

fn copy(_app: Arc<NivaApp>, _window: Arc<NivaWindow>, request: ApiRequest) -> Result<()> {
    let (from, to, options) = request
        .args()
        .optional::<(String, String, Option<CopyOptions>)>(3)?;
    copy_paths(Path::new(&from), Path::new(&to), options)
}

fn copy_paths(from: &Path, to: &Path, options: Option<CopyOptions>) -> Result<()> {
    use crate::app::fs_ops;
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
    remove_path(Path::new(&path))
}

fn remove_path(path: &Path) -> Result<()> {
    crate::app::fs_ops::remove_path(path)
}

fn create_dir(_app: Arc<NivaApp>, _window: Arc<NivaWindow>, request: ApiRequest) -> Result<()> {
    let (path,) = request.args().get::<(String,)>()?;
    create_directory(Path::new(&path))
}

fn create_directory(path: &Path) -> Result<()> {
    std::fs::create_dir(path)?;
    Ok(())
}

fn create_dir_all(_app: Arc<NivaApp>, _window: Arc<NivaWindow>, request: ApiRequest) -> Result<()> {
    let (path,) = request.args().get::<(String,)>()?;
    create_directories(Path::new(&path))
}

fn create_directories(path: &Path) -> Result<()> {
    std::fs::create_dir_all(path)?;
    Ok(())
}

fn read_dir(
    _app: Arc<NivaApp>,
    _window: Arc<NivaWindow>,
    request: ApiRequest,
) -> Result<Vec<String>> {
    let (path,) = request.args().optional::<(Option<String>,)>(1)?;
    read_directory(path.as_deref().map(Path::new).unwrap_or(Path::new(".")))
}

fn read_directory(path: &Path) -> Result<Vec<String>> {
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
    read_directory_all(Path::new(&path), excludes)
}

fn read_directory_all(path: &Path, excludes: Option<Vec<String>>) -> Result<Vec<String>> {
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
    let (path, chunk_size): (String, Option<u64>) = request.args().optional(2)?;
    let chunk_size = chunk_size.unwrap_or(65536).clamp(1024, 1 << 20) as usize;

    // Whole loop on the elastic pool: blocking reads, cooperative cancel.
    crate::blocking!({
        let size = read_file_chunks(
            Path::new(&path),
            chunk_size,
            || ctx.is_cancelled(),
            |data, end| ctx.chunk(data, end),
        )?;
        if let Some(size) = size {
            ctx.respond(Ok(json!({ "size": size })));
        }
        Ok(())
    })
    .await?;
    Ok(())
}

fn read_file_chunks(
    path: &Path,
    chunk_size: usize,
    mut is_cancelled: impl FnMut() -> bool,
    mut emit_chunk: impl FnMut(&[u8], bool),
) -> Result<Option<u64>> {
    use std::io::Read;

    let mut file = std::fs::File::open(path)?;
    let size = file.metadata()?.len();
    let mut buf = vec![0u8; chunk_size];
    loop {
        if is_cancelled() {
            return Ok(None);
        }
        let n = file.read(&mut buf)?;
        if n == 0 {
            break;
        }
        emit_chunk(&buf[..n], false);
    }
    emit_chunk(&[], true);
    Ok(Some(size))
}

/// Streaming file write: binary chunks arrive from the client, END commits.
/// Terminal result carries `{bytes}`. Replaces unary write/append.
async fn write_stream(ctx: CallContext, request: ApiRequest) -> Result<()> {
    let (path, append): (String, Option<bool>) = request.args().optional(2)?;
    let append = append.unwrap_or(false);

    crate::blocking!({
        let bytes = write_file_chunks(
            Path::new(&path),
            append,
            || ctx.next_chunk_blocking(),
            || ctx.is_cancelled(),
        )?;
        if let Some(bytes) = bytes {
            ctx.respond(Ok(json!({ "bytes": bytes })));
        }
        Ok(())
    })
    .await?;
    Ok(())
}

fn write_file_chunks(
    path: &Path,
    append: bool,
    mut next_chunk: impl FnMut() -> Option<crate::app::api_manager::InboundChunk>,
    mut is_cancelled: impl FnMut() -> bool,
) -> Result<Option<u64>> {
    use std::io::Write;

    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(!append)
        .append(append)
        .open(path)?;
    let mut bytes = 0u64;
    while let Some(chunk) = next_chunk() {
        if is_cancelled() {
            return Ok(None);
        }
        file.write_all(&chunk.data)?;
        bytes += chunk.data.len() as u64;
        if chunk.end {
            break;
        }
    }
    file.flush()?;
    Ok(Some(bytes))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::api_manager::InboundChunk;
    use serde_json::json;
    use std::{
        path::PathBuf,
        sync::atomic::{AtomicU64, Ordering},
    };

    static NEXT_TEMP_DIR: AtomicU64 = AtomicU64::new(0);

    struct TestDir(PathBuf);

    impl TestDir {
        fn new() -> Self {
            let id = NEXT_TEMP_DIR.fetch_add(1, Ordering::Relaxed);
            let path =
                std::env::temp_dir().join(format!("niva-fs-api-{}-{id}", std::process::id()));
            let _ = std::fs::remove_dir_all(&path);
            std::fs::create_dir_all(&path).unwrap();
            Self(path)
        }

        fn path(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for TestDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn inbound(data: &[u8], end: bool) -> InboundChunk {
        InboundChunk {
            seq: 1,
            data: data.to_vec(),
            end,
        }
    }

    #[test]
    fn stat_and_exists_cover_files_directories_and_missing_paths() {
        let temp = TestDir::new();
        let file = temp.path().join("note.txt");
        std::fs::write(&file, "hello").unwrap();

        let info = stat_path(&file).unwrap();
        assert_eq!(info["isFile"], true);
        assert_eq!(info["isDir"], false);
        assert_eq!(info["isSymlink"], false);
        assert_eq!(info["size"], 5);
        for field in ["modified", "accessed", "created"] {
            assert!(
                info[field].as_u64().is_some(),
                "{field} should be a timestamp"
            );
        }

        let dir_info = stat_path(temp.path()).unwrap();
        assert_eq!(dir_info["isDir"], true);
        assert_eq!(dir_info["isFile"], false);
        assert!(path_exists(&file));
        assert!(path_exists(temp.path()));
        assert!(!path_exists(&temp.path().join("absent")));
        assert!(stat_path(&temp.path().join("absent")).is_err());
    }

    #[test]
    fn copy_options_apply_camel_case_fields_and_defaults() {
        let options: CopyOptions = serde_json::from_value(json!({
            "overwrite": true,
            "skipExist": true,
            "bufferSize": 4096,
            "copyInside": true,
            "contentOnly": true,
            "depth": 3
        }))
        .unwrap();
        let options = _resolve_copy_options(Some(options));
        assert!(options.overwrite);
        assert!(options.skip_exist);
        assert!(options.copy_inside);
        assert!(options.content_only);
        assert_eq!(options.depth, 3);

        let defaults = _resolve_copy_options(None);
        assert!(!defaults.overwrite);
        assert!(!defaults.skip_exist);
        assert!(!defaults.copy_inside);
        assert!(!defaults.content_only);
        assert_eq!(defaults.depth, 0);
    }

    #[test]
    fn copy_dispatches_files_and_directories_and_applies_skip_exist() {
        let temp = TestDir::new();
        let source_file = temp.path().join("source.txt");
        let target_dir = temp.path().join("target");
        std::fs::write(&source_file, "file data").unwrap();
        std::fs::create_dir(&target_dir).unwrap();
        copy_paths(&source_file, &target_dir, None).unwrap();
        assert_eq!(
            std::fs::read(target_dir.join("source.txt")).unwrap(),
            b"file data"
        );

        let source_dir = temp.path().join("tree");
        std::fs::create_dir_all(source_dir.join("nested")).unwrap();
        std::fs::write(source_dir.join("nested/item.txt"), "nested data").unwrap();
        let out = temp.path().join("out");
        copy_paths(&source_dir, &out, None).unwrap();
        assert_eq!(
            std::fs::read(out.join("tree/nested/item.txt")).unwrap(),
            b"nested data"
        );

        let skip_options: CopyOptions =
            serde_json::from_value(json!({ "skipExist": true })).unwrap();
        assert!(copy_paths(&source_file, &target_dir, Some(skip_options)).is_ok());
        assert_eq!(
            std::fs::read(target_dir.join("source.txt")).unwrap(),
            b"file data"
        );
    }

    #[test]
    fn move_dispatches_files_and_directories_and_removes_sources() {
        let temp = TestDir::new();
        let source_file = temp.path().join("source.txt");
        let file_target_dir = temp.path().join("file-dest");
        std::fs::write(&source_file, "file data").unwrap();
        std::fs::create_dir(&file_target_dir).unwrap();
        move_paths(&source_file, &file_target_dir, None).unwrap();
        assert!(!source_file.exists());
        assert_eq!(
            std::fs::read(file_target_dir.join("source.txt")).unwrap(),
            b"file data"
        );

        let source_dir = temp.path().join("tree");
        std::fs::create_dir_all(source_dir.join("nested")).unwrap();
        std::fs::write(source_dir.join("nested/item.txt"), "nested data").unwrap();
        let dir_target = temp.path().join("dir-dest");
        move_paths(&source_dir, &dir_target, None).unwrap();
        assert!(!source_dir.exists());
        assert_eq!(
            std::fs::read(dir_target.join("tree/nested/item.txt")).unwrap(),
            b"nested data"
        );
    }

    #[test]
    fn remove_deletes_a_file_or_a_directory_tree() {
        let temp = TestDir::new();
        let file = temp.path().join("remove.txt");
        let dir = temp.path().join("remove-tree");
        std::fs::write(&file, "x").unwrap();
        std::fs::create_dir_all(dir.join("nested")).unwrap();
        std::fs::write(dir.join("nested/item.txt"), "x").unwrap();

        remove_path(&file).unwrap();
        remove_path(&dir).unwrap();
        assert!(!file.exists());
        assert!(!dir.exists());
    }

    #[test]
    fn create_dir_is_single_level_and_create_dir_all_builds_parents() {
        let temp = TestDir::new();
        let one_level = temp.path().join("one-level");
        create_directory(&one_level).unwrap();
        assert!(one_level.is_dir());

        let nested = temp.path().join("parents/child/grandchild");
        assert!(create_directory(&nested).is_err());
        create_directories(&nested).unwrap();
        assert!(nested.is_dir());
    }

    #[test]
    fn read_dir_lists_only_immediate_entry_names() {
        let temp = TestDir::new();
        std::fs::create_dir(temp.path().join("folder")).unwrap();
        std::fs::write(temp.path().join("a.txt"), "a").unwrap();

        let mut entries = read_directory(temp.path()).unwrap();
        entries.sort();
        assert_eq!(entries, ["a.txt", "folder"]);
    }

    #[test]
    fn read_dir_all_recurses_and_excludes_matching_paths() {
        let temp = TestDir::new();
        let root = temp.path().join("tree");
        std::fs::create_dir_all(root.join("nested")).unwrap();
        std::fs::create_dir_all(root.join("ignored-dir")).unwrap();
        std::fs::write(root.join("top.txt"), "top").unwrap();
        std::fs::write(root.join("nested/keep.txt"), "keep").unwrap();
        std::fs::write(root.join("ignored-dir/skip.txt"), "skip").unwrap();

        let mut files = read_directory_all(&root, Some(vec!["*ignored-dir*".to_owned()])).unwrap();
        files.sort();
        assert_eq!(files, ["nested/keep.txt", "top.txt"]);
    }

    #[test]
    fn read_stream_emits_binary_chunks_then_end_and_reports_file_size() {
        let temp = TestDir::new();
        let path = temp.path().join("stream.bin");
        let data: Vec<u8> = (0..2500).map(|n| (n % 251) as u8).collect();
        std::fs::write(&path, &data).unwrap();

        let mut chunks = Vec::new();
        let size = read_file_chunks(
            &path,
            1024,
            || false,
            |chunk, end| {
                chunks.push((chunk.to_vec(), end));
            },
        )
        .unwrap();

        assert_eq!(size, Some(data.len() as u64));
        assert_eq!(
            chunks
                .iter()
                .map(|(chunk, _)| chunk.len())
                .collect::<Vec<_>>(),
            [1024, 1024, 452, 0]
        );
        assert!(chunks[..3].iter().all(|(_, end)| !end));
        assert_eq!(chunks.last().unwrap(), &(Vec::new(), true));
        assert_eq!(
            chunks
                .iter()
                .flat_map(|(chunk, _)| chunk.iter().copied())
                .collect::<Vec<_>>(),
            data
        );
    }

    #[test]
    fn read_stream_stops_without_end_chunk_after_cancellation() {
        let temp = TestDir::new();
        let path = temp.path().join("stream.bin");
        std::fs::write(&path, vec![7u8; 2500]).unwrap();
        let cancelled = std::cell::Cell::new(false);
        let mut chunks = Vec::new();

        let size = read_file_chunks(
            &path,
            1024,
            || cancelled.get(),
            |chunk, end| {
                chunks.push((chunk.len(), end));
                cancelled.set(true);
            },
        )
        .unwrap();

        assert_eq!(size, None);
        assert_eq!(chunks, [(1024, false)]);
    }

    #[test]
    fn write_stream_joins_chunks_stops_at_end_and_appends_when_requested() {
        let temp = TestDir::new();
        let path = temp.path().join("written.bin");
        let mut inbound_chunks = vec![
            inbound(b"first ", false),
            inbound(b"second", true),
            inbound(b"ignored", true),
        ]
        .into_iter();

        assert_eq!(
            write_file_chunks(&path, false, || inbound_chunks.next(), || false).unwrap(),
            Some(12)
        );
        assert_eq!(std::fs::read(&path).unwrap(), b"first second");
        assert!(
            inbound_chunks.next().is_some(),
            "END should stop chunk reads"
        );

        let mut append_chunks = vec![inbound(b" + more", true)].into_iter();
        assert_eq!(
            write_file_chunks(&path, true, || append_chunks.next(), || false).unwrap(),
            Some(7)
        );
        assert_eq!(std::fs::read(&path).unwrap(), b"first second + more");
    }

    #[test]
    fn write_stream_cancellation_does_not_commit_a_result() {
        let temp = TestDir::new();
        let path = temp.path().join("written.bin");
        std::fs::write(&path, "previous").unwrap();
        let mut inbound_chunks = vec![inbound(b"new data", true)].into_iter();

        assert_eq!(
            write_file_chunks(&path, false, || inbound_chunks.next(), || true).unwrap(),
            None
        );
        assert_eq!(std::fs::read(&path).unwrap(), b"");
    }
}
