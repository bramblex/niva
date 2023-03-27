use std::sync::Arc;

use anyhow::anyhow;
use serde_json::json;
use tao::{
    event::{Event, WindowEvent, TrayEvent},
    event_loop::{ControlFlow, EventLoopWindowTarget},
};

use super::{window_manager::niva_window::NivaWindow, NivaApp, NivaEvent};

pub struct EventHandler {
    app: Arc<NivaApp>,
    main_window: Arc<NivaWindow>,
}

impl EventHandler {
    pub fn new(app: Arc<NivaApp>, main_window: Arc<NivaWindow>) -> Self {
        Self { app, main_window }
    }

    pub fn handle(
        &self,
        event: Event<NivaEvent>,
        target: &EventLoopWindowTarget<NivaEvent>,
        control_flow: &mut ControlFlow,
    ) {
        *control_flow = ControlFlow::Wait;

        // TODO: Send other events to webview
        match event {
            Event::NewEvents(_) => (), //

            Event::WindowEvent {
                event, window_id, ..
            } => {
                let window = self.app.get_window_inner(window_id).unwrap();

                match event {
                    WindowEvent::Focused(focused) => {
                        window.send_ipc_event("window.focused", focused);
                    },
                    WindowEvent::ScaleFactorChanged { scale_factor, new_inner_size } => {
                        window.send_ipc_event(
                            "window.scaleFactorChanged",
                            json!({
                                "scaleFactor": scale_factor,
                                "newInnerSize": new_inner_size
                            }),
                        );
                    },
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
                        );
                    },
                    WindowEvent::CloseRequested => {
                        if window.id == self.main_window.id {
                            *control_flow = ControlFlow::Exit;
                        } else {
                            self.app.close_window(window.id);
                        }
                    },
                    _ => (),
                }
            }

            Event::MenuEvent {
                menu_id, window_id, ..
            } => {
                let window = window_id
                    .ok_or(anyhow!("Window id not founc."))
                    .and_then(|window_id| self.app.get_window_inner(window_id));

                match window {
                    Ok(window) => {
                        window.send_ipc_event("menu.clicked", menu_id.0);
                    }
                    Err(_) => {
                        self.main_window.send_ipc_event("menu.clicked", menu_id.0);
                    }
                }
            }

            Event::TrayEvent { event, .. } => match event {
                TrayEvent::RightClick => {
                    self.main_window.send_ipc_event("tray.rightClicked", "");
                },
                TrayEvent::LeftClick => {
                    self.main_window.send_ipc_event("tray.leftClicked", "");
                },
                TrayEvent::DoubleClick => {
                    self.main_window.send_ipc_event("tray.doubleClicked", "");
                },
                _ => ()
            },

            Event::UserEvent(callback) => {
                let result = callback(target, control_flow);
                match result {
                    Ok(_) => (),
                    Err(err) => {
                        self.main_window
                            .send_ipc_event("ipc.error", err.to_string());
                    }
                }
            }

            _ => (),
        }
    }
}
