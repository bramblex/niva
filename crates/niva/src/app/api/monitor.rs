use anyhow::Result;
use serde_json::{Value, json};

use tao::monitor::MonitorHandle;

use crate::app::NivaApp;
use crate::app::api_manager::ApiRequest;
use crate::app::window_manager::window::NivaWindow;
use crate::{app::api_manager::ApiManager, logical};
use std::sync::Arc;

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

async fn list(
    _app: Arc<NivaApp>,
    window: Arc<NivaWindow>,
    _request: ApiRequest,
) -> Result<Vec<Value>> {
    Ok(window.available_monitors().map(monitor_to_value).collect())
}

async fn current(
    _app: Arc<NivaApp>,
    window: Arc<NivaWindow>,
    _request: ApiRequest,
) -> Result<Value> {
    match window.current_monitor() {
        Some(monitor) => Ok(monitor_to_value(monitor)),
        None => Ok(json!(null)),
    }
}

async fn primary(
    _app: Arc<NivaApp>,
    window: Arc<NivaWindow>,
    _request: ApiRequest,
) -> Result<Value> {
    match window.primary_monitor() {
        Some(monitor) => Ok(monitor_to_value(monitor)),
        None => Ok(json!(null)),
    }
}

async fn from_point(
    _app: Arc<NivaApp>,
    window: Arc<NivaWindow>,
    request: ApiRequest,
) -> Result<Value> {
    let (x, y) = request.args().get::<(f64, f64)>()?;
    match window.monitor_from_point(x, y) {
        Some(monitor) => Ok(monitor_to_value(monitor)),
        None => Ok(json!(null)),
    }
}
