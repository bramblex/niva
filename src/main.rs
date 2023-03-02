use std::sync::{Arc, Mutex};
use wry::webview;

pub mod api;
mod config;
mod static_server;
mod thread_pool;
mod window;

fn get_work_dir() -> std::path::PathBuf {
    let cwd = std::env::current_dir().unwrap();
    let args: Vec<String> = std::env::args().collect();

    // custom work dir
    if args.len() > 1 {
        let custom_path = std::path::Path::new(args[1].as_str()).to_path_buf();
        return cwd.join(custom_path);
    }

    let executable_path = std::env::current_exe().unwrap();
    return executable_path.parent().unwrap().to_path_buf();
}

struct WebviewWarper(webview::WebView);
unsafe impl Send for WebviewWarper {}
unsafe impl Sync for WebviewWarper {}

fn main() {
    // 获取工作目录
    let work_dir = get_work_dir();
    std::env::set_current_dir(&work_dir).unwrap();

    println!("work_dir: {:?}", work_dir);

    // 获取配置文件路径
    let config_path = work_dir.join("tauri-lite.json");
    println!("config_path: {:?}", config_path);

    // 读取配置文件
    let config = config::get_config(&config_path);
    println!("config: {:?}", config);

    // init thread pool
    let api_task_pool = Arc::new(Mutex::new(thread_pool::ThreadPool::new(5)));
    let webview_eval_pool = thread_pool::ThreadPool::new(1);

    let entry_path = work_dir.join(config.entry.clone().unwrap_or("index.html".to_string()));
    let entry_url =
        static_server::start(api_task_pool.clone(), entry_path.clone(), work_dir.clone());

    let agent = ureq::AgentBuilder::new()
        .tls_connector(Arc::new(native_tls::TlsConnector::new().unwrap()))
        .build();

    let r = agent.get("https://tauri.app/").call().unwrap();
    let s = r.into_string().unwrap();
    println!("s: {}", s);

    // 打开 wry 主窗口
    window::open_main_window(&entry_url, &config);

    // use wry::{
    //     application::{
    //         event::{Event, StartCause, WindowEvent},
    //         event_loop::{ControlFlow, EventLoop},
    //         window::WindowBuilder,
    //     },
    //     webview::WebViewBuilder,
    // };

    // let event_loop = EventLoop::<String>::with_user_event();
    // let event_loop_proxy = event_loop.create_proxy();

    // let window = WindowBuilder::new()
    //     .with_title("Hello World")
    //     .build(&event_loop)
    //     .unwrap();

    // let webview = WebViewBuilder::new(window)
    //     .unwrap()
    //     .with_ipc_handler(move |_, _msg| {
    //         let event_loop_proxy = event_loop_proxy.clone();
    //         api_task_pool.lock().unwrap().run(move || {
    //             let mut msg = String::new();
    //             msg += "window.postMessage(\"";
    //             msg += _msg.escape_default().to_string().as_str();
    //             msg += "\")";
    //             std::thread::sleep(std::time::Duration::from_millis(5000));
    //             event_loop_proxy.send_event(msg).unwrap();
    //         })
    //     })
    //     .with_url(webview_url.as_str())
    //     .unwrap()
    //     .build()
    //     .unwrap();

    // let webview_warper = Arc::new(Mutex::new(WebviewWarper(webview)));
    // event_loop.run(move |event, _, control_flow| {
    //     *control_flow = ControlFlow::Wait;

    //     match event {
    //         Event::UserEvent(script) => {
    //             let webview_warper = webview_warper.clone();
    //             webview_eval_pool.run(move || {
    //                 let webview_warper = webview_warper.lock().unwrap();
    //                 webview_warper.0.evaluate_script(&script).unwrap();
    //             });
    //         }
    //         Event::NewEvents(StartCause::Init) => println!("Wry has started!"),
    //         Event::WindowEvent {
    //             event: WindowEvent::CloseRequested,
    //             ..
    //         } => *control_flow = ControlFlow::Exit,
    //         _ => (),
    //     }
    // });
}
