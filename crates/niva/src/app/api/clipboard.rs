use anyhow::Result;
use niva_macros::niva_event_api;

use tao::clipboard::Clipboard;

use crate::app::api_manager::ApiManager;

pub fn register_api_instances(api_manager: &mut ApiManager) {
    api_manager.register_event_api("clipboard.read", read);
    api_manager.register_event_api("clipboard.write", write);
}

#[niva_event_api]
fn read() -> Result<Option<String>> {
    Ok(Clipboard::new().read_text())
}

#[niva_event_api]
fn write(text: String) -> Result<()> {
    Clipboard::new().write_text(text);
    Ok(())
}
