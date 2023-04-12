use anyhow::{Ok, Result};
use niva_macros::niva_api;
use serde::Deserialize;
use serde_json::{json, Value};
use wry::application::window::Window;

use crate::app::api_manager::ApiManager;

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

#[niva_api]
fn show_message(title: String, content: Option<String>, level: Option<MessageLevel>) -> Result<()> {
    let parent = window.webview.window();
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

#[niva_api]
fn pick_file(filters: Option<Vec<String>>, start_dir: Option<String>) -> Result<Value> {
    let parent = window.webview.window();
    let dialog = _create_dialog(parent, filters, start_dir);

    match dialog.pick_file() {
        Some(file) => Ok(json!(file)),
        None => Ok(json!(null)),
    }
}

#[niva_api]
fn pick_files(filters: Option<Vec<String>>, start_dir: Option<String>) -> Result<Value> {
    let parent = window.webview.window();
    let dialog = _create_dialog(parent, filters, start_dir);

    match dialog.pick_files() {
        Some(files) => Ok(json!(files)),
        None => Ok(json!(null)),
    }
}

#[niva_api]
fn pick_dir(start_dir: Option<String>) -> Result<Value> {
    let parent = window.webview.window();
    let dialog = _create_dialog(parent, None, start_dir);

    match dialog.pick_folder() {
        Some(dir) => Ok(json!(dir)),
        None => Ok(json!(null)),
    }
}

#[niva_api]
fn pick_dirs(start_dir: Option<String>) -> Result<Value> {
    let parent = window.webview.window();
    let dialog = _create_dialog(parent, None, start_dir);

    match dialog.pick_folders() {
        Some(dirs) => Ok(json!(dirs)),
        None => Ok(json!(null)),
    }
}

#[niva_api]
fn save_file(filters: Option<Vec<String>>, start_dir: Option<String>) -> Result<Value> {
    let parent = window.webview.window();
    let dialog = _create_dialog(parent, filters, start_dir);

    match dialog.save_file() {
        Some(file) => Ok(json!(file)),
        None => Ok(json!(null)),
    }
}
