use anyhow::{Result, anyhow};
use std::sync::Arc;

use serde_json::json;
use tao::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoopWindowTarget},
    window::WindowId,
};

use crate::{lock, log_if_err, try_or_log_err};

use super::{NivaApp, NivaEvent, utils::split_id, window_manager::WindowManager};

pub struct EventHandler {
    app: Arc<NivaApp>,
}

impl EventHandler {
    pub fn new(app: Arc<NivaApp>) -> Self {
        Self { app }
    }

    /// Install global handlers for menu / tray-icon / hotkey events.
    /// Those crates are not tied to the tao event loop, so their events are
    /// forwarded to windows through the event-loop proxy (thread-safe).
    pub fn install_external_handlers(app: Arc<NivaApp>) {
        let menu_app = app.clone();
        muda::MenuEvent::set_event_handler(Some(move |event: muda::MenuEvent| {
            log_if_err!(Self::dispatch_menu_event(&menu_app, &event));
        }));

        let tray_app = app.clone();
        tray_icon::TrayIconEvent::set_event_handler(Some(
            move |event: tray_icon::TrayIconEvent| {
                log_if_err!(Self::dispatch_tray_event(&tray_app, event));
            },
        ));

        let hotkey_app = app.clone();
        global_hotkey::GlobalHotKeyEvent::set_event_handler(Some(
            move |event: global_hotkey::GlobalHotKeyEvent| {
                log_if_err!(Self::dispatch_hotkey_event(&hotkey_app, &event));
            },
        ));
    }

    fn dispatch_menu_event(app: &Arc<NivaApp>, event: &muda::MenuEvent) -> Result<()> {
        let merged_id: u16 = event
            .id()
            .0
            .parse()
            .map_err(|_| anyhow!("Invalid menu id"))?;
        let (window_id, id) = split_id(merged_id);
        let window = app.window()?.get_window(window_id)?;
        window.send_ipc_event("menu.clicked", id);
        Ok(())
    }

    fn dispatch_tray_event(app: &Arc<NivaApp>, event: tray_icon::TrayIconEvent) -> Result<()> {
        use tray_icon::{MouseButton, MouseButtonState, TrayIconEvent};
        match event {
            TrayIconEvent::Click {
                id,
                button,
                button_state,
                ..
            } => {
                if button_state != MouseButtonState::Up {
                    return Ok(());
                }
                let (window_id, tray_id) = app
                    .tray()?
                    .get_window_id_by_tray_id(id.as_ref())
                    .ok_or(anyhow!("Tray not found"))?;
                let window = app.window()?.get_window(window_id)?;
                match button {
                    MouseButton::Left => {
                        window.send_ipc_event("tray.leftClicked", json!(tray_id));
                    }
                    MouseButton::Right => {
                        window.send_ipc_event("tray.rightClicked", json!(tray_id));
                    }
                    _ => {}
                }
                Ok(())
            }
            TrayIconEvent::DoubleClick { id, .. } => {
                let (window_id, tray_id) = app
                    .tray()?
                    .get_window_id_by_tray_id(id.as_ref())
                    .ok_or(anyhow!("Tray not found"))?;
                let window = app.window()?.get_window(window_id)?;
                window.send_ipc_event("tray.doubleClicked", json!(tray_id));
                Ok(())
            }
            _ => Ok(()),
        }
    }

    fn dispatch_hotkey_event(
        app: &Arc<NivaApp>,
        event: &global_hotkey::GlobalHotKeyEvent,
    ) -> Result<()> {
        // Only key-press events carry an id we registered.
        let (window_id, id) = app
            .shortcut()?
            .lookup(event.id())
            .ok_or(anyhow!("Shortcut not found"))?;
        let window = app.window()?.get_window(window_id)?;
        window.send_ipc_event("shortcut.emit", id);
        Ok(())
    }

    pub fn handle(
        &self,
        event: Event<'_, NivaEvent>,
        target: &EventLoopWindowTarget<NivaEvent>,
        control_flow: &mut ControlFlow,
    ) {
        try_or_log_err!({
            *control_flow = ControlFlow::Wait;
            match event {
                Event::WindowEvent {
                    event, window_id, ..
                } => self.handle_window_event(event, window_id, control_flow)?,
                Event::UserEvent(callback) => {
                    self.handle_user_event(callback, target, control_flow)?
                }
                _ => (),
            }
            Ok(())
        });
    }

    fn handle_window_event(
        &self,
        event: WindowEvent<'_>,
        window_id: WindowId,
        control_flow: &mut ControlFlow,
    ) -> Result<()> {
        if let WindowEvent::Destroyed = event {
            let closed = self.app.window()?.close_window_inner(window_id)?;
            WindowManager::cleanup_window(&self.app, &closed)?;
        }

        let window = self.app.window()?.get_window_inner(window_id)?;
        match event {
            WindowEvent::Focused(focused) => {
                #[cfg(target_os = "macos")]
                {
                    let _ = focused;
                    window.switch_menu();
                }
                #[cfg(not(target_os = "macos"))]
                {
                    let _ = focused;
                }
                window.send_ipc_event("window.focused", focused);
            }
            WindowEvent::ScaleFactorChanged {
                scale_factor,
                new_inner_size,
            } => {
                window.send_ipc_event(
                    "window.scaleFactorChanged",
                    json!({
                        "scaleFactor": scale_factor,
                        "newInnerSize": new_inner_size
                    }),
                );
            }
            WindowEvent::ThemeChanged(theme) => {
                window.send_ipc_event(
                    "window.themeChanged",
                    match theme {
                        tao::window::Theme::Dark => "dark",
                        tao::window::Theme::Light => "light",
                        _ => "system",
                    },
                );
            }
            WindowEvent::CloseRequested => {
                let is_block_closed_requested = { lock!(window.state)?.is_block_closed_requested };
                if is_block_closed_requested {
                    window.send_ipc_event("window.closeRequested", json!(null));
                } else {
                    let closed = self.app.window()?.close_window_inner(window_id)?;
                    WindowManager::cleanup_window(&self.app, &closed)?;
                    if window.id == 0 {
                        *control_flow = ControlFlow::Exit;
                    }
                }
            }
            _ => (),
        }
        return Ok(());
    }

    fn handle_user_event(
        &self,
        callback: NivaEvent,
        target: &EventLoopWindowTarget<NivaEvent>,
        control_flow: &mut ControlFlow,
    ) -> Result<()> {
        callback(target, control_flow)
    }
}
