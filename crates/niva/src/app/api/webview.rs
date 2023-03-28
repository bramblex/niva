use std::sync::Arc;

use anyhow::Result;

use wry::application::event_loop::ControlFlow;

use crate::app::{
    api_manager::{ApiManager, ApiRequest},
    window_manager::window::NivaWindow,
    NivaApp, NivaWindowTarget,
};

pub fn register_apis(api_manager: &mut ApiManager) {
    api_manager.register_event_api("webview.isDevtoolsOpen", is_devtools_open);
    api_manager.register_event_api("webview.openDevtools", open_devtools);
    api_manager.register_event_api("webview.closeDevtools", close_devtools);
    api_manager.register_event_api("webview.setBackgroundColor", set_background_color);
}

fn is_devtools_open(
    app: Arc<NivaApp>,
    window: Arc<NivaWindow>,
    request: ApiRequest,
    target: &NivaWindowTarget,
    _control_flow: &mut ControlFlow,
) -> Result<bool> {
    Ok(window.webview.is_devtools_open())
}

fn open_devtools(
    app: Arc<NivaApp>,
    window: Arc<NivaWindow>,
    request: ApiRequest,
    target: &NivaWindowTarget,
    _control_flow: &mut ControlFlow,
) -> Result<()> {
    window.webview.open_devtools();
    Ok(())
}

fn close_devtools(
    app: Arc<NivaApp>,
    window: Arc<NivaWindow>,
    request: ApiRequest,
    target: &NivaWindowTarget,
    _control_flow: &mut ControlFlow,
) -> Result<()> {
    window.webview.close_devtools();
    Ok(())
}

fn set_background_color(
    app: Arc<NivaApp>,
    window: Arc<NivaWindow>,
    request: ApiRequest,
    target: &NivaWindowTarget,
    _control_flow: &mut ControlFlow,
) -> Result<()> {
    let color = request.args().single::<(u8, u8, u8, u8)>()?;
    window.webview.set_background_color(color)?;
    Ok(())
}
