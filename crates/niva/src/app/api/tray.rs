use anyhow::Result;
use niva_macros::niva_event_api;

use crate::app::{
    api_manager::ApiManager,
    tray_manager::{NivaTrayOptions, NivaTrayUpdateOptions},
};

pub fn register_api_instances(api_manager: &mut ApiManager) {
    api_manager.register_event_api("tray.create", create);
    api_manager.register_event_api("tray.destroy", destroy);
    api_manager.register_event_api("tray.destroyAll", destroy_all);
    api_manager.register_event_api("tray.list", list);
    api_manager.register_event_api("tray.update", update);
}

#[niva_event_api]
fn create(options: NivaTrayOptions, window_id: Option<u16>) -> Result<u16> {
    app.tray()?
        .create(window_id.unwrap_or(window.id), &options, target)
}

#[niva_event_api]
fn destroy(id: u16, window_id: Option<u16>) -> Result<()> {
    app.tray()?.destroy(window_id.unwrap_or(window.id), id)
}

#[niva_event_api]
fn destroy_all(window_id: Option<u16>) -> Result<()> {
    app.tray()?.destroy_all(window_id.unwrap_or(window.id))
}

#[niva_event_api]
fn list(window_id: Option<u16>) -> Result<Vec<u16>> {
    app.tray()?.list(window_id.unwrap_or(window.id))
}

#[niva_event_api]
fn update(id: u16, options: NivaTrayUpdateOptions, window_id: Option<u16>) -> Result<()> {
    app.tray()?
        .update(window_id.unwrap_or(window.id), id, &options)
}
