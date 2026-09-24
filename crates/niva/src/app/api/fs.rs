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
    api_manager.register_blocking_api("fs.node", node_operation);
    api_manager.register_stream_api_with("fs.openHandle", Some(None), node_open_handle);
    api_manager.register_stream_api("fs.handle", node_handle_operation);
    api_manager.register_stream_api_with("fs.watch", Some(None), node_watch);
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

// Buffered JSON operations serve both WS unary calls and synchronous XHR.
// Streams remain binary WS calls. Byte results use base64 to avoid UTF-8 loss.
fn node_operation(
    _app: Arc<NivaApp>,
    _window: Arc<NivaWindow>,
    request: ApiRequest,
) -> Result<Value> {
    let (operation, arguments): (String, Value) = request.args().get()?;
    node_fs_operation(&operation, &arguments)
}

fn node_fs_operation(operation: &str, args: &Value) -> Result<Value> {
    use base64::{Engine, engine::general_purpose::STANDARD};
    use std::io::{Read, Write};
    let path = Path::new(
        args["path"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("path must be a string"))?,
    );
    match operation {
        "readFile" => {
            let mut file = node_open(path, args["flag"].as_str().unwrap_or("r"), 0o666)?;
            let mut bytes = Vec::new();
            file.read_to_end(&mut bytes)?;
            Ok(json!(STANDARD.encode(bytes)))
        }
        "writeFile" | "appendFile" => {
            let bytes = STANDARD.decode(
                args["data"]
                    .as_str()
                    .ok_or_else(|| anyhow::anyhow!("missing bytes"))?,
            )?;
            let flag = args["flag"]
                .as_str()
                .unwrap_or(if operation == "appendFile" { "a" } else { "w" });
            let mut file = node_open(path, flag, args["mode"].as_u64().unwrap_or(0o666) as u32)?;
            file.write_all(&bytes)?;
            Ok(Value::Null)
        }
        "stat" | "lstat" => {
            let metadata = if operation == "lstat" {
                std::fs::symlink_metadata(path)?
            } else {
                std::fs::metadata(path)?
            };
            Ok(node_stat(&metadata))
        }
        "realpath" => Ok(json!(std::fs::canonicalize(path)?)),
        "rename" => {
            std::fs::rename(path, node_destination(args)?)?;
            Ok(Value::Null)
        }
        "unlink" => {
            std::fs::remove_file(path)?;
            Ok(Value::Null)
        }
        "copyFile" => {
            let destination = node_destination(args)?;
            let flags = args["flags"].as_u64().unwrap_or(0);
            anyhow::ensure!(flags & !1 == 0, "ENOTSUP: clone flags are unsupported");
            let mut input = std::fs::File::open(path)?;
            let input_metadata = input.metadata()?;
            let mut options = std::fs::OpenOptions::new();
            options.write(true);
            if flags & 1 != 0 {
                options.create_new(true);
            } else {
                options.create(true);
            }
            // Check path identity before opening, then compare open handles
            // below so hard links and path races cannot truncate the source.
            if matches!(
                (std::fs::canonicalize(path), std::fs::canonicalize(destination)),
                (Ok(source), Ok(destination)) if source == destination
            ) {
                anyhow::bail!("EINVAL: source and destination are the same file");
            }
            let mut output = options.open(destination)?;
            anyhow::ensure!(
                !same_file(&input, &output)?,
                "EINVAL: source and destination are the same file"
            );
            output.set_len(0)?;
            use std::io::Seek;
            output.seek(std::io::SeekFrom::Start(0))?;
            std::io::copy(&mut input, &mut output)?;
            output.set_permissions(input_metadata.permissions())?;
            Ok(Value::Null)
        }
        "mkdir" => {
            let recursive = args["recursive"].as_bool().unwrap_or(false);
            let mut first = None;
            if recursive {
                let mut current = path;
                while !current.exists() {
                    first = Some(current.to_path_buf());
                    match current.parent() {
                        Some(parent) if !parent.as_os_str().is_empty() => current = parent,
                        _ => break,
                    }
                }
            }
            let mut builder = std::fs::DirBuilder::new();
            builder.recursive(recursive);
            #[cfg(unix)]
            {
                use std::os::unix::fs::DirBuilderExt;
                builder.mode(args["mode"].as_u64().unwrap_or(0o777) as u32);
            }
            builder.create(path)?;
            Ok(json!(first))
        }
        "readdir" => {
            let entries: Result<Vec<Value>> = std::fs::read_dir(path)?.map(|entry| {
                let entry = entry?;
                Ok(json!({"name": entry.file_name().to_string_lossy(), "stats":node_stat(&std::fs::symlink_metadata(entry.path())?)}))
            }).collect();
            Ok(json!(entries?))
        }
        "rm" => {
            let metadata = match std::fs::symlink_metadata(path) {
                Ok(metadata) => metadata,
                Err(error)
                    if error.kind() == std::io::ErrorKind::NotFound
                        && args["force"].as_bool().unwrap_or(false) =>
                {
                    return Ok(Value::Null);
                }
                Err(error) => return Err(error.into()),
            };
            if metadata.is_dir() {
                anyhow::ensure!(
                    args["recursive"].as_bool().unwrap_or(false),
                    "ERR_FS_EISDIR: recursive removal required"
                );
                std::fs::remove_dir_all(path)?;
            } else {
                std::fs::remove_file(path)?;
            }
            Ok(Value::Null)
        }
        "access" => {
            let mode = args["mode"].as_u64().unwrap_or(0);
            anyhow::ensure!(mode <= 7, "invalid access mode");
            #[cfg(unix)]
            {
                use std::os::unix::ffi::OsStrExt;
                let path = std::ffi::CString::new(path.as_os_str().as_bytes())?;
                // SAFETY: NUL-terminated pathname is alive for this call.
                if unsafe { libc::access(path.as_ptr(), mode as i32) } != 0 {
                    return Err(std::io::Error::last_os_error().into());
                }
            }
            #[cfg(not(unix))]
            {
                let metadata = std::fs::metadata(path)?;
                if mode & 2 != 0 && metadata.permissions().readonly() {
                    anyhow::bail!("EACCES: read-only path");
                }
            }
            Ok(Value::Null)
        }
        _ => anyhow::bail!("unknown filesystem operation"),
    }
}

