mod event;
mod menu;
mod webview;
mod window;

use serde_json::json;
use wry::webview::{FileDropEvent, WebContext, WebView};

use crate::{
    api_manager::{self, ApiManager},
    environment::EnvironmentRef, event::MainEventLoop,
};

pub struct WebviewWarper(WebView);
unsafe impl Send for WebviewWarper {}
unsafe impl Sync for WebviewWarper {}

pub fn open(
    env: EnvironmentRef,
    entry_url: String,
    main_event_loop: MainEventLoop,
    api_manager: ApiManager,
) -> ! {
    let main_window = window::create(env.clone(), &main_event_loop);
    let mut web_context = WebContext::new(Some(env.data_dir.clone()));
    let event_loop_proxy = main_event_loop.create_proxy();

    let main_webview = webview::create(
        env.clone(),
        &mut web_context,
        entry_url,
        main_window,
        move |window, request_str| {
            api_manager.call_api(window, request_str);
        },
        move |_, event| {
            let event_loop_proxy = event_loop_proxy.clone();
            match event {
                FileDropEvent::Dropped { paths, .. } => {
                    event_loop_proxy
                        .ipc_send_event("fileDrop.drop", json!({ "paths": paths }))
                        .unwrap();
                }
                FileDropEvent::Hovered { paths, .. } => {
                    event_loop_proxy
                        .ipc_send_event("fileDrop.hover", json!({ "paths": paths }))
                        .unwrap();
                }
                FileDropEvent::Cancelled => {
                    event_loop_proxy
                        .ipc_send_event("fileDrop.cancel", json!({}))
                        .unwrap();
                }
                _ => (),
            }
            false
        },
    );

    let main_webview_warper = WebviewWarper(main_webview);
    main_event_loop.0.run(move |event, target, control_flow| {
        event::handle(&main_webview_warper, event, target, control_flow)
    });
}
