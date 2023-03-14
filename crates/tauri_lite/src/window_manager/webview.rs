use wry::{
    application::window::Window,
    webview::{WebView, WebViewBuilder},
};

use super::{options::WindowOptions, EventLoopProxy};

static PRELOAD_JS: &str = include_str!("../assets/preload.js");

pub fn create(
    event_loop: EventLoopProxy,
    window: Window,
    options: &WindowOptions,
) -> WebView
{
    let mut webview_builder = WebViewBuilder::new(window).unwrap();

    webview_builder = webview_builder.with_initialization_script(PRELOAD_JS);
    webview_builder = webview_builder.with_clipboard(true);

    if let Some(devtools) = &options.devtools {
        webview_builder = webview_builder.with_devtools(*devtools);
    }

    if let Some(background_color) = &options.background_color {
        webview_builder = webview_builder.with_background_color(*background_color);
    }

    let webview = webview_builder
        .with_ipc_handler(move |_, event| {
            event_loop.send_event(event).unwrap();
        })
        .with_url("file:///Users/qiaojian/Workspace/tauri_lite/packages/example/index.html")
        .unwrap()
        .build()
        .unwrap();

    webview
}
