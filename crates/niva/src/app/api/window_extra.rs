use anyhow::Result;

#[cfg(target_os = "macos")]
use tao::platform::macos::{ActivationPolicy, EventLoopWindowTargetExtMacOS, WindowExtMacOS};

#[cfg(target_os = "windows")]
use tao::platform::windows::WindowExtWindows;

#[cfg(target_os = "windows")]
use tao::window::Theme;

use std::sync::Arc;

use crate::app::{
    NivaApp,
    api_manager::{ApiManager, ApiRequest},
    main_exec::run_on_main,
    window_manager::window::NivaWindow,
};

#[cfg(target_os = "macos")]
use crate::app::window_manager::options::NivaPosition;

macro_rules! match_window {
    ($app:ident, $window:ident, $id:ident) => {
        let $window = match $id {
            Some(id) => $app.window()?.get_window(id)?,
            None => $window,
        };
    };
}

pub fn register_api_instances(api_manager: &mut ApiManager) {
    #[cfg(target_os = "windows")]
    {
        api_manager.register_api("windowExtra.setEnable", set_enable);
        api_manager.register_blocking_api("windowExtra.setTaskbarIcon", set_taskbar_icon);
        api_manager.register_api("windowExtra.theme", theme);
        api_manager.register_api("windowExtra.resetDeadKeys", reset_dead_keys);
        api_manager.register_api("windowExtra.beginResizeDrag", begin_resize_drag);
        api_manager.register_api("windowExtra.setSkipTaskbar", set_skip_taskbar);
        api_manager.register_api("windowExtra.setUndecoratedShadow", set_undecorated_shadow);
        api_manager.register_api("windowExtra.setOverlayIcon", set_overlay_icon);
        api_manager.register_api("windowExtra.setRtl", set_rtl);
        api_manager.register_api("windowExtra.hasUndecoratedShadow", has_undecorated_shadow);
    }

    #[cfg(target_os = "macos")]
    {
        api_manager.register_api("windowExtra.simpleFullscreen", simple_fullscreen);
        api_manager.register_api("windowExtra.setSimpleFullscreen", set_simple_fullscreen);
        api_manager.register_api("windowExtra.hasShadow", has_shadow);
        api_manager.register_api("windowExtra.setHasShadow", set_has_shadow);
        api_manager.register_api("windowExtra.setIsDocumentEdited", set_is_document_edited);
        api_manager.register_api("windowExtra.isDocumentEdited", is_document_edited);
        api_manager.register_api(
            "windowExtra.setAllowsAutomaticWindowTabbing",
            set_allows_automatic_window_tabbing,
        );
        api_manager.register_api(
            "windowExtra.allowsAutomaticWindowTabbing",
            allows_automatic_window_tabbing,
        );
        api_manager.register_api("windowExtra.setTabbingIdentifier", set_tabbing_identifier);
        api_manager.register_api("windowExtra.tabbingIdentifier", tabbing_identifier);
        api_manager.register_api("windowExtra.setTrafficLightInset", set_traffic_light_inset);
        api_manager.register_api(
            "windowExtra.setActivationPolicyAtRuntime",
            set_activation_policy_at_runtime,
        );
        api_manager.register_api("windowExtra.setDockVisibility", set_dock_visibility);
        api_manager.register_api("windowExtra.setBadgeLabel", set_badge_label);
    }
}

#[cfg(target_os = "windows")]
async fn set_overlay_icon(
    app: Arc<NivaApp>,
    window: Arc<NivaWindow>,
    request: ApiRequest,
) -> Result<()> {
    let (icon_path, id) = request.args().optional::<(Option<String>, Option<u8>)>(2)?;
    let app2 = app.clone();
    run_on_main(&app, move |_target, _control_flow| {
        match_window!(app2, window, id);
        let icon = icon_path
            .as_deref()
            .map(|path| app2.resource().load_icon(path))
            .transpose()?;
        window.set_overlay_icon(icon.as_ref());
        Ok(())
    })
    .await
}

#[cfg(target_os = "windows")]
async fn set_rtl(app: Arc<NivaApp>, window: Arc<NivaWindow>, request: ApiRequest) -> Result<()> {
    let (rtl, id) = request.args().optional::<(bool, Option<u8>)>(2)?;
    let app2 = app.clone();
    run_on_main(&app, move |_target, _control_flow| {
        match_window!(app2, window, id);
        window.set_rtl(rtl);
        Ok(())
    })
    .await
}

#[cfg(target_os = "windows")]
async fn has_undecorated_shadow(
    app: Arc<NivaApp>,
    window: Arc<NivaWindow>,
    request: ApiRequest,
) -> Result<bool> {
    let (id,) = request.args().optional::<(Option<u8>,)>(1)?;
    let app2 = app.clone();
    run_on_main(&app, move |_target, _control_flow| {
        match_window!(app2, window, id);
        Ok(window.has_undecorated_shadow())
    })
    .await
}

