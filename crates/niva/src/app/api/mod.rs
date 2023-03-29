use super::api_manager::ApiManager;

mod clipboard;
mod dialog;
mod fs;
mod http;
mod os;
mod process;
mod resource;
mod shortcut;
mod webview;
mod window;
mod tray;
mod monitor;

pub fn register_api_instances(api_manager: &mut ApiManager) {
    dialog::register_api_instances(api_manager);
    window::register_api_instances(api_manager);
    fs::register_api_instances(api_manager);
    http::register_api_instances(api_manager);
    os::register_apis(api_manager);
    process::register_apis(api_manager);
    webview::register_apis(api_manager);
    resource::register_apis(api_manager);
    clipboard::register_api_instances(api_manager);
    shortcut::register_api_instances(api_manager);
    tray::register_api_instances(api_manager);
}
