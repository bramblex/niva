use std::collections::HashMap;

use wry::{application::window::WindowId, webview::WebView};
use self::options::WindowOptions;

mod menu;
mod options;
mod webview;
mod window;

pub type CallbackEvent = String;
pub type EventLoop = wry::application::event_loop::EventLoop<CallbackEvent>;
pub type EventLoopProxy = wry::application::event_loop::EventLoopProxy<CallbackEvent>;
pub type EventLoopWindowTarget = wry::application::event_loop::EventLoopWindowTarget<CallbackEvent>;

pub struct WindowManager<> {
    event_loop: EventLoopProxy,
    webview_map: HashMap<WindowId, WebView>,
}

impl WindowManager {
    pub fn new(event_loop: EventLoopProxy) -> Self {
        Self {
            event_loop,
            webview_map: HashMap::new(),
        }
    }

    pub fn get_window(&self, window_id: WindowId) -> Option<&WebView> {
        self.webview_map.get(&window_id)
    }

    pub fn remove_window(&mut self, window_id: WindowId) -> Option<WebView> {
        self.webview_map.remove(&window_id)
    }

    pub fn create_window(
        &mut self,
        target: &EventLoopWindowTarget,
        options: &WindowOptions,
    ) -> WindowId {
        let window = window::create(target, options);
        let window_id = window.id();
        let webview = webview::create(self.event_loop.clone(), window, options);
        self.webview_map.insert(window_id.clone(), webview);
        window_id
    }
}
