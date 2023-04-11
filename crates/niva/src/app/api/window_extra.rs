use anyhow::{anyhow, Result};
use niva_macros::niva_api;
use serde_json::{json, Value};

#[cfg(target_os = "macos")]
use tao::platform::macos::WindowExtMacOS;

#[cfg(target_os = "windows")]
use tao::platform::windows::WindowExtWindows;

use tao::{
    event_loop::ControlFlow,
    window::{CursorIcon, Fullscreen, Theme, UserAttentionType},
};

use crate::app::{
    api_manager::ApiManager,
    window_manager::options::{NivaPosition, NivaSize, NivaWindowOptions, WindowMenuOptions},
};

macro_rules! match_window {
    ($app:ident, $window:ident, $id:ident) => {
        let $window = match $id {
            Some(id) => $app.window()?.get_window(id)?,
            None => $window,
        };
    };
}

pub fn register_api_instances(api_manager: &mut ApiManager) {}

// windows
#[cfg(target_os = "windows")]
#[niva_api]
fn set_enable(enabled: bool, id: Option<u16>) -> Result<()> {
    match_window!(app, window, id);
    window.set_enable(enabled);
    Ok(())
}

#[cfg(target_os = "windows")]
#[niva_api]
fn set_taskbar_icon(taskbar_icon: String, id: Option<u16>) -> Result<()> {
    match_window!(app, window, id);
    let taskbar_icon = app.resource().load_icon(&taskbar_icon)?;
    window.set_taskbar_icon(Some(taskbar_icon));
    Ok(())
}

#[cfg(target_os = "windows")]
#[niva_api]
fn theme(id: Option<u16>) -> Result<String> {
    match_window!(app, window, id);
    match window.theme() {
        Theme::Dark => Ok("dark".to_string()),
        Theme::Light => Ok("light".to_string()),
        _ => Ok("system".to_string()),
    }
}

#[cfg(target_os = "windows")]
#[niva_api]
fn reset_dead_keys(id: Option<u16>) -> Result<()> {
    match_window!(app, window, id);
    window.reset_dead_keys();
    Ok(())
}

#[cfg(target_os = "windows")]
#[niva_api]
fn begin_resize_drag(edge: isize, button: u32, x: i32, y: i32, id: Option<u16>) -> Result<()> {
    match_window!(app, window, id);
    window.begin_resize_drag(edge, button, x, y);
    Ok(())
}

#[cfg(target_os = "windows")]
#[niva_api]
fn set_skip_taskbar(skip: bool, id: Option<u16>) -> Result<()> {
    match_window!(app, window, id);
    window.set_skip_taskbar(skip);
    Ok(())
}

#[cfg(target_os = "windows")]
#[niva_api]
fn set_undecorated_shadow(shadow: bool, id: Option<u16>) -> Result<()> {
    match_window!(app, window, id);
    window.set_undecorated_shadow(shadow);
    Ok(())
}

#[cfg(target_os = "macos")]
#[niva_api]
fn simple_fullscreen(id: Option<u16>) -> Result<bool> {
    match_window!(app, window, id);
    Ok(window.simple_fullscreen())
}

#[cfg(target_os = "macos")]
#[niva_api]
fn set_simple_fullscreen(fullscreen: bool, id: Option<u16>) -> Result<bool> {
    match_window!(app, window, id);
    Ok(window.set_simple_fullscreen(fullscreen))
}

#[niva_api]
#[cfg(target_os = "macos")]
fn has_shadow(id: Option<u16>) -> Result<bool> {
    match_window!(app, window, id);
    Ok(window.has_shadow())
}

#[cfg(target_os = "macos")]
#[niva_api]
fn set_has_shadow(has_shadow: bool, id: Option<u16>) -> Result<()> {
    match_window!(app, window, id);
    window.set_has_shadow(has_shadow);
    Ok(())
}

#[cfg(target_os = "macos")]
#[niva_api]
fn set_is_document_edited(edited: bool, id: Option<u16>) -> Result<()> {
    match_window!(app, window, id);
    window.set_is_document_edited(edited);
    Ok(())
}

#[cfg(target_os = "macos")]
#[niva_api]
fn is_document_edited(id: Option<u16>) -> Result<bool> {
    match_window!(app, window, id);
    Ok(window.is_document_edited())
}

#[cfg(target_os = "macos")]
#[niva_api]
fn set_allows_automatic_window_tabbing(enabled: bool, id: Option<u16>) -> Result<()> {
    match_window!(app, window, id);
    window.set_allows_automatic_window_tabbing(enabled);
    Ok(())
}

#[cfg(target_os = "macos")]
#[niva_api]
fn allows_automatic_window_tabbing(id: Option<u16>) -> Result<bool> {
    match_window!(app, window, id);
    Ok(window.allows_automatic_window_tabbing())
}

#[cfg(target_os = "macos")]
#[niva_api]
fn set_tabbing_identifier(identifier: String, id: Option<u16>) -> Result<()> {
    match_window!(app, window, id);
    window.set_tabbing_identifier(&identifier);
    Ok(())
}

#[cfg(target_os = "macos")]
#[niva_api]
fn tabbing_identifier(id: Option<u16>) -> Result<String> {
    match_window!(app, window, id);
    Ok(window.tabbing_identifier())
}
