use anyhow::Result;
use directories::UserDirs;
use niva_macros::niva_api;
use serde_json::{json, Value};
use sys_locale::get_locale;
use std::sync::Arc;

use crate::app::{
    api_manager::{ApiManager, ApiRequest},
    window_manager::window::NivaWindow,
    NivaApp,
};

pub fn register_apis(api_manager: &mut ApiManager) {
    api_manager.register_api("os.info", info);
    api_manager.register_api("os.dirs", dirs);
    api_manager.register_api("os.sep", sep);
    api_manager.register_api("os.eol", eol);
    api_manager.register_api("os.locale", locale);
}

fn info(_: Arc<NivaApp>, _: Arc<NivaWindow>, _: ApiRequest) -> Result<Value> {
    let info = os_info::get();
    Ok(json!({
        "os": info.os_type().to_string(),
        "arch": std::env::consts::ARCH.to_string(),
        "version": info.version().to_string(),
    }))
}

fn dirs(app: Arc<NivaApp>, _: Arc<NivaWindow>, _: ApiRequest) -> Result<Value> {
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

fn sep(_: Arc<NivaApp>, _: Arc<NivaWindow>, _: ApiRequest) -> Result<String> {
    Ok(std::path::MAIN_SEPARATOR.to_string())
}

fn eol(_: Arc<NivaApp>, _: Arc<NivaWindow>, _: ApiRequest) -> Result<String> {
    #[cfg(target_os = "windows")]
    let eol = "\r\n";
    #[cfg(not(target_os = "windows"))]
    let eol = "\n";

    Ok(eol.to_string())
}

#[niva_api]
fn locale() -> Result<String> {
     Ok(get_locale().unwrap_or("en-US".to_string()))
}