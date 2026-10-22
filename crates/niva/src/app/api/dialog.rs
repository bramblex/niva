use anyhow::{Ok, Result};
use serde::Deserialize;
use serde_json::{Value, json};
use tao::window::Window;

use crate::app::NivaApp;
use crate::app::api_manager::ApiManager;
use crate::app::api_manager::ApiRequest;
use crate::app::main_exec::run_on_main;
use crate::app::window_manager::window::NivaWindow;
use std::sync::Arc;

pub fn register_api_instances(api_manager: &mut ApiManager) {
    api_manager.register_api_with("dialog.showMessage", Some(None), show_message);
    api_manager.register_api_with("dialog.pickFile", Some(None), pick_file);
    api_manager.register_api_with("dialog.pickFiles", Some(None), pick_files);
    api_manager.register_api_with("dialog.pickDir", Some(None), pick_dir);
    api_manager.register_api_with("dialog.pickDirs", Some(None), pick_dirs);
    api_manager.register_api_with("dialog.saveFile", Some(None), save_file);
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

async fn show_message(
    app: Arc<NivaApp>,
    window: Arc<NivaWindow>,
    request: ApiRequest,
) -> Result<()> {
    run_on_main(&app, move |_target, _control_flow| {
        let (title, content, level) =
            request
                .args()
                .optional::<(String, Option<String>, Option<MessageLevel>)>(3)?;
        let parent = &*window.window;
        let content = content.unwrap_or_default();
        let level = level.unwrap_or(MessageLevel::Info);

        Ok(rfd::AsyncMessageDialog::new()
            .set_title(&title)
            .set_description(&content)
            .set_parent(parent)
            .set_level(match level {
                MessageLevel::Info => rfd::MessageLevel::Info,
                MessageLevel::Warning => rfd::MessageLevel::Warning,
                MessageLevel::Error => rfd::MessageLevel::Error,
            })
            .show())
    })
    .await?
    .await;
    Ok(())
}

fn _create_dialog(
    parent: &Window,
    filters: Option<Vec<String>>,
    start_dir: Option<String>,
) -> rfd::AsyncFileDialog {
    let mut dialog = rfd::AsyncFileDialog::new();
    if let Some(extensions) = filters
        && !extensions.is_empty()
    {
        let extensions = extensions.iter().map(|s| s.as_str()).collect::<Vec<&str>>();
        dialog = dialog.add_filter("pick", &extensions);
    }
    if let Some(dir) = start_dir {
        dialog = dialog.set_directory(dir);
    }
    dialog.set_parent(parent)
}

async fn pick_file(
    app: Arc<NivaApp>,
    window: Arc<NivaWindow>,
    request: ApiRequest,
) -> Result<Value> {
    let result = run_on_main(&app, move |_target, _control_flow| {
        let (filters, start_dir) = request
            .args()
            .optional::<(Option<Vec<String>>, Option<String>)>(2)?;
        let parent = &*window.window;
        let dialog = _create_dialog(parent, filters, start_dir);

        Ok(dialog.pick_file())
    })
    .await?
    .await;
    Ok(result
        .map(|file| json!(file.path().to_path_buf()))
        .unwrap_or(Value::Null))
}

async fn pick_files(
    app: Arc<NivaApp>,
    window: Arc<NivaWindow>,
    request: ApiRequest,
) -> Result<Value> {
    let result = run_on_main(&app, move |_target, _control_flow| {
        let (filters, start_dir) = request
            .args()
            .optional::<(Option<Vec<String>>, Option<String>)>(2)?;
        let parent = &*window.window;
        let dialog = _create_dialog(parent, filters, start_dir);

        Ok(dialog.pick_files())
    })
    .await?
    .await;
    Ok(result
        .map(|files| {
            json!(
                files
                    .into_iter()
                    .map(|file| file.path().to_path_buf())
                    .collect::<Vec<_>>()
            )
        })
        .unwrap_or(Value::Null))
}

async fn pick_dir(
    app: Arc<NivaApp>,
    window: Arc<NivaWindow>,
    request: ApiRequest,
) -> Result<Value> {
    let result = run_on_main(&app, move |_target, _control_flow| {
        let (start_dir,) = request.args().optional::<(Option<String>,)>(1)?;
        let parent = &*window.window;
        let dialog = _create_dialog(parent, None, start_dir);

        Ok(dialog.pick_folder())
    })
    .await?
    .await;
    Ok(result
        .map(|file| json!(file.path().to_path_buf()))
        .unwrap_or(Value::Null))
}

async fn pick_dirs(
    app: Arc<NivaApp>,
    window: Arc<NivaWindow>,
    request: ApiRequest,
) -> Result<Value> {
    let result = run_on_main(&app, move |_target, _control_flow| {
        let (start_dir,) = request.args().optional::<(Option<String>,)>(1)?;
        let parent = &*window.window;
        let dialog = _create_dialog(parent, None, start_dir);

        Ok(dialog.pick_folders())
    })
    .await?
    .await;
    Ok(result
        .map(|files| {
            json!(
                files
                    .into_iter()
                    .map(|file| file.path().to_path_buf())
                    .collect::<Vec<_>>()
            )
        })
        .unwrap_or(Value::Null))
}

async fn save_file(
    app: Arc<NivaApp>,
    window: Arc<NivaWindow>,
    request: ApiRequest,
) -> Result<Value> {
    let result = run_on_main(&app, move |_target, _control_flow| {
        let (filters, start_dir) = request
            .args()
            .optional::<(Option<Vec<String>>, Option<String>)>(2)?;
        let parent = &*window.window;
        let dialog = _create_dialog(parent, filters, start_dir);

        Ok(dialog.save_file())
    })
    .await?
    .await;
    Ok(result
        .map(|file| json!(file.path().to_path_buf()))
        .unwrap_or(Value::Null))
}
