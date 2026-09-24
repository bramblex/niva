use crate::app::NivaApp;
use crate::app::api_manager::{ApiManager, ApiRequest, CallContext};
use crate::app::main_exec::run_on_main;
use crate::app::window_manager::window::NivaWindow;
use anyhow::{Ok, Result};
use serde::Deserialize;
use serde_json::{Value, json};
use std::path::Path;
use std::sync::Arc;
use tao::event_loop::ControlFlow;

pub fn register_apis(api_manager: &mut ApiManager) {
    api_manager.register_api("process.pid", pid);
    api_manager.register_blocking_api_with("process.spawnSync", Some(None), spawn_sync);
    api_manager.register_blocking_api("process.write", write_stdio);
    api_manager.register_stream_api_with("process.stdin", Some(None), read_stdin);
    api_manager.register_api("process.currentDir", current_dir);
    api_manager.register_api("process.currentExe", current_exe);
    api_manager.register_api("process.env", env);
    api_manager.register_api("process.args", args);
    api_manager.register_api("process.setCurrentDir", set_current_dir);
    api_manager.register_api("process.exit", exit);
    api_manager.register_api("process.version", version);
    api_manager.register_blocking_api("process.open", open);
    api_manager.register_stream_api_with("process.execStream", Some(None), exec_stream);
    api_manager.register_stream_api("process.signal", signal_child);
}

async fn pid(_app: Arc<NivaApp>, _window: Arc<NivaWindow>, _request: ApiRequest) -> Result<u32> {
    Ok(process_id())
}

fn process_id() -> u32 {
    std::process::id()
}

async fn current_dir(
    _app: Arc<NivaApp>,
    _window: Arc<NivaWindow>,
    _request: ApiRequest,
) -> Result<Value> {
    current_directory_value()
}

fn current_directory_value() -> Result<Value> {
    Ok(json!(std::env::current_dir()?))
}

async fn current_exe(
    _app: Arc<NivaApp>,
    _window: Arc<NivaWindow>,
    _request: ApiRequest,
) -> Result<Value> {
    current_executable_value()
}

fn current_executable_value() -> Result<Value> {
    Ok(json!(std::env::current_exe()?))
}

async fn env(_app: Arc<NivaApp>, _window: Arc<NivaWindow>, _request: ApiRequest) -> Result<Value> {
    Ok(environment_value(std::env::vars()))
}

fn environment_value(variables: impl IntoIterator<Item = (String, String)>) -> Value {
    json!(
        variables
            .into_iter()
            .collect::<std::collections::HashMap<String, String>>()
    )
}

async fn args(_app: Arc<NivaApp>, _window: Arc<NivaWindow>, _request: ApiRequest) -> Result<Value> {
    Ok(arguments_value(std::env::args()))
}

fn arguments_value(arguments: impl IntoIterator<Item = String>) -> Value {
    json!(arguments.into_iter().collect::<Vec<String>>())
}

async fn set_current_dir(
    _app: Arc<NivaApp>,
    _window: Arc<NivaWindow>,
    request: ApiRequest,
) -> Result<()> {
    let (path,) = request.args().get::<(String,)>()?;
    set_current_directory(Path::new(&path))
}

fn set_current_directory(path: &Path) -> Result<()> {
    std::env::set_current_dir(path)?;
    Ok(())
}

async fn exit(app: Arc<NivaApp>, _window: Arc<NivaWindow>, request: ApiRequest) -> Result<()> {
    let (code,): (Option<i32>,) = request.args().optional(1)?;
    run_on_main(&app, move |_target, control_flow| {
        *control_flow = ControlFlow::ExitWithCode(code.unwrap_or(0));
        Ok(())
    })
    .await
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ExecOptions {
    pub env: Option<std::collections::HashMap<String, String>>,
    pub current_dir: Option<String>,
    pub detached: Option<bool>,
}

fn build_exec_command(
    program: String,
    args: Option<Vec<String>>,
    options: Option<ExecOptions>,
) -> (std::process::Command, bool) {
    let mut cmd = std::process::Command::new(program);
    if let Some(args) = args {
        cmd.args(args);
    }
    let mut detached = false;
    if let Some(options) = options {
        if let Some(current_dir) = options.current_dir {
            cmd.current_dir(current_dir);
        }
        if let Some(env) = options.env {
            cmd.env_clear().envs(env);
        }
        detached = options.detached.unwrap_or(false);
    }
    cmd.stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());
    (cmd, detached)
}

fn open(_app: Arc<NivaApp>, _window: Arc<NivaWindow>, request: ApiRequest) -> Result<()> {
    let (uri,) = request.args().get::<(String,)>()?;
    opener::open(uri)?;
    Ok(())
}

