use anyhow::Result;
use niva_macros::niva_event_api;

use crate::app::api_manager::ApiManager;

pub fn register_api_instances(api_manager: &mut ApiManager) {
    api_manager.register_event_api("shortcut.register", register);
    api_manager.register_event_api("shortcut.unregister", unregister);
    api_manager.register_event_api("shortcut.unregisterAll", unregister_all);
    api_manager.register_event_api("shortcut.list", list);
}

#[niva_event_api]
fn register(accelerator_str: String, window_id: Option<u8>) -> Result<u8> {
    app.shortcut()?
        .register(window_id.unwrap_or(window.id), accelerator_str)
}

#[niva_event_api]
fn unregister(id: u8, window_id: Option<u8>) -> Result<()> {
    app.shortcut()?
        .unregister(window_id.unwrap_or(window.id), id)
}

#[niva_event_api]
fn unregister_all(window_id: Option<u8>) -> Result<()> {
    app.shortcut()?
        .unregister_all(window_id.unwrap_or(window.id))
}

#[niva_event_api]
fn list(window_id: Option<u8>) -> Result<Vec<(u8, String)>> {
    app.shortcut()?.list(window_id.unwrap_or(window.id))
}
