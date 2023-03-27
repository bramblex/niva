// remove console window in windows system
#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

mod niva_app;

use anyhow::{Result};
use niva_app::{NivaApp, NivaEventLoop};

fn main() -> Result<()> {
    let event_loop = NivaEventLoop::with_user_event();
    let app = NivaApp::new(&event_loop)?;
    app.run(event_loop)
}