async fn version(
    _app: Arc<NivaApp>,
    _window: Arc<NivaWindow>,
    _request: ApiRequest,
) -> Result<String> {
    Ok(package_version())
}

fn package_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

/// Full-duplex streaming exec: stdout/stderr arrive as `stdout`/`stderr`
/// stream events, stdin is fed from inbound binary chunks (END closes it),
/// and the terminal result carries the exit status. Cancellation, timeout,
/// or loss of the owning connection kills and reaps the child. `detached`
/// explicitly opts out: the child outlives the stream call.
async fn exec_stream(ctx: CallContext, request: ApiRequest) -> Result<()> {
    use std::io::Write;

    let (program, args, options): (String, Option<Vec<String>>, Option<ExecOptions>) =
        request.args().optional(3)?;
    let (mut cmd, detached) = build_exec_command(program, args, options);
    let mut child = cmd.spawn()?;
    let owner = (ctx.window.id, ctx.connection_id, ctx.id);
    child_pids()
        .lock()
        .map_err(|_| anyhow::anyhow!("child registry poisoned"))?
        .insert(owner, child.id());
    let _registration = ChildRegistration(owner);
    ctx.push("spawn", json!({"pid":child.id()}));

    // Detached: answer child id immediately, no pumps, no wait.
    if detached {
        let id = child.id();
        // Detached children intentionally outlive this call; nobody consumes
        // stdio (the parent ends of the pipes close when `child` is dropped).
        ctx.respond(Ok(json!(id)));
        return Ok(());
    }

    let stdout = child.stdout.take();
    let stderr = child.stderr.take();
    let stdin = child.stdin.take();

    // Keep Child ownership on a dedicated thread. The async handler only
    // awaits its result, so timeout/cancellation never blocks the API driver.
    let ctx2 = ctx.clone();
    let (wait_tx, wait_rx) = async_channel::bounded(1);
    if let Err((err, mut child)) =
        spawn_child_supervisor(child, move || ctx2.is_cancelled(), wait_tx)
    {
        return crate::blocking!({
            let _ = child.kill();
            child.wait()?;
            Err(anyhow::anyhow!("spawn child supervisor failed: {err}"))
        })
        .await;
    }

    // Byte pumps (dedicated threads: blocking reads, exact bytes).
    let mut pumps = Vec::new();
    if let Some(stdout) = stdout {
        pumps.push(pump_bytes(&ctx, stdout, false)?);
    }
    if let Some(stderr) = stderr {
        pumps.push(pump_bytes(&ctx, stderr, true)?);
    }

    // Stdin pump runs on its own thread: it must never precede wait()
    // (no stdin input would stall the child forever).
    if let Some(mut stdin) = stdin {
        let ctx2 = ctx.clone();
        std::thread::Builder::new()
            .name("niva-exec-stdin".into())
            .spawn(move || {
                while let Some(chunk) = ctx2.next_chunk_blocking() {
                    if stdin.write_all(&chunk.data).is_err() {
                        break;
                    }
                    if chunk.end {
                        break;
                    }
                }
                // Dropping stdin signals EOF to the child.
            })
            .map_err(|err| anyhow::anyhow!("spawn stdin pump failed: {err}"))?;
    }

    let outcome = wait_rx
        .recv()
        .await
        .map_err(|_| anyhow::anyhow!("child supervisor stopped without a result"))??;
    if let ChildWaitOutcome::Exited(status) = outcome {
        crate::blocking!({
            for pump in pumps {
                let _ = pump.join();
            }
            Ok(())
        })
        .await?;
        #[cfg(unix)]
        let signal = {
            use std::os::unix::process::ExitStatusExt;
            status.signal()
        };
        #[cfg(not(unix))]
        let signal: Option<i32> = None;
        ctx.respond(Ok(json!({ "status": status.code(),"signal":signal })));
    }
    Ok(())
}

#[derive(Debug)]
enum ChildWaitOutcome {
    Exited(std::process::ExitStatus),
    Cancelled,
}

