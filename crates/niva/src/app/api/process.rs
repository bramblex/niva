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
/// stream events (one per line), stdin is fed from inbound binary chunks
/// (END closes it), terminal result carries the exit status.
/// No timeout by default: interactive sessions are client-governed
/// (cancel message or window close abandons the child, it is not killed).
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
        // Child keeps running; nobody consumes stdio (inherits closed pipes).
        ctx.respond(Ok(json!(id)));
        return Ok(());
    }

    // Byte pumps (dedicated threads: blocking reads, exact bytes).
    if let Some(stdout) = child.stdout.take() {
        pump_bytes(&ctx, stdout, false)?;
    }
    if let Some(stderr) = child.stderr.take() {
        pump_bytes(&ctx, stderr, true)?;
    }

    // Stdin pump runs on its own thread: it must never precede wait()
    // (no stdin input would stall the child forever).
    if let Some(mut stdin) = child.stdin.take() {
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

    // wait() blocks: elastic pool, never the driver thread.
    let ctx2 = ctx.clone();
    crate::blocking!({
        let mut child = child;
        let status = child.wait()?;
        ctx2.respond(Ok(json!({ "status": status.code() })));
        Ok(())
    })
    .await?;
    Ok(())
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
