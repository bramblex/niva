use anyhow::Result;

use std::sync::Arc;

use crate::app::{
    NivaApp,
    api_manager::{ApiManager, ApiRequest},
    main_exec::run_on_main,
    tray_manager::{NivaTrayOptions, NivaTrayUpdateOptions},
    window_manager::window::NivaWindow,
};

pub fn register_api_instances(api_manager: &mut ApiManager) {
    api_manager.register_api("tray.create", create);
    api_manager.register_api("tray.destroy", destroy);
    api_manager.register_api("tray.destroyAll", destroy_all);
    api_manager.register_api("tray.list", list);
    api_manager.register_api("tray.update", update);
}

async fn create(app: Arc<NivaApp>, window: Arc<NivaWindow>, request: ApiRequest) -> Result<u8> {
    let app2 = app.clone();
    run_on_main(&app, move |target, _control_flow| {
        let (options, window_id) = request
            .args()
            .optional::<(NivaTrayOptions, Option<u8>)>(2)?;
        app2.tray()?
            .create(window_id.unwrap_or(window.id), &options, target)
    })
    .await
}

async fn destroy(app: Arc<NivaApp>, window: Arc<NivaWindow>, request: ApiRequest) -> Result<()> {
    let app2 = app.clone();
    run_on_main(&app, move |_target, _control_flow| {
        let (id, window_id) = request.args().optional::<(u8, Option<u8>)>(2)?;
        app2.tray()?.destroy(window_id.unwrap_or(window.id), id)
    })
    .await
}

async fn destroy_all(
    app: Arc<NivaApp>,
    window: Arc<NivaWindow>,
    request: ApiRequest,
) -> Result<()> {
    let app2 = app.clone();
    run_on_main(&app, move |_target, _control_flow| {
        let (window_id,) = request.args().optional::<(Option<u8>,)>(1)?;
        app2.tray()?.destroy_all(window_id.unwrap_or(window.id))
    })
    .await
}

async fn list(app: Arc<NivaApp>, window: Arc<NivaWindow>, request: ApiRequest) -> Result<Vec<u8>> {
    let app2 = app.clone();
    run_on_main(&app, move |_target, _control_flow| {
        let (window_id,) = request.args().optional::<(Option<u8>,)>(1)?;
        app2.tray()?.list(window_id.unwrap_or(window.id))
    })
    .await
}

async fn update(app: Arc<NivaApp>, window: Arc<NivaWindow>, request: ApiRequest) -> Result<()> {
    let app2 = app.clone();
    run_on_main(&app, move |_target, _control_flow| {
        let (id, options, window_id) = request
            .args()
            .optional::<(u8, NivaTrayUpdateOptions, Option<u8>)>(3)?;
        app2.tray()?
            .update(window_id.unwrap_or(window.id), id, &options)
    })
    .await
}
