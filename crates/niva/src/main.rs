// remove console window in windows system
#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

mod app;
use anyhow::Result;
use app::{NivaApp};

fn main() -> Result<()> {
    // let mut event_loop = NivaEventLoop::with_user_event();
    // let app = NivaApp::new(&mut event_loop)?;
    // app.run(event_loop)
    // let arguments = NivaArguments::new()?;
    // println!("{:?}", arguments);
    // let mut target = json!({
    //     "hello": {
    //         "world": {
    //             "arr": [0, 1,2, { }]
    //         },
    //     }
    // });
    // set_json_value(&mut target, "hello.world.arr.3.test", json!(789))?;
    // println!("{}", serde_json::to_string_pretty(&target)?);

    let app = NivaApp::new()?;
    println!("{:?}", app.launch_info);

    Ok(())
}
