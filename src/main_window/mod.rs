mod menu;
mod event;
mod webview;
mod window;

use wry::{
    application::{
        event_loop::{EventLoop},
    },
    webview::WebView,
};

use crate::{env::Config, thread_pool::ThreadPool};
use std::sync::{Arc, Mutex};

use self::event::UserEventContent;

pub struct WebviewWarper(WebView);
unsafe impl Send for WebviewWarper {}
unsafe impl Sync for WebviewWarper {}


pub fn open(
    entry_url: String,
    config: &Config,
    thread_pool: Arc<Mutex<ThreadPool>>,
    api_call: fn(String) -> String,
) -> ! {
    let event_loop = EventLoop::<UserEventContent>::with_user_event();
    let event_loop_proxy = event_loop.create_proxy();

    let main_window = window::create(config, &event_loop);

    let main_webview = webview::create(
        entry_url,
        config,
        main_window,
        move |_, request_str| {
            // TODO: need to inject window apis
            let event_loop_proxy = event_loop_proxy.clone();
            thread_pool.lock().unwrap().run(move || {
                let response_str = api_call(request_str.to_string());
                event_loop_proxy.send_event(response_str).unwrap();
            });
        },
    );

    let main_webview_warper = WebviewWarper(main_webview);
    event_loop.run(move |event, _, control_flow| {
        event::handle(&main_webview_warper, event, control_flow)
    });
}
