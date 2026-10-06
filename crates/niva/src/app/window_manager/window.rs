use crate::lock_force;

use anyhow::Result;
use std::{
    ops::Deref,
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
    },
};

use serde::Serialize;
#[cfg(target_os = "windows")]
use tao::platform::windows::WindowExtWindows;
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
    #[cfg(target_os = "windows")]
    has_parent_window: bool,

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

        let niva_window = arc(Self {
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
            #[cfg(target_os = "windows")]
            has_parent_window: options
                .windows_extra
                .as_ref()
                .is_some_and(|extra| extra.parent_window.is_some()),
            next_ws_id: AtomicU64::new(1),
            next_event_seq: AtomicU64::new(1),

            state: Mutex::new(NivaWindowState {
                is_block_closed_requested: false,
                is_menu_visible: true,
            }),
        });
        #[cfg(target_os = "windows")]
        niva_window.register_initial_menu_accelerator();
        Ok(niva_window)
    }

    #[cfg(not(target_os = "windows"))]
    fn attach_built_menu(&self, menu: &muda::Menu) -> Result<()> {
        NivaBuilder::attach_menu(&self.window, menu)?;
        *lock_force!(self.menu_handle) = Some(menu.clone());
        Ok(())
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
            crate::log_if_err!(self.attach_built_menu(&menu));
        }
    }

    pub fn set_menu(self: &Arc<Self>, options: &Option<WindowMenuOptions>) -> Result<()> {
        #[cfg(target_os = "windows")]
        if self.has_parent_window && options.is_some() {
            anyhow::bail!("Windows child windows do not support menus");
        }

        *lock_force!(self.menu_options) = options.clone();
        let visible = lock_force!(self.state).is_menu_visible;

        #[cfg(target_os = "windows")]
        {
            // Windows owns a separate menu for every HWND, so an unfocused
            // window must receive its updated menu immediately as well.
            if visible {
                let menu_options = lock_force!(self.menu_options).clone();
                return self.replace_windows_menu(
                    NivaBuilder::build_menu(self.id, &self.app, &menu_options),
                    visible,
                );
            }
            return Ok(());
        }

        #[cfg(not(target_os = "windows"))]
        {
            if self.is_focused() && visible {
                let menu_options = lock_force!(self.menu_options).clone();
                if let Some(menu) = NivaBuilder::build_menu(self.id, &self.app, &menu_options) {
                    self.attach_built_menu(&menu)?;
                }
            }
            Ok(())
        }
    }

    pub fn show_menu(self: &Arc<Self>) -> Result<()> {
        #[cfg(target_os = "windows")]
        {
            let menu_options = lock_force!(self.menu_options).clone();
            return self.replace_windows_menu(
                NivaBuilder::build_menu(self.id, &self.app, &menu_options),
                true,
            );
        }

        #[cfg(not(target_os = "windows"))]
        {
            lock_force!(self.state).is_menu_visible = true;
            let menu_options = lock_force!(self.menu_options).clone();
            if let Some(menu) = NivaBuilder::build_menu(self.id, &self.app, &menu_options) {
                self.attach_built_menu(&menu)?;
            }
            Ok(())
        }
    }

    pub fn hide_menu(self: &Arc<Self>) -> Result<()> {
        #[cfg(target_os = "windows")]
        {
            return self.replace_windows_menu(None, false);
        }

        #[cfg(not(target_os = "windows"))]
        {
            lock_force!(self.state).is_menu_visible = false;
            let handle = lock_force!(self.menu_handle).take();
            if let Some(menu) = handle {
                NivaBuilder::detach_menu(&self.window, &menu)?;
            }
            Ok(())
        }
    }

    #[cfg(target_os = "windows")]
    fn replace_windows_menu(
        &self,
        next: Option<muda::Menu>,
        requested_visible: bool,
    ) -> Result<()> {
        let mut current = {
            let mut handle = lock_force!(self.menu_handle);
            std::mem::take(&mut *handle)
        };
        let mut visible = lock_force!(self.state).is_menu_visible;
        let result = replace_menu_handle(
            &mut current,
            next,
            &mut visible,
            requested_visible,
            |menu| NivaBuilder::detach_menu(&self.window, menu),
            |menu| NivaBuilder::attach_menu(&self.window, menu),
        );
        *lock_force!(self.menu_handle) = current;
        lock_force!(self.state).is_menu_visible = visible;
        self.refresh_menu_accelerator();
        result
    }

    #[cfg(target_os = "windows")]
    fn refresh_menu_accelerator(&self) {
        let haccel = {
            lock_force!(self.menu_handle)
                .as_ref()
                .map(muda::Menu::haccel)
        };
        crate::app::set_window_menu_accelerator(self.window.hwnd(), haccel);
    }

    /// Release the HWND's menu while still on the UI thread. Outstanding API
    /// calls may keep `NivaWindow` alive after it leaves the manager, so the
    /// thread-bound Muda handle cannot be left for `NivaWindow::drop`.
    #[cfg(target_os = "windows")]
    pub(crate) fn release_menu_on_main(&self) -> Result<()> {
        use windows::Win32::{Foundation::HWND, UI::WindowsAndMessaging::IsWindow};

        let hwnd = self.window.hwnd();
        let Some(menu) = lock_force!(self.menu_handle).take() else {
            crate::app::remove_window_menu_accelerator(hwnd);
            lock_force!(self.state).is_menu_visible = false;
            return Ok(());
        };

        // WM_NCDESTROY makes Muda remove the HWND from its own menu registry.
        // In that case dropping the menu now is safe and stays on this thread.
        let native_window_alive = unsafe { IsWindow(Some(HWND(hwnd as _))).as_bool() };
        if native_window_alive && let Err(error) = NivaBuilder::detach_menu(&self.window, &menu) {
            let haccel = menu.haccel();
            *lock_force!(self.menu_handle) = Some(menu);
            crate::app::set_window_menu_accelerator(hwnd, Some(haccel));
            return Err(error);
        }

        crate::app::remove_window_menu_accelerator(hwnd);
        drop(menu);
        lock_force!(self.state).is_menu_visible = false;
        Ok(())
    }

    #[cfg(target_os = "windows")]
    fn register_initial_menu_accelerator(&self) {
        self.refresh_menu_accelerator();
    }

    pub fn is_menu_visible(self: &Arc<Self>) -> bool {
        lock_force!(self.state).is_menu_visible
    }

    /// Allocate a unique ID for a WebSocket connection scoped to this window.
    /// The HTTP server pump retains the sender itself.
    pub fn next_ws_connection_id(&self) -> u64 {
        self.next_ws_id.fetch_add(1, Ordering::Relaxed)
    }

    /// Push a small window event over the stable IPC/evaluate_script bridge.
    pub fn send_ipc_event<E: Into<String>, P: Serialize>(
        self: &Arc<Self>,
        event: E,
        payload: P,
    ) -> bool {
        let payload = serde_json::to_value(payload).unwrap_or(serde_json::Value::Null);
        let seq = self.next_event_seq();
        let envelope = ServerMsg::event(None, seq, event.into(), payload);
        let encoded = envelope.encode();
        self.send_ipc_envelope(&encoded)
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

    /// Deliver an event envelope through the stable local Native IPC bridge.
    /// Evaluation is queued onto the event loop so callers may safely invoke
    /// this from the main thread.
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

#[cfg(any(target_os = "windows", test))]
fn replace_menu_handle<T>(
    current: &mut Option<T>,
    next: Option<T>,
    visible: &mut bool,
    requested_visible: bool,
    detach: impl FnOnce(&T) -> Result<()>,
    attach: impl FnOnce(&T) -> Result<()>,
) -> Result<()> {
    let result = (|| {
        if let Some(old) = current.take() {
            if let Err(error) = detach(&old) {
                *current = Some(old);
                return Err(error);
            }
            // Muda's Windows Drop removes its menu from every attached HWND. Drop
            // the old handle only after explicit detach, before attaching the new
            // handle, so its destructor cannot clear the replacement menu.
            drop(old);
        }

        if let Some(next) = next {
            // Muda's Windows initializer fails before changing the HWND when it
            // reports an error, so an unattached candidate can be dropped safely.
            attach(&next)?;
            *current = Some(next);
        }
        Ok(())
    })();
    // An empty menu does not change the visibility preference. A later
    // setMenu(Some(...)) must still attach when the menu was visible.
    *visible = if result.is_ok() {
        requested_visible
    } else {
        current.is_some()
    };
    result
}

#[cfg(test)]
mod menu_lifecycle_tests {
    use super::replace_menu_handle;
    use anyhow::anyhow;
    use std::sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    };

    struct FakeMenu {
        id: &'static str,
        hwnd_menu: Arc<Mutex<Option<&'static str>>>,
        operations: Arc<Mutex<Vec<String>>>,
        attached: Arc<AtomicBool>,
    }

    impl Drop for FakeMenu {
        fn drop(&mut self) {
            self.operations
                .lock()
                .unwrap()
                .push(format!("drop:{}", self.id));
            if self.attached.swap(false, Ordering::SeqCst) {
                // Muda drops its HWND registration by unconditionally calling
                // SetMenu(hwnd, NULL); an old registration can therefore
                // clear a replacement menu unless it was explicitly detached.
                *self.hwnd_menu.lock().unwrap() = None;
            }
        }
    }

    fn fake_menu(
        id: &'static str,
        hwnd_menu: &Arc<Mutex<Option<&'static str>>>,
        operations: &Arc<Mutex<Vec<String>>>,
        attached: bool,
    ) -> FakeMenu {
        FakeMenu {
            id,
            hwnd_menu: hwnd_menu.clone(),
            operations: operations.clone(),
            attached: Arc::new(AtomicBool::new(attached)),
        }
    }

    #[test]
    fn replacement_detaches_and_drops_old_menu_before_attaching_new_menu() {
        let hwnd_menu = Arc::new(Mutex::new(Some("old")));
        let operations = Arc::new(Mutex::new(Vec::new()));
        let mut current = Some(fake_menu("old", &hwnd_menu, &operations, true));
        let next = fake_menu("new", &hwnd_menu, &operations, false);

        replace_menu_handle(
            &mut current,
            Some(next),
            &mut true,
            true,
            |menu| {
                operations
                    .lock()
                    .unwrap()
                    .push(format!("detach:{}", menu.id));
                menu.attached.store(false, Ordering::SeqCst);
                let mut native = hwnd_menu.lock().unwrap();
                *native = None;
                Ok(())
            },
            |menu| {
                operations
                    .lock()
                    .unwrap()
                    .push(format!("attach:{}", menu.id));
                menu.attached.store(true, Ordering::SeqCst);
                *hwnd_menu.lock().unwrap() = Some(menu.id);
                Ok(())
            },
        )
        .unwrap();

        assert_eq!(*hwnd_menu.lock().unwrap(), Some("new"));
        assert_eq!(
            *operations.lock().unwrap(),
            ["detach:old", "drop:old", "attach:new"]
        );
        drop(current);
        assert_eq!(*hwnd_menu.lock().unwrap(), None);
    }

    #[test]
    fn clearing_menu_preserves_visibility_for_the_next_menu() {
        let mut current = Some("old");
        let mut visible = true;
        replace_menu_handle(
            &mut current,
            None,
            &mut visible,
            true,
            |_| Ok(()),
            |_| Ok(()),
        )
        .unwrap();
        assert!(visible);
        assert!(current.is_none());
        replace_menu_handle(
            &mut current,
            Some("new"),
            &mut visible,
            true,
            |_| Ok(()),
            |_| Ok(()),
        )
        .unwrap();
        assert!(visible);
        assert_eq!(current, Some("new"));
        replace_menu_handle(
            &mut current,
            None,
            &mut visible,
            false,
            |_| Ok(()),
            |_| Ok(()),
        )
        .unwrap();
        assert!(!visible);
    }

    #[test]
    fn failed_detach_retains_the_attached_handle_and_does_not_attach_candidate() {
        let hwnd_menu = Arc::new(Mutex::new(Some("old")));
        let operations = Arc::new(Mutex::new(Vec::new()));
        let mut current = Some(fake_menu("old", &hwnd_menu, &operations, true));
        let next = fake_menu("new", &hwnd_menu, &operations, false);

        let result = replace_menu_handle(
            &mut current,
            Some(next),
            &mut true,
            true,
            |_| Err(anyhow!("detach failed")),
            |_| {
                operations.lock().unwrap().push("unexpected attach".into());
                Ok(())
            },
        );

        assert!(result.is_err());
        assert_eq!(current.as_ref().map(|menu| menu.id), Some("old"));
        assert_eq!(*hwnd_menu.lock().unwrap(), Some("old"));
        assert!(
            !operations
                .lock()
                .unwrap()
                .iter()
                .any(|operation| operation == "unexpected attach")
        );
    }

    #[test]
    fn failed_attach_leaves_no_owned_handle_or_native_menu() {
        let hwnd_menu = Arc::new(Mutex::new(Some("old")));
        let operations = Arc::new(Mutex::new(Vec::new()));
        let mut current = Some(fake_menu("old", &hwnd_menu, &operations, true));
        let next = fake_menu("new", &hwnd_menu, &operations, false);

        let result = replace_menu_handle(
            &mut current,
            Some(next),
            &mut true,
            true,
            |menu| {
                operations
                    .lock()
                    .unwrap()
                    .push(format!("detach:{}", menu.id));
                menu.attached.store(false, Ordering::SeqCst);
                *hwnd_menu.lock().unwrap() = None;
                Ok(())
            },
            |_| Err(anyhow!("attach failed before native mutation")),
        );

        assert!(result.is_err());
        assert!(current.is_none());
        assert_eq!(*hwnd_menu.lock().unwrap(), None);
        assert_eq!(
            *operations.lock().unwrap(),
            ["detach:old", "drop:old", "drop:new"]
        );
    }
}

fn serialize_script_json<T: Serialize + ?Sized>(value: &T) -> anyhow::Result<String> {
    let encoded = serde_json::to_string(value)?;
    Ok(encoded
        .replace('\u{2028}', "\\u2028")
        .replace('\u{2029}', "\\u2029"))
}
