use std::sync::Arc;

use anyhow::{Result, anyhow};
use serde_json::Value;

use crate::app::{
    NivaApp,
    api_manager::{ApiManager, ApiRequest},
    window_manager::window::NivaWindow,
};

pub fn register_api_instances(api_manager: &mut ApiManager) {
    api_manager.register_api("host.send", send);
}

async fn send(app: Arc<NivaApp>, window: Arc<NivaWindow>, request: ApiRequest) -> Result<()> {
    if window.id != 0 {
        return Err(anyhow!("host.send is main-window only"));
    }
    let bridge = app
        .stdio_bridge()
        .ok_or_else(|| anyhow!("host.send requires --stdio"))?;

    let args = request.args().get::<Vec<Value>>()?;
    let name = args
        .first()
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow!("host.send requires a string name"))?;
    let data = args.get(1).cloned();
    bridge.send_message(name, data)
}
