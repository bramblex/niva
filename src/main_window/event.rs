use serde_json::json;
use wry::application::{
    event::{Event, WindowEvent},
    event_loop::ControlFlow,
};

use super::WebviewWarper;

#[derive(Debug)]
pub enum EventContent {
    Event(String, serde_json::Value),
    Callback(String),
}

fn send_callback(main_webview_warper: &WebviewWarper, response: String) {
    main_webview_warper
        .0
        .evaluate_script(&format!("TauriLite.__resolve__({response})"))
        .unwrap();
}

fn send_event(main_webview_warper: &WebviewWarper, event: &str, data: serde_json::Value) {
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

        Event::UserEvent(content) => match content {
            EventContent::Callback(response) => send_callback(main_webview_warper, response),
            EventContent::Event(event, data) => send_event(main_webview_warper, &event, data),
        },

        _ => (),
    }
}
