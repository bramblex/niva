use anyhow::Result;
use glob::Pattern;
use serde::Deserialize;
use serde_json::{Value, json};

use std::{
    fs::{self, OpenOptions},
    io::{Read, Seek, SeekFrom, Write},
    path::{Component, Path, PathBuf},
    time::UNIX_EPOCH,
};

use crate::app::NivaApp;
use crate::app::api_manager::{
    ApiCallOwner, ApiManager, ApiRequest, CallContext, CancellationContext, IpcCallContext,
};
use crate::app::window_manager::window::NivaWindow;
use std::sync::Arc;

pub fn register_api_instances(api_manager: &mut ApiManager) {
    api_manager.register_blocking_api("fs.stat", stat);
    api_manager.register_cancellable_api("fs.node", node_operation);
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
    api_manager.register_ipc_api("fs.readText", ipc_read_text);
    api_manager.register_ipc_api("fs.writeText", ipc_write_text);
    api_manager.register_ipc_api("fs.appendText", ipc_append_text);
    api_manager.register_ipc_api("fs.node", ipc_node_operation);
}

const MAX_IPC_TEXT_FILE_BYTES: u64 = 1024 * 1024;
const MAX_IPC_TEXT_INPUT_BYTES: usize = 256 * 1024;
const MAX_IPC_READ_DIR_ENTRIES: usize = 512;

const IPC_NODE_OPERATIONS: &[&str] = &[
    "stat", "lstat", "readdir", "access", "realpath", "mkdir", "rename", "copyFile", "rm",
    "unlink", "cp",
];

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct IpcTextFileOptions {
    flag: Option<String>,
    mode: Option<u32>,
}

async fn ipc_read_text(ctx: IpcCallContext, request: ApiRequest) -> Result<String> {
    let (path, encoding, options): (String, Option<String>, Option<IpcTextFileOptions>) =
        request.args().optional(3)?;
    validate_ipc_text_encoding(encoding.as_deref())?;
    if let Some(options) = options {
        anyhow::ensure!(
            options
                .flag
                .as_deref()
                .is_none_or(|flag| matches!(flag, "r" | "rs")),
            "ENOTSUP: IPC text reads support only the read flag"
        );
    }
    crate::blocking!({
        anyhow::ensure!(!ctx.is_cancelled(), "ECANCELED: IPC request cancelled");
        read_ipc_text_file(Path::new(&path))
    })
    .await
}

async fn ipc_write_text(ctx: IpcCallContext, request: ApiRequest) -> Result<()> {
    ipc_write_text_inner(ctx, request, false).await
}

async fn ipc_append_text(ctx: IpcCallContext, request: ApiRequest) -> Result<()> {
    ipc_write_text_inner(ctx, request, true).await
}

async fn ipc_write_text_inner(
    ctx: IpcCallContext,
    request: ApiRequest,
    append: bool,
) -> Result<()> {
    let (path, text, encoding, options): (
        String,
        String,
        Option<String>,
        Option<IpcTextFileOptions>,
    ) = request.args().optional(4)?;
    validate_ipc_text_encoding(encoding.as_deref())?;
    anyhow::ensure!(
        text.len() <= MAX_IPC_TEXT_INPUT_BYTES,
        "EFBIG: IPC text input exceeds 256 KiB"
    );
    let options = options.unwrap_or_default();
    let flag = options
        .flag
        .as_deref()
        .unwrap_or(if append { "a" } else { "w" });
    anyhow::ensure!(
        if append {
            matches!(flag, "a" | "ax")
        } else {
            matches!(flag, "w" | "wx")
        },
        "ENOTSUP: unsupported IPC text file flag"
    );
    let exclusive = flag.ends_with('x');
    let mode = options.mode.unwrap_or(0o666);
    anyhow::ensure!(mode <= 0o777, "EINVAL: invalid file mode");
    crate::blocking!({
        anyhow::ensure!(!ctx.is_cancelled(), "ECANCELED: IPC request cancelled");
        write_ipc_text_file(Path::new(&path), text.as_bytes(), append, exclusive, mode)
    })
    .await
}

fn validate_ipc_text_encoding(encoding: Option<&str>) -> Result<()> {
    anyhow::ensure!(
        encoding.is_none_or(|encoding| matches!(
            encoding.to_ascii_lowercase().as_str(),
            "utf8" | "utf-8"
        )),
        "ERR_ENCODING_NOT_SUPPORTED: IPC text APIs support UTF-8 only"
    );
    Ok(())
}

fn read_ipc_text_file(path: &Path) -> Result<String> {
    let bytes = read_ipc_regular_file(path, MAX_IPC_TEXT_FILE_BYTES)?;
    String::from_utf8(bytes)
        .map_err(|_| anyhow::anyhow!("ERR_INVALID_ENCODING: file does not contain valid UTF-8"))
}

fn read_ipc_regular_file(path: &Path, limit: u64) -> Result<Vec<u8>> {
    ensure_not_windows_device_path(path)?;
    let before = fs::metadata(path)?;
    anyhow::ensure!(before.is_file(), "EINVAL: IPC file must be a regular file");
    anyhow::ensure!(
        before.len() <= limit,
        "EFBIG: IPC file exceeds the 1 MiB limit"
    );
    let mut options = OpenOptions::new();
    options.read(true);
    set_nonblocking_open(&mut options);
    let mut file = options.open(path)?;
    let opened_before = file.metadata()?;
    anyhow::ensure!(
        opened_before.is_file() && opened_before.len() <= limit,
        "EINVAL: IPC file changed to a non-regular or oversized file"
    );
    let mut bytes = Vec::with_capacity(opened_before.len() as usize);
    Read::take(&mut file, limit.saturating_add(1)).read_to_end(&mut bytes)?;
    anyhow::ensure!(
        bytes.len() as u64 <= limit,
        "EFBIG: IPC file exceeds the limit"
    );
    ensure_same_open_file_size(&opened_before, &file.metadata()?)?;
    Ok(bytes)
}

fn write_ipc_text_file(
    path: &Path,
    bytes: &[u8],
    append: bool,
    exclusive: bool,
    mode: u32,
) -> Result<()> {
    ensure_not_windows_device_path(path)?;
    if path.exists() {
        let before = fs::metadata(path)?;
        anyhow::ensure!(before.is_file(), "EINVAL: IPC file must be a regular file");
    }
    let mut options = OpenOptions::new();
    options.write(true).append(append);
    if append {
        options.create(!exclusive);
        if exclusive {
            options.create_new(true);
        }
    } else if exclusive {
        options.create_new(true);
    } else {
        options.create(true);
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(mode);
    }
    set_nonblocking_open(&mut options);
    let mut file = options.open(path)?;
    let opened = file.metadata()?;
    anyhow::ensure!(
        opened.is_file(),
        "EINVAL: IPC file changed to a non-regular file"
    );
    if append {
        anyhow::ensure!(
            opened.len().saturating_add(bytes.len() as u64) <= MAX_IPC_TEXT_FILE_BYTES,
            "EFBIG: IPC text file would exceed the 1 MiB limit"
        );
    } else {
        anyhow::ensure!(
            bytes.len() as u64 <= MAX_IPC_TEXT_FILE_BYTES,
            "EFBIG: IPC text file exceeds the 1 MiB limit"
        );
    }
    if !append {
        file.set_len(0)?;
        file.seek(SeekFrom::Start(0))?;
    }
    file.write_all(bytes)?;
    file.flush()?;
    Ok(())
}

