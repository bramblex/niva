use anyhow::Result;
use serde::Serialize;

use crate::app::NivaApp;
use crate::app::api_manager::ApiManager;
use crate::app::api_manager::ApiRequest;
use crate::app::main_exec::run_on_main;
use crate::app::window_manager::window::NivaWindow;
use std::sync::Arc;

pub fn register_api_instances(api_manager: &mut ApiManager) {
    api_manager.register_api("shortcut.register", register);
    api_manager.register_api("shortcut.unregister", unregister);
    api_manager.register_api("shortcut.unregisterAll", unregister_all);
    api_manager.register_api("shortcut.list", list);
}

async fn register(app: Arc<NivaApp>, window: Arc<NivaWindow>, request: ApiRequest) -> Result<u8> {
    let app2 = app.clone();
    run_on_main(&app, move |_target, _control_flow| {
        let (accelerator_str, window_id) = request.args().optional::<(String, Option<u8>)>(2)?;
        app2.shortcut()?
            .register(window_id.unwrap_or(window.id), accelerator_str)
    })
    .await
}

async fn unregister(app: Arc<NivaApp>, window: Arc<NivaWindow>, request: ApiRequest) -> Result<()> {
    let app2 = app.clone();
    run_on_main(&app, move |_target, _control_flow| {
        let (id, window_id) = request.args().optional::<(u8, Option<u8>)>(2)?;
        app2.shortcut()?
            .unregister(window_id.unwrap_or(window.id), id)
    })
    .await
}

async fn unregister_all(
    app: Arc<NivaApp>,
    window: Arc<NivaWindow>,
    request: ApiRequest,
) -> Result<()> {
    let app2 = app.clone();
    run_on_main(&app, move |_target, _control_flow| {
        let (window_id,) = request.args().optional::<(Option<u8>,)>(1)?;
        app2.shortcut()?
            .unregister_all(window_id.unwrap_or(window.id))
    })
    .await
}

async fn list(
    app: Arc<NivaApp>,
    window: Arc<NivaWindow>,
    request: ApiRequest,
) -> Result<Vec<ShortcutEntry>> {
    let app2 = app.clone();
    run_on_main(&app, move |_target, _control_flow| {
        let (window_id,) = request.args().optional::<(Option<u8>,)>(1)?;
        Ok(shortcut_entries(
            app2.shortcut()?.list(window_id.unwrap_or(window.id))?,
        ))
    })
    .await
}

#[derive(Serialize)]
struct ShortcutEntry {
    id: u8,
    accelerator: String,
}

fn shortcut_entries(rows: Vec<(u8, String)>) -> Vec<ShortcutEntry> {
    rows.into_iter()
        .map(|(id, accelerator)| ShortcutEntry { id, accelerator })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::shortcut_entries;
    use serde_json::json;

    #[test]
    fn list_serializes_entries_with_the_public_id_and_accelerator_fields() {
        let entries = shortcut_entries(vec![(7, "shift+control+alt+F10".into())]);
        assert_eq!(
            serde_json::to_value(entries).unwrap(),
            json!([{ "id": 7, "accelerator": "shift+control+alt+F10" }])
        );
    }
}
