use crate::app::NivaApp;
use crate::app::api_manager::{
    ApiCallOwner, ApiManager, ApiRequest, CallContext, CancellationContext,
};
use crate::app::main_exec::run_on_main;
use crate::app::window_manager::window::NivaWindow;
use anyhow::{Ok, Result, anyhow};
use serde::Deserialize;
use serde_json::{Value, json};
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;
use tao::event_loop::ControlFlow;

pub fn register_apis(api_manager: &mut ApiManager) {
    api_manager.register_api("process.pid", pid);
    api_manager.register_cancellable_api("process.spawnSync", spawn_sync);
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
    api_manager.register_cancellable_api("process.execText", exec_text);
    api_manager.register_cancellable_api("process.execFileText", exec_file_text);
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
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        cmd.process_group(0);
    }
    (cmd, detached)
}

const MAX_TEXT_EXEC_OUTPUT_BYTES: usize = 64 * 1024;
const DEFAULT_TEXT_EXEC_TIMEOUT_MS: u64 = 10_000;

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct TextExecOptions {
    cwd: Option<String>,
    env: Option<std::collections::HashMap<String, String>>,
    timeout_ms: Option<u64>,
    max_output_bytes: Option<usize>,
}

fn build_text_exec_command(
    program: String,
    args: Vec<String>,
    options: &TextExecOptions,
) -> std::process::Command {
    let mut command = std::process::Command::new(program);
    command.args(args);
    if let Some(cwd) = &options.cwd {
        command.current_dir(cwd);
    }
    if let Some(env) = &options.env {
        command.env_clear().envs(env);
    }
    command
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        command.process_group(0);
    }
    command
}

async fn exec_text(
    context: CancellationContext,
    _app: Arc<NivaApp>,
    _window: Arc<NivaWindow>,
    request: ApiRequest,
) -> Result<Value> {
    let (command, options): (String, Option<TextExecOptions>) = request.args().optional(2)?;
    anyhow::ensure!(!command.is_empty(), "EINVAL: command must not be empty");
    execute_text_command(context, command, None, options.unwrap_or_default()).await
}

async fn exec_file_text(
    context: CancellationContext,
    _app: Arc<NivaApp>,
    _window: Arc<NivaWindow>,
    request: ApiRequest,
) -> Result<Value> {
    let (file, args, options): (String, Vec<String>, Option<TextExecOptions>) =
        request.args().optional(3)?;
    anyhow::ensure!(
        !file.is_empty(),
        "EINVAL: executable path must not be empty"
    );
    anyhow::ensure!(
        !file.contains('\0') && args.iter().all(|argument| !argument.contains('\0')),
        "EINVAL: executable and arguments cannot contain null bytes"
    );
    execute_text_command(context, file, Some(args), options.unwrap_or_default()).await
}

async fn execute_text_command(
    context: CancellationContext,
    program: String,
    args: Option<Vec<String>>,
    options: TextExecOptions,
) -> Result<Value> {
    let timeout_ms = options.timeout_ms.unwrap_or(DEFAULT_TEXT_EXEC_TIMEOUT_MS);
    anyhow::ensure!(
        (1..=30_000).contains(&timeout_ms),
        "EINVAL: timeoutMs must be between 1 and 30000 ms"
    );
    let max_output_bytes = options
        .max_output_bytes
        .unwrap_or(MAX_TEXT_EXEC_OUTPUT_BYTES);
    anyhow::ensure!(
        (1..=MAX_TEXT_EXEC_OUTPUT_BYTES).contains(&max_output_bytes),
        "EINVAL: maxOutputBytes must be between 1 and 65536"
    );

    let command = match args {
        Some(args) => build_text_exec_command(program, args, &options),
        None => build_text_exec_shell_command(&program, &options),
    };
    let owner = process_owner(&context);
    let worker_context = context.clone();
    let cancel_context = context.clone();
    let work = crate::blocking!({
        run_text_command(
            command,
            owner,
            Duration::from_millis(timeout_ms),
            max_output_bytes,
            move || worker_context.is_cancelled(),
        )
    });
    smol::future::or(work, async move {
        cancel_context.cancelled().await;
        Err(anyhow!(
            "ECANCELED: process call was cancelled with its owner"
        ))
    })
    .await
}