fn node_destination(args: &Value) -> Result<&Path> {
    Ok(Path::new(
        args["destination"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("missing destination"))?,
    ))
}

fn node_open(path: &Path, flag: &str, mode: u32) -> Result<std::fs::File> {
    let mut options = std::fs::OpenOptions::new();
    match flag {
        "r" | "rs" => {
            options.read(true);
        }
        "r+" | "rs+" => {
            options.read(true).write(true);
        }
        "w" | "w+" | "wx" | "wx+" | "xw" | "xw+" => {
            options.write(true).read(flag.contains('+'));
            if flag.contains('x') {
                options.create_new(true);
            } else {
                options.create(true).truncate(true);
            }
        }
        "a" | "a+" | "ax" | "ax+" | "xa" | "xa+" => {
            options.append(true).read(flag.contains('+'));
            if flag.contains('x') {
                options.create_new(true);
            } else {
                options.create(true);
            }
        }
        _ => anyhow::bail!("EINVAL: unsupported file flag {flag}"),
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(mode);
    }
    #[cfg(not(unix))]
    let _ = mode;
    Ok(options.open(path)?)
}

fn same_file(source: &std::fs::File, destination: &std::fs::File) -> Result<bool> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        let source = source.metadata()?;
        let destination = destination.metadata()?;
        Ok(source.dev() == destination.dev() && source.ino() == destination.ino())
    }
    #[cfg(windows)]
    {
        use std::os::windows::io::AsRawHandle;
        use windows::Win32::{
            Foundation::HANDLE,
            Storage::FileSystem::{BY_HANDLE_FILE_INFORMATION, GetFileInformationByHandle},
        };
        let mut source_info = BY_HANDLE_FILE_INFORMATION::default();
        let mut destination_info = BY_HANDLE_FILE_INFORMATION::default();
        // SAFETY: both handles are live files owned by the caller; the API
        // writes fixed-size metadata records into initialized output storage.
        unsafe {
            GetFileInformationByHandle(HANDLE(source.as_raw_handle() as *mut _), &mut source_info)?;
            GetFileInformationByHandle(
                HANDLE(destination.as_raw_handle() as *mut _),
                &mut destination_info,
            )?;
        }
        Ok(
            source_info.dwVolumeSerialNumber == destination_info.dwVolumeSerialNumber
                && source_info.nFileIndexHigh == destination_info.nFileIndexHigh
                && source_info.nFileIndexLow == destination_info.nFileIndexLow,
        )
    }
    #[cfg(not(any(unix, windows)))]
    {
        let _ = (source, destination);
        Ok(false)
    }
}

