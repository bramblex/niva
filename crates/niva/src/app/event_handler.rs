use anyhow::Result;
use std::sync::Arc;

use anyhow::anyhow;
use serde_json::json;
use tao::{
    accelerator::AcceleratorId,
    event::{Event, TrayEvent, WindowEvent},
    event_loop::{ControlFlow, EventLoopWindowTarget},
    menu::MenuId,
    window::WindowId,
    TrayId,
};

use crate::{log_if_err, try_or_log_err};

use super::{window_manager::window::NivaWindow, NivaApp, NivaEvent};

pub struct EventHandler {
    app: Arc<NivaApp>,
}

impl EventHandler {
    pub fn new(app: Arc<NivaApp>) -> Self {
        Self { app }
    }

    pub fn handle(
        &self,
        event: Event<NivaEvent>,
        target: &EventLoopWindowTarget<NivaEvent>,
        control_flow: &mut ControlFlow,
    ) {
        try_or_log_err!({
            *control_flow = ControlFlow::Wait;
            match event {
                Event::WindowEvent {
                    event, window_id, ..
                } => self.handle_window_event(event, window_id)?,
                Event::UserEvent(callback) => {
                    self.handle_user_event(callback, target, control_flow)?
                }
                Event::MenuEvent {
                    menu_id, window_id, ..
                } => self.handle_menu_event(menu_id, window_id)?,
                Event::TrayEvent { event, id, .. } => self.handle_tray_event(event, id)?,
                Event::GlobalShortcutEvent(id) => self.handle_shortcut_event(id)?,
                _ => (),
            }
            Ok(())
        });
    }

    fn handle_window_event(
        &self,
        event: WindowEvent,
        window_id: WindowId,
    ) -> Result<()> {

        match event {
            WindowEvent::Destroyed => {
                self.app.window()?.close_window_inner(window_id)?;
            }
            _ => ()
        }

        let window = self.app.window()?.get_window_inner(window_id)?;
        match event {
            WindowEvent::Focused(focused) => {
                #[cfg(target_os = "macos")]
                window.switch_menu();
                window.send_ipc_event("window.focused", focused)?;
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
                )?;
            }
            WindowEvent::ThemeChanged(theme) => {
                window.send_ipc_event(
                    "window.themeChanged",
                    json!({
                        "theme": match theme {
                            tao::window::Theme::Dark => "dark",
                            tao::window::Theme::Light => "light",
                            _ => "",
                        }
                    }),
                )?;
            }
            WindowEvent::CloseRequested => {
                window.send_ipc_event("window.closeRequested", json!(null))?;
            }
            _ => (),
        }
        return Ok(());
    }

    fn handle_menu_event(&self, menu_id: MenuId, window_id: Option<WindowId>) -> Result<()> {
        let window = self
            .app
            .window()?
            .get_window_inner(window_id.ok_or(anyhow!("Window id not found!"))?)?;

        window.send_ipc_event("menu.clicked", menu_id.0)
    }

    fn handle_tray_event(&self, event: TrayEvent, id: TrayId) -> Result<()> {
        let id = id.0;
        let (window_id, _) = self.app.tray()?.get(id)?.clone();
        let window = self.app.window()?.get_window(window_id)?;

        match event {
            TrayEvent::RightClick => window.send_ipc_event("tray.rightClicked", json!(id)),
            TrayEvent::LeftClick => window.send_ipc_event("tray.leftClicked", json!(id)),
            TrayEvent::DoubleClick => window.send_ipc_event("tray.doubleClicked", json!(id)),
            _ => Ok(()),
        }
    }

    fn handle_shortcut_event(&self, id: AcceleratorId) -> Result<()> {
        let id = id.0;
        let (window_id, _, _) = self.app.shortcut()?.get(id)?.clone();
        let window = self.app.window()?.get_window(window_id)?;

        window.send_ipc_event("shortcut.emit", id)
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