/// Poll without blocking the API driver, killing and reaping on cancellation.
fn wait_for_child(
    mut child: std::process::Child,
    is_cancelled: impl Fn() -> bool,
) -> std::io::Result<ChildWaitOutcome> {
    loop {
        if let Some(status) = child.try_wait()? {
            return std::result::Result::Ok(ChildWaitOutcome::Exited(status));
        }
        if is_cancelled() {
            // The child can exit between try_wait() and kill(). If so, collect
            // its status rather than treating that harmless race as an error.
            if let Err(kill_error) = child.kill() {
                if let Some(status) = child.try_wait()? {
                    return std::result::Result::Ok(ChildWaitOutcome::Exited(status));
                }
                return Err(kill_error);
            }
            child.wait()?;
            return std::result::Result::Ok(ChildWaitOutcome::Cancelled);
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
}

/// Start the sole owner of a child process and report when it has been reaped.
fn spawn_child_supervisor(
    child: std::process::Child,
    is_cancelled: impl Fn() -> bool + Send + 'static,
    wait_tx: async_channel::Sender<std::io::Result<ChildWaitOutcome>>,
) -> std::result::Result<(), (std::io::Error, std::process::Child)> {
    use std::sync::Mutex;

    // Retain a recoverable handle if OS thread creation fails so the caller
    // can terminate and reap it on the blocking pool before returning an error.
    let child_slot = Arc::new(Mutex::new(Some(child)));
    let worker_slot = Arc::clone(&child_slot);
    let spawn_result = std::thread::Builder::new()
        .name("niva-exec-child".into())
        .spawn(move || {
            let child = worker_slot.lock().ok().and_then(|mut slot| slot.take());
            let result = match child {
                Some(child) => wait_for_child(child, is_cancelled),
                None => std::result::Result::Err(std::io::Error::other("child handle unavailable")),
            };
            let _ = wait_tx.send_blocking(result);
        });

    if let Err(err) = spawn_result {
        let child = child_slot
            .lock()
            .ok()
            .and_then(|mut slot| slot.take())
            .expect("failed thread creation must leave the child in its slot");
        return Err((err, child));
    }
    std::result::Result::Ok(())
}

/// Pump an output pipe in raw byte chunks into stream frames.
fn pump_bytes<R: std::io::Read + Send + 'static>(
    ctx: &CallContext,
    stream: R,
    is_stderr: bool,
) -> Result<std::thread::JoinHandle<()>> {
    let ctx = ctx.clone();
    std::thread::Builder::new()
        .name("niva-exec-pump".into())
        .spawn(move || {
            let mut buf = vec![0u8; 32768];
            let mut reader = stream;
            loop {
                if ctx.is_cancelled() {
                    break;
                }
                match reader.read(&mut buf) {
                    std::result::Result::Ok(0) => break,
                    std::result::Result::Ok(n) => {
                        if is_stderr {
                            ctx.chunk_stderr(&buf[..n], false);
                        } else {
                            ctx.chunk(&buf[..n], false);
                        }
                    }
                    std::result::Result::Err(_) => break,
                }
            }
            if is_stderr {
                ctx.chunk_stderr(&[], true);
            } else {
                ctx.chunk(&[], true);
            }
        })
        .map_err(|err| anyhow::anyhow!("spawn pump failed: {err}"))
}

fn write_stdio(app: Arc<NivaApp>, window: Arc<NivaWindow>, request: ApiRequest) -> Result<()> {
    use base64::Engine;
    use std::io::Write;
    anyhow::ensure!(window.id == 0, "process stdio is main-window-only");
    let (channel, data): (String, String) = request.args().get()?;
    let data = base64::engine::general_purpose::STANDARD.decode(data)?;
    match channel.as_str() {
        "stdout" => {
            anyhow::ensure!(
                !app.launch_info.arguments.stdio,
                "ERR_STDIO_RESERVED: stdout carries the host protocol"
            );
            std::io::stdout().lock().write_all(&data)?;
        }
        "stderr" => std::io::stderr().lock().write_all(&data)?,
        _ => anyhow::bail!("invalid stdio channel"),
    }
    Ok(())
}

async fn read_stdin(ctx: CallContext, _request: ApiRequest) -> Result<()> {
    anyhow::ensure!(ctx.window.id == 0, "process stdio is main-window-only");
    anyhow::ensure!(
        !ctx.app.launch_info.arguments.stdio,
        "ERR_STDIO_RESERVED: stdin carries the host protocol"
    );
    let owner = (ctx.window.id, ctx.connection_id, ctx.id);
    #[cfg(unix)]
    {
        crate::blocking!({
            use std::io::Read;
            let _owner = StdinReaderRegistration::acquire(owner)?;
            let mut stdin = std::io::stdin().lock();
            let mut buffer = [0; 65536];
            while !ctx.is_cancelled() {
                let mut poll = libc::pollfd {
                    fd: libc::STDIN_FILENO,
                    events: libc::POLLIN,
                    revents: 0,
                };
                // SAFETY: one initialized pollfd lives for the duration of the call.
                let ready = unsafe { libc::poll(&mut poll, 1, 100) };
                if ready < 0 {
                    return Err(std::io::Error::last_os_error().into());
                }
                if ready == 0 {
                    continue;
                }
                let size = stdin.read(&mut buffer)?;
                if size == 0 {
                    ctx.chunk(&[], true);
                    ctx.respond(Ok(Value::Null));
                    break;
                }
                ctx.chunk(&buffer[..size], false);
            }
            Ok(())
        })
        .await
    }
    #[cfg(windows)]
    {
        crate::blocking!(read_stdin_windows(ctx, owner)).await
    }
    #[cfg(not(unix))]
    #[cfg(not(windows))]
    {
        anyhow::bail!("ENOTSUP: native stdin not implemented on this platform")
    }
}

type StdinReaderKey = (u8, u64, u64);

fn stdin_reader_owner() -> &'static std::sync::Mutex<Option<StdinReaderKey>> {
    static OWNER: std::sync::OnceLock<std::sync::Mutex<Option<StdinReaderKey>>> =
        std::sync::OnceLock::new();
    OWNER.get_or_init(|| std::sync::Mutex::new(None))
}

