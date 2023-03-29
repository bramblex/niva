use anyhow::{anyhow, Result};
use std::{borrow::Cow, ops::Deref, sync::Arc};

use serde::Serialize;
use serde_json::json;
use tao::{
    dpi,
    event_loop::ControlFlow,
    window::{Window, WindowId},
};
use wry::{
    http::Response,
    webview::{WebContext, WebView, WebViewBuilder},
};

use crate::{
    app::{utils::{arc, ArcMut}, NivaApp, NivaEvent, NivaEventLoopProxy, NivaId, NivaWindowTarget},
    unsafe_impl_sync_send,
};

use super::{
    builder::NivaBuilder,
    options::{NivaWindowOptions, Position, Size},
};

impl From<Position> for dpi::Position {
    fn from(val: Position) -> Self {
        dpi::Position::Logical(dpi::LogicalPosition::new(val.0, val.1))
    }
}

impl From<Size> for dpi::Size {
    fn from(val: Size) -> Self {
        dpi::Size::Logical(dpi::LogicalSize::new(val.0, val.1))
    }
}

pub struct NivaWindow {
    pub id: NivaId,
    pub window_id: WindowId,
    pub webview: WebView,
    pub options: NivaWindowOptions,
    event_loop_proxy: NivaEventLoopProxy,
}

unsafe_impl_sync_send!(NivaWindow);
impl Deref for NivaWindow {
    type Target = Window;
    fn deref(&self) -> &Self::Target {
        self.webview.window()
    }
}

impl NivaWindow {
    pub fn new(
        app: Arc<NivaApp>,
        id: NivaId,
        options: &NivaWindowOptions,
        web_context: &mut WebContext,
        target: &NivaWindowTarget,
    ) -> Result<Arc<NivaWindow>> {
        let window = NivaBuilder::build_window(&app, id, options, target)?;
        let webview = NivaBuilder::build_webview(&app, options, window, web_context)?;

        Ok(arc(Self {
            id,
            window_id: webview.window().id(),
            webview,
            options: options.clone(),
            event_loop_proxy: app.event_loop_proxy.clone(),
        }))
    }

    #[cfg(target_os = "macos")]
    pub fn set_current_menu(self: &Arc<Self>) {
        self.set_menu(NivaBuilder::build_menu(&self.options.menu));
    }

    pub fn update_menu() {
    }

    pub fn send_event<F: Fn(&NivaWindowTarget, &mut ControlFlow) -> Result<()> + Send + 'static>(
        self: &Arc<Self>,
        f: F,
    ) -> Result<()> {
        self.event_loop_proxy
            .send_event(NivaEvent::new(f))
            .map_err(|_| anyhow!("Failed to send event"))
    }

    pub fn send_ipc_event<E: Into<String>, P: Serialize>(
        self: &Arc<Self>,
        event: E,
        payload: P,
    ) -> Result<()> {
        let event: String = event.into();
        let payload = serde_json::to_string(&payload)?;
        let _self = self.clone();
        self.send_event(move |_, _| {
            _self
                .webview
                .evaluate_script(&format!("Niva.__emit__(\"{event}\", {payload})"))?;
            Ok(())
        })
    }

    pub fn send_ipc_callback<D: Serialize>(self: &Arc<Self>, data: D) -> Result<()> {
        self.send_ipc_event("ipc.callback", json!(data))
    }
}