fn ensure_same_open_file_size(before: &std::fs::Metadata, after: &std::fs::Metadata) -> Result<()> {
    anyhow::ensure!(
        before.len() == after.len() && before.modified().ok() == after.modified().ok(),
        "EAGAIN: IPC file changed while reading"
    );
    Ok(())
}

#[cfg(unix)]
fn set_nonblocking_open(options: &mut OpenOptions) {
    use std::os::unix::fs::OpenOptionsExt;
    options.custom_flags(libc::O_NONBLOCK);
}

#[cfg(not(unix))]
fn set_nonblocking_open(_options: &mut OpenOptions) {}

#[cfg(windows)]
fn ensure_not_windows_device_path(path: &Path) -> Result<()> {
    let path = path.to_string_lossy().to_ascii_lowercase();
    anyhow::ensure!(
        !path.starts_with("\\\\.\\")
            && !path.starts_with("\\\\?\\globalroot\\")
            && !path.starts_with("\\\\?\\pipe\\"),
        "EINVAL: IPC text APIs do not open device paths"
    );
    Ok(())
}

#[cfg(not(windows))]
fn ensure_not_windows_device_path(_path: &Path) -> Result<()> {
    Ok(())
}

async fn ipc_node_operation(ctx: IpcCallContext, request: ApiRequest) -> Result<Value> {
    let (operation, arguments): (String, Value) = request.args().get()?;
    anyhow::ensure!(
        IPC_NODE_OPERATIONS.contains(&operation.as_str()),
        "ENOTSUP: filesystem operation is unavailable over IPC"
    );
    if operation == "readdir" {
        let path = arguments["path"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("path must be a string"))?
            .to_owned();
        return crate::blocking!({
            anyhow::ensure!(!ctx.is_cancelled(), "ECANCELED: IPC request cancelled");
            read_ipc_directory(Path::new(&path))
        })
        .await;
    }
    crate::blocking!({
        anyhow::ensure!(!ctx.is_cancelled(), "ECANCELED: IPC request cancelled");
        node_fs_operation(&operation, &arguments)
    })
    .await
}

fn read_ipc_directory(path: &Path) -> Result<Value> {
    let mut entries = Vec::new();
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        if entries.len() == MAX_IPC_READ_DIR_ENTRIES {
            anyhow::bail!("EFBIG: IPC directory exceeds 512 entries");
        }
        let name = entry.file_name().to_string_lossy().into_owned();
        let metadata = fs::symlink_metadata(entry.path())?;
        entries.push(json!({"name":name,"stats":node_stat(&metadata)}));
    }
    Ok(json!(entries))
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
async fn node_operation(
    context: CancellationContext,
    _app: Arc<NivaApp>,
    _window: Arc<NivaWindow>,
    request: ApiRequest,
) -> Result<Value> {
    let (operation, arguments): (String, Value) = request.args().get()?;
    if SYNC_FD_OPERATIONS.contains(&operation.as_str()) {
        let owner = sync_fd_owner(&context)?;
        let cancellation = context.clone();
        return crate::blocking!({
            sync_fd_operation(&owner, &operation, &arguments, move || {
                cancellation.is_cancelled()
            })
        })
        .await;
    }
    let cancellation = context.clone();
    crate::blocking!({
        anyhow::ensure!(
            !cancellation.is_cancelled(),
            "ECANCELED: file operation cancelled"
        );
        node_fs_operation(&operation, &arguments)
    })
    .await
}

const MAX_SYNC_FDS_PER_SESSION: usize = 256;
const MAX_SYNC_FDS_GLOBAL: usize = 4096;
const MAX_SYNC_FD_IO_BYTES: usize = 8 * 1024 * 1024;
const SYNC_FD_OPERATIONS: &[&str] = &["open", "read", "write", "fstat", "close"];
type SyncFdOwner = (u8, String);
type SyncFdFiles = std::collections::HashMap<i32, Arc<std::sync::Mutex<std::fs::File>>>;
#[derive(Default)]
struct SyncFdSession {
    files: SyncFdFiles,
    reserved: std::collections::HashSet<i32>,
}
type SyncFdSessions = std::collections::HashMap<SyncFdOwner, SyncFdSession>;

fn sync_fd_sessions() -> &'static std::sync::Mutex<SyncFdSessions> {
    static SESSIONS: std::sync::OnceLock<std::sync::Mutex<SyncFdSessions>> =
        std::sync::OnceLock::new();
    SESSIONS.get_or_init(|| std::sync::Mutex::new(SyncFdSessions::new()))
}

fn sync_fd_owner(context: &CancellationContext) -> Result<SyncFdOwner> {
    match &context.owner {
        ApiCallOwner::Synchronous { session_id } => Ok((context.window_id, session_id.clone())),
        _ => anyhow::bail!("ENOTSUP: synchronous file descriptors require trusted sync XHR"),
    }
}

pub(crate) fn cancel_sync_fd_session(window_id: u8, session_id: &str) {
    if let Ok(mut sessions) = sync_fd_sessions().lock() {
        sessions.remove(&(window_id, session_id.to_owned()));
    }
}

pub(crate) fn cancel_sync_fd_window(window_id: u8) {
    if let Ok(mut sessions) = sync_fd_sessions().lock() {
        sessions.retain(|(owner_window, _), _| *owner_window != window_id);
    }
}

struct SyncFdReservation {
    owner: SyncFdOwner,
    fd: i32,
    committed: bool,
}

impl SyncFdReservation {
    fn reserve(owner: &SyncFdOwner) -> Result<Self> {
        let mut sessions = sync_fd_sessions()
            .lock()
            .map_err(|_| anyhow::anyhow!("synchronous file descriptor registry poisoned"))?;
        let total_open = sessions
            .values()
            .map(|session| session.files.len() + session.reserved.len())
            .sum::<usize>();
        anyhow::ensure!(
            total_open < MAX_SYNC_FDS_GLOBAL,
            "EMFILE: too many synchronous file descriptors across all sessions"
        );
        let session = sessions.entry(owner.clone()).or_default();
        anyhow::ensure!(
            session.files.len() + session.reserved.len() < MAX_SYNC_FDS_PER_SESSION,
            "EMFILE: too many synchronous file descriptors"
        );
        let fd = (3..=i32::MAX)
            .find(|fd| !session.files.contains_key(fd) && !session.reserved.contains(fd))
            .ok_or_else(|| anyhow::anyhow!("EMFILE: logical file descriptor space exhausted"))?;
        session.reserved.insert(fd);
        Ok(Self {
            owner: owner.clone(),
            fd,
            committed: false,
        })
    }

    fn commit(mut self, file: std::fs::File, is_cancelled: impl Fn() -> bool) -> Result<Value> {
        anyhow::ensure!(
            !is_cancelled(),
            "ECANCELED: file descriptor open was cancelled"
        );
        let mut sessions = sync_fd_sessions()
            .lock()
            .map_err(|_| anyhow::anyhow!("synchronous file descriptor registry poisoned"))?;
        anyhow::ensure!(
            !is_cancelled(),
            "ECANCELED: file descriptor open was cancelled"
        );
        let session = sessions.get_mut(&self.owner).ok_or_else(|| {
            anyhow::anyhow!("ECANCELED: file descriptor owner session was closed")
        })?;
        anyhow::ensure!(
            session.reserved.remove(&self.fd),
            "ECANCELED: file descriptor reservation was cancelled"
        );
        session
            .files
            .insert(self.fd, Arc::new(std::sync::Mutex::new(file)));
        self.committed = true;
        Ok(json!(self.fd))
    }
}

