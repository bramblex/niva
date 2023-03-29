use anyhow::Result;

use std::sync::Arc;
use tao::event_loop::ControlFlow;

use crate::app::{
    api_manager::{ApiManager, ApiRequest},
    tray_manager::{NivaTrayOptions, NivaTrayUpdateOptions},
    window_manager::window::NivaWindow,
    NivaApp, NivaWindowTarget,
};

pub fn register_api_instances(api_manager: &mut ApiManager) {
    api_manager.register_event_api("tray.create", create);
    api_manager.register_event_api("tray.destroy", destroy);
    api_manager.register_event_api("tray.destroyAll", destroy_all);
    api_manager.register_event_api("tray.list", list);
    api_manager.register_event_api("tray.update", update);
}

fn create(
    app: Arc<NivaApp>,
    _window: Arc<NivaWindow>,
    request: ApiRequest,
    target: &NivaWindowTarget,
    _control_flow: &mut ControlFlow,
) -> Result<u16> {
    let options = request.args().single::<NivaTrayOptions>()?;
    let id = app.tray()?.create(&options, target)?;
    Ok(id)
}

fn destroy(
    app: Arc<NivaApp>,
    _window: Arc<NivaWindow>,
    request: ApiRequest,
    _target: &NivaWindowTarget,
    _control_flow: &mut ControlFlow,
) -> Result<()> {
    let id = request.args().single::<u16>()?;
    app.tray()?.destroy(id)?;
    Ok(())
}

fn destroy_all(
    app: Arc<NivaApp>,
    _window: Arc<NivaWindow>,
    _request: ApiRequest,
    _target: &NivaWindowTarget,
    _control_flow: &mut ControlFlow,
) -> Result<()> {
    app.tray()?.destroy_all()?;
    Ok(())
}

fn list(
    app: Arc<NivaApp>,
    _window: Arc<NivaWindow>,
    _request: ApiRequest,
    _target: &NivaWindowTarget,
    _control_flow: &mut ControlFlow,
) -> Result<Vec<u16>> {
    app.tray()?.list()
}

fn update(
    app: Arc<NivaApp>,
    _window: Arc<NivaWindow>,
    request: ApiRequest,
    _target: &NivaWindowTarget,
    _control_flow: &mut ControlFlow,
) -> Result<()> {
    let (id, options) = request.args().get::<(u16, NivaTrayUpdateOptions)>()?;
    app.tray()?.update(id, &options)?;
    Ok(())
}
