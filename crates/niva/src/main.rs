// remove console window in windows system
#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

mod app;
use anyhow::Result;
use app::{NivaApp, NivaEvent};
use tao::event_loop::EventLoopBuilder;

fn main() -> Result<()> {
    let mut event_loop = EventLoopBuilder::<NivaEvent>::with_user_event().build();
    let app = NivaApp::new(&mut event_loop)?;
    app.run(event_loop)
}