impl Drop for SyncFdReservation {
    fn drop(&mut self) {
        if self.committed {
            return;
        }
        if let Ok(mut sessions) = sync_fd_sessions().lock() {
            if let Some(session) = sessions.get_mut(&self.owner) {
                session.reserved.remove(&self.fd);
                if session.files.is_empty() && session.reserved.is_empty() {
                    sessions.remove(&self.owner);
                }
            }
        }
    }
}

fn sync_fd_operation(
    owner: &SyncFdOwner,
    operation: &str,
    args: &Value,
    is_cancelled: impl Fn() -> bool,
) -> Result<Value> {
    use base64::{Engine, engine::general_purpose::STANDARD};
    use std::io::{Read, Seek, SeekFrom, Write};

    match operation {
        "open" => {
            anyhow::ensure!(
                !is_cancelled(),
                "ECANCELED: file descriptor open was cancelled"
            );
            let path = args["path"]
                .as_str()
                .ok_or_else(|| anyhow::anyhow!("path must be a string"))?;
            let flag = args["flag"].as_str().unwrap_or("r");
            let mode = args["mode"].as_u64().unwrap_or(0o666) as u32;
            // Reserve both the quota and a logical descriptor before opening
            // the path so an EMFILE failure cannot truncate a "w" target.
            let reservation = SyncFdReservation::reserve(owner)?;
            anyhow::ensure!(
                !is_cancelled(),
                "ECANCELED: file descriptor open was cancelled"
            );
            let file = node_open_sync_fd(Path::new(path), flag, mode)?;
            reservation.commit(file, is_cancelled)
        }
        "close" => {
            let fd = sync_fd_argument(args)?;
            let mut sessions = sync_fd_sessions()
                .lock()
                .map_err(|_| anyhow::anyhow!("synchronous file descriptor registry poisoned"))?;
            let session = sessions.get_mut(owner).ok_or_else(|| {
                anyhow::anyhow!("EBADF: file descriptor is not owned by this session")
            })?;
            session
                .files
                .remove(&fd)
                .ok_or_else(|| anyhow::anyhow!("EBADF: invalid file descriptor"))?;
            if session.files.is_empty() && session.reserved.is_empty() {
                sessions.remove(owner);
            }
            Ok(Value::Null)
        }
        "read" | "write" | "fstat" => {
            let fd = sync_fd_argument(args)?;
            let handle = {
                let sessions = sync_fd_sessions().lock().map_err(|_| {
                    anyhow::anyhow!("synchronous file descriptor registry poisoned")
                })?;
                sessions
                    .get(owner)
                    .and_then(|session| session.files.get(&fd))
                    .cloned()
                    .ok_or_else(|| anyhow::anyhow!("EBADF: invalid file descriptor"))?
            };
            anyhow::ensure!(
                !is_cancelled(),
                "ECANCELED: file descriptor operation was cancelled"
            );
            let mut file = handle
                .lock()
                .map_err(|_| anyhow::anyhow!("synchronous file descriptor is busy"))?;
            match operation {
                "fstat" => Ok(node_stat(&file.metadata()?)),
                "write" => {
                    let bytes = STANDARD.decode(
                        args["data"]
                            .as_str()
                            .ok_or_else(|| anyhow::anyhow!("missing base64 data"))?,
                    )?;
                    anyhow::ensure!(
                        bytes.len() <= MAX_SYNC_FD_IO_BYTES,
                        "EFBIG: synchronous write exceeds the 8 MiB limit"
                    );
                    let restore = explicit_file_position(args)?
                        .map(|position| -> Result<u64> {
                            let current = file.stream_position()?;
                            file.seek(SeekFrom::Start(position))?;
                            Ok(current)
                        })
                        .transpose()?;
                    let written = file.write(&bytes)?;
                    if let Some(position) = restore {
                        file.seek(SeekFrom::Start(position))?;
                    }
                    Ok(json!({"bytesWritten":written}))
                }
                "read" => {
                    let length = args["length"].as_u64().unwrap_or(0);
                    anyhow::ensure!(
                        length <= MAX_SYNC_FD_IO_BYTES as u64,
                        "EFBIG: synchronous read exceeds the 8 MiB limit"
                    );
                    let restore = explicit_file_position(args)?
                        .map(|position| -> Result<u64> {
                            let current = file.stream_position()?;
                            file.seek(SeekFrom::Start(position))?;
                            Ok(current)
                        })
                        .transpose()?;
                    let mut bytes = vec![0; length as usize];
                    let read = file.read(&mut bytes)?;
                    bytes.truncate(read);
                    if let Some(position) = restore {
                        file.seek(SeekFrom::Start(position))?;
                    }
                    Ok(json!({"bytesRead":read,"data":STANDARD.encode(bytes)}))
                }
                _ => unreachable!("operation was checked above"),
            }
        }
        _ => anyhow::bail!("ENOTSUP: synchronous file descriptor operation is unavailable"),
    }
}

fn sync_fd_argument(args: &Value) -> Result<i32> {
    let fd = args["fd"]
        .as_i64()
        .ok_or_else(|| anyhow::anyhow!("fd must be a non-negative integer"))?;
    anyhow::ensure!(
        (3..=i32::MAX as i64).contains(&fd),
        "EBADF: invalid file descriptor"
    );
    Ok(fd as i32)
}

fn explicit_file_position(args: &Value) -> Result<Option<u64>> {
    match args.get("position") {
        None | Some(Value::Null) => Ok(None),
        Some(Value::Number(position)) => position
            .as_u64()
            .map(Some)
            .ok_or_else(|| anyhow::anyhow!("EINVAL: file position must be a non-negative integer")),
        Some(_) => anyhow::bail!("EINVAL: invalid file position"),
    }
}

#[derive(Clone, Copy)]
struct NodeFsCpOptions {
    recursive: bool,
    force: bool,
    error_on_exist: bool,
    dereference: bool,
    preserve_timestamps: bool,
    verbatim_symlinks: bool,
    directory_only: bool,
    directory_finalize: bool,
    mode: u32,
}

fn node_cp_bool(args: &Value, name: &str, default: bool) -> Result<bool> {
    match args.get(name) {
        None | Some(Value::Null) => Ok(default),
        Some(Value::Bool(value)) => Ok(*value),
        Some(_) => anyhow::bail!("ERR_INVALID_ARG_TYPE: options.{name} must be a boolean"),
    }
}

fn node_cp_options(args: &Value) -> Result<NodeFsCpOptions> {
    let mode = match args.get("mode") {
        None | Some(Value::Null) => 0,
        Some(Value::Number(value)) => value
            .as_u64()
            .and_then(|value| u32::try_from(value).ok())
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "ERR_INVALID_ARG_VALUE: options.mode must be a non-negative 32-bit integer"
                )
            })?,
        Some(_) => anyhow::bail!("ERR_INVALID_ARG_TYPE: options.mode must be an integer"),
    };
    anyhow::ensure!(mode & !7 == 0, "EINVAL: unsupported fs.cp mode flags");
    let options = NodeFsCpOptions {
        recursive: node_cp_bool(args, "recursive", false)?,
        force: node_cp_bool(args, "force", true)?,
        error_on_exist: node_cp_bool(args, "errorOnExist", false)?,
        dereference: node_cp_bool(args, "dereference", false)?,
        preserve_timestamps: node_cp_bool(args, "preserveTimestamps", false)?,
        verbatim_symlinks: node_cp_bool(args, "verbatimSymlinks", false)?,
        directory_only: node_cp_bool(args, "directoryOnly", false)?,
        directory_finalize: node_cp_bool(args, "directoryFinalize", false)?,
        mode,
    };
    anyhow::ensure!(
        !(options.dereference && options.verbatim_symlinks),
        "ERR_INCOMPATIBLE_OPTION_PAIR: dereference and verbatimSymlinks are mutually exclusive"
    );
    Ok(options)
}

