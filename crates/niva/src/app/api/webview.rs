use anyhow::Result;

use niva_macros::{niva_api, niva_event_api};

use crate::app::api_manager::ApiManager;

pub fn register_apis(api_manager: &mut ApiManager) {
    api_manager.register_event_api("webview.isDevtoolsOpen", is_devtools_open);
    api_manager.register_event_api("webview.openDevtools", open_devtools);
    api_manager.register_event_api("webview.closeDevtools", close_devtools);
    api_manager.register_api("webview.baseUrl", base_url);
    api_manager.register_api("webview.baseFileSystemUrl", base_filesystem_url);
}

#[niva_event_api]
fn is_devtools_open() -> Result<bool> {
    Ok(window.webview.is_devtools_open())
}

#[niva_event_api]
fn open_devtools() -> Result<()> {
    window.webview.open_devtools();
    Ok(())
}

#[niva_event_api]
fn close_devtools() -> Result<()> {
    window.webview.close_devtools();
    Ok(())
}

#[niva_api]
fn base_url() -> Result<String> {
    let (port, _) = app.server_info()?;
    Ok(format!("http://127.0.0.1:{port}/"))
}

#[niva_api]
fn base_filesystem_url() -> Result<String> {
    let (port, _) = app.server_info()?;
    Ok(format!("http://127.0.0.1:{port}/__niva_fs/"))
}
