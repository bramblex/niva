use crate::app::NivaApp;
use crate::app::api_manager::{ApiManager, ApiRequest, CallContext};
use crate::app::main_exec::run_on_main;
use crate::app::window_manager::window::NivaWindow;
use anyhow::{Ok, Result};
use serde::Deserialize;
use serde_json::{Value, json};
use std::sync::Arc;
use tao::event_loop::ControlFlow;

pub fn register_apis(api_manager: &mut ApiManager) {
    api_manager.register_api("process.pid", pid);
    api_manager.register_api("process.currentDir", current_dir);
    api_manager.register_api("process.currentExe", current_exe);
    api_manager.register_api("process.env", env);
    api_manager.register_api("process.args", args);
    api_manager.register_api("process.setCurrentDir", set_current_dir);
    api_manager.register_api("process.exit", exit);
    api_manager.register_api("process.version", version);
    api_manager.register_blocking_api("process.open", open);
    api_manager.register_stream_api("process.execStream", exec_stream);
}

async fn pid(_app: Arc<NivaApp>, _window: Arc<NivaWindow>, _request: ApiRequest) -> Result<u32> {
    Ok(std::process::id())
}

async fn current_dir(
    _app: Arc<NivaApp>,
    _window: Arc<NivaWindow>,
    _request: ApiRequest,
) -> Result<Value> {
    Ok(json!(std::env::current_dir()?))
}

async fn current_exe(
    _app: Arc<NivaApp>,
    _window: Arc<NivaWindow>,
    _request: ApiRequest,
) -> Result<Value> {
    Ok(json!(std::env::current_exe()?))
}

async fn env(_app: Arc<NivaApp>, _window: Arc<NivaWindow>, _request: ApiRequest) -> Result<Value> {
    let env = std::env::vars().collect::<std::collections::HashMap<String, String>>();
    Ok(json!(env))
}

async fn args(_app: Arc<NivaApp>, _window: Arc<NivaWindow>, _request: ApiRequest) -> Result<Value> {
    let args = std::env::args().collect::<Vec<String>>();
    Ok(json!(args))
}

async fn set_current_dir(
    _app: Arc<NivaApp>,
    _window: Arc<NivaWindow>,
    request: ApiRequest,
) -> Result<()> {
    let (path,) = request.args().get::<(String,)>()?;
    std::env::set_current_dir(path)?;
    Ok(())
}

async fn exit(app: Arc<NivaApp>, _window: Arc<NivaWindow>, _request: ApiRequest) -> Result<()> {
    run_on_main(&app, move |_target, control_flow| {
        *control_flow = ControlFlow::Exit;
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
    Ok(env!("CARGO_PKG_VERSION").to_string())
}

/// Full-duplex streaming exec: stdout/stderr arrive as `stdout`/`stderr`
/// stream events, stdin is fed from inbound binary chunks (END closes it),
/// and the terminal result carries the exit status. Cancellation, timeout,
/// or loss of the owning connection kills and reaps the child. `detached`
/// explicitly opts out: the child outlives the stream call.
async fn exec_stream(ctx: CallContext, request: ApiRequest) -> Result<()> {
    use std::io::Write;

    let (cmd, args, options): (String, Option<Vec<String>>, Option<ExecOptions>) =
        request.args().optional(3)?;
    let mut cmd = std::process::Command::new(cmd);
    if let Some(args) = args {
        cmd.args(args);
    }
    let mut detached = false;
    if let Some(options) = options {
        if let Some(current_dir) = options.current_dir {
            cmd.current_dir(current_dir);
        }
        if let Some(env) = options.env {
            cmd.envs(env);
        }
        detached = options.detached.unwrap_or(false);
    }

    let mut child = cmd
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()?;

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
    if let Some(stdout) = stdout {
        pump_bytes(&ctx, stdout, false)?;
    }
    if let Some(stderr) = stderr {
        pump_bytes(&ctx, stderr, true)?;
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
        ctx.respond(Ok(json!({ "status": status.code() })));
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
) -> Result<()> {
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
        .map_err(|err| anyhow::anyhow!("spawn pump failed: {err}"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::time::{Duration, Instant};

    const CHILD_TEST_ENV: &str = "NIVA_PROCESS_SUPERVISOR_CHILD";

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
}
