use crate::lock_force;

use anyhow::{Result, anyhow};
use std::{
    ops::Deref,
    sync::{Arc, Mutex},
};

use serde::Serialize;
use serde_json::json;
use tao::{
    event_loop::ControlFlow,
    window::{Window, WindowId},
};
use wry::WebView;

use crate::{
    app::{
        NivaApp, NivaEvent, NivaEventLoopProxy, NivaWindowTarget,
        utils::{ArcMut, arc, arc_mut},
    },
    unsafe_impl_sync_send,
};

use super::{
    WindowManager,
    builder::NivaBuilder,
    options::{NivaWindowOptions, WindowMenuOptions},
};

pub struct NivaWindowState {
    pub is_block_closed_requested: bool,
    pub is_menu_visible: bool,
}

pub struct NivaWindow {
    pub id: u8,
    pub window_id: WindowId,
    /// Owned native window (wry no longer gives access to it via WebView).
    /// Declared after `webview` so it drops after the webview.
    pub webview: WebView,
    pub window: Window,
    pub menu_options: ArcMut<Option<WindowMenuOptions>>,
    app: Arc<NivaApp>,
    event_loop_proxy: NivaEventLoopProxy,

    /// Attached muda menu handle. Must be kept alive while attached:
    /// dropping it removes the native menu.
    menu_handle: Mutex<Option<muda::Menu>>,

    /// Live API WebSocket sender for this window's webview, if connected.
    ws_tx: Mutex<Option<std::sync::mpsc::Sender<String>>>,

    pub state: Mutex<NivaWindowState>,
}

// NivaWindow is accessed from the webview IPC/drag-drop handlers and the
// event loop. muda menu handles are thread-bound (Rc-based); all menu
// operations happen on the main thread, same as before with tao menus.
unsafe_impl_sync_send!(NivaWindow);
impl Deref for NivaWindow {
    type Target = Window;
    fn deref(&self) -> &Self::Target {
        &self.window
    }
}

impl NivaWindow {
    pub fn new(
        app: Arc<NivaApp>,
        manager: &mut WindowManager,
        id: u8,
        options: &NivaWindowOptions,
        target: &NivaWindowTarget,
    ) -> Result<Arc<NivaWindow>> {
        let (window, menu) = NivaBuilder::build_window(&app, manager, id, options, target)?;
        let window_id = window.id();
        let webview = NivaBuilder::build_webview(
            &app,
            id,
            options,
            &window,
            &mut manager.web_context,
            window_id,
        )?;

        Ok(arc(Self {
            app: app.clone(),
            id,
            window_id,
            webview,
            window,
            menu_options: arc_mut(options.menu.clone()),
            event_loop_proxy: app.event_loop_proxy.clone(),
            menu_handle: Mutex::new(menu),
            ws_tx: Mutex::new(None),

            state: Mutex::new(NivaWindowState {
                is_block_closed_requested: false,
                is_menu_visible: true,
            }),
        }))
    }

    fn attach_built_menu(&self, menu: &muda::Menu) {
        NivaBuilder::attach_menu(&self.window, menu);
        // Replacing the handle drops the previous native menu.
        *lock_force!(self.menu_handle) = Some(menu.clone());
    }

    #[cfg(target_os = "macos")]
    pub fn switch_menu(self: &Arc<Self>) {
        if !lock_force!(self.state).is_menu_visible {
            return;
        }
        let menu_options = lock_force!(self.menu_options);
        if let Some(menu) = NivaBuilder::build_menu(self.id, &self.app, &menu_options) {
            self.attach_built_menu(&menu);
        }
    }

    pub fn set_menu(self: &Arc<Self>, options: &Option<WindowMenuOptions>) {
        let mut menu_options = lock_force!(self.menu_options);
        *menu_options = options.clone();
        let visible = lock_force!(self.state).is_menu_visible;
        if self.is_focused()
            && visible
            && let Some(menu) = NivaBuilder::build_menu(self.id, &self.app, &menu_options)
        {
            self.attach_built_menu(&menu);
        }
    }

    pub fn show_menu(self: &Arc<Self>) {
        lock_force!(self.state).is_menu_visible = true;
        let menu_options = lock_force!(self.menu_options);
        if let Some(menu) = NivaBuilder::build_menu(self.id, &self.app, &menu_options) {
            self.attach_built_menu(&menu);
        }
    }

    pub fn hide_menu(self: &Arc<Self>) {
        lock_force!(self.state).is_menu_visible = false;
        let handle = lock_force!(self.menu_handle).take();
        if let Some(menu) = handle {
            NivaBuilder::detach_menu(&self.window, &menu);
        }
    }

    pub fn is_menu_visible(self: &Arc<Self>) -> bool {
        lock_force!(self.state).is_menu_visible
    }

    pub fn send_event<F: Fn(&NivaWindowTarget, &mut ControlFlow) -> Result<()> + Send + 'static>(
        self: &Arc<Self>,
        f: F,
    ) -> Result<()> {
        self.event_loop_proxy
            .send_event(NivaEvent::new(f))
            .map_err(|_| anyhow!("Failed to send event"))
    }

    /// Register (or replace) the live API WebSocket sender for this window.
    /// Called by the HTTP server when the webview connects.
    pub fn set_ws_sender(self: &Arc<Self>, tx: Option<std::sync::mpsc::Sender<String>>) {
        *lock_force!(self.ws_tx) = tx;
    }

    /// Try pushing a pre-built `["event", name, payload]` envelope to the
    /// window's WebSocket. Returns false when no socket is connected.
    pub fn send_ws_envelope(self: &Arc<Self>, envelope: &str) -> bool {
        let guard = lock_force!(self.ws_tx);
        match guard.as_ref() {
            Some(tx) => tx.send(envelope.to_string()).is_ok(),
            None => false,
        }
    }

    pub fn send_ipc_event<E: Into<String>, P: Serialize>(
        self: &Arc<Self>,
        event: E,
        payload: P,
    ) -> Result<()> {
        let envelope = serde_json::json!(["event", event.into(), payload]);
        if self.send_ws_envelope(&envelope.to_string()) {
            return Ok(());
        }
        self.send_envelope_eval(&envelope)
    }

    /// Deliver a pre-built envelope by evaluating `Niva.__emit__` on the main
    /// thread. Used when no WebSocket is connected yet (page still loading).
    pub fn send_envelope_eval(self: &Arc<Self>, envelope: &serde_json::Value) -> Result<()> {
        let arr = envelope.as_array().ok_or(anyhow!("Invalid IPC envelope"))?;
        let event = arr
            .first()
            .and_then(|v| v.as_str())
            .ok_or(anyhow!("Invalid IPC envelope"))?
            .to_string();
        let payload = arr.get(2).cloned().unwrap_or(serde_json::Value::Null);
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
