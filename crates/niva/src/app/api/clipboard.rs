use std::sync::Arc;

use anyhow::Result;
use arboard::Clipboard;

use crate::app::{
    NivaApp,
    api_manager::{ApiManager, ApiRequest},
    main_exec::run_on_main,
    window_manager::window::NivaWindow,
};

pub fn register_api_instances(api_manager: &mut ApiManager) {
    api_manager.register_api("clipboard.read", read);
    api_manager.register_api("clipboard.write", write);
}

async fn read(
    app: Arc<NivaApp>,
    _window: Arc<NivaWindow>,
    _request: ApiRequest,
) -> Result<Option<String>> {
    run_on_main(&app, move |_, _| {
        let mut clipboard = Clipboard::new()?;
        Ok(clipboard.get_text().ok())
    })
    .await
}

async fn write(app: Arc<NivaApp>, _window: Arc<NivaWindow>, request: ApiRequest) -> Result<()> {
    let (text,): (String,) = request.args().optional(1)?;
    run_on_main(&app, move |_, _| {
        let mut clipboard = Clipboard::new()?;
        clipboard.set_text(text)?;
        Ok(())
    })
    .await
}
