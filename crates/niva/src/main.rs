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

use crate::app::{event::NivaEventLoop, window::NivaWindow};

fn main() {
    smol::block_on(async {
        let event_loop = NivaEventLoop::with_user_event();
        let app = NivaApp::new().await.unwrap();

        let name = "base";
        let rm = app.resource_manager.lock().await;
        let resource = rm.get(name).unwrap();
        println!("{} {}", name, resource.clone().base_url());

        println!("{}", rm.transfer_url("").unwrap());
        println!("{}", rm.transfer_url("index.html").unwrap());
        println!(
            "{}",
            rm.transfer_url("http://base.resource.niva/index.html")
                .unwrap()
        );
        println!(
            "{}",
            rm.transfer_url("http://aaa.bbb.ccc/index.html").unwrap()
        );

        let window = NivaWindow::new(
            app.clone(),
            &event_loop,
            resource.base_url().to_string(),
        ).unwrap();

        // smol::spawn(async {
        //     let mut i = 0;
        //     loop {
        //         i += 1;
        //         Timer::after(Duration::from_secs(1)).await;
        //         println!("{}", i);
        //     }
        // })
        // .detach();

        // let base_url = url::Url::parse("http://aaa.bb.ccc/")?;
        // let r = base_url.join("http://ccc.ddd.eee/hello/world")?;

        // let content = base.read("index.html", 0, 10).await?;
        // println!("{:?}", std::str::from_utf8(&content));

        // let mut event_loop = NivaEventLoop::with_user_event();
        // let app = NivaApp::new(&mut event_loop)?;
        // app.run(event_loop);
        // Timer::after(Duration::from_secs(u64::MAX)).await;
        event_loop.run(|_, _, _| {});
    });
}
