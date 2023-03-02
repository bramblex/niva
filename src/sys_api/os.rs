use super::{ApiRequest, ApiResponse};
use directories::UserDirs;
use serde_json::{json, Value};

pub fn call(request: ApiRequest) -> ApiResponse {
    return match request.method.as_str() {
        "info" => platform(),
        "dirs" => dirs(),
        _ => ApiResponse::err("Method not found".to_string()),
    };
}

fn platform() -> ApiResponse {
    let info = os_info::get();

    return ApiResponse::ok(json!({
        "type": info.os_type().to_string(),
        "version": info.version().to_string(),
    }));
}

fn unwrap_path(path: &std::path::Path) -> Value {
    return Value::String(path.to_str().unwrap_or("").to_string());
}

fn unwrap_path_opt(path_result: Option<&std::path::Path>) -> Value {
    if path_result.is_none() {
        return Value::Null;
    }
    return Value::String(path_result.unwrap().to_str().unwrap_or("").to_string());
}

fn dirs() -> ApiResponse {
    let user_dirs = UserDirs::new().unwrap();

    return ApiResponse::ok(json!({
        "home": unwrap_path(user_dirs.home_dir()),
                "temp": unwrap_path(&std::env::temp_dir()),
        "audio": unwrap_path_opt(user_dirs.audio_dir()),
        "desktop": unwrap_path_opt(user_dirs.desktop_dir()),
        "document": unwrap_path_opt(user_dirs.document_dir()),
        "download": unwrap_path_opt(user_dirs.download_dir()),
        "font": unwrap_path_opt(user_dirs.font_dir()),
        "picture": unwrap_path_opt(user_dirs.picture_dir()),
        "public": unwrap_path_opt(user_dirs.public_dir()),
        "template": unwrap_path_opt(user_dirs.template_dir()),
        "video": unwrap_path_opt(user_dirs.video_dir()),
    }));
}