fn node_fs_cp(source: &Path, destination: &Path, args: &Value) -> Result<Value> {
    let options = node_cp_options(args)?;
    let created = node_cp_copy(source, destination, &options)?;
    if options.directory_only && !options.directory_finalize {
        return Ok(json!(created));
    }
    Ok(Value::Null)
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
        "cp" => {
            let destination = node_destination(args)?;
            node_fs_cp(path, destination, args)
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

fn node_cp_metadata(path: &Path, dereference: bool) -> std::io::Result<std::fs::Metadata> {
    if dereference {
        fs::metadata(path)
    } else {
        fs::symlink_metadata(path)
    }
}

fn node_cp_optional_metadata(path: &Path, dereference: bool) -> Result<Option<std::fs::Metadata>> {
    match node_cp_metadata(path, dereference) {
        Ok(metadata) => Ok(Some(metadata)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error.into()),
    }
}

fn node_cp_same_identity(
    source: &Path,
    destination: &Path,
    source_metadata: &std::fs::Metadata,
    destination_metadata: &std::fs::Metadata,
) -> Result<bool> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        if source_metadata.ino() != 0
            && source_metadata.dev() == destination_metadata.dev()
            && source_metadata.ino() == destination_metadata.ino()
        {
            return Ok(true);
        }
    }
    if source_metadata.file_type().is_symlink() || destination_metadata.file_type().is_symlink() {
        return Ok(false);
    }
    #[cfg(windows)]
    {
        let _ = (source_metadata, destination_metadata);
        if let (Ok(source), Ok(destination)) = (fs::File::open(source), fs::File::open(destination))
        {
            if same_file(&source, &destination)? {
                return Ok(true);
            }
        }
    }
    Ok(matches!(
        (fs::canonicalize(source), fs::canonicalize(destination)),
        (Ok(source), Ok(destination)) if source == destination
    ))
}

fn node_cp_normalize_absolute(path: &Path) -> Result<PathBuf> {
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()?.join(path)
    };
    let mut normalized = PathBuf::new();
    for component in absolute.components() {
        match component {
            Component::Prefix(prefix) => normalized.push(prefix.as_os_str()),
            Component::RootDir => normalized.push(component.as_os_str()),
            Component::CurDir => {}
            Component::ParentDir => {
                normalized.pop();
            }
            Component::Normal(name) => normalized.push(name),
        }
    }
    Ok(normalized)
}

fn node_cp_resolved_destination(path: &Path) -> Result<PathBuf> {
    let absolute = node_cp_normalize_absolute(path)?;
    let Some(file_name) = absolute.file_name() else {
        return Ok(fs::canonicalize(&absolute).unwrap_or(absolute));
    };
    let parent = absolute
        .parent()
        .ok_or_else(|| anyhow::anyhow!("EINVAL: destination has no parent"))?;
    let mut ancestor = parent.to_path_buf();
    let mut suffix = Vec::new();
    loop {
        match fs::metadata(&ancestor) {
            Ok(metadata) if metadata.is_dir() => break,
            Ok(_) => anyhow::bail!("ENOTDIR: destination parent is not a directory"),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                let name = ancestor
                    .file_name()
                    .ok_or_else(|| anyhow::anyhow!("ENOENT: destination parent does not exist"))?
                    .to_os_string();
                suffix.push(name);
                anyhow::ensure!(ancestor.pop(), "ENOENT: destination parent does not exist");
            }
            Err(error) => return Err(error.into()),
        }
    }
    let mut resolved = fs::canonicalize(&ancestor)?;
    for name in suffix.iter().rev() {
        resolved.push(name);
    }
    resolved.push(file_name);
    node_cp_normalize_absolute(&resolved)
}

fn node_cp_is_subdirectory(source: &Path, destination: &Path) -> Result<bool> {
    let source_absolute = node_cp_normalize_absolute(source)?;
    let destination_absolute = node_cp_resolved_destination(destination)?;
    if destination_absolute.starts_with(&source_absolute) {
        return Ok(true);
    }
    let source_real = fs::canonicalize(source)?;
    Ok(destination_absolute.starts_with(source_real))
}

fn node_cp_symlink_target(link: &Path, target: &Path, verbatim: bool) -> Result<PathBuf> {
    if verbatim || target.is_absolute() {
        return Ok(target.to_path_buf());
    }
    let parent = link.parent().unwrap_or_else(|| Path::new("."));
    node_cp_normalize_absolute(&parent.join(target))
}

fn node_cp_path_is_within(parent: &Path, child: &Path) -> Result<bool> {
    let parent = node_cp_normalize_absolute(parent)?;
    let child = node_cp_normalize_absolute(child)?;
    Ok(child.starts_with(parent))
}

fn node_cp_create_symlink(target: &Path, destination: &Path, source: &Path) -> Result<()> {
    #[cfg(unix)]
    {
        let _ = source;
        std::os::unix::fs::symlink(target, destination)?;
    }
    #[cfg(windows)]
    {
        let is_directory = fs::metadata(source).is_ok_and(|metadata| metadata.is_dir());
        if is_directory {
            std::os::windows::fs::symlink_dir(target, destination)?;
        } else {
            std::os::windows::fs::symlink_file(target, destination)?;
        }
    }
    #[cfg(not(any(unix, windows)))]
    {
        let _ = (target, destination, source);
        anyhow::bail!("ENOTSUP: symbolic link copying is not supported on this platform");
    }
    Ok(())
}

fn node_cp_copy_link(
    source: &Path,
    destination: &Path,
    options: &NodeFsCpOptions,
    destination_metadata: Option<&std::fs::Metadata>,
) -> Result<()> {
    let raw_target = fs::read_link(source)?;
    let target = node_cp_symlink_target(source, &raw_target, options.verbatim_symlinks)?;
    let Some(destination_metadata) = destination_metadata else {
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent)?;
        }
        return node_cp_create_symlink(&target, destination, source);
    };
    if !destination_metadata.file_type().is_symlink() {
        // Node attempts symlink() against a non-link destination; it reports
        // EEXIST and leaves the existing path untouched.
        anyhow::bail!("EEXIST: destination already exists");
    }

    let raw_destination_target = fs::read_link(destination)?;
    let resolved_source_target = node_cp_symlink_target(source, &raw_target, false)?;
    let resolved_destination_target =
        node_cp_symlink_target(destination, &raw_destination_target, false)?;
    anyhow::ensure!(
        !node_cp_path_is_within(&resolved_source_target, &resolved_destination_target)?,
        "ERR_FS_CP_EINVAL: source and destination symlink targets overlap"
    );
    let source_target_metadata = fs::metadata(source)?;
    anyhow::ensure!(
        !source_target_metadata.is_dir()
            || !node_cp_path_is_within(&resolved_destination_target, &resolved_source_target)?,
        "ERR_FS_CP_SYMLINK_TO_SUBDIRECTORY: cannot copy a symlink into its target directory"
    );
    fs::remove_file(destination)?;
    node_cp_create_symlink(&target, destination, source)
}

