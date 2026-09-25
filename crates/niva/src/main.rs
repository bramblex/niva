// remove console window in windows system
#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]
// Deadlock discipline:std Mutex guards must never cross an `.await`.
// api_manager reads (handler map) are lock-free after init.
#![deny(clippy::await_holding_lock)]

mod app;
use anyhow::Result;
use app::{NivaApp, NivaEvent, logging};
use tao::event_loop::EventLoopBuilder;

fn main() {
    init_bootstrap_logger();
    std::panic::set_hook(Box::new(|panic| {
        crate::niva_log!(logging::Level::Error, "panic: {panic}");
    }));

    if let Err(error) = run() {
        // Never write framework diagnostics to a user's stdout/stderr. NivaApp
        // switches the logger to the validated full-UUID data directory once
        // configuration has been parsed; this bootstrap file covers earlier
        // startup failures that have no valid app identity yet.
        crate::niva_log!(logging::Level::Error, "startup failed: {error:#}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let mut event_loop = EventLoopBuilder::<NivaEvent>::with_user_event().build();
    let app = NivaApp::new(&mut event_loop)?;
    app.run(event_loop)
}

fn init_bootstrap_logger() {
    let data_dir = directories::BaseDirs::new().map(|base_dirs| base_dirs.data_dir().join("niva"));
    if data_dir
        .as_deref()
        .is_some_and(|path| logging::init(path).is_ok())
    {
        return;
    }

    // Covers account/profile initialization failures and unwritable app-data
    // folders. If even the system temp directory is unwritable, file logging
    // remains unavailable rather than falling back to the process streams.
    let _ = logging::init(&std::env::temp_dir().join("niva-bootstrap"));
}
