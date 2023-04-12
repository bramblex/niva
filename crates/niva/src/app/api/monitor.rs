use anyhow::Result;
use niva_macros::niva_api;
use serde_json::{json, Value};

use tao::monitor::MonitorHandle;

use crate::{app::api_manager::ApiManager, logical};

pub fn register_api_instances(api_manager: &mut ApiManager) {
    api_manager.register_api("monitor.list", list);
    api_manager.register_api("monitor.current", current);
    api_manager.register_api("monitor.primary", primary);
    api_manager.register_api("monitor.fromPoint", from_point);
}

fn monitor_to_value(monitor: MonitorHandle) -> Value {
    json!({
        "name": monitor.name(),
        "size": logical!(monitor, size),
        "position": logical!(monitor, position),
        "physicalSize": monitor.size(),
        "physicalPosition": monitor.position(),
        "scaleFactor": monitor.scale_factor(),
    })
}

#[niva_api]
fn list() -> Result<Vec<Value>> {
    Ok(window.available_monitors().map(monitor_to_value).collect())
}

#[niva_api]
fn current() -> Result<Value> {
    match window.current_monitor() {
        Some(monitor) => Ok(monitor_to_value(monitor)),
        None => Ok(json!(null)),
    }
}

#[niva_api]
fn primary() -> Result<Value> {
    match window.primary_monitor() {
        Some(monitor) => Ok(monitor_to_value(monitor)),
        None => Ok(json!(null)),
    }
}

#[niva_api]
fn from_point(x: f64, y: f64) -> Result<Value> {
    match window.monitor_from_point(x, y) {
        Some(monitor) => Ok(monitor_to_value(monitor)),
        None => Ok(json!(null)),
    }
}