fn node_cp_copy_file(
    source: &Path,
    destination: &Path,
    source_metadata: &std::fs::Metadata,
    destination_metadata: Option<&std::fs::Metadata>,
    options: &NodeFsCpOptions,
) -> Result<()> {
    if let Some(destination_metadata) = destination_metadata {
        anyhow::ensure!(
            !destination_metadata.is_dir(),
            "ERR_FS_CP_NON_DIR_TO_DIR: cannot overwrite a directory with a non-directory"
        );
        if options.force {
            fs::remove_file(destination)?;
        } else if options.error_on_exist {
            anyhow::bail!("ERR_FS_CP_EEXIST: destination already exists");
        } else {
            return Ok(());
        }
    }
    if options.mode & 4 != 0 {
        anyhow::bail!("ENOTSUP: COPYFILE_FICLONE_FORCE is unavailable");
    }
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent)?;
    }
    if options.mode & 1 != 0 {
        let mut input = fs::File::open(source)?;
        let mut output = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(destination)?;
        if let Err(error) = std::io::copy(&mut input, &mut output) {
            drop(output);
            let _ = fs::remove_file(destination);
            return Err(error.into());
        }
    } else {
        fs::copy(source, destination)?;
    }
    if options.preserve_timestamps {
        let source_metadata = fs::metadata(source)?;
        let times = std::fs::FileTimes::new()
            .set_accessed(source_metadata.accessed()?)
            .set_modified(source_metadata.modified()?);
        OpenOptions::new()
            .write(true)
            .open(destination)?
            .set_times(times)?;
    }
    fs::set_permissions(destination, source_metadata.permissions())?;
    Ok(())
}

fn node_cp_copy_directory(
    source: &Path,
    destination: &Path,
    source_metadata: &std::fs::Metadata,
    destination_metadata: Option<&std::fs::Metadata>,
    options: &NodeFsCpOptions,
) -> Result<bool> {
    if options.directory_finalize {
        anyhow::ensure!(
            destination_metadata.is_some_and(|metadata| metadata.is_dir()),
            "ERR_FS_CP_EINVAL: cannot finalize a directory that was not created"
        );
        fs::set_permissions(destination, source_metadata.permissions())?;
        return Ok(false);
    }
    let created = if destination_metadata.is_some() {
        false
    } else {
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::create_dir(destination)?;
        true
    };
    if !options.directory_only {
        for entry in fs::read_dir(source)? {
            let entry = entry?;
            node_cp_copy(&entry.path(), &destination.join(entry.file_name()), options)?;
        }
    }
    if created && !options.directory_only {
        fs::set_permissions(destination, source_metadata.permissions())?;
    }
    Ok(created)
}

fn node_cp_copy(source: &Path, destination: &Path, options: &NodeFsCpOptions) -> Result<bool> {
    let source_metadata = node_cp_metadata(source, options.dereference)?;
    let destination_metadata = node_cp_optional_metadata(destination, options.dereference)?;
    if let Some(destination_metadata) = destination_metadata.as_ref() {
        anyhow::ensure!(
            !node_cp_same_identity(source, destination, &source_metadata, destination_metadata)?,
            "ERR_FS_CP_EINVAL: source and destination are the same file"
        );
        if source_metadata.is_dir() && !destination_metadata.is_dir() {
            anyhow::bail!(
                "ERR_FS_CP_DIR_TO_NON_DIR: cannot overwrite a non-directory with a directory"
            );
        }
        if !source_metadata.is_dir() && destination_metadata.is_dir() {
            anyhow::bail!(
                "ERR_FS_CP_NON_DIR_TO_DIR: cannot overwrite a directory with a non-directory"
            );
        }
    }
    if source_metadata.is_dir() && node_cp_is_subdirectory(source, destination)? {
        anyhow::bail!("ERR_FS_CP_EINVAL: cannot copy a directory to a subdirectory of itself");
    }
    let file_type = source_metadata.file_type();
    if source_metadata.is_dir() {
        if !options.recursive {
            anyhow::bail!("ERR_FS_EISDIR: source is a directory (not copied)");
        }
        return node_cp_copy_directory(
            source,
            destination,
            &source_metadata,
            destination_metadata.as_ref(),
            options,
        );
    }
    if file_type.is_symlink() {
        node_cp_copy_link(source, destination, options, destination_metadata.as_ref())?;
        return Ok(false);
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::FileTypeExt;
        if file_type.is_socket() {
            anyhow::bail!("ERR_FS_CP_SOCKET: cannot copy a socket file");
        }
        if file_type.is_fifo() {
            anyhow::bail!("ERR_FS_CP_FIFO_PIPE: cannot copy a FIFO pipe");
        }
        if file_type.is_char_device() || file_type.is_block_device() {
            node_cp_copy_file(
                source,
                destination,
                &source_metadata,
                destination_metadata.as_ref(),
                options,
            )?;
            return Ok(false);
        }
    }
    if source_metadata.is_file() {
        node_cp_copy_file(
            source,
            destination,
            &source_metadata,
            destination_metadata.as_ref(),
            options,
        )?;
        return Ok(false);
    }
    anyhow::bail!("ERR_FS_CP_UNKNOWN: cannot copy an unknown file type")
}

fn node_destination(args: &Value) -> Result<&Path> {
    Ok(Path::new(
        args["destination"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("missing destination"))?,
    ))
}

fn node_open(path: &Path, flag: &str, mode: u32) -> Result<std::fs::File> {
    node_open_with_options(path, flag, mode, false)
}

fn node_open_sync_fd(path: &Path, flag: &str, mode: u32) -> Result<std::fs::File> {
    ensure_not_windows_device_path(path)?;
    let file = node_open_with_options(path, flag, mode, true)?;
    anyhow::ensure!(
        file.metadata()?.is_file(),
        "EINVAL: sync file descriptors require regular files"
    );
    Ok(file)
}

