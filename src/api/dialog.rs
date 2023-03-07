use super::{ApiRequest, ApiResponse};
use serde::Deserialize;
use serde_json::json;

pub fn call(request: ApiRequest) -> ApiResponse {
    match request.method.as_str() {
        "pickFile" => pick_file(request),
        "pickFiles" => pick_files(request),
        "pickDir" => pick_dir(request),
        "pickDirs" => pick_dirs(request),
        "saveFile" => save_file(request),
        "showMessage" => show_message(request),
        _ => ApiResponse::err(request.callback_id, "method not found"),
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PickOptions {
    filters: Option<Vec<String>>,
    start_dir: Option<String>,
}

fn _create_dialog(options: PickOptions) -> rfd::FileDialog {
    let mut dialog = rfd::FileDialog::new();
    if let Some(extensions) = options.filters {
        let extensions = extensions.iter().map(|s| s.as_str()).collect::<Vec<&str>>();
        dialog = dialog.add_filter("pick", &extensions);
    }
    if let Some(dir) = options.start_dir {
        dialog = dialog.set_directory(&dir);
    }
    return dialog;
}

fn pick_file(request: ApiRequest) -> ApiResponse {
    let options = serde_json::from_value::<PickOptions>(request.data);
    if options.is_err() {
        return ApiResponse::err(request.callback_id, "Invalid options");
    }
    let options = options.unwrap();
    let result = _create_dialog(options).pick_file();

    match result {
        Some(file) => ApiResponse::ok(request.callback_id, json!({ "file": file })),
        None => ApiResponse::err(request.callback_id, "No file picked"),
    }
}

fn pick_files(request: ApiRequest) -> ApiResponse {
    let options = serde_json::from_value::<PickOptions>(request.data);
    if options.is_err() {
        return ApiResponse::err(request.callback_id, "Invalid options");
    }
    let options = options.unwrap();
    let result = _create_dialog(options).pick_files();

    match result {
        Some(files) => ApiResponse::ok(request.callback_id, json!({ "file": files })),
        None => ApiResponse::err(request.callback_id, "No file picked"),
    }
}

fn pick_dir(request: ApiRequest) -> ApiResponse {
    let options = serde_json::from_value::<PickOptions>(request.data);
    if options.is_err() {
        return ApiResponse::err(request.callback_id, "Invalid options");
    }
    let options = options.unwrap();
    let result = _create_dialog(options).pick_folder();

    match result {
        Some(dir) => ApiResponse::ok(request.callback_id, json!({ "dir": dir })),
        None => ApiResponse::err(request.callback_id, "No file picked"),
    }
}

fn pick_dirs(request: ApiRequest) -> ApiResponse {
    let options = serde_json::from_value::<PickOptions>(request.data);
    if options.is_err() {
        return ApiResponse::err(request.callback_id, "Invalid options");
    }
    let options = options.unwrap();
    let result = _create_dialog(options).pick_folders();

    match result {
        Some(dirs) => ApiResponse::ok(request.callback_id, json!({ "dirs": dirs })),
        None => ApiResponse::err(request.callback_id, "No file picked"),
    }
}

fn save_file(request: ApiRequest) -> ApiResponse {
    let options = serde_json::from_value::<PickOptions>(request.data);
    if options.is_err() {
        return ApiResponse::err(request.callback_id, "Invalid options");
    }
    let options = options.unwrap();
    let result = _create_dialog(options).save_file();

    match result {
        Some(file) => ApiResponse::ok(request.callback_id, json!({ "file": file })),
        None => ApiResponse::err(request.callback_id, "No file picked"),
    }
}

#[derive(Deserialize)]
struct MessageOptions {
    pub title: String,
    pub content: Option<String>,
    pub level: Option<String>,
}

fn show_message(request: ApiRequest) -> ApiResponse {
    let options = serde_json::from_value::<MessageOptions>(request.data);
    if options.is_err() {
        return ApiResponse::err(request.callback_id, "Invalid Request.");
    }
    let options = options.unwrap();

    let result = rfd::MessageDialog::new()
        .set_title(&options.title)
        .set_description(&options.content.unwrap_or_default())
        .set_level(match options.level.unwrap_or_default().as_str() {
            "info" => rfd::MessageLevel::Info,
            "warning" => rfd::MessageLevel::Warning,
            "error" => rfd::MessageLevel::Error,
            _ => rfd::MessageLevel::Info,
        })
        .show();

    return ApiResponse::ok(request.callback_id, json!({ "result": result }));
}
