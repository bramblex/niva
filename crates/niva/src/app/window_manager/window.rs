use crate::lock_force;

use anyhow::Result;
use std::{
    ops::Deref,
    sync::{Arc, Mutex},
};

use serde::Serialize;
use tao::window::{Window, WindowId};
use wry::WebView;

use crate::{
    app::{
        NivaApp, NivaWindowTarget,
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

    /// Attached muda menu handle. Must be kept alive while attached:
    /// dropping it removes the native menu.
    menu_handle: Mutex<Option<muda::Menu>>,

    /// Live API WebSocket sender for this window's webview, if connected.
    ws_tx: Mutex<Option<std::sync::mpsc::Sender<WsOut>>>,

    pub state: Mutex<NivaWindowState>,
}

/// Outbound WebSocket traffic for one window.
pub enum WsOut {
    Text(String),
    Binary(Vec<u8>),
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
        // Clone out and drop the guard: building does file IO and attaching
        // takes menu_handle — never nest menu_options -> menu_handle.
        let menu_options = lock_force!(self.menu_options).clone();
        if let Some(menu) = NivaBuilder::build_menu(self.id, &self.app, &menu_options) {
            self.attach_built_menu(&menu);
        }
    }

    pub fn set_menu(self: &Arc<Self>, options: &Option<WindowMenuOptions>) {
        *lock_force!(self.menu_options) = options.clone();
        let visible = lock_force!(self.state).is_menu_visible;
        if self.is_focused() && visible {
            let menu_options = lock_force!(self.menu_options).clone();
            if let Some(menu) = NivaBuilder::build_menu(self.id, &self.app, &menu_options) {
                self.attach_built_menu(&menu);
            }
        }
    }

    pub fn show_menu(self: &Arc<Self>) {
        lock_force!(self.state).is_menu_visible = true;
        let menu_options = lock_force!(self.menu_options).clone();
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

    /// Register (or replace) the live API WebSocket sender for this window.
    /// Called by the HTTP server when the webview connects.
    pub fn set_ws_sender(self: &Arc<Self>, tx: Option<std::sync::mpsc::Sender<WsOut>>) {
        *lock_force!(self.ws_tx) = tx;
    }

    /// Try pushing a pre-built `["event", name, payload]` envelope to the
    /// window's WebSocket. Returns false when no socket is connected.
    pub fn send_ws_envelope(self: &Arc<Self>, envelope: &str) -> bool {
        let guard = lock_force!(self.ws_tx);
        match guard.as_ref() {
            Some(tx) => tx.send(WsOut::Text(envelope.to_string())).is_ok(),
            None => false,
        }
    }

    /// Try pushing a binary chunk frame to the window's WebSocket.
    pub fn send_ws_binary(self: &Arc<Self>, frame: &[u8]) -> bool {
        let guard = lock_force!(self.ws_tx);
        match guard.as_ref() {
            Some(tx) => tx.send(WsOut::Binary(frame.to_vec())).is_ok(),
            None => false,
        }
    }

    /// Push an event to the page over its WebSocket. Returns false (dropped,
    /// no log) when no socket is connected — by definition nobody can be
    /// listening, e.g. the page is still booting. All API traffic is WS-only.
    pub fn send_ipc_event<E: Into<String>, P: Serialize>(
        self: &Arc<Self>,
        event: E,
        payload: P,
    ) -> bool {
        let envelope = serde_json::json!(["event", event.into(), payload]);
        self.send_ws_envelope(&envelope.to_string())
    }
}
