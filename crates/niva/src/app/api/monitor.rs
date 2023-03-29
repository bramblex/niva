use anyhow::Result;
use serde_json::{json, Value};

use tao::monitor::MonitorHandle;

use crate::{app::api_manager::ApiManager, logical};

pub fn register_api_instances(api_manager: &mut ApiManager) {
    api_manager.register_api("monitor.list", |_, window, _| -> Result<Vec<Value>> {
        Ok(window.available_monitors().map(monitor_to_value).collect())
    });

    api_manager.register_api("monitor.current", |_, window, _| -> Result<Value> {
        match window.current_monitor() {
            Some(monitor) => Ok(monitor_to_value(monitor)),
            None => Ok(json!(null)),
        }
    });

    api_manager.register_api("monitor.primary", |_, window, _| -> Result<Value> {
        match window.primary_monitor() {
            Some(monitor) => Ok(monitor_to_value(monitor)),
            None => Ok(json!(null)),
        }
    });

    api_manager.register_api("monitor.fromPoint", |_, window, request| -> Result<Value> {
        let (x, y) = request.args().get::<(f64, f64)>()?;
        match window.monitor_from_point(x, y) {
            Some(monitor) => Ok(monitor_to_value(monitor)),
            None => Ok(json!(null)),
        }
    });
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
