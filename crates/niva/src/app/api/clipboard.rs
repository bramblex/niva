use anyhow::Result;
use arboard::Clipboard;
use niva_macros::niva_event_api;

use crate::app::api_manager::ApiManager;

pub fn register_api_instances(api_manager: &mut ApiManager) {
    api_manager.register_event_api("clipboard.read", read);
    api_manager.register_event_api("clipboard.write", write);
}

#[niva_event_api]
fn read() -> Result<Option<String>> {
    let mut clipboard = Clipboard::new()?;
    Ok(clipboard.get_text().ok())
}

#[niva_event_api]
fn write(text: String) -> Result<()> {
    let mut clipboard = Clipboard::new()?;
    clipboard.set_text(text)?;
    Ok(())
}
