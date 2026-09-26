use crate::lock_force;

use anyhow::Result;
use std::{
    collections::HashMap,
    ops::Deref,
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
    },
};

use serde::Serialize;
use tao::window::{Window, WindowId};
use wry::WebView;

use crate::{
    app::{
        NivaApp, NivaEvent, NivaWindowTarget,
        api_manager::protocol::ServerMsg,
        utils::{ArcMut, arc, arc_mut},
    },
    unsafe_impl_sync_send,
};

use super::{
    WindowManager,
    builder::NivaBuilder,
    options::{NivaWindowOptions, WindowMenuOptions},
    permissions::WindowPermissions,
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
    pub token: String,
    /// Exact browser origin permitted to use this window's WebSocket token.
    pub trusted_ws_origin: Option<String>,
    pub permissions: WindowPermissions,
    app: Arc<NivaApp>,

    /// Attached muda menu handle. Must be kept alive while attached:
    /// dropping it removes the native menu.
    menu_handle: Mutex<Option<muda::Menu>>,

    /// Live API WebSocket sender for this window's webview, if connected.
    ws_txs: Mutex<HashMap<u64, std::sync::mpsc::Sender<WsOut>>>,
    next_ws_id: AtomicU64,
    next_event_seq: AtomicU64,

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
        let permissions = WindowPermissions::new(&options.permissions)?;
        let mut token_bytes = [0u8; 16];
        getrandom::fill(&mut token_bytes)
            .map_err(|err| anyhow::anyhow!("Unable to generate window token: {err}"))?;
        let token: String = token_bytes
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect();
        let (window, menu) = NivaBuilder::build_window(&app, manager, id, options, target)?;
        let window_id = window.id();
        let (webview, trusted_ws_origin) = NivaBuilder::build_webview(
            &app,
            id,
            options,
            &window,
            &mut manager.web_context,
            window_id,
            &token,
        )?;

        #[cfg(target_os = "macos")]
        crate::app::ipc_macos::install(&webview, app.clone(), id)?;
        #[cfg(target_os = "windows")]
        crate::app::ipc_windows_frames::install(&webview, app.clone(), id)?;

        Ok(arc(Self {
            app: app.clone(),
            id,
            window_id,
            webview,
            window,
            menu_options: arc_mut(options.menu.clone()),
            token,
            trusted_ws_origin,
            permissions,
            menu_handle: Mutex::new(menu),
            ws_txs: Mutex::new(HashMap::new()),
            next_ws_id: AtomicU64::new(1),
            next_event_seq: AtomicU64::new(1),

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

    /// Each frame owns a separate socket and call-id namespace.
    pub fn register_ws_sender(&self, tx: std::sync::mpsc::Sender<WsOut>) -> u64 {
        let connection_id = self.next_ws_id.fetch_add(1, Ordering::Relaxed);
        lock_force!(self.ws_txs).insert(connection_id, tx);
        connection_id
    }

    pub fn remove_ws_sender(&self, connection_id: u64) {
        lock_force!(self.ws_txs).remove(&connection_id);
    }

    /// Window events are broadcast to all connected frames. Call results are
    /// sent directly to the connection that issued the call by ApiManager.
    pub fn send_ws_envelope(self: &Arc<Self>, envelope: &str) -> bool {
        let mut connections = lock_force!(self.ws_txs);
        connections.retain(|_, tx| tx.send(WsOut::Text(envelope.to_string())).is_ok());
        !connections.is_empty()
    }

    /// Push a window event to all connected same-origin frames.
    pub fn send_ipc_event<E: Into<String>, P: Serialize>(
        self: &Arc<Self>,
        event: E,
        payload: P,
    ) -> bool {
        let payload = serde_json::to_value(payload).unwrap_or(serde_json::Value::Null);
        let seq = self.next_event_seq();
        let envelope = ServerMsg::event(None, seq, event.into(), payload);
        let encoded = envelope.encode();
        self.send_ws_envelope(&encoded) || self.send_ipc_envelope(&encoded)
    }

    /// Push one Channel frame through the Native IPC bridge. Binary frames are
    /// encoded only at this transport boundary; API handlers keep passing
    /// their original bytes through `CallContext`.
    pub fn send_ipc_frame(
        self: &Arc<Self>,
        session_id: &str,
        call_id: u64,
        channel_seq: u64,
        frame: WsOut,
    ) -> bool {
        use base64::Engine;

        let frame = match frame {
            WsOut::Text(data) => serde_json::json!({ "t": "text", "data": data }),
            WsOut::Binary(bytes) => serde_json::json!({
                "t": "binary",
                "data": base64::engine::general_purpose::STANDARD.encode(bytes),
            }),
        };
        let message = serde_json::json!({
            "sessionId": session_id,
            "id": call_id,
            "seq": channel_seq,
            "frame": frame,
        });
        let Ok(encoded) = serialize_script_json(&message) else {
            return false;
        };
        self.send_ipc_script(format!(
            "if(typeof window.__niva_native_frame==='function')window.__niva_native_frame({encoded});"
        ))
    }

    /// Deliver the same event envelope through the local Native IPC bridge
    /// when no WebSocket connection is available. Evaluation is queued onto
    /// the event loop so callers may safely invoke this from the main thread.
    fn send_ipc_envelope(self: &Arc<Self>, envelope: &str) -> bool {
        let Ok(encoded) = serialize_script_json(envelope) else {
            return false;
        };
        self.send_ipc_script(format!(
            "if(typeof window.__niva_native_event==='function')window.__niva_native_event({encoded});"
        ))
    }

    fn send_ipc_script(self: &Arc<Self>, script: String) -> bool {
        if self.trusted_ws_origin.is_none() {
            return false;
        }
        let app = self.app.clone();
        let window_id = self.id;
        let expected_origin = self.trusted_ws_origin.clone().unwrap_or_default();
        let scheme = super::super::custom_protocol::scheme_for_uuid(&app.launch_info.uuid);
        let event = NivaEvent::new(move |_, _| {
            let window = app.window()?.get_window(window_id)?;
            let Ok(current_url) = window.webview.url() else {
                return Ok(());
            };
            let Ok(current_url) = url::Url::parse(&current_url) else {
                return Ok(());
            };
            let current_origin =
                super::super::custom_protocol::origin_from_page_url(&current_url, &scheme)
                    .unwrap_or_else(|| current_url.origin().ascii_serialization());
            if current_origin == expected_origin {
                window.webview.evaluate_script(&script)?;
            }
            Ok(())
        });
        self.app.event_loop_proxy.send_event(event).is_ok()
    }

    pub(crate) fn next_event_seq(&self) -> u64 {
        self.next_event_seq.fetch_add(1, Ordering::Relaxed)
    }
}

fn serialize_script_json<T: Serialize + ?Sized>(value: &T) -> anyhow::Result<String> {
    let encoded = serde_json::to_string(value)?;
    Ok(encoded
        .replace('\u{2028}', "\\u2028")
        .replace('\u{2029}', "\\u2029"))
}
