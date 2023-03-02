mod config;
mod server;
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

fn listen_available_port() -> Option<(std::net::TcpListener, String)> {
    for port in 1025..65535 {
        match std::net::TcpListener::bind(("127.0.0.1", port)) {
            Ok(l) => {
                let url = "http://127.0.0.1:".to_string() + port.to_string().as_str();
                return Some((l, url));
            }
            _ => {}
        }
    }
    return None;
}

fn main() {
    // 获取工作目录
    let work_dir = get_work_dir();
    println!("work_dir: {:?}", work_dir);

    // 获取配置文件路径
    let config_path = work_dir.join("tauri-lite.json");
    println!("config_path: {:?}", config_path);

    // 读取配置文件
    let config = config::get_config(&config_path);
    println!("config: {:?}", config);

    // 获取可用端口
    let (listener, entry_url) = listen_available_port().unwrap();

    // 启动 tauri-lite 后端 server
    let entry_path = work_dir.join(config.entry.clone());
    std::thread::spawn(move || {
        server::start(entry_path, work_dir, listener);
    });

    // 打开 wry 主窗口
    window::open_main_window(&entry_url, &config);
}
