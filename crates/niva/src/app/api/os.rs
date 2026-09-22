use anyhow::Result;
use directories::UserDirs;
use serde_json::{Value, json};
use sys_locale::get_locale;

use crate::app::NivaApp;
use crate::app::api_manager::ApiManager;
use crate::app::api_manager::ApiRequest;
use crate::app::window_manager::window::NivaWindow;
use std::sync::Arc;

pub fn register_apis(api_manager: &mut ApiManager) {
    api_manager.register_api("os.info", info);
    api_manager.register_api("os.dirs", dirs);
    api_manager.register_api("os.sep", sep);
    api_manager.register_api("os.eol", eol);
    api_manager.register_api("os.locale", locale);
}

async fn info(_app: Arc<NivaApp>, _window: Arc<NivaWindow>, _request: ApiRequest) -> Result<Value> {
    let info = os_info::get();
    Ok(json!({
        "os": info.os_type().to_string(),
        "arch": std::env::consts::ARCH.to_string(),
        "version": info.version().to_string(),
    }))
}

async fn dirs(app: Arc<NivaApp>, _window: Arc<NivaWindow>, _request: ApiRequest) -> Result<Value> {
    let user_dirs = UserDirs::new();

    match user_dirs {
        Some(user_dirs) => Ok(json!({
            "temp": app.launch_info.temp_dir,
            "data": app.launch_info.data_dir,

            "home": user_dirs.home_dir(),
            "audio": user_dirs.audio_dir(),
            "desktop": user_dirs.desktop_dir(),
            "document": user_dirs.document_dir(),
            "download": user_dirs.download_dir(),
            "font": user_dirs.font_dir(),
            "picture": user_dirs.picture_dir(),
            "public": user_dirs.public_dir(),
            "template": user_dirs.template_dir(),
            "video": user_dirs.video_dir(),
        })),
        None => Ok(json!({
            "temp": app.launch_info.temp_dir,
            "data": app.launch_info.data_dir,
        })),
    }
}

async fn sep(_app: Arc<NivaApp>, _window: Arc<NivaWindow>, _request: ApiRequest) -> Result<String> {
    Ok(std::path::MAIN_SEPARATOR.to_string())
}

async fn eol(_app: Arc<NivaApp>, _window: Arc<NivaWindow>, _request: ApiRequest) -> Result<String> {
    #[cfg(target_os = "windows")]
    let eol = "\r\n";
    #[cfg(target_os = "macos")]
    let eol = "\n";

    Ok(eol.to_string())
}

async fn locale(
    _app: Arc<NivaApp>,
    _window: Arc<NivaWindow>,
    _request: ApiRequest,
) -> Result<String> {
    Ok(get_locale().unwrap_or("en-US".to_string()))
}
