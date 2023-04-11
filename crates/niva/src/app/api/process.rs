include!(concat!(env!("OUT_DIR"), "/version.rs"));

use crate::app::api_manager::ApiManager;
use anyhow::{Ok, Result};
use niva_macros::{niva_api, niva_event_api};
use serde::Deserialize;
use serde_json::{json, Value};
use tao::event_loop::ControlFlow;

pub fn register_apis(api_manager: &mut ApiManager) {
    api_manager.register_api("process.pid", pid);
    api_manager.register_api("process.currentDir", current_dir);
    api_manager.register_api("process.currentExe", current_exe);
    api_manager.register_api("process.env", env);
    api_manager.register_api("process.args", args);
    api_manager.register_api("process.setCurrentDir", set_current_dir);
    api_manager.register_event_api("process.exit", exit);
    api_manager.register_api("process.version", version);
    api_manager.register_async_api("process.exec", exec);
    api_manager.register_async_api("process.open", open);
}

#[niva_api]
fn pid() -> Result<u32> {
    Ok(std::process::id())
}

#[niva_api]
fn current_dir() -> Result<Value> {
    Ok(json!(std::env::current_dir()?))
}

#[niva_api]
fn current_exe() -> Result<Value> {
    Ok(json!(std::env::current_exe()?))
}

#[niva_api]
fn env() -> Result<Value> {
    let env = std::env::vars().collect::<std::collections::HashMap<String, String>>();
    Ok(json!(env))
}

#[niva_api]
fn args() -> Result<Value> {
    let args = std::env::args().collect::<Vec<String>>();
    Ok(json!(args))
}

#[niva_api]
fn set_current_dir(path: String) -> Result<()> {
    std::env::set_current_dir(path)?;
    Ok(())
}

#[niva_event_api]
fn exit() -> Result<()> {
    *control_flow = ControlFlow::Exit;
    Ok(())
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ExecOptions {
    pub env: Option<std::collections::HashMap<String, String>>,
    pub current_dir: Option<String>,
    pub detached: Option<bool>,
}

#[niva_api]
fn exec(cmd: String, args: Option<Vec<String>>, options: Option<ExecOptions>) -> Result<Value> {
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

    if detached {
        let child = cmd.spawn()?;
        return Ok(json!(child.id()));
    }

    let output = cmd.output()?;

    Ok(json!({
            "status": output.status.code(),
            "stdout": String::from_utf8(output.stdout)?,
            "stderr": String::from_utf8(output.stderr)?,
    }))
}

#[niva_api]
fn open(uri: String) -> Result<()> {
    opener::open(uri)?;
    Ok(())
}

#[niva_api]
fn version() -> Result<String> {
    if let Some(version) = GIT_BUILD_VERSION {
        Ok(version.to_string())
    } else {
        Ok("unknown".to_string())
    }
}