struct StdinReaderRegistration(StdinReaderKey);

impl StdinReaderRegistration {
    fn acquire(owner: StdinReaderKey) -> Result<Self> {
        let mut active = stdin_reader_owner()
            .lock()
            .map_err(|_| anyhow::anyhow!("stdin reader registry poisoned"))?;
        anyhow::ensure!(
            active.is_none(),
            "EBUSY: process.stdin already has an active reader"
        );
        *active = Some(owner);
        Ok(Self(owner))
    }
}

impl Drop for StdinReaderRegistration {
    fn drop(&mut self) {
        if let std::result::Result::Ok(mut active) = stdin_reader_owner().lock()
            && *active == Some(self.0)
        {
            *active = None;
        }
    }
}

#[cfg(windows)]
fn read_stdin_windows(ctx: CallContext, owner: StdinReaderKey) -> Result<()> {
    use std::io::Read;
    use windows::Win32::{
        Foundation::CloseHandle,
        System::{
            IO::CancelSynchronousIo,
            Threading::{GetCurrentThreadId, OpenThread, THREAD_TERMINATE},
        },
    };

    let _owner = StdinReaderRegistration::acquire(owner)?;
    let (thread_id_tx, thread_id_rx) = async_channel::bounded(1);
    let reader_ctx = ctx.clone();
    let reader = std::thread::Builder::new()
        .name("niva-stdin-reader".into())
        .spawn(move || -> Result<()> {
            // Report the OS thread id before acquiring/reading stdin. Cancellation
            // is retried by the owner thread until this worker has fully exited.
            let _ = thread_id_tx.send_blocking(unsafe { GetCurrentThreadId() });
            let mut stdin = std::io::stdin().lock();
            let mut buffer = [0u8; 65536];
            loop {
                if reader_ctx.is_cancelled() {
                    break;
                }
                let size = match stdin.read(&mut buffer) {
                    std::result::Result::Ok(size) => size,
                    Err(_) if reader_ctx.is_cancelled() => break,
                    Err(error) => return Err(error.into()),
                };
                if reader_ctx.is_cancelled() {
                    break;
                }
                if size == 0 {
                    reader_ctx.chunk(&[], true);
                    reader_ctx.respond(Ok(Value::Null));
                    break;
                }
                reader_ctx.chunk(&buffer[..size], false);
            }
            Ok(())
        })?;
    let thread_id = thread_id_rx
        .recv_blocking()
        .map_err(|_| anyhow::anyhow!("stdin reader failed before publishing its thread id"))?;
    while !reader.is_finished() {
        if ctx.is_cancelled()
            && let std::result::Result::Ok(thread) =
                unsafe { OpenThread(THREAD_TERMINATE, false, thread_id) }
        {
            // ERROR_NOT_FOUND means no synchronous read is pending yet; retry
            // until the worker exits to cover the check/read race.
            let _ = unsafe { CancelSynchronousIo(thread) };
            let _ = unsafe { CloseHandle(thread) };
        }
        std::thread::sleep(std::time::Duration::from_millis(25));
    }
    reader
        .join()
        .map_err(|_| anyhow::anyhow!("stdin reader panicked"))??;
    Ok(())
}

fn spawn_sync(_app: Arc<NivaApp>, _window: Arc<NivaWindow>, request: ApiRequest) -> Result<Value> {
    let (program, args, options): (String, Option<Vec<String>>, Option<Value>) =
        request.args().optional(3)?;
    spawn_sync_command(
        program,
        args.unwrap_or_default(),
        options.unwrap_or_else(|| json!({})),
    )
}

