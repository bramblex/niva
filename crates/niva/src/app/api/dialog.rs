use std::sync::Arc;

use anyhow::{Ok, Result};
use serde::Deserialize;
use serde_json::{json, Value};
use wry::application::window::Window;

use crate::app::{api_manager::{ApiManager, ApiRequest}, NivaApp, window_manager::window::NivaWindow};

pub fn register_api_instances(api_manager: &mut ApiManager) {
    api_manager.register_api("dialog.showMessage", show_message);
    api_manager.register_api("dialog.pickFile", pick_file);
    api_manager.register_api("dialog.pickFiles", pick_files);
    api_manager.register_api("dialog.pickDir", pick_dir);
    api_manager.register_api("dialog.pickDirs", pick_dirs);
    api_manager.register_api("dialog.saveFile", save_file);
}

#[derive(Deserialize)]
enum MessageLevel {
    #[serde(rename = "info")]
    Info,
    #[serde(rename = "warning")]
    Warning,
    #[serde(rename = "error")]
    Error,
}

fn show_message(_app: Arc<NivaApp>, parent: Arc<NivaWindow>, request: ApiRequest) -> Result<()> {
    let parent = parent.webview.window();
    let (title, content, level) = request
        .args()
        .optional::<(String, Option<String>, Option<MessageLevel>)>(3)?;
    let content = content.unwrap_or_default();
    let level = level.unwrap_or(MessageLevel::Info);

    rfd::MessageDialog::new()
        .set_title(&title)
        .set_description(&content)
        .set_parent(parent)
        .set_level(match level {
            MessageLevel::Info => rfd::MessageLevel::Info,
            MessageLevel::Warning => rfd::MessageLevel::Warning,
            MessageLevel::Error => rfd::MessageLevel::Error,
        })
        .show();

    Ok(())
}

fn _create_dialog(
    parent: &Window,
    filters: Option<Vec<String>>,
    start_dir: Option<String>,
) -> rfd::FileDialog {
    let mut dialog = rfd::FileDialog::new();
    if let Some(extensions) = filters {
        if !extensions.is_empty() {
            let extensions = extensions.iter().map(|s| s.as_str()).collect::<Vec<&str>>();
            dialog = dialog.add_filter("pick", &extensions);
        }
    }
    if let Some(dir) = start_dir {
        dialog = dialog.set_directory(dir);
    }
    dialog.set_parent(parent)
}

fn pick_file(_app: Arc<NivaApp>,parent: Arc<NivaWindow>,   request: ApiRequest) -> Result<Value> {
    let parent = parent.webview.window();
    let (filters, start_dir) = request
        .args()
        .optional::<(Option<Vec<String>>, Option<String>)>(2)?;
    let dialog = _create_dialog(parent, filters, start_dir);

    match dialog.pick_file() {
        Some(file) => Ok(json!(file)),
        None => Ok(json!(null)),
    }
}

fn pick_files(_app: Arc<NivaApp>,parent: Arc<NivaWindow>,   request: ApiRequest) -> Result<Value> {
    let parent = parent.webview.window();
    let (filters, start_dir) = request
        .args()
        .optional::<(Option<Vec<String>>, Option<String>)>(2)?;
    let dialog = _create_dialog(parent, filters, start_dir);

    match dialog.pick_files() {
        Some(files) => Ok(json!(files)),
        None => Ok(json!(null)),
    }
}

fn pick_dir(_app: Arc<NivaApp>, parent: Arc<NivaWindow>, request: ApiRequest) -> Result<Value> {
    let parent = parent.webview.window();
    let (start_dir,) = request.args().optional::<(Option<String>,)>(1)?;
    let dialog = _create_dialog(parent, None, start_dir);

    match dialog.pick_folder() {
        Some(dir) => Ok(json!(dir)),
        None => Ok(json!(null)),
    }
}

fn pick_dirs(_app: Arc<NivaApp>, parent: Arc<NivaWindow>, request: ApiRequest) -> Result<Value> {
    let parent = parent.webview.window();
    let (start_dir,) = request.args().optional::<(Option<String>,)>(1)?;
    let dialog = _create_dialog(parent, None, start_dir);

    match dialog.pick_folders() {
        Some(dirs) => Ok(json!(dirs)),
        None => Ok(json!(null)),
    }
}

fn save_file(_app: Arc<NivaApp>, parent: Arc<NivaWindow>, request: ApiRequest) -> Result<Value> {
    let parent = parent.webview.window();
    let (filters, start_dir) = request
        .args()
        .optional::<(Option<Vec<String>>, Option<String>)>(2)?;
    let dialog = _create_dialog(parent, filters, start_dir);

    match dialog.save_file() {
        Some(file) => Ok(json!(file)),
        None => Ok(json!(null)),
    }
}