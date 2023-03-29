use crate::{app::api_manager::ApiManager, args_match};
use anyhow::{Ok, Result};

pub fn register_api_instances(api_manager: &mut ApiManager) {
    #[cfg(target_os = "macos")]
    {
        use std::process::Command;
        api_manager.register_async_api("extra.getActiveWindowId", |_, _, _| -> Result<u32> {
            let script = r#"
        tell application "System Events"
            set frontApp to first application process whose frontmost is true
            set frontWindow to first window of frontApp
            set windowID to id of frontWindow
            return windowID
        end tell
        "#;

            let output = Command::new("osascript").arg("-e").arg(script).output()?;

            if output.status.success() {
                let window_id_str = std::str::from_utf8(&output.stdout)?.trim();
                let window_id = window_id_str.parse::<u32>()?;
                Ok(window_id)
            } else {
                Err(anyhow!("Failed to get active window ID"))
            }
        });

        api_manager.register_async_api("extra.focusWindowById", |_, _, request| -> Result<()> {
            let window_id = request.args().single::<u32>()?;
            let window_id_str = window_id.to_string();

            let script = format!(
                r#"
            tell application "System Events"
                set frontmost of (first window whose id is {}) to true
            end tell
            "#,
                window_id_str
            );

            let output = Command::new("osascript").arg("-e").arg(&script).output()?;

            if output.status.success() {
                Ok(())
            } else {
                Err(anyhow!("Failed to focus window by ID"))
            }
        });
    }

    #[cfg(target_os = "windows")]
    {
        use winapi::shared::windef::HWND;
        use winapi::um::winuser::{GetForegroundWindow, SetForegroundWindow};
        api_manager.register_async_api("extra.getActiveWindowId", |_, _, _| -> Result<String> {
            let hwnd = unsafe { GetForegroundWindow() as usize };
            Ok(hwnd.to_string())
        });

        api_manager.register_async_api("extra.focusWindowById", |_, _, request| -> Result<()> {
            args_match!(request, hwnd_str: String);
            let hwnd = hwnd_str.parse::<usize>()? as HWND;
            unsafe {
                SetForegroundWindow(hwnd);
            }
            Ok(())
        });
    }
}

// fn get_active_window_id()