#[cfg(target_os = "macos")]
async fn set_traffic_light_inset(
    app: Arc<NivaApp>,
    window: Arc<NivaWindow>,
    request: ApiRequest,
) -> Result<()> {
    let (position, id) = request.args().optional::<(NivaPosition, Option<u8>)>(2)?;
    let app2 = app.clone();
    run_on_main(&app, move |_target, _control_flow| {
        use wry::WebViewExtMacOS;

        match_window!(app2, window, id);
        window.set_traffic_light_inset(position);
        // Wry owns the replacement parent view. Its setter updates the
        // controls now and remembers the inset for later redraws.
        window
            .webview
            .set_traffic_light_inset(wry::dpi::LogicalPosition::new(position.x, position.y))?;
        Ok(())
    })
    .await
}

#[cfg(target_os = "macos")]
async fn set_activation_policy_at_runtime(
    app: Arc<NivaApp>,
    _window: Arc<NivaWindow>,
    request: ApiRequest,
) -> Result<()> {
    let (policy,) = request.args().optional::<(String,)>(1)?;
    let policy = match policy.as_str() {
        "regular" => ActivationPolicy::Regular,
        "accessory" => ActivationPolicy::Accessory,
        "prohibited" => ActivationPolicy::Prohibited,
        _ => return Err(anyhow::anyhow!("Unknown macOS activation policy: {policy}")),
    };
    run_on_main(&app, move |target, _control_flow| {
        target.set_activation_policy_at_runtime(policy);
        Ok(())
    })
    .await
}

#[cfg(target_os = "macos")]
async fn set_dock_visibility(
    app: Arc<NivaApp>,
    _window: Arc<NivaWindow>,
    request: ApiRequest,
) -> Result<()> {
    let (visible,) = request.args().optional::<(bool,)>(1)?;
    run_on_main(&app, move |target, _control_flow| {
        target.set_dock_visibility(visible);
        Ok(())
    })
    .await
}

#[cfg(target_os = "macos")]
async fn set_badge_label(
    app: Arc<NivaApp>,
    _window: Arc<NivaWindow>,
    request: ApiRequest,
) -> Result<()> {
    let (label,) = request.args().optional::<(Option<String>,)>(1)?;
    run_on_main(&app, move |target, _control_flow| {
        target.set_badge_label(label);
        Ok(())
    })
    .await
}

// windows
#[cfg(target_os = "windows")]
async fn set_enable(app: Arc<NivaApp>, window: Arc<NivaWindow>, request: ApiRequest) -> Result<()> {
    let (enabled, id) = request.args().optional::<(bool, Option<u8>)>(2)?;
    match_window!(app, window, id);
    window.set_enable(enabled);
    Ok(())
}

#[cfg(target_os = "windows")]
fn set_taskbar_icon(app: Arc<NivaApp>, window: Arc<NivaWindow>, request: ApiRequest) -> Result<()> {
    let (taskbar_icon, id) = request.args().optional::<(String, Option<u8>)>(2)?;
    match_window!(app, window, id);
    let taskbar_icon = app.resource().load_icon(&taskbar_icon)?;
    window.set_taskbar_icon(Some(taskbar_icon));
    Ok(())
}

#[cfg(target_os = "windows")]
async fn theme(app: Arc<NivaApp>, window: Arc<NivaWindow>, request: ApiRequest) -> Result<String> {
    let (id,) = request.args().optional::<(Option<u8>,)>(1)?;
    match_window!(app, window, id);
    match window.theme() {
        Theme::Dark => Ok("dark".to_string()),
        Theme::Light => Ok("light".to_string()),
        _ => Ok("system".to_string()),
    }
}

#[cfg(target_os = "windows")]
async fn reset_dead_keys(
    app: Arc<NivaApp>,
    window: Arc<NivaWindow>,
    request: ApiRequest,
) -> Result<()> {
    let (id,) = request.args().optional::<(Option<u8>,)>(1)?;
    match_window!(app, window, id);
    window.reset_dead_keys();
    Ok(())
}

#[cfg(target_os = "windows")]
async fn begin_resize_drag(
    app: Arc<NivaApp>,
    window: Arc<NivaWindow>,
    request: ApiRequest,
) -> Result<()> {
    let (edge, button, x, y, id) = request
        .args()
        .optional::<(isize, u32, i32, i32, Option<u8>)>(5)?;
    match_window!(app, window, id);
    window.begin_resize_drag(edge, button, x, y);
    Ok(())
}

#[cfg(target_os = "windows")]
async fn set_skip_taskbar(
    app: Arc<NivaApp>,
    window: Arc<NivaWindow>,
    request: ApiRequest,
) -> Result<()> {
    let (skip, id) = request.args().optional::<(bool, Option<u8>)>(2)?;
    match_window!(app, window, id);
    window.set_skip_taskbar(skip)?;
    Ok(())
}

