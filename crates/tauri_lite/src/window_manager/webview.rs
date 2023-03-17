use std::sync::Arc;

use serde_json::json;
use wry::{
    application::window::Window,
    webview::{FileDropEvent, WebContext, WebView, WebViewBuilder},
};

use crate::{api_manager::ApiManager, event_loop::MainEventLoopProxy};

use super::options::WindowOptions;

static PRELOAD_JS: &str = include_str!("../assets/preload.js");

pub fn create(
    event_loop: MainEventLoopProxy,
    api_manager: Arc<ApiManager>,
    window: Window,
    options: &WindowOptions,
    web_context: &mut WebContext,
    base_url: String,
    entry: String,
) -> WebView {
    let mut webview_builder = WebViewBuilder::new(window).unwrap();

    webview_builder = webview_builder.with_initialization_script(PRELOAD_JS);
    webview_builder = webview_builder.with_clipboard(true);
    webview_builder = webview_builder.with_accept_first_mouse(true);

    if let Some(devtools) = &options.devtools {
        webview_builder = webview_builder.with_devtools(*devtools);
    }

    if let Some(transparent) = &options.transparent {
        webview_builder = webview_builder.with_transparent(*transparent);
    }

    if let Some(background_color) = &options.background_color {
        webview_builder = webview_builder.with_background_color(*background_color);
    }

    webview_builder =
        webview_builder.with_navigation_handler(move |url| url.starts_with(&base_url));

    webview_builder = webview_builder.with_web_context(web_context);

    let webview = webview_builder
        .with_ipc_handler(move |window, request_str| {
            api_manager.call_api(window, request_str);
        })
        .with_file_drop_handler(move |_: &Window, event| {
            match event {
                FileDropEvent::Hovered { paths, position } => event_loop
                    .ipc_send_event(
                        "fileDrop.hovered",
                        json!({
                            "paths": paths,
                            "position": (position.x, position.y)
                        }),
                    )
                    .unwrap(),
                FileDropEvent::Dropped { paths, position } => event_loop
                    .ipc_send_event(
                        "fileDrop.dropped",
                        json!({
                            "paths": paths,
                            "position": (position.x, position.y)
                        }),
                    )
                    .unwrap(),
                FileDropEvent::Cancelled => event_loop
                    .ipc_send_event("fileDrop.cancelled", json!(null))
                    .unwrap(),
                _ => (),
            }
            false
        })
        .with_url(&entry)
        .unwrap()
        .build()
        .unwrap();

    webview
}
