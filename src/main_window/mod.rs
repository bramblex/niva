mod event;
mod menu;
mod webview;
mod window;
mod window_api;

use serde_json::json;
use wry::{
    application::event_loop::EventLoop,
    http::{request, response},
    webview::{FileDropEvent, WebView},
};

use crate::{
    env::Config,
    sys_api::{ApiRequest, ApiResponse},
    thread_pool::ThreadPool,
};
use std::sync::{Arc, Mutex};

use self::event::EventContent;

pub struct WebviewWarper(WebView);
unsafe impl Send for WebviewWarper {}
unsafe impl Sync for WebviewWarper {}

pub fn open(
    entry_url: String,
    config: &Config,
    thread_pool: Arc<Mutex<ThreadPool>>,
    api_call: fn(ApiRequest) -> ApiResponse,
) -> ! {
    let event_loop = EventLoop::<EventContent>::with_user_event();

    let main_window = window::create(config, &event_loop);

    let event_loop_proxy = event_loop.create_proxy();
    let event_loop_proxy2 = event_loop.create_proxy();

    let main_webview = webview::create(
        entry_url,
        config,
        main_window,
        move |window, request_str| {
            let request_result = serde_json::from_str::<ApiRequest>(request_str.as_str());
            if request_result.is_err() {
                event_loop_proxy
                    .send_event(EventContent::new(
                        "ipc.error",
                        json!({
                            "type": "ApiRequest parse error",
                            "reason": request_result.err().unwrap().to_string(),
                        }),
                    ))
                    .unwrap();
                return;
            }
            let request = request_result.unwrap();

            match request.namespace.as_str() {
                "window" => {
                    event_loop_proxy
                        .send_event(EventContent::new(
                            "ipc.callback",
                            window_api::call(window, request),
                        ))
                        .unwrap();
                }
                _ => {
                    let event_loop_proxy = event_loop_proxy.clone();
                    thread_pool.lock().unwrap().run(move || {
                        let response = api_call(request);
                        event_loop_proxy
                            .send_event(EventContent::new("ipc.callback", response))
                            .unwrap();
                    });
                }
            }
        },
        move |_, event| {
            let event_loop_proxy2 = event_loop_proxy2.clone();
            match event {
                FileDropEvent::Dropped { paths, .. } => {
                    event_loop_proxy2
                        .send_event(EventContent::new(
                            "fileDrop.drop",
                            json!({ "paths": paths }),
                        ))
                        .unwrap();
                }
                FileDropEvent::Hovered { paths, .. } => {
                    event_loop_proxy2
                        .send_event(EventContent::new(
                            "fileDrop.hover",
                            json!({ "paths": paths }),
                        ))
                        .unwrap();
                }
                FileDropEvent::Cancelled => {
                    event_loop_proxy2
                        .send_event(EventContent::new("fileDrop.cancel", json!({})))
                        .unwrap();
                }
                _ => (),
            }
            return false;
        },
    );

    let main_webview_warper = WebviewWarper(main_webview);
    event_loop.run(move |event, _, control_flow| {
        event::handle(&main_webview_warper, event, control_flow)
    });
}