fn build_text_exec_shell_command(
    command: &str,
    options: &TextExecOptions,
) -> std::process::Command {
    #[cfg(windows)]
    {
        build_text_exec_command(
            "cmd.exe".into(),
            vec!["/d".into(), "/s".into(), "/c".into(), command.into()],
            options,
        )
    }
    #[cfg(unix)]
    {
        build_text_exec_command("/bin/sh".into(), vec!["-c".into(), command.into()], options)
    }
    #[cfg(not(any(unix, windows)))]
    {
        build_text_exec_command(command.into(), Vec::new(), options)
    }
}

fn process_owner(context: &CancellationContext) -> ProcessCallOwner {
    match &context.owner {
        ApiCallOwner::WebSocket { connection_id } => ProcessCallOwner::WebSocket {
            window_id: context.window_id,
            connection_id: *connection_id,
            call_id: context.call_id,
        },
        ApiCallOwner::Synchronous { session_id } => ProcessCallOwner::Synchronous {
            window_id: context.window_id,
            session_id: session_id.clone(),
            call_id: context.call_id,
        },
        ApiCallOwner::Ipc {
            source_origin,
            session_id,
            frame_id,
            generation,
        } => ProcessCallOwner::Ipc {
            window_id: context.window_id,
            source_origin: source_origin.clone(),
            session_id: session_id.clone(),
            frame_id: *frame_id,
            generation: *generation,
            call_id: context.call_id,
        },
    }
}

struct CapturedOutput {
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    total: usize,
    limit: usize,
}

fn spawn_output_capture<R: std::io::Read + Send + 'static>(
    mut reader: R,
    stderr: bool,
    captured: Arc<std::sync::Mutex<CapturedOutput>>,
    overflow: Arc<std::sync::atomic::AtomicBool>,
) -> std::io::Result<std::thread::JoinHandle<std::io::Result<()>>> {
    std::thread::Builder::new()
        .name(if stderr {
            "niva-exec-text-stderr".into()
        } else {
            "niva-exec-text-stdout".into()
        })
        .spawn(move || {
            let mut buffer = [0u8; 8192];
            loop {
                let size = reader.read(&mut buffer)?;
                if size == 0 {
                    return std::result::Result::Ok(());
                }
                let complete = {
                    let mut output = captured
                        .lock()
                        .map_err(|_| std::io::Error::other("process output capture poisoned"))?;
                    let keep = size.min(output.limit.saturating_sub(output.total));
                    if stderr {
                        output.stderr.extend_from_slice(&buffer[..keep]);
                    } else {
                        output.stdout.extend_from_slice(&buffer[..keep]);
                    }
                    output.total += keep;
                    keep == size
                };
                if !complete {
                    overflow.store(true, std::sync::atomic::Ordering::Release);
                    return std::result::Result::Ok(());
                }
            }
        })
}

