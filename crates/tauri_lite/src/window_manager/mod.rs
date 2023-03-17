use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use crate::{
    api_manager::ApiManager,
    environment::EnvironmentRef,
    event_loop::{MainEventLoopProxy, MainEventLoopTarget},
};
use wry::{
    application::window::WindowId,
    webview::{WebContext, WebView},
};

use self::options::WindowOptions;

mod menu;
pub mod options;
mod webview;
mod window;

pub type WebViewRef = Arc<Mutex<WebView>>;

pub struct WindowManager {
    env: EnvironmentRef,
    base_url: String,
    api_manager: Arc<ApiManager>,
    event_loop: MainEventLoopProxy,
    webview_map: HashMap<WindowId, WebViewRef>,
}

impl WindowManager {
    pub fn new(
        env: EnvironmentRef,
        base_url: String,
        api_manager: ApiManager,
        event_loop: MainEventLoopProxy,
    ) -> Self {
        Self {
            env,
            base_url,
            api_manager: Arc::new(api_manager),
            event_loop,
            webview_map: HashMap::new(),
        }
    }

    #[allow(dead_code)]
    pub fn get_window(&self, window_id: WindowId) -> Option<WebViewRef> {
        self.webview_map.get(&window_id).cloned()
    }

    #[allow(dead_code)]
    pub fn remove_window(&mut self, window_id: WindowId) -> Option<WebViewRef> {
        self.webview_map.remove(&window_id)
    }

    pub fn create_window(
        &mut self,
        target: &MainEventLoopTarget,
        options: &WindowOptions,
    ) -> (WindowId, WebViewRef) {
        let window = window::create(target, options);
        let window_id = window.id();

        let base_url = self
            .env
            .debug_entry
            .clone()
            .unwrap_or(self.base_url.clone());

        let mut web_context = WebContext::new(Some(self.env.data_dir.clone()));

        let entry = options.entry.clone().unwrap_or("".to_string());
        let entry = format!("{base_url}/{entry}");
        let webview = Arc::new(Mutex::new(webview::create(
            self.event_loop.clone(),
            self.api_manager.clone(),
            window,
            options,
            &mut web_context,
            base_url,
            entry,
        )));

        self.webview_map
            .insert(window_id, webview.clone());
        (window_id, webview)
    }
}
