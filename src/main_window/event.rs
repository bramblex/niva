use wry::application::{
    event::{Event, WindowEvent},
    event_loop::ControlFlow,
};

use super::WebviewWarper;

pub type UserEventContent = String;

fn send_callback(main_webview_warper: &WebviewWarper, response: String) {
    main_webview_warper
        .0
        .evaluate_script(&format!("TauriLite.__resolve__({response})"))
        .unwrap();
}

// fn send_event(main_webview_warper: &WebviewWarper, event: &str, data: serde_json::Value) {
//     let data_str = serde_json::to_string::<serde_json::Value>(&data).unwrap();
//     main_webview_warper
//         .0
//         .evaluate_script(&format!("TauriLite.__emit__({event}, {data_str})"))
//         .unwrap();
// }

pub fn handle(
    main_webview_warper: &WebviewWarper,
    event: Event<String>,
    control_flow: &mut ControlFlow,
) {
    *control_flow = ControlFlow::Wait;

		// TODO: Send other events to webview
    match event {
        Event::UserEvent(response) => {
            send_callback(&main_webview_warper, response)
        }
        Event::WindowEvent {
            event: WindowEvent::CloseRequested,
            ..
        } => *control_flow = ControlFlow::Exit,
        _ => (),
    }
}