fn node_open_with_options(
    path: &Path,
    flag: &str,
    mode: u32,
    nonblocking: bool,
) -> Result<std::fs::File> {
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
        if nonblocking {
            options.custom_flags(libc::O_NONBLOCK);
        }
    }
    #[cfg(not(unix))]
    let _ = (mode, nonblocking);
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
    fn ipc_text_files_enforce_utf8_and_total_append_size() {
        let temp = TestDir::new();
        let path = temp.path().join("text.txt");
        write_ipc_text_file(&path, "中文".as_bytes(), false, false, 0o600).unwrap();
        write_ipc_text_file(&path, "追加".as_bytes(), true, false, 0o600).unwrap();
        assert_eq!(read_ipc_text_file(&path).unwrap(), "中文追加");

        std::fs::write(&path, vec![b'x'; MAX_IPC_TEXT_FILE_BYTES as usize]).unwrap();
        let error = write_ipc_text_file(&path, b"y", true, false, 0o600).unwrap_err();
        assert!(error.to_string().contains("EFBIG"));
        assert_eq!(
            std::fs::metadata(&path).unwrap().len(),
            MAX_IPC_TEXT_FILE_BYTES
        );

        std::fs::write(&path, [0xff]).unwrap();
        assert!(
            read_ipc_text_file(&path)
                .unwrap_err()
                .to_string()
                .contains("ERR_INVALID_ENCODING")
        );
    }

    #[test]
    fn ipc_fs_operation_manifest_covers_the_intended_metadata_surface_only() {
        for operation in [
            "stat", "lstat", "readdir", "access", "realpath", "mkdir", "rename", "copyFile", "rm",
            "unlink", "cp",
        ] {
            assert!(IPC_NODE_OPERATIONS.contains(&operation), "{operation}");
        }
        for operation in [
            "readFile",
            "writeFile",
            "appendFile",
            "open",
            "read",
            "write",
            "fstat",
            "close",
            "watch",
        ] {
            assert!(!IPC_NODE_OPERATIONS.contains(&operation), "{operation}");
        }
    }

    #[test]
    fn synchronous_logical_file_descriptors_are_session_owned_and_preserve_offsets() {
        use base64::{Engine, engine::general_purpose::STANDARD};

        let temp = TestDir::new();
        let path = temp.path().join("sync-fd.txt");
        let suffix = NEXT_TEMP_DIR.fetch_add(1, Ordering::Relaxed);
        let owner = (250, format!("fd-owner-{}-{suffix}", std::process::id()));
        let other_session = (
            250,
            format!("fd-other-session-{}-{suffix}", std::process::id()),
        );
        let other_window = (251, owner.1.clone());
        let fd = sync_fd_operation(
            &owner,
            "open",
            &json!({"path":path,"flag":"w+","mode":384}),
            || false,
        )
        .unwrap()
        .as_i64()
        .unwrap() as i32;

        assert_eq!(
            sync_fd_operation(
                &owner,
                "write",
                &json!({"fd":fd,"data":STANDARD.encode(b"hello"),"position":null}),
                || false,
            )
            .unwrap()["bytesWritten"],
            5
        );
        assert_eq!(
            sync_fd_operation(
                &owner,
                "write",
                &json!({"fd":fd,"data":STANDARD.encode(b"X"),"position":1}),
                || false,
            )
            .unwrap()["bytesWritten"],
            1
        );
        let read = sync_fd_operation(
            &owner,
            "read",
            &json!({"fd":fd,"length":5,"position":0}),
            || false,
        )
        .unwrap();
        assert_eq!(
            STANDARD.decode(read["data"].as_str().unwrap()).unwrap(),
            b"hXllo"
        );
        let stats = sync_fd_operation(&owner, "fstat", &json!({"fd":fd}), || false).unwrap();
        assert_eq!(stats["isFile"], true);
        assert_eq!(stats["size"], 5);
        assert!(sync_fd_operation(&other_session, "fstat", &json!({"fd":fd}), || false).is_err());
        assert!(sync_fd_operation(&other_window, "fstat", &json!({"fd":fd}), || false).is_err());

        sync_fd_operation(&owner, "close", &json!({"fd":fd}), || false).unwrap();
        assert!(sync_fd_operation(&owner, "fstat", &json!({"fd":fd}), || false).is_err());
        let reused = sync_fd_operation(&owner, "open", &json!({"path":path,"flag":"r"}), || false)
            .unwrap()
            .as_i64()
            .unwrap();
        assert_eq!(reused, fd as i64);
        cancel_sync_fd_session(owner.0, &owner.1);
        assert!(!sync_fd_sessions().lock().unwrap().contains_key(&owner));
        assert!(sync_fd_operation(&owner, "fstat", &json!({"fd":reused}), || false).is_err());
    }

    #[test]
    fn synchronous_file_descriptor_session_cleanup_is_window_scoped() {
        let temp = TestDir::new();
        let path = temp.path().join("sync-fd-window.txt");
        let suffix = NEXT_TEMP_DIR.fetch_add(1, Ordering::Relaxed);
        let owner = (248, format!("fd-window-owner-{suffix}"));
        let sibling_window = (247, owner.1.clone());
        let fd = sync_fd_operation(&owner, "open", &json!({"path":path,"flag":"w"}), || false)
            .unwrap()
            .as_i64()
            .unwrap();
        let sibling_fd = sync_fd_operation(
            &sibling_window,
            "open",
            &json!({"path":path,"flag":"r"}),
            || false,
        )
        .unwrap()
        .as_i64()
        .unwrap();
        cancel_sync_fd_window(owner.0);
        assert!(sync_fd_operation(&owner, "fstat", &json!({"fd":fd}), || false).is_err());
        assert!(
            sync_fd_operation(&sibling_window, "fstat", &json!({"fd":sibling_fd}), || {
                false
            })
            .is_ok()
        );
        cancel_sync_fd_window(sibling_window.0);
    }

    #[test]
    fn cancelled_sync_open_does_not_leave_a_registered_descriptor() {
        let temp = TestDir::new();
        let path = temp.path().join("cancelled-open.txt");
        let suffix = NEXT_TEMP_DIR.fetch_add(1, Ordering::Relaxed);
        let owner = (246, format!("fd-cancelled-open-{suffix}"));
        let checks = std::cell::Cell::new(0);
        let result = sync_fd_operation(&owner, "open", &json!({"path":path,"flag":"w"}), || {
            let current = checks.get() + 1;
            checks.set(current);
            current >= 3
        });
        assert!(result.unwrap_err().to_string().contains("ECANCELED"));
        assert!(!sync_fd_sessions().lock().unwrap().contains_key(&owner));
    }

    #[test]
    fn synchronous_file_descriptors_are_scoped_to_window_and_session() {
        use base64::{Engine, engine::general_purpose::STANDARD};

        let temp = TestDir::new();
        let path = temp.path().join("sync-fd.txt");
        let suffix = NEXT_TEMP_DIR.fetch_add(1, Ordering::Relaxed);
        let owner = (250, format!("fd-owner-{}-{suffix}", std::process::id()));
        let sibling = (250, format!("fd-sibling-{}-{suffix}", std::process::id()));
        let fd = sync_fd_operation(
            &owner,
            "open",
            &json!({"path":path,"flag":"w+","mode":384}),
            || false,
        )
        .unwrap()
        .as_i64()
        .unwrap() as i32;

        assert_eq!(
            sync_fd_operation(
                &owner,
                "write",
                &json!({"fd":fd,"data":STANDARD.encode(b"hello"),"position":null}),
                || false,
            )
            .unwrap()["bytesWritten"],
            5
        );
        let stats = sync_fd_operation(&owner, "fstat", &json!({"fd":fd}), || false).unwrap();
        assert_eq!(stats["isFile"], true);
        assert_eq!(stats["size"], 5);
        let read = sync_fd_operation(
            &owner,
            "read",
            &json!({"fd":fd,"length":5,"position":0}),
            || false,
        )
        .unwrap();
        assert_eq!(
            STANDARD.decode(read["data"].as_str().unwrap()).unwrap(),
            b"hello"
        );

        let wrong_owner =
            sync_fd_operation(&sibling, "fstat", &json!({"fd":fd}), || false).unwrap_err();
        assert!(wrong_owner.to_string().contains("EBADF"));
        sync_fd_operation(&owner, "close", &json!({"fd":fd}), || false).unwrap();
        assert!(sync_fd_operation(&owner, "fstat", &json!({"fd":fd}), || false).is_err());
    }

    #[test]
    fn synchronous_session_cleanup_closes_all_owned_file_descriptors() {
        let temp = TestDir::new();
        let path = temp.path().join("sync-fd-cleanup.txt");
        let suffix = NEXT_TEMP_DIR.fetch_add(1, Ordering::Relaxed);
        let owner = (249, format!("fd-cleanup-{}-{suffix}", std::process::id()));
        let fd = sync_fd_operation(&owner, "open", &json!({"path":path,"flag":"w"}), || false)
            .unwrap()
            .as_i64()
            .unwrap();
        cancel_sync_fd_session(owner.0, &owner.1);
        assert!(sync_fd_operation(&owner, "fstat", &json!({"fd":fd}), || false).is_err());
    }

    #[test]
    fn ipc_fs_directory_workflow_creates_renames_copies_lists_and_removes() {
        let temp = TestDir::new();
        let root = temp.path().join("workflow");
        let nested = root.join("nested");
        node_fs_operation("mkdir", &json!({"path":nested,"recursive":true})).unwrap();
        let original = nested.join("original.txt");
        write_ipc_text_file(&original, b"workflow", false, false, 0o600).unwrap();
        let renamed = nested.join("renamed.txt");
        node_fs_operation("rename", &json!({"path":original,"destination":renamed})).unwrap();
        let copied = nested.join("copied.txt");
        node_fs_operation(
            "copyFile",
            &json!({"path":renamed,"destination":copied,"flags":0}),
        )
        .unwrap();
        let entries = read_ipc_directory(&nested).unwrap();
        let names = entries
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|entry| entry["name"].as_str())
            .collect::<std::collections::HashSet<_>>();
        assert!(names.contains("renamed.txt"));
        assert!(names.contains("copied.txt"));
        assert!(!names.contains("original.txt"));

        let copy = root.join("copy");
        node_fs_operation(
            "cp",
            &json!({"path":nested,"destination":copy,"recursive":true}),
        )
        .unwrap();
        assert_eq!(
            read_ipc_text_file(&copy.join("renamed.txt")).unwrap(),
            "workflow"
        );
        node_fs_operation("rm", &json!({"path":root,"recursive":true})).unwrap();
        assert!(!root.exists());
    }

    #[test]
    fn node_cp_merges_directories_and_obeys_force_error_on_exist_and_mode() {
        let temp = TestDir::new();
        let source = temp.path().join("source");
        let destination = temp.path().join("destination");
        fs::create_dir_all(source.join("nested")).unwrap();
        fs::write(source.join("existing.txt"), "new").unwrap();
        fs::write(source.join("nested/new.txt"), "nested").unwrap();
        fs::create_dir_all(&destination).unwrap();
        fs::write(destination.join("existing.txt"), "old").unwrap();

        node_fs_operation(
            "cp",
            &json!({"path":source,"destination":destination,"recursive":true,"force":false}),
        )
        .unwrap();
        assert_eq!(
            fs::read_to_string(destination.join("existing.txt")).unwrap(),
            "old"
        );
        assert_eq!(
            fs::read_to_string(destination.join("nested/new.txt")).unwrap(),
            "nested"
        );

        let error = node_fs_operation(
            "cp",
            &json!({"path":source,"destination":destination,"recursive":true,"force":false,"errorOnExist":true}),
        )
        .unwrap_err();
        assert!(error.to_string().contains("ERR_FS_CP_EEXIST"));
        assert_eq!(
            fs::read_to_string(destination.join("existing.txt")).unwrap(),
            "old"
        );

        node_fs_operation(
            "cp",
            &json!({"path":source,"destination":destination,"recursive":true,"force":true,"errorOnExist":true}),
        )
        .unwrap();
        assert_eq!(
            fs::read_to_string(destination.join("existing.txt")).unwrap(),
            "new"
        );

        let forced_clone = node_fs_operation(
            "cp",
            &json!({"path":source.join("existing.txt"),"destination":temp.path().join("clone.txt"),"mode":4}),
        )
        .unwrap_err();
        assert!(forced_clone.to_string().contains("ENOTSUP"));
    }

    #[test]
    fn node_cp_preserves_types_errors_and_rejects_copy_into_self() {
        let temp = TestDir::new();
        let source_dir = temp.path().join("source-dir");
        fs::create_dir_all(&source_dir).unwrap();
        fs::write(source_dir.join("file.txt"), "content").unwrap();
        let destination_file = temp.path().join("destination-file");
        fs::write(&destination_file, "old").unwrap();
        let no_recursive = node_fs_operation(
            "cp",
            &json!({"path":source_dir,"destination":temp.path().join("copy")} ),
        )
        .unwrap_err();
        assert!(no_recursive.to_string().contains("ERR_FS_EISDIR"));

        let directory_to_file = node_fs_operation(
            "cp",
            &json!({"path":source_dir,"destination":destination_file,"recursive":true}),
        )
        .unwrap_err();
        assert!(
            directory_to_file
                .to_string()
                .contains("ERR_FS_CP_DIR_TO_NON_DIR")
        );

        let file_to_directory = node_fs_operation(
            "cp",
            &json!({"path":source_dir.join("file.txt"),"destination":source_dir,"recursive":true}),
        )
        .unwrap_err();
        assert!(
            file_to_directory
                .to_string()
                .contains("ERR_FS_CP_NON_DIR_TO_DIR")
        );

        let into_self = node_fs_operation(
            "cp",
            &json!({"path":source_dir,"destination":source_dir.join("child"),"recursive":true}),
        )
        .unwrap_err();
        assert!(into_self.to_string().contains("ERR_FS_CP_EINVAL"));

        let incompatible = node_fs_operation(
            "cp",
            &json!({"path":source_dir.join("file.txt"),"destination":temp.path().join("copy.txt"),"dereference":true,"verbatimSymlinks":true}),
        )
        .unwrap_err();
        assert!(
            incompatible
                .to_string()
                .contains("ERR_INCOMPATIBLE_OPTION_PAIR")
        );
    }

    #[test]
    fn node_cp_directory_only_defers_source_permissions_until_finalize() {
        let temp = TestDir::new();
        let source = temp.path().join("source");
        let destination = temp.path().join("destination");
        fs::create_dir_all(source.join("nested")).unwrap();
        fs::write(source.join("should-not-copy.txt"), "content").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&source, fs::Permissions::from_mode(0o500)).unwrap();
        }
        let created = node_fs_operation(
            "cp",
            &json!({"path":source,"destination":destination,"recursive":true,"directoryOnly":true}),
        )
        .unwrap();
        assert_eq!(created, json!(true));
        assert!(destination.is_dir());
        assert!(fs::read_dir(&destination).unwrap().next().is_none());
        node_fs_operation(
            "cp",
            &json!({"path":source.join("should-not-copy.txt"),"destination":destination.join("should-not-copy.txt")}),
        )
        .unwrap();
        node_fs_operation(
            "cp",
            &json!({"path":source,"destination":destination,"recursive":true,"directoryOnly":true,"directoryFinalize":true}),
        )
        .unwrap();
        assert_eq!(
            fs::read_to_string(destination.join("should-not-copy.txt")).unwrap(),
            "content"
        );
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                fs::metadata(destination).unwrap().permissions().mode() & 0o777,
                0o500
            );
        }
    }

    #[test]
    fn node_cp_preserves_file_timestamps_when_requested() {
        use std::time::{Duration, SystemTime};

        let temp = TestDir::new();
        let source = temp.path().join("source.txt");
        let destination = temp.path().join("destination.txt");
        fs::write(&source, "timestamped").unwrap();
        let expected = SystemTime::UNIX_EPOCH + Duration::from_secs(1_700_000_000);
        fs::File::open(&source)
            .unwrap()
            .set_times(std::fs::FileTimes::new().set_modified(expected))
            .unwrap();

        node_fs_operation(
            "cp",
            &json!({"path":source,"destination":destination,"preserveTimestamps":true,"mode":2}),
        )
        .unwrap();

        assert_eq!(
            fs::metadata(destination).unwrap().modified().unwrap(),
            expected
        );
    }

    #[cfg(unix)]
    #[test]
    fn node_cp_preserves_or_dereferences_symlinks_and_resolves_targets() {
        use std::os::unix::fs::symlink;

        let temp = TestDir::new();
        let source = temp.path().join("source");
        fs::create_dir_all(source.join("target-dir")).unwrap();
        fs::write(source.join("target.txt"), "link contents").unwrap();
        fs::write(
            source.join("target-dir/nested.txt"),
            "directory link contents",
        )
        .unwrap();
        symlink("target.txt", source.join("link.txt")).unwrap();
        symlink("target-dir", source.join("link-dir")).unwrap();

        let destination = temp.path().join("default-copy");
        node_fs_operation(
            "cp",
            &json!({"path":source,"destination":destination,"recursive":true}),
        )
        .unwrap();
        let copied_link = destination.join("link.txt");
        assert!(
            fs::symlink_metadata(&copied_link)
                .unwrap()
                .file_type()
                .is_symlink()
        );
        let copied_target = fs::read_link(&copied_link).unwrap();
        assert!(copied_target.is_absolute());
        assert_eq!(fs::read_to_string(copied_target).unwrap(), "link contents");
        assert!(
            fs::symlink_metadata(destination.join("link-dir"))
                .unwrap()
                .file_type()
                .is_symlink()
        );

        let dereferenced = temp.path().join("dereferenced-copy");
        node_fs_operation(
            "cp",
            &json!({"path":source,"destination":dereferenced,"recursive":true,"dereference":true}),
        )
        .unwrap();
        assert!(
            !fs::symlink_metadata(dereferenced.join("link.txt"))
                .unwrap()
                .file_type()
                .is_symlink()
        );
        assert_eq!(
            fs::read_to_string(dereferenced.join("link.txt")).unwrap(),
            "link contents"
        );
        assert!(
            !fs::symlink_metadata(dereferenced.join("link-dir"))
                .unwrap()
                .file_type()
                .is_symlink()
        );
        assert_eq!(
            fs::read_to_string(dereferenced.join("link-dir/nested.txt")).unwrap(),
            "directory link contents"
        );

        let verbatim = temp.path().join("verbatim-copy");
        node_fs_operation(
            "cp",
            &json!({"path":source,"destination":verbatim,"recursive":true,"verbatimSymlinks":true}),
        )
        .unwrap();
        assert_eq!(
            fs::read_link(verbatim.join("link.txt")).unwrap(),
            PathBuf::from("target.txt")
        );

        let conflicting_destination = temp.path().join("existing.txt");
        fs::write(&conflicting_destination, "keep").unwrap();
        let conflict = node_fs_operation(
            "cp",
            &json!({"path":source.join("link.txt"),"destination":conflicting_destination}),
        )
        .unwrap_err();
        assert!(conflict.to_string().contains("EEXIST"));
        assert_eq!(
            fs::read_to_string(temp.path().join("existing.txt")).unwrap(),
            "keep"
        );
    }

    #[cfg(unix)]
    #[test]
    fn node_cp_rejects_symlink_target_cycles() {
        use std::os::unix::fs::symlink;

        let temp = TestDir::new();
        let source = temp.path().join("source");
        fs::create_dir_all(source.join("target/sub")).unwrap();
        symlink("target", source.join("link-to-target")).unwrap();

        let destination_inside = temp.path().join("destination-inside");
        symlink(source.join("target/sub"), &destination_inside).unwrap();
        let cycle = node_fs_operation(
            "cp",
            &json!({"path":source.join("link-to-target"),"destination":destination_inside}),
        )
        .unwrap_err();
        assert!(cycle.to_string().contains("ERR_FS_CP_EINVAL"));

        let source_subdirectory = source.join("link-to-subdirectory");
        symlink("target/sub", &source_subdirectory).unwrap();
        let destination_ancestor = temp.path().join("destination-ancestor");
        symlink(source.join("target"), &destination_ancestor).unwrap();
        let ancestor = node_fs_operation(
            "cp",
            &json!({"path":source_subdirectory,"destination":destination_ancestor}),
        )
        .unwrap_err();
        assert!(
            ancestor
                .to_string()
                .contains("ERR_FS_CP_SYMLINK_TO_SUBDIRECTORY")
        );
    }

    #[test]
    fn synchronous_logical_file_descriptors_preserve_position_and_scope() {
        use base64::{Engine, engine::general_purpose::STANDARD};

        let temp = TestDir::new();
        let path = temp.path().join("sync-descriptor.txt");
        let owner = (250, "sync-fd-owner-a".to_owned());
        let other_session = (250, "sync-fd-owner-b".to_owned());
        let other_window = (251, owner.1.clone());
        let fd = sync_fd_operation(
            &owner,
            "open",
            &json!({"path":path,"flag":"w+","mode":384}),
            || false,
        )
        .unwrap()
        .as_i64()
        .unwrap() as i32;

        assert_eq!(
            sync_fd_operation(
                &owner,
                "write",
                &json!({"fd":fd,"data":STANDARD.encode(b"hello"),"position":null}),
                || false,
            )
            .unwrap()["bytesWritten"],
            5
        );
        assert_eq!(
            sync_fd_operation(
                &owner,
                "write",
                &json!({"fd":fd,"data":STANDARD.encode(b"X"),"position":1}),
                || false,
            )
            .unwrap()["bytesWritten"],
            1
        );
        let read = sync_fd_operation(
            &owner,
            "read",
            &json!({"fd":fd,"length":5,"position":0}),
            || false,
        )
        .unwrap();
        assert_eq!(
            STANDARD.decode(read["data"].as_str().unwrap()).unwrap(),
            b"hXllo"
        );
        let stats = sync_fd_operation(&owner, "fstat", &json!({"fd":fd}), || false).unwrap();
        assert_eq!(stats["isFile"], true);
        assert_eq!(stats["size"], 5);
        assert!(sync_fd_operation(&other_session, "fstat", &json!({"fd":fd}), || false).is_err());
        assert!(sync_fd_operation(&other_window, "fstat", &json!({"fd":fd}), || false).is_err());

        sync_fd_operation(&owner, "close", &json!({"fd":fd}), || false).unwrap();
        assert!(sync_fd_operation(&owner, "fstat", &json!({"fd":fd}), || false).is_err());
        let reused = sync_fd_operation(&owner, "open", &json!({"path":path,"flag":"r"}), || false)
            .unwrap()
            .as_i64()
            .unwrap();
        assert_eq!(reused, fd as i64);
        cancel_sync_fd_session(owner.0, &owner.1);
        assert!(!sync_fd_sessions().lock().unwrap().contains_key(&owner));
        assert!(sync_fd_operation(&owner, "fstat", &json!({"fd":reused}), || false).is_err());
    }

    #[test]
    fn synchronous_file_descriptor_registry_is_bounded_and_window_scoped() {
        let temp = TestDir::new();
        let path = temp.path().join("sync-descriptor-limit.txt");
        let suffix = NEXT_TEMP_DIR.fetch_add(1, Ordering::Relaxed);
        let owner = (244, format!("sync-fd-window-cleanup-{suffix}"));
        let other = (243, owner.1.clone());
        let fd = sync_fd_operation(&owner, "open", &json!({"path":path,"flag":"w"}), || false)
            .unwrap()
            .as_i64()
            .unwrap();
        assert!(sync_fd_sessions().lock().unwrap().contains_key(&owner));
        cancel_sync_fd_window(owner.0);
        assert!(!sync_fd_sessions().lock().unwrap().contains_key(&owner));
        assert!(!sync_fd_sessions().lock().unwrap().contains_key(&other));
        assert!(sync_fd_operation(&owner, "fstat", &json!({"fd":fd}), || false).is_err());
    }

    #[test]
    fn synchronous_descriptor_quota_failure_does_not_truncate_target() {
        let temp = TestDir::new();
        let path = temp.path().join("sync-descriptor-quota.txt");
        std::fs::write(&path, b"preserve this content").unwrap();
        let owner = (245, format!("sync-fd-quota-{}", std::process::id()));
        for _ in 0..MAX_SYNC_FDS_PER_SESSION {
            sync_fd_operation(&owner, "open", &json!({"path":path,"flag":"r"}), || false).unwrap();
        }

        let error = sync_fd_operation(&owner, "open", &json!({"path":path,"flag":"w"}), || false)
            .unwrap_err();
        assert!(error.to_string().contains("EMFILE"));
        assert_eq!(std::fs::read(&path).unwrap(), b"preserve this content");
        cancel_sync_fd_session(owner.0, &owner.1);
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
