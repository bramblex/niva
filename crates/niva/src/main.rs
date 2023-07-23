// remove console window in windows system
#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

mod app;
mod utils;

use anyhow::Result;
use app::NivaApp;


fn main() -> Result<()> {
    let mut event_loop = NivaApp::create_event_loop();
    let app = NivaApp::new(&mut event_loop)?;
    app.run(event_loop)
}
