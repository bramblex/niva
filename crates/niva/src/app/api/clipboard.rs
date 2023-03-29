use anyhow::Result;

use std::sync::Arc;
use tao::{clipboard::Clipboard, event_loop::ControlFlow};

use crate::{app::{
    api_manager::{ApiManager, ApiRequest},
    window_manager::window::NivaWindow,
    NivaApp, NivaWindowTarget,
}, args_match};

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
    args_match!(request, text: String);
    Clipboard::new().write_text(text);
    Ok(())
}