fn node_stat(meta: &std::fs::Metadata) -> Value {
    let millis = |time: std::io::Result<std::time::SystemTime>| {
        time.ok()
            .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
            .map(|d| d.as_secs_f64() * 1000.0)
            .unwrap_or(0.0)
    };
    let mut value = json!({"isFile":meta.is_file(),"isDir":meta.is_dir(),"isSymlink":meta.file_type().is_symlink(),"size":meta.len(),"atimeMs":millis(meta.accessed()),"mtimeMs":millis(meta.modified()),"birthtimeMs":millis(meta.created())});
    #[cfg(unix)]
    {
        use std::os::unix::fs::{FileTypeExt, MetadataExt};
        value["dev"] = json!(meta.dev());
        value["ino"] = json!(meta.ino());
        value["mode"] = json!(meta.mode());
        value["nlink"] = json!(meta.nlink());
        value["uid"] = json!(meta.uid());
        value["gid"] = json!(meta.gid());
        value["rdev"] = json!(meta.rdev());
        value["blksize"] = json!(meta.blksize());
        value["blocks"] = json!(meta.blocks());
        value["ctimeMs"] = json!(meta.ctime() as f64 * 1000.0 + meta.ctime_nsec() as f64 / 1e6);
        value["isBlockDevice"] = json!(meta.file_type().is_block_device());
        value["isCharacterDevice"] = json!(meta.file_type().is_char_device());
        value["isFIFO"] = json!(meta.file_type().is_fifo());
        value["isSocket"] = json!(meta.file_type().is_socket());
    }
    value
}

struct NodeFileHandle {
    owner: (u8, u64),
    file: std::sync::Mutex<std::fs::File>,
    closed: async_channel::Sender<()>,
}
type NodeFiles = std::sync::Mutex<std::collections::HashMap<String, Arc<NodeFileHandle>>>;
fn node_files() -> &'static NodeFiles {
    static FILES: std::sync::OnceLock<NodeFiles> = std::sync::OnceLock::new();
    FILES.get_or_init(|| std::sync::Mutex::new(std::collections::HashMap::new()))
}

struct NodeFileRegistration {
    id: String,
    handle: Arc<NodeFileHandle>,
}

impl NodeFileRegistration {
    fn insert(id: String, handle: Arc<NodeFileHandle>) -> Result<Self> {
        let mut files = node_files()
            .lock()
            .map_err(|_| anyhow::anyhow!("file registry poisoned"))?;
        anyhow::ensure!(
            files.len() < 256,
            "EMFILE: too many open compatibility files"
        );
        anyhow::ensure!(!files.contains_key(&id), "EEXIST: file handle id collision");
        files.insert(id.clone(), handle.clone());
        Ok(Self { id, handle })
    }
}

