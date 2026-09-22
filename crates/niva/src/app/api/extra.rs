use crate::app::api_manager::ApiManager;

use crate::app::NivaApp;
use crate::app::api_manager::ApiRequest;
use crate::app::main_exec::run_on_main;
use crate::app::window_manager::window::NivaWindow;
use anyhow::{Result, anyhow};
use std::sync::Arc;

pub fn register_api_instances(api_manager: &mut ApiManager) {
    api_manager.register_blocking_api("extra.getActiveWindowId", get_active_window_id);
    #[cfg(target_os = "macos")]
    api_manager.register_api("extra.focusByWindowId", focus_by_window_id);
    #[cfg(target_os = "windows")]
    api_manager.register_blocking_api("extra.focusByWindowId", focus_by_window_id);

    #[cfg(target_os = "macos")]
    {
        api_manager.register_api("extra.hideApplication", hide_application);
        api_manager.register_api("extra.showApplication", show_application);
        api_manager.register_api("extra.hideOtherApplications", hide_other_applications);
        api_manager.register_api("extra.setActivationPolicy", set_activation_policy);
    }
}

#[cfg(target_os = "macos")]
async fn hide_application(
    app: Arc<NivaApp>,
    _window: Arc<NivaWindow>,
    _request: ApiRequest,
) -> Result<()> {
    run_on_main(&app, move |target, _control_flow| {
        use tao::platform::macos::EventLoopWindowTargetExtMacOS;
        target.hide_application();
        Ok(())
    })
    .await
}

#[cfg(target_os = "macos")]
async fn show_application(
    app: Arc<NivaApp>,
    _window: Arc<NivaWindow>,
    _request: ApiRequest,
) -> Result<()> {
    run_on_main(&app, move |target, _control_flow| {
        use tao::platform::macos::EventLoopWindowTargetExtMacOS;
        target.show_application();
        Ok(())
    })
    .await
}

#[cfg(target_os = "macos")]
async fn hide_other_applications(
    app: Arc<NivaApp>,
    _window: Arc<NivaWindow>,
    _request: ApiRequest,
) -> Result<()> {
    run_on_main(&app, move |target, _control_flow| {
        use tao::platform::macos::EventLoopWindowTargetExtMacOS;
        target.hide_other_applications();
        Ok(())
    })
    .await
}

#[cfg(target_os = "macos")]
async fn set_activation_policy(
    app: Arc<NivaApp>,
    _window: Arc<NivaWindow>,
    request: ApiRequest,
) -> Result<()> {
    run_on_main(&app, move |target, _control_flow| {
        let (policy,) = request.args().get::<(NivaActivationPolicy,)>()?;
        use crate::app::options::NivaActivationPolicy;
        use tao::platform::macos::{ActivationPolicy, EventLoopWindowTargetExtMacOS};

        let policy = match policy {
            NivaActivationPolicy::Regular => ActivationPolicy::Regular,
            NivaActivationPolicy::Accessory => ActivationPolicy::Accessory,
            NivaActivationPolicy::Prohibited => ActivationPolicy::Prohibited,
        };
        target.set_activation_policy_at_runtime(policy);
        Ok(())
    })
    .await
}

#[cfg(target_os = "macos")]
fn get_active_window_id(
    _app: Arc<NivaApp>,
    _window: Arc<NivaWindow>,
    _request: ApiRequest,
) -> Result<Option<String>> {
    match x_win::get_active_window() {
        Ok(window) => Ok(Some(format!("{}_{}", window.info.process_id, window.id))),
        Err(_) => Ok(None),
    }
}

#[cfg(target_os = "macos")]
async fn focus_by_window_id(
    app: Arc<NivaApp>,
    _window: Arc<NivaWindow>,
    request: ApiRequest,
) -> Result<bool> {
    let (id_string,) = request.args().get::<(String,)>()?;
    // NSRunningApplication is main-thread-only: hop to the event loop.
    run_on_main(&app, move |_, _| {
        use objc2_app_kit::{NSApplicationActivationOptions, NSRunningApplication};

        let result = id_string.split("_").collect::<Vec<&str>>();
        if result.len() != 2 {
            return Err(anyhow!("invalid window id"));
        }
        let process_id = result[0].parse::<u32>()?;

        let target: Option<objc2::rc::Retained<NSRunningApplication>> =
            NSRunningApplication::runningApplicationWithProcessIdentifier(process_id as _);
        match target {
            // NOTE: ActivateIgnoringOtherApps is deprecated since macOS 14
            // and has no effect there (system restricts programmatic
            // activation); kept for older systems, matching legacy behavior.
            Some(app) => Ok(app.activateWithOptions(
                #[allow(deprecated)]
                NSApplicationActivationOptions::ActivateIgnoringOtherApps,
            )),
            None => Ok(false),
        }
    })
    .await
}

#[cfg(target_os = "windows")]
fn get_active_window_id(
    _app: Arc<NivaApp>,
    _window: Arc<NivaWindow>,
    _request: ApiRequest,
) -> Result<String> {
    use windows::Win32::UI::WindowsAndMessaging::GetForegroundWindow;

    let hwnd = unsafe { GetForegroundWindow() };
    Ok((hwnd.0 as usize).to_string())
}

#[cfg(target_os = "windows")]
fn focus_by_window_id(
    _app: Arc<NivaApp>,
    _window: Arc<NivaWindow>,
    request: ApiRequest,
) -> Result<()> {
    let (hwnd_str,) = request.args().get::<(String,)>()?;
    use windows::Win32::{Foundation::HWND, UI::WindowsAndMessaging::SetForegroundWindow};

    let hwnd = HWND(hwnd_str.parse::<isize>()? as _);
    unsafe {
        SetForegroundWindow(hwnd);
    }
    Ok(())
}