fn spawn_sync_command(program: String, args: Vec<String>, options: Value) -> Result<Value> {
    use base64::{Engine, engine::general_purpose::STANDARD};
    use std::{
        io::{Read, Write},
        sync::atomic::{AtomicBool, Ordering},
        time::{Duration, Instant},
    };
    let mut command = std::process::Command::new(program);
    command.args(args);
    if let Some(cwd) = options["cwd"].as_str() {
        command.current_dir(cwd);
    }
    if let Some(env) = options["env"].as_object() {
        command.env_clear();
        for (key, value) in env {
            if let Some(value) = value.as_str() {
                command.env(key, value);
            }
        }
    }
    command
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());
    let input = STANDARD
        .decode(options["input"].as_str().unwrap_or(""))
        .map_err(|error| anyhow::anyhow!("invalid base64 spawnSync input: {error}"))?;
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        command.process_group(0);
    }
    let child = command.spawn()?;
    let pid = child.id();
    #[cfg(unix)]
    let mut child = SpawnSyncChild::new(child, pid, true);
    #[cfg(not(unix))]
    let mut child = SpawnSyncChild::new(child, pid);
    let max = options["maxBuffer"]
        .as_u64()
        .unwrap_or(1024 * 1024)
        .min(64 * 1024 * 1024) as usize;
    let overflow = Arc::new(AtomicBool::new(false));
    fn collect<R: Read + Send + 'static>(
        mut source: R,
        max: usize,
        overflow: Arc<AtomicBool>,
        name: &'static str,
    ) -> std::io::Result<std::thread::JoinHandle<std::io::Result<Vec<u8>>>> {
        std::thread::Builder::new()
            .name(name.into())
            .spawn(move || {
                let mut output = Vec::new();
                let mut buffer = [0; 8192];
                loop {
                    let size = source.read(&mut buffer)?;
                    if size == 0 {
                        break;
                    }
                    let keep = size.min(max.saturating_sub(output.len()));
                    output.extend_from_slice(&buffer[..keep]);
                    if keep < size {
                        overflow.store(true, Ordering::Release);
                        break;
                    }
                }
                std::result::Result::Ok(output)
            })
    }
    let stdout_pipe = child
        .child_mut()
        .stdout
        .take()
        .ok_or_else(|| anyhow::anyhow!("spawnSync stdout pipe unavailable"))?;
    let stdout = collect(stdout_pipe, max, overflow.clone(), "niva-spawn-sync-stdout")?;
    let stderr_pipe = child
        .child_mut()
        .stderr
        .take()
        .ok_or_else(|| anyhow::anyhow!("spawnSync stderr pipe unavailable"))?;
    let stderr = collect(stderr_pipe, max, overflow.clone(), "niva-spawn-sync-stderr")?;
    let mut stdin = child
        .child_mut()
        .stdin
        .take()
        .ok_or_else(|| anyhow::anyhow!("spawnSync stdin pipe unavailable"))?;
    let writer = std::thread::Builder::new()
        .name("niva-spawn-sync-stdin".into())
        .spawn(move || stdin.write_all(&input))?;
    let start = Instant::now();
    let timeout = options["timeout"].as_u64().unwrap_or(0);
    let mut failure = None;
    let status = loop {
        if let Some(status) = child.try_wait()? {
            break status;
        }
        if overflow.load(Ordering::Acquire)
            || (timeout > 0 && start.elapsed() >= Duration::from_millis(timeout))
        {
            failure = Some(if overflow.load(Ordering::Acquire) {
                "ENOBUFS"
            } else {
                "ETIMEDOUT"
            });
            break child.terminate_and_wait()?;
        }
        std::thread::sleep(Duration::from_millis(5));
    };
    let stdout_result = stdout.join();
    let stderr_result = stderr.join();
    let writer_result = writer.join();
    let stdout = stdout_result.map_err(|_| anyhow::anyhow!("stdout reader panicked"))??;
    let stderr = stderr_result.map_err(|_| anyhow::anyhow!("stderr reader panicked"))??;
    let writer_result = writer_result.map_err(|_| anyhow::anyhow!("stdin writer panicked"))?;
    if let Err(error) = writer_result
        && error.kind() != std::io::ErrorKind::BrokenPipe
    {
        return Err(error.into());
    }
    if overflow.load(Ordering::Acquire) {
        failure = Some("ENOBUFS");
    }
    #[cfg(unix)]
    let signal = {
        use std::os::unix::process::ExitStatusExt;
        status.signal()
    };
    #[cfg(not(unix))]
    let signal: Option<i32> = None;
    Ok(
        json!({"pid":pid,"status":status.code(),"signal":signal,"stdout":STANDARD.encode(stdout),"stderr":STANDARD.encode(stderr),"error":failure}),
    )
}

struct SpawnSyncChild {
    child: std::process::Child,
    #[cfg(unix)]
    pid: u32,
    #[cfg(unix)]
    process_group: bool,
    reaped: bool,
}

