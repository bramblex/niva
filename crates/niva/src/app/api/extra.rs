use crate::{app::api_manager::ApiManager, opts_match};

use anyhow::{anyhow, Result};
use niva_macros::niva_api;
use serde_json::json;

pub fn register_api_instances(api_manager: &mut ApiManager) {
    api_manager.register_async_api("extra.getActiveWindowId", get_active_window_id);
    api_manager.register_async_api("extra.focusByWindowId", focus_by_window_id);
}

#[cfg(target_os = "macos")]
#[niva_api]
fn get_active_window_id() -> Result<String> {
    use active_win_pos_rs::get_active_window;

    let window = get_active_window()?;
    Ok(format!("{}_{}", window.process_id, window.window_id))
}

#[cfg(target_os = "macos")]
#[niva_api]
fn focus_by_window_id() -> Result<bool> {
    use cocoa::appkit::NSApplicationActivateIgnoringOtherApps;
    use cocoa::base::{nil, NO};
    use objc::runtime::{Class, Object, Sel};
    use objc::{class, msg_send, sel, sel_impl};
    opts_match!(request, id_string: String);
    let result = id_string.split("_").collect::<Vec<&str>>();

    if result.len() != 2 {
        return Err(anyhow!("invalid window id"));
    }
    let process_id = result[0].parse::<u32>()?;
    let window_id = result[1].parse::<u64>()?;

    unsafe {
        let app_class = class!(NSRunningApplication);
        let app_with_process_id: *mut Object = msg_send![
            app_class,
            runningApplicationWithProcessIdentifier: process_id as i64
        ];
        if app_with_process_id != nil {
            let success: bool = msg_send![
                app_with_process_id,
                activateWithOptions: NSApplicationActivateIgnoringOtherApps
            ];

            if !success {
                return Ok(true);
            }
        }
    }
    Ok(false)
}

#[cfg(target_os = "windows")]
#[niva_api]
fn get_active_window_id() -> Result<String> {
    use winapi::um::winuser::GetForegroundWindow;

    let hwnd = unsafe { GetForegroundWindow() as usize };
    Ok(hwnd.to_string())
}

#[cfg(target_os = "windows")]
#[niva_api]
fn focus_by_window_id(hwnd_str: String) -> Result<()> {
    use winapi::shared::windef::HWND;
    use winapi::um::winuser::SetForegroundWindow;

    let hwnd = hwnd_str.parse::<usize>()? as HWND;
    unsafe {
        SetForegroundWindow(hwnd);
    }
    Ok(())
}
