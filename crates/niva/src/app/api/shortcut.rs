use anyhow::Result;
use serde_json::json;
use std::sync::Arc;
use tao::{clipboard::Clipboard, event_loop::ControlFlow};
use wry::http::request;

use crate::app::{
    api_manager::{ApiManager, ApiRequest},
    window_manager::{options::NivaWindowOptions, window::NivaWindow},
    NivaApp, NivaId, NivaWindowTarget,
};

pub fn register_api_instances(api_manager: &mut ApiManager) {
    api_manager.register_event_api("shortcut.register", register);
    api_manager.register_event_api("shortcut.unregister", unregister);
    api_manager.register_event_api("shortcut.unregisterAll", unregister_all);
    api_manager.register_event_api("shortcut.list", list);
}

fn register(
    app: Arc<NivaApp>,
    _: Arc<NivaWindow>,
    request: ApiRequest,
    _: &NivaWindowTarget,
    _: &mut ControlFlow,
) -> Result<()> {
    let (id, accelerator_str) = request.args().get::<(u16, String)>()?;
    app.shortcut()?.register(id, accelerator_str)
}

fn unregister(
    app: Arc<NivaApp>,
    _: Arc<NivaWindow>,
    request: ApiRequest,
    _: &NivaWindowTarget,
    _: &mut ControlFlow,
) -> Result<()> {
    let id = request.args().single::<u16>()?;
    app.shortcut()?.unregister(id)
}

fn unregister_all(
    app: Arc<NivaApp>,
    _: Arc<NivaWindow>,
    _: ApiRequest,
    _: &NivaWindowTarget,
    _: &mut ControlFlow,
) -> Result<()> {
    app.shortcut()?.unregister_all()
}

fn list(
    app: Arc<NivaApp>,
    _: Arc<NivaWindow>,
    _: ApiRequest,
    _: &NivaWindowTarget,
    _: &mut ControlFlow,
) -> Result<Vec<(u16, String)>> {
    app.shortcut()?.list()
}
