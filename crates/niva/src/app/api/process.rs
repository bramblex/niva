include!(concat!(env!("OUT_DIR"), "/version.rs"));

use crate::app::{
    api_manager::{ApiManager, ApiRequest},
    window_manager::window::NivaWindow,
    NivaApp, NivaWindowTarget,
};
use anyhow::{Ok, Result};
use serde::Deserialize;
use serde_json::{json, Value};
use std::sync::Arc;
use tao::event_loop::ControlFlow;

pub fn register_apis(api_manager: &mut ApiManager) {
    api_manager.register_api("process.pid", pid);
    api_manager.register_api("process.currentDir", current_dir);
    api_manager.register_api("process.currentExe", current_exe);
    api_manager.register_api("process.env", env);
    api_manager.register_api("process.args", args);
    api_manager.register_api("process.setCurrentDir", set_current_dir);
    api_manager.register_event_api("process.exit", exit);
    api_manager.register_async_api("process.exec", exec);
    api_manager.register_async_api("process.open", open);
    api_manager.register_api("process.version", version);
}

fn pid(_: Arc<NivaApp>, _: Arc<NivaWindow>, _: ApiRequest) -> Result<u32> {
    Ok(std::process::id())
}

fn current_dir(_: Arc<NivaApp>, _: Arc<NivaWindow>, _: ApiRequest) -> Result<Value> {
    Ok(json!(std::env::current_dir()?))
}

fn current_exe(_: Arc<NivaApp>, _: Arc<NivaWindow>, _: ApiRequest) -> Result<Value> {
    Ok(json!(std::env::current_exe()?))
}

fn env(_: Arc<NivaApp>, _: Arc<NivaWindow>, _: ApiRequest) -> Result<Value> {
    let env = std::env::vars().collect::<std::collections::HashMap<String, String>>();
    Ok(json!(env))
}

fn args(_: Arc<NivaApp>, _: Arc<NivaWindow>, _: ApiRequest) -> Result<Value> {
    let args = std::env::args().collect::<Vec<String>>();
    Ok(json!(args))
}

fn set_current_dir(_: Arc<NivaApp>, _: Arc<NivaWindow>, request: ApiRequest) -> Result<()> {
    let path = request.args().single::<String>()?;
    std::env::set_current_dir(path)?;
    Ok(())
}

fn exit(
    _: Arc<NivaApp>,
    _: Arc<NivaWindow>,
    _: ApiRequest,
    _: &NivaWindowTarget,
    control_flow: &mut ControlFlow,
) -> Result<()> {
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

fn exec(_: Arc<NivaApp>, _: Arc<NivaWindow>, request: ApiRequest) -> Result<Value> {
    let (cmd, args, options) = request
        .args()
        .optional::<(String, Option<Vec<String>>, Option<ExecOptions>)>(3)?;

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
            "stdout": String::from_utf8(output.stdout).unwrap(),
            "stderr": String::from_utf8(output.stderr).unwrap(),
    }))
}

fn open(_: Arc<NivaApp>, _: Arc<NivaWindow>, request: ApiRequest) -> Result<()> {
    let uri = request.args().single::<String>()?;
    opener::open(uri)?;
    Ok(())
}

fn version(_: Arc<NivaApp>, _: Arc<NivaWindow>, _: ApiRequest) -> Result<String> {
    if let Some(version) = GIT_BUILD_VERSION {
        Ok(version.to_string())
    } else {
        Ok("unknown".to_string())
    }
}