impl Drop for NodeFileRegistration {
    fn drop(&mut self) {
        if let Ok(mut files) = node_files().lock()
            && files
                .get(&self.id)
                .is_some_and(|registered| Arc::ptr_eq(registered, &self.handle))
        {
            files.remove(&self.id);
        }
    }
}

async fn node_open_handle(ctx: CallContext, request: ApiRequest) -> Result<()> {
    let (path, flag, mode): (String, Option<String>, Option<u32>) = request.args().optional(3)?;
    let file = crate::blocking!(node_open(
        Path::new(&path),
        flag.as_deref().unwrap_or("r"),
        mode.unwrap_or(0o666)
    ))
    .await?;
    let mut bytes = [0u8; 24];
    getrandom::fill(&mut bytes).map_err(|_| anyhow::anyhow!("handle entropy unavailable"))?;
    let id: String = bytes.iter().map(|byte| format!("{byte:02x}")).collect();
    let (closed, closing) = async_channel::bounded(1);
    let handle = Arc::new(NodeFileHandle {
        owner: (ctx.window.id, ctx.connection_id),
        file: std::sync::Mutex::new(file),
        closed,
    });
    let _registration = NodeFileRegistration::insert(id.clone(), handle)?;
    ctx.push("open", json!({"handle":id}));
    smol::future::or(ctx.cancelled(), async {
        let _ = closing.recv().await;
    })
    .await;
    if !ctx.is_cancelled() {
        ctx.respond(Ok(Value::Null));
    }
    Ok(())
}

async fn node_handle_operation(ctx: CallContext, request: ApiRequest) -> Result<()> {
    use base64::{Engine, engine::general_purpose::STANDARD};
    use std::io::{Read, Seek, SeekFrom, Write};
    let (id, operation, args): (String, String, Value) = request.args().get()?;
    let handle = {
        let files = node_files()
            .lock()
            .map_err(|_| anyhow::anyhow!("file registry poisoned"))?;
        let handle = files
            .get(&id)
            .ok_or_else(|| anyhow::anyhow!("EBADF: file closed"))?;
        anyhow::ensure!(
            handle.owner == (ctx.window.id, ctx.connection_id),
            "EACCES: file belongs to another connection"
        );
        handle.clone()
    };
    let result = crate::blocking!({
        if operation == "close" {
            node_files()
                .lock()
                .map_err(|_| anyhow::anyhow!("file registry poisoned"))?
                .remove(&id);
            handle.closed.close();
            Ok(Value::Null)
        } else {
            let mut file = handle
                .file
                .lock()
                .map_err(|_| anyhow::anyhow!("file lock poisoned"))?;
            let restore = if let Some(position) = args["position"].as_u64() {
                let old = file.stream_position()?;
                file.seek(SeekFrom::Start(position))?;
                Some(old)
            } else {
                None
            };
            let operation_result = (|| -> Result<Value> {
                match operation.as_str() {
                    "read" => {
                        let length = args["length"].as_u64().unwrap_or(65536);
                        anyhow::ensure!(length <= 8 * 1024 * 1024, "read chunk too large");
                        let mut data = vec![0; length as usize];
                        let size = file.read(&mut data)?;
                        data.truncate(size);
                        Ok(json!({"bytesRead":size,"data":STANDARD.encode(data)}))
                    }
                    "write" => {
                        let data = STANDARD.decode(args["data"].as_str().unwrap_or(""))?;
                        let size = file.write(&data)?;
                        Ok(json!({"bytesWritten":size}))
                    }
                    "stat" => Ok(node_stat(&file.metadata()?)),
                    "sync" => {
                        file.sync_all()?;
                        Ok(Value::Null)
                    }
                    "datasync" => {
                        file.sync_data()?;
                        Ok(Value::Null)
                    }
                    "truncate" => {
                        file.set_len(args["length"].as_u64().unwrap_or(0))?;
                        Ok(Value::Null)
                    }
                    _ => anyhow::bail!("ENOTSUP: unsupported FileHandle operation"),
                }
            })();
            if let Some(position) = restore {
                file.seek(SeekFrom::Start(position))?;
            }
            operation_result
        }
    })
    .await;
    ctx.respond(result);
    Ok(())
}