impl SpawnSyncChild {
    #[cfg(unix)]
    fn new(child: std::process::Child, pid: u32, process_group: bool) -> Self {
        Self {
            child,
            pid,
            process_group,
            reaped: false,
        }
    }

    #[cfg(not(unix))]
    fn new(child: std::process::Child, _pid: u32) -> Self {
        Self {
            child,
            reaped: false,
        }
    }

    fn child_mut(&mut self) -> &mut std::process::Child {
        &mut self.child
    }

    fn try_wait(&mut self) -> std::io::Result<Option<std::process::ExitStatus>> {
        let status = self.child.try_wait()?;
        if status.is_some() {
            self.reaped = true;
        }
        std::result::Result::Ok(status)
    }

    fn terminate_and_wait(&mut self) -> std::io::Result<std::process::ExitStatus> {
        self.terminate();
        let status = self.child.wait()?;
        self.reaped = true;
        std::result::Result::Ok(status)
    }

    fn terminate(&mut self) {
        #[cfg(unix)]
        if self.process_group {
            // SAFETY: this process group was created specifically for this child.
            let _ = unsafe { libc::kill(-(self.pid as i32), libc::SIGKILL) };
        }
        let _ = self.child.kill();
    }
}

impl Drop for SpawnSyncChild {
    fn drop(&mut self) {
        if self.reaped {
            return;
        }
        if matches!(self.child.try_wait(), std::result::Result::Ok(Some(_))) {
            self.reaped = true;
            return;
        }
        self.terminate();
        let _ = self.child.wait();
        self.reaped = true;
    }
}

