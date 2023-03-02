// remove console window in windows system
#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

mod env;
mod main_window;
mod static_server;
mod sys_api;
mod thread_pool;

use std::sync::{Arc, Mutex};
use thread_pool::ThreadPool;

fn main() {
    let env_result = env::init();
    if let Err(err) = env_result {
        println!("Init Environment Error: {:?}", err.to_string());
        return;
    }
    let (work_dir, config) = env_result.unwrap();
    println!("Init Environment Success");
    println!("work_dir: {:?}", work_dir);
    println!("config: {:?}", config);

    let workers = config.workers.unwrap_or(5);
    let thread_pool = Arc::new(Mutex::new(ThreadPool::new(workers)));
    println!("Init thread pool workers: {:?}", workers);

    let entry_url = static_server::start(&work_dir, &config, &thread_pool);
    println!("Static server started at {:?}", entry_url);

    println!("Open main window");
    main_window::open(&entry_url, &config, thread_pool, sys_api::call);
}
