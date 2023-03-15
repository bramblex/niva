use anyhow::Result;
use serde::Serialize;
use std::{
    fmt::{Debug, Formatter},
    ops::Deref,
    pin::Pin,
};

pub mod event_handler;

use wry::{
    application::event_loop::{ControlFlow, EventLoop, EventLoopProxy, EventLoopWindowTarget},
    webview::WebView,
};

use crate::api_manager::ApiResponse;

type Callback =
    Pin<Box<dyn Fn(&WebView, &EventLoopWindowTarget<CallbackEvent>, &mut ControlFlow) + Send>>;
pub struct CallbackEvent(pub Callback);

impl CallbackEvent {
    pub fn call(
        &self,
        webview: &WebView,
        event_loop: &EventLoopWindowTarget<CallbackEvent>,
        control_flow: &mut ControlFlow,
    ) {
        (self.0)(webview, event_loop, control_flow)
    }
}

impl Debug for CallbackEvent {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CallbackEvent").finish()
    }
}

pub struct MainEventLoop(pub EventLoop<CallbackEvent>);

pub type MainEventLoopTarget = EventLoopWindowTarget<CallbackEvent>;

impl MainEventLoop {
    pub fn new() -> Self {
        MainEventLoop(EventLoop::<CallbackEvent>::with_user_event())
    }

    pub fn create_proxy(&self) -> MainEventLoopProxy {
        MainEventLoopProxy(self.0.create_proxy())
    }
}

impl Deref for MainEventLoop {
    type Target = EventLoop<CallbackEvent>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Clone)]
pub struct MainEventLoopProxy(pub EventLoopProxy<CallbackEvent>);

impl Deref for MainEventLoopProxy {
    type Target = EventLoopProxy<CallbackEvent>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

unsafe impl Send for MainEventLoopProxy {}
unsafe impl Sync for MainEventLoopProxy {}

impl MainEventLoopProxy {
    pub fn ipc_send_callback(&self, response: ApiResponse) -> Result<()> {
        self.ipc_send_event("ipc.callback", response)
    }

    pub fn ipc_send_event<S: Into<String>, D: Serialize>(&self, event: S, data: D) -> Result<()> {
        let event = event.into();
        let data_str = serde_json::to_string::<D>(&data).unwrap();
        self.0
            .send_event(CallbackEvent(Box::pin(move |webview, _, _| {
                webview
                    .evaluate_script(&format!("TauriLite.__emit__(\"{event}\", {data_str})"))
                    .unwrap();
            })))
            .unwrap();
        Ok(())
    }
}
