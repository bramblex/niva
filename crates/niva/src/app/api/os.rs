use anyhow::Result;
use directories::UserDirs;
use serde_json::{Value, json};
use sys_locale::get_locale;

use crate::app::NivaApp;
use crate::app::api_manager::ApiManager;
use crate::app::api_manager::ApiRequest;
use crate::app::window_manager::window::NivaWindow;
use std::path::Path;
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
    Ok(info_value(
        info.os_type().to_string(),
        std::env::consts::ARCH.to_string(),
        info.version().to_string(),
    ))
}

fn info_value(os: String, arch: String, version: String) -> Value {
    json!({ "os": os, "arch": arch, "version": version })
}

async fn dirs(app: Arc<NivaApp>, _window: Arc<NivaWindow>, _request: ApiRequest) -> Result<Value> {
    let user_dirs = UserDirs::new();
    Ok(dirs_value(
        &app.launch_info.temp_dir,
        &app.launch_info.data_dir,
        user_dirs.as_ref(),
    ))
}

fn dirs_value(temp: &Path, data: &Path, user_dirs: Option<&UserDirs>) -> Value {
    match user_dirs {
        Some(user_dirs) => json!({
            "temp": temp,
            "data": data,
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
        }),
        None => json!({ "temp": temp, "data": data }),
    }
}

async fn sep(_app: Arc<NivaApp>, _window: Arc<NivaWindow>, _request: ApiRequest) -> Result<String> {
    Ok(path_separator().to_string())
}

fn path_separator() -> char {
    std::path::MAIN_SEPARATOR
}

async fn eol(_app: Arc<NivaApp>, _window: Arc<NivaWindow>, _request: ApiRequest) -> Result<String> {
    Ok(line_ending().to_string())
}

fn line_ending() -> &'static str {
    #[cfg(target_os = "windows")]
    {
        "\r\n"
    }
    #[cfg(not(target_os = "windows"))]
    {
        "\n"
    }
}

async fn locale(
    _app: Arc<NivaApp>,
    _window: Arc<NivaWindow>,
    _request: ApiRequest,
) -> Result<String> {
    Ok(locale_value(get_locale()))
}

fn locale_value(locale: Option<String>) -> String {
    locale.unwrap_or_else(|| "en-US".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn info_includes_os_architecture_and_version_as_strings() {
        let info = info_value("macos".into(), "aarch64".into(), "14.0".into());
        assert_eq!(
            info,
            json!({ "os": "macos", "arch": "aarch64", "version": "14.0" })
        );
    }

    #[test]
    fn dirs_always_include_app_paths_and_add_user_directories_when_available() {
        let temp = Path::new("/app/temp");
        let data = Path::new("/app/data");

        let minimal = dirs_value(temp, data, None);
        assert_eq!(minimal, json!({ "temp": temp, "data": data }));

        if let Some(user_dirs) = UserDirs::new() {
            let dirs = dirs_value(temp, data, Some(&user_dirs));
            assert_eq!(dirs["temp"], json!(temp));
            assert_eq!(dirs["data"], json!(data));
            assert_eq!(dirs["home"], json!(user_dirs.home_dir()));
            for key in [
                "audio", "desktop", "document", "download", "font", "picture", "public",
                "template", "video",
            ] {
                assert!(dirs.get(key).is_some(), "missing directory field {key}");
            }
        }
    }

    #[test]
    fn separator_and_line_ending_match_the_target_platform() {
        assert_eq!(path_separator(), std::path::MAIN_SEPARATOR);
        #[cfg(target_os = "windows")]
        assert_eq!(line_ending(), "\r\n");
        #[cfg(not(target_os = "windows"))]
        assert_eq!(line_ending(), "\n");
    }

    #[test]
    fn locale_uses_system_value_or_en_us_fallback() {
        assert_eq!(locale_value(Some("zh-CN".into())), "zh-CN");
        assert_eq!(locale_value(None), "en-US");
    }
}
