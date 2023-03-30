// remove console window in windows system
#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

mod app;
use anyhow::Result;
use app::{NivaApp, NivaEventLoop};

fn main() -> Result<()> {
    let mut event_loop = NivaEventLoop::with_user_event();
    let app = NivaApp::new(&mut event_loop)?;
    app.run(event_loop)
}