use anyhow::Result;
use serde_json::json;
use std::sync::Arc;
use tao::{event_loop::ControlFlow, clipboard::Clipboard};

use crate::app::{
    api_manager::{ApiManager, ApiRequest},
    window_manager::{options::NivaWindowOptions, window::NivaWindow},
    NivaApp, NivaId, NivaWindowTarget,
};

pub fn register_api_instances(api_manager: &mut ApiManager) {
    api_manager.register_event_api("clipboard.read", read);
    api_manager.register_event_api("clipboard.write", write);
}

fn read(
    _: Arc<NivaApp>,
    _: Arc<NivaWindow>,
    _: ApiRequest,
    _: &NivaWindowTarget,
    _: &mut ControlFlow,
) -> Result<Option<String>> {
    Ok(Clipboard::new().read_text())
}

fn write(
    _: Arc<NivaApp>,
    _: Arc<NivaWindow>,
    request: ApiRequest,
    _: &NivaWindowTarget,
    _: &mut ControlFlow,
) -> Result<()> {
    let text = request.args().single::<String>()?;
    Clipboard::new().write_text(text);
    Ok(())
}
