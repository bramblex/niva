use serde_json::json;
use wry::application::{
    event::{Event, WindowEvent},
    event_loop::ControlFlow,
    window::Theme,
};

use super::WebviewWarper;

#[derive(Debug)]
pub struct EventContent(pub String, pub serde_json::Value);

impl EventContent {
    pub fn new<E, D>(event: E, data: D) -> Self
    where
        E: Into<String>,
        D: Into<serde_json::Value>,
    {
        Self(event.into(), data.into())
    }
}

fn send_event<S>(main_webview_warper: &WebviewWarper, event: S, data: serde_json::Value)
where
    S: Into<String>,
{
    let event = event.into();
    let data_str = serde_json::to_string::<serde_json::Value>(&data).unwrap();
    main_webview_warper
        .0
        .evaluate_script(&format!("TauriLite.__emit__(\"{event}\", {data_str})"))
        .unwrap();
}

pub fn handle_window_event(
    main_webview_warper: &WebviewWarper,
    event: &WindowEvent,
    control_flow: &mut ControlFlow,
) {
    match event {
        WindowEvent::Focused(focused) => send_event(
            main_webview_warper,
            "window.focused",
            json!({ "focused": focused }),
        ),
        WindowEvent::ScaleFactorChanged {
            scale_factor,
            new_inner_size,
        } => send_event(
            main_webview_warper,
            "window.scaleFactorChanged",
            json!({ "scaleFactor": scale_factor, "newInnerSize": new_inner_size }),
        ),
        WindowEvent::ThemeChanged(theme) => send_event(
            main_webview_warper,
            "window.themeChanged",
            json!({ "theme": match theme {
                Theme::Dark => "dark",
                Theme::Light => "light",
                _ => "",
            }}),
        ),
        WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
        _ => (),
    }
}

pub fn handle(
    main_webview_warper: &WebviewWarper,
    event: Event<EventContent>,
    control_flow: &mut ControlFlow,
) {
    *control_flow = ControlFlow::Wait;

    // TODO: Send other events to webview
    match event {
        Event::NewEvents(_) => (), //

        Event::WindowEvent { event, .. } => {
            handle_window_event(main_webview_warper, &event, control_flow)
        }

        Event::MenuEvent { menu_id, .. } => send_event(
            main_webview_warper,
            "menu.click",
            json!({ "menu_id": menu_id.0 }),
        ),

        Event::UserEvent(EventContent(event, data)) => {
            send_event(main_webview_warper, event, data);
        }

        _ => (),
    }
}
