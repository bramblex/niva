// remove console window in windows system
#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

mod api;
mod environment;
mod main_window;
mod static_server;
mod thread_pool;

use std::sync::{Arc, Mutex};
use thread_pool::ThreadPool;

fn main() {
    let env_result = environment::init();
    if let Err(err) = env_result {
        println!("Init Environment Error: {:?}", err.to_string());
        return;
    }
    let env = env_result.unwrap();
    println!("Init Environment Success");
    println!("work_dir: {:?}", env.work_dir);
    println!("config: {:?}", env.config);

    let workers = env.config.workers.unwrap_or(5);
    let thread_pool = Arc::new(Mutex::new(ThreadPool::new(workers)));
    println!("Init thread pool workers: {:?}", workers);

    let entry_url: String = static_server::start(env.clone(), thread_pool.clone());
    println!("Static server started at {:?}", entry_url);

    println!("Open main window");
    let debug_entry_url = env.debug_entry_url.clone();
    main_window::open(
        env,
        debug_entry_url.unwrap_or(entry_url),
        thread_pool,
        api::call,
    );
}

// use tauri_lite_lib::tl_api;

// struct ApiRequest {
//     pub callback_id: u32,
//     pub name: String,
//     pub method: String,
//     pub args: Vec<serde_json::Value>,
// }

// struct ApiResponse {
//     pub callback_id: u32,
//     pub code: i32,
//     pub msg: String,
//     pub data: serde_json::Value,
// }

// #[derive(Debug)]
// struct ApiError(i32, String);

// #[tl_api]
// fn add(a: u32, b: u32, c: Option<u32>) -> Result<u32, String> {
//     Ok(a + b + c.unwrap_or(0))
// }

// fn main() {
//     use serde_json::json;

//     let res = add(ApiRequest {
//         callback_id: 0,
//         name: "add".to_string(),
//         method: "add".to_string(),
//         args: vec![json!(1), json!(2)],
//     });

//     println!("{:?}", res.data);
// }