type ChildKey = (u8, u64, u64);
fn child_pids() -> &'static std::sync::Mutex<std::collections::HashMap<ChildKey, u32>> {
    static PIDS: std::sync::OnceLock<std::sync::Mutex<std::collections::HashMap<ChildKey, u32>>> =
        std::sync::OnceLock::new();
    PIDS.get_or_init(|| std::sync::Mutex::new(std::collections::HashMap::new()))
}
struct ChildRegistration(ChildKey);
impl Drop for ChildRegistration {
    fn drop(&mut self) {
        if let std::result::Result::Ok(mut map) = child_pids().lock() {
            map.remove(&self.0);
        }
    }
}
async fn signal_child(ctx: CallContext, request: ApiRequest) -> Result<()> {
    let (call_id, signal): (u64, Option<String>) = request.args().optional(2)?;
    let signal = signal.as_deref().unwrap_or("SIGTERM");
    let pid = child_pids()
        .lock()
        .map_err(|_| anyhow::anyhow!("child registry poisoned"))?
        .get(&(ctx.window.id, ctx.connection_id, call_id))
        .copied();
    let Some(pid) = pid else {
        ctx.respond(Ok(false));
        return Ok(());
    };
    #[cfg(unix)]
    {
        let signal = match signal {
            "SIGTERM" => libc::SIGTERM,
            "SIGKILL" => libc::SIGKILL,
            "SIGINT" => libc::SIGINT,
            "SIGHUP" => libc::SIGHUP,
            "0" => 0,
            _ => anyhow::bail!("EINVAL: unsupported signal"),
        };
        // SAFETY: PID comes from a child created by this exact connection.
        let result = unsafe { libc::kill(pid as i32, signal) };
        ctx.respond(Ok(result == 0));
    }
    #[cfg(windows)]
    {
        use windows::Win32::{
            Foundation::CloseHandle,
            System::Threading::{OpenProcess, PROCESS_TERMINATE, TerminateProcess},
        };
        anyhow::ensure!(
            matches!(signal, "SIGTERM" | "SIGKILL" | "SIGINT"),
            "EINVAL: unsupported signal"
        );
        // SAFETY: process handle belongs to the owned child and is closed once.
        let killed = unsafe {
            match OpenProcess(PROCESS_TERMINATE, false, pid) {
                std::result::Result::Ok(handle) => {
                    let result = TerminateProcess(handle, 1).is_ok();
                    let _ = CloseHandle(handle);
                    result
                }
                Err(_) => false,
            }
        };
        ctx.respond(Ok(killed));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::time::{Duration, Instant};

    const CHILD_TEST_ENV: &str = "NIVA_PROCESS_SUPERVISOR_CHILD";
    const SET_CURRENT_DIR_CHILD_ENV: &str = "NIVA_SET_CURRENT_DIR_CHILD";
    const SET_CURRENT_DIR_TARGET_ENV: &str = "NIVA_SET_CURRENT_DIR_TARGET";

    static NEXT_TEMP_DIR: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

    struct TestDir(std::path::PathBuf);

    impl TestDir {
        fn new() -> Self {
            let id = NEXT_TEMP_DIR.fetch_add(1, Ordering::Relaxed);
            let path =
                std::env::temp_dir().join(format!("niva-process-api-{}-{id}", std::process::id()));
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

    #[test]
    fn process_metadata_methods_return_the_current_process_values() {
        assert_eq!(process_id(), std::process::id());

        let current_dir = current_directory_value().unwrap();
        assert_eq!(
            serde_json::from_value::<std::path::PathBuf>(current_dir).unwrap(),
            std::env::current_dir().unwrap()
        );

        let current_exe = current_executable_value().unwrap();
        let current_exe = serde_json::from_value::<std::path::PathBuf>(current_exe).unwrap();
        assert!(current_exe.is_absolute());
        assert!(current_exe.is_file());

        assert_eq!(package_version(), env!("CARGO_PKG_VERSION"));
    }

    #[test]
    fn stdin_reader_registration_allows_only_one_owner_and_releases_on_drop() {
        let first = StdinReaderRegistration::acquire((254, u64::MAX, u64::MAX)).unwrap();
        assert!(StdinReaderRegistration::acquire((254, u64::MAX, u64::MAX - 1)).is_err());
        drop(first);
        assert!(StdinReaderRegistration::acquire((254, u64::MAX, u64::MAX - 1)).is_ok());
    }

    #[test]
    fn env_and_args_are_returned_as_json_map_and_ordered_array() {
        let environment = environment_value([
            ("NIVA_TEST_A".to_string(), "first value".to_string()),
            ("NIVA_TEST_B".to_string(), "two".to_string()),
        ]);
        assert_eq!(environment["NIVA_TEST_A"], "first value");
        assert_eq!(environment["NIVA_TEST_B"], "two");

        let arguments = arguments_value(["niva".to_string(), "two words".to_string()]);
        assert_eq!(arguments, json!(["niva", "two words"]));

        let live_args = arguments_value(std::env::args());
        assert!(live_args.as_array().is_some_and(|args| {
            !args.is_empty() && args.iter().all(serde_json::Value::is_string)
        }));
    }

    #[test]
    fn set_current_dir_changes_only_an_isolated_child_process() {
        if std::env::var_os(SET_CURRENT_DIR_CHILD_ENV).is_some() {
            let target = std::env::var_os(SET_CURRENT_DIR_TARGET_ENV).unwrap();
            let target = std::path::PathBuf::from(target).canonicalize().unwrap();
            set_current_directory(&target).unwrap();
            assert_eq!(
                std::env::current_dir().unwrap(),
                target,
                "the child process should adopt the requested directory"
            );
            return;
        }

        let temp = TestDir::new();
        let target = temp.path().join("working-directory");
        std::fs::create_dir(&target).unwrap();
        let target = target.canonicalize().unwrap();
        let status = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "app::api::process::tests::set_current_dir_changes_only_an_isolated_child_process",
                "--nocapture",
            ])
            .env(SET_CURRENT_DIR_CHILD_ENV, "1")
            .env(SET_CURRENT_DIR_TARGET_ENV, &target)
            .status()
            .unwrap();
        assert!(status.success(), "child API check exited with {status}");

        assert!(set_current_directory(&temp.path().join("missing")).is_err());
    }

    #[test]
    fn child_process_entrypoint() {
        if std::env::var_os(CHILD_TEST_ENV).is_some() {
            // The parent test cancels this controlled child well before it
            // exits naturally, so a missing kill would make the assertion fail.
            std::thread::sleep(Duration::from_secs(5));
        }
    }

    #[test]
    fn cancellation_kills_and_reaps_a_controlled_child() {
        let child = std::process::Command::new(std::env::current_exe().unwrap())
            .args(["child_process_entrypoint", "--nocapture"])
            .env(CHILD_TEST_ENV, "1")
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .unwrap();
        let cancel = Arc::new(AtomicBool::new(false));
        let worker_cancel = Arc::clone(&cancel);
        let (wait_tx, wait_rx) = async_channel::bounded(1);
        spawn_child_supervisor(
            child,
            move || worker_cancel.load(Ordering::Acquire),
            wait_tx,
        )
        .unwrap();

        let cancel_signal = Arc::clone(&cancel);
        let signal_thread = std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(100));
            cancel_signal.store(true, Ordering::Release);
        });

        let started = Instant::now();
        let outcome = smol::block_on(wait_rx.recv()).unwrap().unwrap();
        let elapsed = started.elapsed();
        signal_thread.join().unwrap();

        assert!(
            matches!(outcome, ChildWaitOutcome::Cancelled),
            "expected cancellation to kill the child, got {outcome:?}"
        );
        assert!(
            elapsed < Duration::from_secs(3),
            "child took {elapsed:?} to stop"
        );
    }

    #[cfg(unix)]
    #[test]
    fn exec_command_applies_arguments_env_directory_and_preserves_streams_and_status() {
        let temp = TestDir::new();
        let current_dir = temp.path().canonicalize().unwrap();
        let options = ExecOptions {
            env: Some(HashMap::from([(
                "NIVA_EXEC_TEST_VALUE".to_string(),
                "provided".to_string(),
            )])),
            current_dir: Some(current_dir.to_string_lossy().into_owned()),
            detached: Some(false),
        };
        let script =
            "printf '%s|' \"$NIVA_EXEC_TEST_VALUE\"; pwd -P; printf 'diagnostic' >&2; exit 7";
        let (mut command, detached) = build_exec_command(
            "sh".to_string(),
            Some(vec!["-c".to_string(), script.to_string()]),
            Some(options),
        );

        assert!(!detached);
        let output = command.output().unwrap();
        assert_eq!(output.status.code(), Some(7));
        assert_eq!(
            String::from_utf8(output.stdout).unwrap(),
            format!("provided|{}\n", current_dir.display())
        );
        assert_eq!(String::from_utf8(output.stderr).unwrap(), "diagnostic");
    }

    #[test]
    fn exec_command_reads_detached_option_and_defaults_it_to_false() {
        let (command, detached) = build_exec_command("unused".into(), None, None);
        assert!(!detached);
        drop(command);

        let options: ExecOptions = serde_json::from_value(json!({ "detached": true })).unwrap();
        let (command, detached) = build_exec_command("unused".into(), None, Some(options));
        assert!(detached);
        drop(command);
    }
}
#[cfg(all(test, unix))]
mod node_process_tests {
    use super::*;
    use base64::Engine;
    use base64::engine::general_purpose::STANDARD;
    use std::time::{Duration, Instant};
    #[test]
    fn synchronous_process_preserves_binary_and_exit_status() {
        let result = spawn_sync_command(
            "/bin/sh".into(),
            vec!["-c".into(), "cat; printf err >&2; exit 7".into()],
            json!({"input":"AP/+"}),
        )
        .unwrap();
        assert_eq!(result["stdout"], "AP/+");
        assert_eq!(result["stderr"], "ZXJy");
        assert_eq!(result["status"], 7);
    }
    #[test]
    fn synchronous_process_timeout_kills_its_process_group() {
        let start = std::time::Instant::now();
        let result = spawn_sync_command(
            "/bin/sh".into(),
            vec!["-c".into(), "sleep 5".into()],
            json!({"timeout":50}),
        )
        .unwrap();
        assert_eq!(result["error"], "ETIMEDOUT");
        assert!(start.elapsed() < std::time::Duration::from_secs(2));
    }
    #[test]
    fn synchronous_process_bounds_captured_output() {
        let result = spawn_sync_command(
            "/bin/sh".into(),
            vec!["-c".into(), "printf 123456789".into()],
            json!({"maxBuffer":4}),
        )
        .unwrap();
        assert_eq!(result["error"], "ENOBUFS");
        assert_eq!(result["stdout"], "MTIzNA==");
    }