async fn node_watch(ctx: CallContext, request: ApiRequest) -> Result<()> {
    use notify::Watcher;
    let (path, recursive): (String, Option<bool>) = request.args().optional(2)?;
    let root = std::path::PathBuf::from(path);
    let (events, receiver) = async_channel::bounded(256);
    let (overflow_tx, overflow_rx) = async_channel::bounded::<()>(1);
    let overflowed = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let callback_overflow = overflowed.clone();
    let callback_signal = overflow_tx.clone();
    let mut watcher = notify::recommended_watcher(move |event| {
        queue_watch_event(&events, &callback_signal, &callback_overflow, event);
    })?;
    watcher.watch(
        &root,
        if recursive.unwrap_or(false) {
            notify::RecursiveMode::Recursive
        } else {
            notify::RecursiveMode::NonRecursive
        },
    )?;
    ctx.push("ready", Value::Null);
    loop {
        let event = smol::future::or(async { receiver.recv().await.ok() }, async {
            smol::future::or(ctx.cancelled(), async {
                let _ = overflow_rx.recv().await;
            })
            .await;
            None
        })
        .await;
        let Some(event) = event else {
            break;
        };
        let event = event?;
        if event.kind.is_access() {
            continue;
        }
        let kind = match event.kind {
            notify::EventKind::Create(_)
            | notify::EventKind::Remove(_)
            | notify::EventKind::Modify(notify::event::ModifyKind::Name(_)) => "rename",
            _ => "change",
        };
        if event.paths.is_empty() {
            ctx.push("change", json!({"eventType":kind,"filename":null}));
        }
        for path in event.paths {
            let name = if root.is_dir() {
                path.strip_prefix(&root)
                    .ok()
                    .map(|p| p.to_string_lossy().into_owned())
            } else {
                path.file_name().map(|p| p.to_string_lossy().into_owned())
            };
            ctx.push("change", json!({"eventType":kind,"filename":name}));
        }
    }
    drop(watcher);
    anyhow::ensure!(
        !overflowed.load(std::sync::atomic::Ordering::Acquire),
        "ENOSPC: filesystem watcher queue overflow"
    );
    Ok(())
}

