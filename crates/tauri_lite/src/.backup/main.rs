// remove console window in windows system
#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

mod api;
mod api_manager;
mod environment;
mod event;
mod main_window;
mod static_server;
mod thread_pool;
mod window_manager;

use std::sync::{Arc, Mutex};
use thread_pool::ThreadPool;

use crate::{event::MainEventLoop, api_manager::ApiManager};

/*
   Environment

   StaticServer(env, task_runner)

   ApiManager
       .new(task_runner, env)
       .register_api(method, ApiInstance)
       .call(method, ApiRequest)

       ApiInstance(
           handle_rpc: (env, window, ipc_message) -> UserEvent((webview) => {}),
           handle_event: (webview, UserEvent) -> (),
       )

   WindowManager
       ::new(env, api_manager)
       .create_window(window_options, entry_url)
       .get_window(window_id)
       .close_window(window_id)
       .run_event_loop()
*/

fn main() {
    let env_result = environment::init();
    if let Err(err) = env_result {
        println!("Init Environment Error: {:?}", err.to_string());
        return;
    }
    let env = env_result.unwrap();
    let main_event_loop = MainEventLoop::new();

    println!("Init Environment Success");
    println!("work_dir: {:?}", env.work_dir);
    println!("config: {:?}", env.config);

    let workers = env.config.workers.unwrap_or(5);
    let thread_pool = Arc::new(Mutex::new(ThreadPool::new(workers)));
    println!("Init thread pool workers: {:?}", workers);

    let api_manager = ApiManager::new(env.clone(), thread_pool.clone(), main_event_loop.create_proxy());

    let entry_url: String = static_server::start(env.clone(), thread_pool.clone());
    println!("Static server started at {:?}", entry_url);

    println!("Open main window");
    let debug_entry_url = env.debug_entry_url.clone();
    main_window::open(
        env,
        debug_entry_url.unwrap_or(entry_url),
        main_event_loop,
        api_manager,
    );
}
