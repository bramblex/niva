// remove console window in windows system
#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use crate::{event_loop::{event_handler::handle, MainEventLoop}};
use anyhow::{Context, Result};
use std::sync::{Arc, Mutex};

mod api_manager;
mod apis;
mod environment;
mod event_loop;
mod resource_manager;
mod static_server;
mod thread_pool;
mod window_manager;

#[cfg(target_os = "windows")]
mod win_utils;

fn main() -> Result<()> {
    let env = environment::init().with_context(|| "Init EnvironmentError")?;

    println!("Init Environment Success");
    println!("{:?}", env);

    let thread_pool = Arc::new(Mutex::new(thread_pool::ThreadPool::new(4)));
    let event_loop = MainEventLoop::new();

    let mut api_manager =
        api_manager::ApiManager::new(env.clone(), thread_pool.clone(), event_loop.create_proxy());

    apis::register_apis(&mut api_manager);

    let base_url = static_server::start(env.resource.clone(), thread_pool);
    let mut window_manager = window_manager::WindowManager::new(
        env.clone(),
        base_url,
        api_manager,
        event_loop.create_proxy(),
    );

    let (_, main_webview) = window_manager.create_window(&event_loop, &env.options.window);

    let event_loop = event_loop.0;
    event_loop.run(move |event, target, control_flow| {
        handle(main_webview.clone(), event, target, control_flow)
    });
}
