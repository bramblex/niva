mod event;
mod menu;
mod webview;
mod window;

use serde_json::json;
use wry::{
    application::event_loop::EventLoop,
    webview::{FileDropEvent, WebView},
};

use crate::{env::Config, thread_pool::ThreadPool};
use std::sync::{Arc, Mutex};

use self::event::EventContent;

pub struct WebviewWarper(WebView);
unsafe impl Send for WebviewWarper {}
unsafe impl Sync for WebviewWarper {}

pub fn open(
    entry_url: String,
    config: &Config,
    thread_pool: Arc<Mutex<ThreadPool>>,
    api_call: fn(String) -> String,
) -> ! {
    let event_loop = EventLoop::<EventContent>::with_user_event();

    let main_window = window::create(config, &event_loop);

    let event_loop_proxy = event_loop.create_proxy();
    let event_loop_proxy2 = event_loop.create_proxy();

    let main_webview = webview::create(
        entry_url,
        config,
        main_window,
        move |_, request_str| {
            // TODO: need to inject window apis
            let event_loop_proxy = event_loop_proxy.clone();
            thread_pool.lock().unwrap().run(move || {
                let response_str = api_call(request_str.to_string());
                event_loop_proxy
                    .send_event(EventContent::Callback(response_str))
                    .unwrap();
            });
        },
        move |_, event| {
            let event_loop_proxy2 = event_loop_proxy2.clone();
            match event {
                FileDropEvent::Dropped { paths, .. } => {
                    event_loop_proxy2
                        .send_event(EventContent::Event(
                            "fileDrop.drop".to_string(),
                            json!({ "paths": paths }),
                        ))
                        .unwrap();
                }
                FileDropEvent::Hovered { paths, .. } => {
                    event_loop_proxy2
                        .send_event(EventContent::Event(
                            "fileDrop.hover".to_string(),
                            json!({ "paths": paths }),
                        ))
                        .unwrap();
                }
                FileDropEvent::Cancelled => {
                    event_loop_proxy2
                        .send_event(EventContent::Event(
                            "fileDrop.cancel".to_string(),
                            json!({}),
                        ))
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
