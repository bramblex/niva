use anyhow::Result;
use directories::UserDirs;
use serde_json::{json, Value};
use wry::application::window::Window;

use crate::{
    api_manager::{ApiManager, ApiRequest},
    environment::EnvironmentRef,
};

pub fn register_apis(api_manager: &mut ApiManager) {
    api_manager.register_api("os.info", info);
    api_manager.register_api("os.dirs", dirs);
    api_manager.register_api("os.sep", sep);
    api_manager.register_api("os.eol", eol);
}

fn info(_: EnvironmentRef, _: &Window, _: ApiRequest) -> Result<Value> {
    let info = os_info::get();
    Ok(json!({
        "os": info.os_type().to_string(),
        "arch": std::env::consts::ARCH.to_string(),
        "version": info.version().to_string(),
    }))
}

fn dirs(env: EnvironmentRef, _: &Window, _: ApiRequest) -> Result<Value> {
    let user_dirs = UserDirs::new().unwrap();

    Ok(json!({
        "work": env.work_dir,
        "temp": env.temp_dir,
        "data": env.data_dir,

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
    }))
}

fn sep(_: EnvironmentRef, _: &Window, _: ApiRequest) -> Result<String> {
    Ok(std::path::MAIN_SEPARATOR.to_string())
}

fn eol(_: EnvironmentRef, _: &Window, _: ApiRequest) -> Result<String> {
    #[cfg(target_os = "windows")]
    let eol = "\r\n";
    #[cfg(not(target_os = "windows"))]
    let eol = "\n";

    Ok(eol.to_string())
}