fn run_text_command(
    mut command: std::process::Command,
    owner: ProcessCallOwner,
    timeout: std::time::Duration,
    max_output_bytes: usize,
    is_cancelled: impl Fn() -> bool,
) -> Result<Value> {
    use std::sync::atomic::{AtomicBool, Ordering};
    anyhow::ensure!(!is_cancelled(), "ECANCELED: process call was cancelled");
    let raw_child = command.spawn()?;
    let pid = raw_child.id();
    let mut child = SpawnSyncChild::new(raw_child, pid, cfg!(unix))?;
    let _registration = register_child(owner, child.control.clone())?;
    if is_cancelled() {
        child.terminate_and_wait()?;
        anyhow::bail!("ECANCELED: process call was cancelled");
    }
    let stdout = child
        .child_mut()
        .stdout
        .take()
        .ok_or_else(|| anyhow!("EIO: child stdout pipe unavailable"))?;
    let stderr = child
        .child_mut()
        .stderr
        .take()
        .ok_or_else(|| anyhow!("EIO: child stderr pipe unavailable"))?;
    let captured = Arc::new(std::sync::Mutex::new(CapturedOutput {
        stdout: Vec::new(),
        stderr: Vec::new(),
        total: 0,
        limit: max_output_bytes,
    }));
    let overflow = Arc::new(AtomicBool::new(false));
    let stdout_reader = spawn_output_capture(stdout, false, captured.clone(), overflow.clone())?;
    let stderr_reader = spawn_output_capture(stderr, true, captured.clone(), overflow.clone())?;
    let started = std::time::Instant::now();
    let exit = loop {
        if is_cancelled() {
            child.terminate_and_wait()?;
            break Err(anyhow!("ECANCELED: process call was cancelled"));
        }
        if overflow.load(Ordering::Acquire) {
            child.terminate_and_wait()?;
            break Err(anyhow!("ENOBUFS: child output exceeded maxOutputBytes"));
        }
        if started.elapsed() >= timeout {
            child.terminate_and_wait()?;
            break Err(anyhow!("ETIMEDOUT: child process exceeded timeoutMs"));
        }
        if let Some(status) = child.try_wait()? {
            // A descendant may inherit a pipe after the direct process exits.
            // Stop the owned tree before joining readers, so it cannot stall
            // this request indefinitely.
            child.terminate_descendants();
            break Ok(status);
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    };
    let stdout_result = stdout_reader
        .join()
        .map_err(|_| anyhow!("EIO: stdout capture thread panicked"))?;
    let stderr_result = stderr_reader
        .join()
        .map_err(|_| anyhow!("EIO: stderr capture thread panicked"))?;
    stdout_result?;
    stderr_result?;
    let exit = exit?;
    anyhow::ensure!(
        !is_cancelled(),
        "ECANCELED: process call was cancelled before the result was delivered"
    );
    let output = captured
        .lock()
        .map_err(|_| anyhow!("process output capture poisoned"))?;
    let stdout = String::from_utf8(output.stdout.clone())
        .map_err(|_| anyhow!("ERR_INVALID_ENCODING: child stdout is not valid UTF-8"))?;
    let stderr = String::from_utf8(output.stderr.clone())
        .map_err(|_| anyhow!("ERR_INVALID_ENCODING: child stderr is not valid UTF-8"))?;
    #[cfg(unix)]
    let signal = {
        use std::os::unix::process::ExitStatusExt;
        exit.signal()
    };
    #[cfg(not(unix))]
    let signal: Option<i32> = None;
    let mut result = json!({
        "stdout": stdout,
        "stderr": stderr,
        "status": exit.code(),
    });
    if let Some(signal) = signal {
        result["signal"] = json!(signal);
    }
    Ok(result)
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
/// or loss of the owning connection kills and reaps the owned child tree.
async fn exec_stream(ctx: CallContext, request: ApiRequest) -> Result<()> {
    use std::io::Write;

    let (program, args, options): (String, Option<Vec<String>>, Option<ExecOptions>) =
        request.args().optional(3)?;
    let (mut cmd, detached) = build_exec_command(program, args, options);
    anyhow::ensure!(
        !detached,
        "ENOTSUP: detached child processes are disabled; use an owned process stream"
    );
    anyhow::ensure!(!ctx.is_cancelled(), "ECANCELED: process call was cancelled");
    let raw_child = cmd.spawn()?;
    let pid = raw_child.id();
    let mut child = SpawnSyncChild::new(raw_child, pid, cfg!(unix))?;
    let owner = ProcessCallOwner::WebSocket {
        window_id: ctx.window.id,
        connection_id: ctx.connection_id,
        call_id: ctx.id,
    };
    let _registration = register_child(owner, child.control.clone())?;
    if ctx.is_cancelled() {
        child.terminate_and_wait()?;
        anyhow::bail!("ECANCELED: process call was cancelled");
    }
    ctx.push("spawn", json!({"pid":child.id()}));

    let stdout = child.child_mut().stdout.take();
    let stderr = child.child_mut().stderr.take();
    let stdin = child.child_mut().stdin.take();

    // Keep Child ownership on a dedicated thread. The async handler only
    // awaits its result, so timeout/cancellation never blocks the API driver.
    let ctx2 = ctx.clone();
    let (wait_tx, wait_rx) = async_channel::bounded(1);
    if let Err((err, mut child)) =
        spawn_child_supervisor(child, move || ctx2.is_cancelled(), wait_tx)
    {
        return crate::blocking!({
            let _ = child.terminate_and_wait();
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
    mut child: SpawnSyncChild,
    is_cancelled: impl Fn() -> bool,
) -> std::io::Result<ChildWaitOutcome> {
    loop {
        if is_cancelled() {
            child.terminate_and_wait()?;
            return std::result::Result::Ok(ChildWaitOutcome::Cancelled);
        }
        if let Some(status) = child.try_wait()? {
            child.terminate_descendants();
            return std::result::Result::Ok(ChildWaitOutcome::Exited(status));
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
}

/// Start the sole owner of a child process and report when it has been reaped.
fn spawn_child_supervisor(
    child: SpawnSyncChild,
    is_cancelled: impl Fn() -> bool + Send + 'static,
    wait_tx: async_channel::Sender<std::io::Result<ChildWaitOutcome>>,
) -> std::result::Result<(), (std::io::Error, SpawnSyncChild)> {
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

fn write_stdio(_app: Arc<NivaApp>, window: Arc<NivaWindow>, request: ApiRequest) -> Result<()> {
    use base64::Engine;
    use std::io::Write;
    anyhow::ensure!(window.id == 0, "process stdio is main-window-only");
    let (channel, data): (String, String) = request.args().get()?;
    let data = base64::engine::general_purpose::STANDARD.decode(data)?;
    match channel.as_str() {
        "stdout" => std::io::stdout().lock().write_all(&data)?,
        "stderr" => std::io::stderr().lock().write_all(&data)?,
        _ => anyhow::bail!("invalid stdio channel"),
    }
    Ok(())
}

async fn read_stdin(ctx: CallContext, _request: ApiRequest) -> Result<()> {
    anyhow::ensure!(ctx.window.id == 0, "process stdio is main-window-only");
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

async fn spawn_sync(
    context: CancellationContext,
    _app: Arc<NivaApp>,
    _window: Arc<NivaWindow>,
    request: ApiRequest,
) -> Result<Value> {
    let (program, args, options): (String, Option<Vec<String>>, Option<Value>) =
        request.args().optional(3)?;
    let owner = process_owner(&context);
    let worker_context = context.clone();
    crate::blocking!({
        spawn_sync_command_owned(
            program,
            args.unwrap_or_default(),
            options.unwrap_or_else(|| json!({})),
            owner,
            move || worker_context.is_cancelled(),
        )
    })
    .await
}

fn spawn_sync_command(program: String, args: Vec<String>, options: Value) -> Result<Value> {
    static NEXT_OWNER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let id = NEXT_OWNER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    spawn_sync_command_owned(
        program,
        args,
        options,
        ProcessCallOwner::Synchronous {
            window_id: u8::MAX,
            session_id: format!("spawn-sync-test-{id}"),
            call_id: id,
        },
        || false,
    )
}

fn spawn_sync_command_owned(
    program: String,
    args: Vec<String>,
    options: Value,
    owner: ProcessCallOwner,
    is_cancelled: impl Fn() -> bool,
) -> Result<Value> {
    use base64::{Engine, engine::general_purpose::STANDARD};
    use std::{
        io::{Read, Write},
        sync::atomic::{AtomicBool, Ordering},
        time::{Duration, Instant},
    };
    anyhow::ensure!(
        !options["detached"].as_bool().unwrap_or(false),
        "ENOTSUP: detached child processes are disabled"
    );
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
    let mut child = SpawnSyncChild::new(child, pid, cfg!(unix))?;
    let _registration = register_child(owner, child.control.clone())?;
    if is_cancelled() {
        child.terminate_and_wait()?;
        anyhow::bail!("ECANCELED: synchronous process call was cancelled");
    }
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
        if is_cancelled() {
            failure = Some("ECANCELED");
            break child.terminate_and_wait()?;
        }
        if let Some(status) = child.try_wait()? {
            child.terminate_descendants();
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
    control: Arc<ChildControl>,
    reaped: bool,
}

impl SpawnSyncChild {
    fn new(child: std::process::Child, pid: u32, process_group: bool) -> std::io::Result<Self> {
        #[cfg(windows)]
        let mut child = child;
        #[cfg(not(windows))]
        let child = child;
        #[cfg(windows)]
        let control = match ChildJob::assign(&child) {
            std::result::Result::Ok(job) => Arc::new(ChildControl { pid, job }),
            Err(error) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(error);
            }
        };
        #[cfg(unix)]
        let control = Arc::new(ChildControl { pid, process_group });
        #[cfg(not(unix))]
        let _ = process_group;
        std::result::Result::Ok(Self {
            child,
            control,
            reaped: false,
        })
    }

    fn child_mut(&mut self) -> &mut std::process::Child {
        &mut self.child
    }

    fn id(&self) -> u32 {
        self.control.pid
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
        self.control.kill_tree();
        let _ = self.child.kill();
    }

    fn terminate_descendants(&mut self) {
        // The direct child may already have exited while background
        // descendants still hold inherited stdout/stderr pipes.
        self.control.kill_tree();
    }
}

impl Drop for SpawnSyncChild {
    fn drop(&mut self) {
        if self.reaped {
            self.terminate_descendants();
            return;
        }
        if matches!(self.child.try_wait(), std::result::Result::Ok(Some(_))) {
            self.reaped = true;
            self.terminate_descendants();
            return;
        }
        self.terminate();
        let _ = self.child.wait();
        self.reaped = true;
    }
}

#[cfg(windows)]
struct ChildJob(windows::Win32::Foundation::HANDLE);

#[cfg(windows)]
impl ChildJob {
    fn assign(child: &std::process::Child) -> std::io::Result<Self> {
        use std::{mem::size_of, os::windows::io::AsRawHandle};
        use windows::{
            Win32::{
                Foundation::{CloseHandle, HANDLE},
                System::JobObjects::{
                    AssignProcessToJobObject, CreateJobObjectW, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
                    JOBOBJECT_EXTENDED_LIMIT_INFORMATION, JobObjectExtendedLimitInformation,
                    SetInformationJobObject,
                },
            },
            core::PCWSTR,
        };

        let job = unsafe { CreateJobObjectW(None, PCWSTR::null()) }
            .map_err(|error| std::io::Error::other(error.to_string()))?;
        let mut limits = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
        limits.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
        let configured = unsafe {
            SetInformationJobObject(
                job,
                JobObjectExtendedLimitInformation,
                (&limits as *const JOBOBJECT_EXTENDED_LIMIT_INFORMATION).cast(),
                size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
            )
        };
        if let Err(error) = configured {
            let _ = unsafe { CloseHandle(job) };
            return Err(std::io::Error::other(error.to_string()));
        }
        let process = HANDLE(child.as_raw_handle() as _);
        if let Err(error) = unsafe { AssignProcessToJobObject(job, process) } {
            let _ = unsafe { CloseHandle(job) };
            return Err(std::io::Error::other(format!(
                "assign child process to kill-on-close job: {error}"
            )));
        }
        std::result::Result::Ok(Self(job))
    }
}

#[cfg(windows)]
impl Drop for ChildJob {
    fn drop(&mut self) {
        use windows::Win32::{Foundation::CloseHandle, System::JobObjects::TerminateJobObject};
        // Closing a kill-on-close job is the normal cancellation path. Try
        // termination first so descendants are also stopped if the handle's
        // close policy is changed unexpectedly.
        unsafe {
            let _ = TerminateJobObject(self.0, 1);
            let _ = CloseHandle(self.0);
        }
    }
}

#[derive(Clone, Hash, PartialEq, Eq)]
enum ProcessCallOwner {
    WebSocket {
        window_id: u8,
        connection_id: u64,
        call_id: u64,
    },
    Synchronous {
        window_id: u8,
        session_id: String,
        call_id: u64,
    },
    Ipc {
        window_id: u8,
        source_origin: String,
        session_id: String,
        frame_id: u64,
        generation: u64,
        call_id: u64,
    },
}

struct ChildControl {
    pid: u32,
    #[cfg(unix)]
    process_group: bool,
    #[cfg(windows)]
    job: ChildJob,
}

impl ChildControl {
    fn kill_tree(&self) {
        #[cfg(unix)]
        if self.process_group {
            // SAFETY: process_group(0) assigned this unique child-owned PGID.
            // Unix children can deliberately escape with setsid(); that is
            // outside what a process group can contain.
            let _ = unsafe { libc::kill(-(self.pid as i32), libc::SIGKILL) };
        }
        #[cfg(windows)]
        unsafe {
            use windows::Win32::System::JobObjects::TerminateJobObject;
            let _ = TerminateJobObject(self.job.0, 1);
        }
    }
}

#[cfg(windows)]
unsafe impl Send for ChildJob {}
#[cfg(windows)]
unsafe impl Sync for ChildJob {}

type ProcessCallControls = std::collections::HashMap<ProcessCallOwner, Arc<ChildControl>>;

fn process_controls() -> &'static std::sync::Mutex<ProcessCallControls> {
    static CONTROLS: std::sync::OnceLock<std::sync::Mutex<ProcessCallControls>> =
        std::sync::OnceLock::new();
    CONTROLS.get_or_init(|| std::sync::Mutex::new(std::collections::HashMap::new()))
}

fn register_child(
    owner: ProcessCallOwner,
    control: Arc<ChildControl>,
) -> Result<ChildRegistration> {
    let mut controls = process_controls()
        .lock()
        .map_err(|_| anyhow::anyhow!("child registry poisoned"))?;
    anyhow::ensure!(
        !controls.contains_key(&owner),
        "EBUSY: duplicate child owner"
    );
    controls.insert(owner.clone(), control.clone());
    Ok(ChildRegistration { owner, control })
}

struct ChildRegistration {
    owner: ProcessCallOwner,
    control: Arc<ChildControl>,
}

impl Drop for ChildRegistration {
    fn drop(&mut self) {
        // A normally completed direct child may have left descendants holding
        // pipes open. Also make future cancellation fail closed if the async
        // handler is dropped outside the usual dispatcher path.
        self.control.kill_tree();
        if let std::result::Result::Ok(mut controls) = process_controls().lock()
            && controls
                .get(&self.owner)
                .is_some_and(|current| Arc::ptr_eq(current, &self.control))
        {
            controls.remove(&self.owner);
        }
    }
}

fn cancel_processes_matching(mut matches: impl FnMut(&ProcessCallOwner) -> bool) {
    let controls = process_controls()
        .lock()
        .map(|controls| {
            controls
                .iter()
                .filter(|(owner, _)| matches(owner))
                .map(|(_, control)| control.clone())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    for control in controls {
        control.kill_tree();
    }
}

pub(crate) fn cancel_ws_child(window_id: u8, connection_id: u64, call_id: u64) {
    cancel_processes_matching(|owner| {
        matches!(owner, ProcessCallOwner::WebSocket { window_id: wid, connection_id: cid, call_id: id }
            if *wid == window_id && *cid == connection_id && *id == call_id)
    });
}

pub(crate) fn cancel_ws_connection(window_id: u8, connection_id: u64) {
    cancel_processes_matching(|owner| {
        matches!(owner, ProcessCallOwner::WebSocket { window_id: wid, connection_id: cid, .. }
            if *wid == window_id && *cid == connection_id)
    });
}

pub(crate) fn cancel_window_children(window_id: u8) {
    cancel_processes_matching(|owner| match owner {
        ProcessCallOwner::WebSocket { window_id: wid, .. }
        | ProcessCallOwner::Synchronous { window_id: wid, .. }
        | ProcessCallOwner::Ipc { window_id: wid, .. } => *wid == window_id,
    });
}

pub(crate) fn cancel_sync_child(window_id: u8, session_id: &str, call_id: u64) {
    cancel_processes_matching(|owner| {
        matches!(owner, ProcessCallOwner::Synchronous {
            window_id: wid,
            session_id: session,
            call_id: id,
        } if *wid == window_id && session == session_id && *id == call_id)
    });
}

pub(crate) fn cancel_sync_session(window_id: u8, session_id: &str) {
    cancel_processes_matching(|owner| {
        matches!(owner, ProcessCallOwner::Synchronous {
            window_id: wid,
            session_id: session,
            ..
        } if *wid == window_id && session == session_id)
    });
}

pub(crate) fn cancel_ipc_child(
    window_id: u8,
    source_origin: &str,
    session_id: &str,
    frame_id: u64,
    generation: u64,
    call_id: u64,
) {
    cancel_processes_matching(|owner| {
        matches!(owner, ProcessCallOwner::Ipc {
            window_id: wid,
            source_origin: origin,
            session_id: session,
            frame_id: fid,
            generation: generation_id,
            call_id: id,
        } if *wid == window_id && origin == source_origin && session == session_id
            && *fid == frame_id && *generation_id == generation && *id == call_id)
    });
}

pub(crate) fn cancel_ipc_session(
    window_id: u8,
    source_origin: &str,
    session_id: &str,
    frame_id: u64,
    generation: u64,
) {
    cancel_processes_matching(|owner| {
        matches!(owner, ProcessCallOwner::Ipc {
            window_id: wid,
            source_origin: origin,
            session_id: session,
            frame_id: fid,
            generation: generation_id,
            ..
        } if *wid == window_id && origin == source_origin && session == session_id
            && *fid == frame_id && *generation_id == generation)
    });
}

pub(crate) fn cancel_ipc_frame(window_id: u8, frame_id: u64, generation: u64) {
    cancel_processes_matching(|owner| {
        matches!(owner, ProcessCallOwner::Ipc { window_id: wid, frame_id: fid, generation: generation_id, .. }
            if *wid == window_id && *fid == frame_id && *generation_id == generation)
    });
}

pub(crate) fn cancel_ipc_window(window_id: u8) {
    cancel_processes_matching(
        |owner| matches!(owner, ProcessCallOwner::Ipc { window_id: wid, .. } if *wid == window_id),
    );
}
async fn signal_child(ctx: CallContext, request: ApiRequest) -> Result<()> {
    let (call_id, signal): (u64, Option<String>) = request.args().optional(2)?;
    let signal = signal.as_deref().unwrap_or("SIGTERM");
    let process = process_controls()
        .lock()
        .map_err(|_| anyhow::anyhow!("child registry poisoned"))?
        .get(&ProcessCallOwner::WebSocket {
            window_id: ctx.window.id,
            connection_id: ctx.connection_id,
            call_id,
        })
        .cloned();
    let Some(process) = process else {
        ctx.respond(Ok(false));
        return Ok(());
    };
    let pid = process.pid;
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
        let mut command = std::process::Command::new(std::env::current_exe().unwrap());
        command
            .args(["child_process_entrypoint", "--nocapture"])
            .env(CHILD_TEST_ENV, "1")
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null());
        #[cfg(unix)]
        {
            use std::os::unix::process::CommandExt;
            command.process_group(0);
        }
        let child = command.spawn().unwrap();
        let pid = child.id();
        let child = SpawnSyncChild::new(child, pid, cfg!(unix)).unwrap();
        let cancel = Arc::new(AtomicBool::new(false));
        let worker_cancel = Arc::clone(&cancel);
        let (wait_tx, wait_rx) = async_channel::bounded(1);
        let supervisor = spawn_child_supervisor(
            child,
            move || worker_cancel.load(Ordering::Acquire),
            wait_tx,
        );
        if let Err((error, mut child)) = supervisor {
            let _ = child.terminate_and_wait();
            panic!("spawn child supervisor failed: {error}");
        }

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
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{Duration, Instant};

    static NEXT_TEST_OWNER: AtomicU64 = AtomicU64::new(0);

    fn test_owner() -> ProcessCallOwner {
        ProcessCallOwner::WebSocket {
            window_id: 253,
            connection_id: NEXT_TEST_OWNER.fetch_add(1, Ordering::Relaxed),
            call_id: NEXT_TEST_OWNER.fetch_add(1, Ordering::Relaxed),
        }
    }

    fn shell_command(script: &str) -> std::process::Command {
        build_text_exec_command(
            "/bin/sh".into(),
            vec!["-c".into(), script.into()],
            &TextExecOptions::default(),
        )
    }

    #[test]
    fn exec_text_uses_the_platform_shell_for_command_strings() {
        let result = run_text_command(
            build_text_exec_shell_command(
                "printf 'niva-ipc'; printf 'diagnostic' >&2; exit 7",
                &TextExecOptions::default(),
            ),
            test_owner(),
            Duration::from_secs(2),
            MAX_TEXT_EXEC_OUTPUT_BYTES,
            || false,
        )
        .unwrap();
        assert_eq!(result["stdout"], "niva-ipc");
        assert_eq!(result["stderr"], "diagnostic");
        assert_eq!(result["status"], 7);

        let result = run_text_command(
            build_text_exec_command(
                "/usr/bin/printf".into(),
                vec!["execFile".into()],
                &TextExecOptions::default(),
            ),
            test_owner(),
            Duration::from_secs(2),
            MAX_TEXT_EXEC_OUTPUT_BYTES,
            || false,
        )
        .unwrap();
        assert_eq!(result["stdout"], "execFile");
    }

    #[test]
    fn text_process_captures_strict_utf8_and_nonzero_status() {
        let result = run_text_command(
            shell_command("printf '中文'; printf 'diagnostic' >&2; exit 7"),
            test_owner(),
            Duration::from_secs(2),
            MAX_TEXT_EXEC_OUTPUT_BYTES,
            || false,
        )
        .unwrap();
        assert_eq!(result["stdout"], "中文");
        assert_eq!(result["stderr"], "diagnostic");
        assert_eq!(result["status"], 7);
        assert!(result.get("signal").is_none());
    }

    #[test]
    fn text_process_timeout_kills_and_reaps_the_owned_group() {
        let started = Instant::now();
        let error = run_text_command(
            shell_command("sleep 5"),
            test_owner(),
            Duration::from_millis(75),
            MAX_TEXT_EXEC_OUTPUT_BYTES,
            || false,
        )
        .unwrap_err();
        assert!(error.to_string().contains("ETIMEDOUT"));
        assert!(started.elapsed() < Duration::from_secs(2));
    }

    #[test]
    fn text_process_bounds_combined_stdout_and_stderr() {
        let error = run_text_command(
            shell_command("printf 123456; printf 789 >&2"),
            test_owner(),
            Duration::from_secs(2),
            8,
            || false,
        )
        .unwrap_err();
        assert!(error.to_string().contains("ENOBUFS"));
    }

    #[test]
    fn text_process_rejects_invalid_utf8() {
        let error = run_text_command(
            shell_command("printf '\\377'"),
            test_owner(),
            Duration::from_secs(2),
            8,
            || false,
        )
        .unwrap_err();
        assert!(error.to_string().contains("ERR_INVALID_ENCODING"));
    }

    #[test]
    fn synchronous_owner_cancellation_kills_the_registered_child_tree() {
        let mut command = std::process::Command::new("/bin/sh");
        command
            .args(["-c", "exec sleep 5"])
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null());
        use std::os::unix::process::CommandExt;
        command.process_group(0);
        let raw_child = command.spawn().unwrap();
        let pid = raw_child.id();
        let mut child = SpawnSyncChild::new(raw_child, pid, true).unwrap();
        let owner = ProcessCallOwner::Synchronous {
            window_id: 254,
            session_id: "test-sync-owner".into(),
            call_id: 99,
        };
        let _registration = register_child(owner, child.control.clone()).unwrap();
        let started = Instant::now();
        cancel_sync_child(254, "test-sync-owner", 99);
        let status = loop {
            if let Some(status) = child.try_wait().unwrap() {
                break status;
            }
            assert!(started.elapsed() < Duration::from_secs(2));
            std::thread::sleep(Duration::from_millis(10));
        };
        assert!(!status.success());
    }
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
    fn synchronous_process_rejects_detached_children() {
        let error = spawn_sync_command(
            "/definitely/not/an/executable".into(),
            Vec::new(),
            json!({"detached":true}),
        )
        .unwrap_err();
        assert!(error.to_string().contains("ENOTSUP"));
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
