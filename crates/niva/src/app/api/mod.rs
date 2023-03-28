use super::api_manager::ApiManager;

mod dialog;
mod window;
mod fs;
mod http;
// mod os;
// mod process;
// mod webview;
// mod resource;

pub fn register_api_instances(api_manager: &mut ApiManager) {
    dialog::register_api_instances(api_manager);
    window::register_api_instances(api_manager);
    fs::register_api_instances(api_manager);
    http::register_api_instances(api_manager);
    // os::register_apis(api_manager);
    // process::register_apis(api_manager);
    // webview::register_apis(api_manager);
    // resource::register_apis(api_manager);
}