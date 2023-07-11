// remove console window in windows system
#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

mod app;
mod utils;

use std::time::Duration;

use anyhow::Result;
use app::NivaApp;
use async_io::Timer;


fn main() -> Result<()> {
    smol::block_on(async {
        let app = NivaApp::new().await?;

        let name = "base";
        let resource = app.resource_manager.lock().await.get(name)?;
        println!("{} {}", name, resource.base_url());

        // let content = base.read("index.html", 0, 10).await?;
        // println!("{:?}", std::str::from_utf8(&content));

        // let mut event_loop = NivaEventLoop::with_user_event();
        // let app = NivaApp::new(&mut event_loop)?;
        // app.run(event_loop);
        Timer::after(Duration::from_secs(u64::MAX)).await;
        app.run()
    })
}