    #[test]
    fn synchronous_process_tolerates_the_child_closing_stdin_early() {
        let input = vec![b'x'; 512 * 1024];
        let input = STANDARD.encode(input);
        let result = spawn_sync_command(
            "/bin/sh".into(),
            vec!["-c".into(), "exit 0".into()],
            json!({"input":input,"timeout":1000}),
        )
        .unwrap();
        assert_eq!(result["status"], 0);
        assert_eq!(result["error"], Value::Null);
    }

    #[test]
    fn synchronous_process_rejects_invalid_input_before_spawning() {
        let error = spawn_sync_command(
            "/definitely/not/an/executable".into(),
            Vec::new(),
            json!({"input":"%%%"}),
        )
        .unwrap_err();
        assert!(error.to_string().to_lowercase().contains("base64"));
    }

    #[test]
    fn abandoned_spawn_sync_child_is_killed_and_reaped() {
        let mut command = std::process::Command::new("/bin/sh");
        command
            .args(["-c", "exec sleep 5"])
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null());
        use std::os::unix::process::CommandExt;
        command.process_group(0);
        let child = command.spawn().unwrap();
        let pid = child.id();
        let start = Instant::now();
        drop(SpawnSyncChild::new(child, pid, true));
        assert!(start.elapsed() < Duration::from_secs(2));
        // SAFETY: signal zero only checks whether the just-reaped pid exists.
        assert_eq!(unsafe { libc::kill(pid as i32, 0) }, -1);
    }
}