#[cfg(target_os = "windows")]
async fn set_undecorated_shadow(
    app: Arc<NivaApp>,
    window: Arc<NivaWindow>,
    request: ApiRequest,
) -> Result<()> {
    let (shadow, id) = request.args().optional::<(bool, Option<u8>)>(2)?;
    match_window!(app, window, id);
    window.set_undecorated_shadow(shadow);
    Ok(())
}

#[cfg(target_os = "macos")]
async fn simple_fullscreen(
    app: Arc<NivaApp>,
    window: Arc<NivaWindow>,
    request: ApiRequest,
) -> Result<bool> {
    let (id,) = request.args().optional::<(Option<u8>,)>(1)?;
    match_window!(app, window, id);
    run_on_main(&app, move |_target, _control_flow| {
        Ok(window.simple_fullscreen())
    })
    .await
}

#[cfg(target_os = "macos")]
async fn set_simple_fullscreen(
    app: Arc<NivaApp>,
    window: Arc<NivaWindow>,
    request: ApiRequest,
) -> Result<bool> {
    let (fullscreen, id) = request.args().optional::<(bool, Option<u8>)>(2)?;
    match_window!(app, window, id);
    run_on_main(&app, move |_target, _control_flow| {
        Ok(window.set_simple_fullscreen(fullscreen))
    })
    .await
}

#[cfg(target_os = "macos")]
async fn has_shadow(
    app: Arc<NivaApp>,
    window: Arc<NivaWindow>,
    request: ApiRequest,
) -> Result<bool> {
    let (id,) = request.args().optional::<(Option<u8>,)>(1)?;
    match_window!(app, window, id);
    run_on_main(&app, move |_target, _control_flow| Ok(window.has_shadow())).await
}

#[cfg(target_os = "macos")]
async fn set_has_shadow(
    app: Arc<NivaApp>,
    window: Arc<NivaWindow>,
    request: ApiRequest,
) -> Result<()> {
    let (has_shadow, id) = request.args().optional::<(bool, Option<u8>)>(2)?;
    match_window!(app, window, id);
    run_on_main(&app, move |_target, _control_flow| {
        window.set_has_shadow(has_shadow);
        Ok(())
    })
    .await
}

#[cfg(target_os = "macos")]
async fn set_is_document_edited(
    app: Arc<NivaApp>,
    window: Arc<NivaWindow>,
    request: ApiRequest,
) -> Result<()> {
    let (edited, id) = request.args().optional::<(bool, Option<u8>)>(2)?;
    match_window!(app, window, id);
    run_on_main(&app, move |_target, _control_flow| {
        window.set_is_document_edited(edited);
        Ok(())
    })
    .await
}

#[cfg(target_os = "macos")]
async fn is_document_edited(
    app: Arc<NivaApp>,
    window: Arc<NivaWindow>,
    request: ApiRequest,
) -> Result<bool> {
    let (id,) = request.args().optional::<(Option<u8>,)>(1)?;
    match_window!(app, window, id);
    run_on_main(&app, move |_target, _control_flow| {
        Ok(window.is_document_edited())
    })
    .await
}

#[cfg(target_os = "macos")]
async fn set_allows_automatic_window_tabbing(
    app: Arc<NivaApp>,
    window: Arc<NivaWindow>,
    request: ApiRequest,
) -> Result<()> {
    let (enabled, id) = request.args().optional::<(bool, Option<u8>)>(2)?;
    match_window!(app, window, id);
    run_on_main(&app, move |_target, _control_flow| {
        window.set_allows_automatic_window_tabbing(enabled);
        Ok(())
    })
    .await
}

#[cfg(target_os = "macos")]
async fn allows_automatic_window_tabbing(
    app: Arc<NivaApp>,
    window: Arc<NivaWindow>,
    request: ApiRequest,
) -> Result<bool> {
    let (id,) = request.args().optional::<(Option<u8>,)>(1)?;
    match_window!(app, window, id);
    run_on_main(&app, move |_target, _control_flow| {
        Ok(window.allows_automatic_window_tabbing())
    })
    .await
}

#[cfg(target_os = "macos")]
async fn set_tabbing_identifier(
    app: Arc<NivaApp>,
    window: Arc<NivaWindow>,
    request: ApiRequest,
) -> Result<()> {
    let (identifier, id) = request.args().optional::<(String, Option<u8>)>(2)?;
    match_window!(app, window, id);
    run_on_main(&app, move |_target, _control_flow| {
        window.set_tabbing_identifier(&identifier);
        Ok(())
    })
    .await
}

#[cfg(target_os = "macos")]
async fn tabbing_identifier(
    app: Arc<NivaApp>,
    window: Arc<NivaWindow>,
    request: ApiRequest,
) -> Result<String> {
    let (id,) = request.args().optional::<(Option<u8>,)>(1)?;
    match_window!(app, window, id);
    run_on_main(&app, move |_target, _control_flow| {
        Ok(window.tabbing_identifier())
    })
    .await
}
