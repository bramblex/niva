use anyhow::Result;

use wry::{webview::WebView, application::event_loop::ControlFlow};

use crate::{api_manager::{ApiManager, ApiRequest}, environment::EnvironmentRef};
pub fn register_apis(api_manager: &mut ApiManager) {
    api_manager.register_event_api("webview.isDevtoolsOpen", is_devtools_open);
    api_manager.register_event_api("webview.openDevtools", open_devtools);
    api_manager.register_event_api("webview.closeDevtools", close_devtools);
    api_manager.register_event_api("webview.setBackgroundColor", set_background_color);
}

fn is_devtools_open(_: EnvironmentRef, webview: &WebView, _: ApiRequest, _: &mut ControlFlow) -> Result<bool> {
    Ok(webview.is_devtools_open())
}

fn open_devtools(_: EnvironmentRef, webview: &WebView, _: ApiRequest, _: &mut ControlFlow) -> Result<()> {
    webview.open_devtools();
    Ok(())
}

fn close_devtools(_: EnvironmentRef, webview: &WebView, _: ApiRequest, _: &mut ControlFlow) -> Result<()> {
    webview.close_devtools();
    Ok(())
}

fn set_background_color(_: EnvironmentRef, webview: &WebView, request: ApiRequest, _: &mut ControlFlow) -> Result<()> {
    let color = request.args().get_single::<(u8, u8, u8, u8)>()?;
    webview.set_background_color(color)?;
    Ok(())
}
