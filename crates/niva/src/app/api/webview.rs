use anyhow::Result;

use crate::app::NivaApp;
use crate::app::api_manager::ApiManager;
use crate::app::api_manager::ApiRequest;
use crate::app::main_exec::run_on_main;
use crate::app::window_manager::window::NivaWindow;
use std::sync::Arc;

pub fn register_apis(api_manager: &mut ApiManager) {
    api_manager.register_api("webview.isDevtoolsOpen", is_devtools_open);
    api_manager.register_api("webview.openDevtools", open_devtools);
    api_manager.register_api("webview.closeDevtools", close_devtools);
    api_manager.register_api("webview.baseUrl", base_url);
    api_manager.register_api("webview.baseFileSystemUrl", base_filesystem_url);
}

async fn is_devtools_open(
    app: Arc<NivaApp>,
    window: Arc<NivaWindow>,
    _request: ApiRequest,
) -> Result<bool> {
    run_on_main(&app, move |_target, _control_flow| {
        Ok(window.webview.is_devtools_open())
    })
    .await
}

async fn open_devtools(
    app: Arc<NivaApp>,
    window: Arc<NivaWindow>,
    _request: ApiRequest,
) -> Result<()> {
    run_on_main(&app, move |_target, _control_flow| {
        window.webview.open_devtools();
        Ok(())
    })
    .await
}

async fn close_devtools(
    app: Arc<NivaApp>,
    window: Arc<NivaWindow>,
    _request: ApiRequest,
) -> Result<()> {
    run_on_main(&app, move |_target, _control_flow| {
        window.webview.close_devtools();
        Ok(())
    })
    .await
}

async fn base_url(
    app: Arc<NivaApp>,
    _window: Arc<NivaWindow>,
    _request: ApiRequest,
) -> Result<String> {
    let (port, _) = app.server_info()?;
    Ok(format!("http://127.0.0.1:{port}/"))
}

async fn base_filesystem_url(
    app: Arc<NivaApp>,
    _window: Arc<NivaWindow>,
    _request: ApiRequest,
) -> Result<String> {
    let (port, _) = app.server_info()?;
    Ok(format!("http://127.0.0.1:{port}/__niva_fs/"))
}
