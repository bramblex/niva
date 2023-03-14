// remove console window in windows system
#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use window_manager::EventLoop;
use wry::application::{
    event::{Event, WindowEvent},
    event_loop::ControlFlow,
};

mod utils;
mod thread_pool;
mod static_server;
mod window_manager;
mod api_manager;

fn main() {
    let event_loop = EventLoop::with_user_event();

    let mut window_manager = window_manager::WindowManager::new(event_loop.create_proxy());
    let main_window_id = window_manager.create_window(&event_loop, &Default::default());

    event_loop.run(move |event, target, control_flow| {
        *control_flow = ControlFlow::Wait;
        match event {
            Event::WindowEvent {
                window_id,
                event: WindowEvent::CloseRequested,
                ..
            } => { 
                if window_id == main_window_id {
                    *control_flow = ControlFlow::Exit;
                } else {
                    window_manager.remove_window(window_id);
                }
            }
            Event::UserEvent(t) => match t.as_str() {
                "open" => {
                    window_manager.create_window(target, &Default::default());
                }
                _ => (),
            },
            _ => (),
        }
    });
}
