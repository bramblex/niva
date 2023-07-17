// remove console window in windows system
#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]
#![feature(coerce_unsized)]
#![feature(unsize)]

mod app;
mod utils;

use std::{ops::Deref, time::Duration};

use anyhow::Result;
use app::{resource::NivaResourceManager, NivaApp};
use async_io::Timer;

use crate::app::{event::NivaEventLoop, window::NivaWindow};

fn main() -> Result<()> {
    let event_loop = NivaEventLoop::with_user_event();
    let app = NivaApp::new()?;

    // let m = app.get_manager::<NivaResourceManager>().unwrap();
    // lock!(m, {
    //     let m = m.as_any().downcast_mut::<NivaResourceManager>().unwrap();
    //     let a = m.transfer_url("awef");
    // });

    with_manager!(app, m0: NivaResourceManager, {
       println!("{:?}", m0.transfer_url("123"));
    });

    app.run(event_loop)

    // smol::block_on(async {
    // let window = NivaWindow::new(
    //     app.clone(),
    //     &event_loop,
    //     resource.base_url().to_string(),
    // ).unwrap();

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
    // });
}
