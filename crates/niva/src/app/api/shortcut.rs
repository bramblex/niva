use anyhow::Result;

use std::sync::Arc;
use tao::event_loop::ControlFlow;

use crate::{app::{
    api_manager::{ApiManager, ApiRequest},
    window_manager::window::NivaWindow,
    NivaApp, NivaWindowTarget,
}, args_match};

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
    args_match!(request, id: u16, accelerator_str: String);
    app.shortcut()?.register(id, accelerator_str)
}

fn unregister(
    app: Arc<NivaApp>,
    _: Arc<NivaWindow>,
    request: ApiRequest,
    _: &NivaWindowTarget,
    _: &mut ControlFlow,
) -> Result<()> {
    args_match!(request, id: u16);
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
