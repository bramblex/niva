use crate::env::Config;
use wry::{
    application::window::Window,
    webview::{WebView, WebViewBuilder, FileDropEvent},
};

static PRELOAD_JS: &str = include_str!("./preload.js");

pub fn create<IpcHandler, FileDropHandler>(
    entry_url: String, 
    config: &Config, 
    window: Window, 
    ipc_handler: IpcHandler,
    file_drop_handler: FileDropHandler
) -> WebView
where
    IpcHandler: Fn(&Window, String) + 'static,
    FileDropHandler : Fn(&Window, FileDropEvent) -> bool + 'static
{
    let mut webview_builder = WebViewBuilder::new(window).unwrap();

    webview_builder = webview_builder.with_initialization_script(PRELOAD_JS);
    webview_builder = webview_builder.with_clipboard(true);

    if let Some(devtools) = &config.devtools {
        webview_builder = webview_builder.with_devtools(*devtools);
    }

    if let Some(background_color) = &config.background_color {
        webview_builder = webview_builder.with_background_color(*background_color);
    }

    let prefix = entry_url.clone();
    webview_builder = webview_builder.with_navigation_handler(move |url| {
        url.starts_with(&prefix)
    });

    let webview = webview_builder
        .with_ipc_handler(ipc_handler)
        .with_file_drop_handler(file_drop_handler)
        .with_url(&entry_url)
        .unwrap()
        .build()
        .unwrap();

    webview
}
