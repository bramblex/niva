use std::sync::Arc;

use anyhow::Result;

use niva_macros::niva_api;
use wry::application::event_loop::ControlFlow;

use crate::app::{
    api_manager::{ApiManager, ApiRequest},
    window_manager::{window::NivaWindow, url::make_base_url},
    NivaApp, NivaWindowTarget,
};

pub fn register_apis(api_manager: &mut ApiManager) {
    api_manager.register_event_api("webview.isDevtoolsOpen", is_devtools_open);
    api_manager.register_event_api("webview.openDevtools", open_devtools);
    api_manager.register_event_api("webview.closeDevtools", close_devtools);
    api_manager.register_api("webview.baseUrl", base_url);
    api_manager.register_api("webview.baseFilesystemUrl", base_filesystem_url);
}

fn is_devtools_open(
    _app: Arc<NivaApp>,
    window: Arc<NivaWindow>,
    _request: ApiRequest,
    _target: &NivaWindowTarget,
    _control_flow: &mut ControlFlow,
) -> Result<bool> {
    Ok(window.webview.is_devtools_open())
}

fn open_devtools(
    _app: Arc<NivaApp>,
    window: Arc<NivaWindow>,
    _request: ApiRequest,
    _target: &NivaWindowTarget,
    _control_flow: &mut ControlFlow,
) -> Result<()> {
    window.webview.open_devtools();
    Ok(())
}

fn close_devtools(
    _app: Arc<NivaApp>,
    window: Arc<NivaWindow>,
    _request: ApiRequest,
    _target: &NivaWindowTarget,
    _control_flow: &mut ControlFlow,
) -> Result<()> {
    window.webview.close_devtools();
    Ok(())
}

#[niva_api]
fn base_url() -> Result<String> {
    Ok(make_base_url("niva", &app.launch_info.id_name))
}

#[niva_api]
fn base_filesystem_url() -> Result<String> {
    Ok(make_base_url("niva", "filesystem"))
}