fn queue_watch_event<T>(
    events: &async_channel::Sender<T>,
    overflow_signal: &async_channel::Sender<()>,
    overflowed: &std::sync::atomic::AtomicBool,
    event: T,
) {
    match events.try_send(event) {
        Ok(()) => {}
        Err(async_channel::TrySendError::Full(_)) => {
            overflowed.store(true, std::sync::atomic::Ordering::Release);
            let _ = overflow_signal.try_send(());
        }
        Err(async_channel::TrySendError::Closed(_)) => {}
    }
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
    fn node_file_operations_preserve_bytes_flags_and_directory_semantics() {
        let temp = TestDir::new();
        let path = temp.path().join("node.bin");
        node_fs_operation("writeFile", &json!({"path":path,"data":"AP/+"})).unwrap();
        assert_eq!(
            node_fs_operation("readFile", &json!({"path":path})).unwrap(),
            json!("AP/+")
        );
        assert!(
            node_fs_operation("writeFile", &json!({"path":path,"data":"YQ==","flag":"wx"}))
                .is_err()
        );
        node_fs_operation("appendFile", &json!({"path":path,"data":"YQ=="})).unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), [0, 255, 254, 97]);
        assert!(node_fs_operation("rm", &json!({"path":temp.path()})).is_err());
        assert!(path.exists());
        let entries = node_fs_operation("readdir", &json!({"path":temp.path()})).unwrap();
        assert_eq!(entries[0]["stats"]["isFile"], true);
        assert!(node_fs_operation("unlink", &json!({"path":temp.path()})).is_err());
        node_fs_operation("unlink", &json!({"path":path})).unwrap();
        node_fs_operation("rm", &json!({"path":path,"force":true})).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn node_lstat_keeps_symlinks_and_removal_does_not_follow_them() {
        let temp = TestDir::new();
        let target = temp.path().join("target");
        let link = temp.path().join("link");
        std::fs::create_dir(&target).unwrap();
        std::os::unix::fs::symlink(&target, &link).unwrap();
        assert_eq!(
            node_fs_operation("lstat", &json!({"path":link})).unwrap()["isSymlink"],
            true
        );
        assert_eq!(
            node_fs_operation("stat", &json!({"path":link})).unwrap()["isDir"],
            true
        );
        node_fs_operation("rm", &json!({"path":link,"recursive":true})).unwrap();
        assert!(target.is_dir());
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
        assert_eq!(
            files,
            [
                format!("nested{}keep.txt", std::path::MAIN_SEPARATOR),
                "top.txt".to_owned(),
            ]
        );
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

    #[test]
    fn node_copy_rejects_hardlink_destinations_without_truncating_source() {
        let temp = TestDir::new();
        let source = temp.path().join("source.bin");
        let destination = temp.path().join("source-hardlink.bin");
        std::fs::write(&source, b"preserve these bytes").unwrap();
        std::fs::hard_link(&source, &destination).unwrap();

        let error = node_fs_operation(
            "copyFile",
            &json!({ "path": source, "destination": destination }),
        )
        .unwrap_err();

        assert!(error.to_string().contains("same file"));
        assert_eq!(std::fs::read(&source).unwrap(), b"preserve these bytes");
        assert_eq!(
            std::fs::read(&destination).unwrap(),
            b"preserve these bytes"
        );
    }

    #[test]
    fn open_handle_registration_drop_removes_the_owned_descriptor() {
        let temp = TestDir::new();
        let path = temp.path().join("registered.bin");
        std::fs::write(&path, b"handle").unwrap();
        let id = format!("test-{}", std::process::id());
        let (closed, _closing) = async_channel::bounded(1);
        let handle = Arc::new(NodeFileHandle {
            owner: (0, 17),
            file: std::sync::Mutex::new(std::fs::File::open(&path).unwrap()),
            closed,
        });

        let registration = NodeFileRegistration::insert(id.clone(), handle.clone()).unwrap();
        assert!(node_files().lock().unwrap().contains_key(&id));
        drop(registration);
        assert!(!node_files().lock().unwrap().contains_key(&id));
        drop(handle);
    }

    #[test]
    fn watcher_receiver_close_is_not_misreported_as_queue_overflow() {
        let (events, receiver) = async_channel::bounded(1);
        let (overflow_signal, overflow_receiver) = async_channel::bounded(1);
        let overflowed = std::sync::atomic::AtomicBool::new(false);
        queue_watch_event(&events, &overflow_signal, &overflowed, 1);
        drop(receiver);
        queue_watch_event(&events, &overflow_signal, &overflowed, 2);
        drop(overflow_signal);

        assert!(!overflowed.load(std::sync::atomic::Ordering::Acquire));
        assert!(overflow_receiver.is_closed());
    }

    #[test]
    fn watcher_full_queue_is_reported_as_overflow() {
        let (events, _receiver) = async_channel::bounded(1);
        let (overflow_signal, overflow_receiver) = async_channel::bounded(1);
        let overflowed = std::sync::atomic::AtomicBool::new(false);
        queue_watch_event(&events, &overflow_signal, &overflowed, 1);
        queue_watch_event(&events, &overflow_signal, &overflowed, 2);

        assert!(overflowed.load(std::sync::atomic::Ordering::Acquire));
        assert_eq!(overflow_receiver.try_recv().unwrap(), ());
    }
}
