use anyhow::{Result, anyhow};
use serde::Deserialize;
use serde_json::{Value, json};

use tao::{
    event_loop::ControlFlow,
    window::{
        CursorIcon, Fullscreen, ProgressBarState, ProgressState, ResizeDirection, Theme,
        UserAttentionType,
    },
};

use std::sync::Arc;

use crate::{
    app::{
        NivaApp,
        api_manager::{ApiManager, ApiRequest},
        main_exec::run_on_main,
        window_manager::{
            WindowManager,
            options::{NivaPosition, NivaSize, NivaWindowOptions, WindowMenuOptions},
            window::NivaWindow,
        },
    },
    lock, logical, logical_try,
};

macro_rules! match_window {
    ($app:ident, $window:ident, $id:ident) => {
        let $window = match $id {
            Some(id) => $app.window()?.get_window(id)?,
            None => $window,
        };
    };
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ProgressBarOptions {
    state: Option<String>,
    progress: Option<u64>,
    desktop_filename: Option<String>,
}

fn parse_theme(theme: Option<&str>) -> Result<Option<Theme>> {
    match theme {
        None | Some("system") => Ok(None),
        Some("light") => Ok(Some(Theme::Light)),
        Some("dark") => Ok(Some(Theme::Dark)),
        Some(value) => Err(anyhow!("Unknown window theme: {value}")),
    }
}

fn parse_resize_direction(direction: &str) -> Result<ResizeDirection> {
    match direction {
        "east" => Ok(ResizeDirection::East),
        "north" => Ok(ResizeDirection::North),
        "northEast" => Ok(ResizeDirection::NorthEast),
        "northWest" => Ok(ResizeDirection::NorthWest),
        "south" => Ok(ResizeDirection::South),
        "southEast" => Ok(ResizeDirection::SouthEast),
        "southWest" => Ok(ResizeDirection::SouthWest),
        "west" => Ok(ResizeDirection::West),
        value => Err(anyhow!("Unknown window resize direction: {value}")),
    }
}

fn parse_progress_state(state: Option<&str>) -> Result<Option<ProgressState>> {
    match state {
        None => Ok(None),
        Some("none") => Ok(Some(ProgressState::None)),
        Some("normal") => Ok(Some(ProgressState::Normal)),
        Some("indeterminate") => Ok(Some(ProgressState::Indeterminate)),
        Some("paused") => Ok(Some(ProgressState::Paused)),
        Some("error") => Ok(Some(ProgressState::Error)),
        Some(value) => Err(anyhow!("Unknown window progress state: {value}")),
    }
}

fn parse_progress_bar(options: ProgressBarOptions) -> Result<ProgressBarState> {
    if options.progress.is_some_and(|value| value > 100) {
        return Err(anyhow!("Window progress must be between 0 and 100"));
    }
    Ok(ProgressBarState {
        state: parse_progress_state(options.state.as_deref())?,
        progress: options.progress,
        desktop_filename: options.desktop_filename,
    })
}

pub fn register_api_instances(api_manager: &mut ApiManager) {
    api_manager.register_api("window.current", current);
    api_manager.register_api("window.open", open);
    api_manager.register_api("window.close", close);
    api_manager.register_api("window.list", list);
    api_manager.register_api("window.sendMessage", send_message);
    api_manager.register_api("window.setMenu", set_menu);
    api_manager.register_api("window.hideMenu", hide_menu);
    api_manager.register_api("window.showMenu", show_menu);
    api_manager.register_api("window.isMenuVisible", is_menu_visible);
    api_manager.register_api("window.scaleFactor", scale_factor);
    api_manager.register_api("window.innerPosition", inner_position);
    api_manager.register_api("window.outerPosition", outer_position);
    api_manager.register_api("window.setOuterPosition", set_outer_position);
    api_manager.register_api("window.innerSize", inner_size);
    api_manager.register_api("window.setInnerSize", set_inner_size);
    api_manager.register_api("window.outerSize", outer_size);
    api_manager.register_api("window.setMinInnerSize", set_min_inner_size);
    api_manager.register_api("window.setMaxInnerSize", set_max_inner_size);
    api_manager.register_api("window.setWindowIcon", set_window_icon);
    api_manager.register_api("window.setTheme", set_theme);
    api_manager.register_api("window.dragResizeWindow", drag_resize_window);
    api_manager.register_api("window.setProgressBar", set_progress_bar);
    api_manager.register_api("window.requestRedraw", request_redraw);
    api_manager.register_api("window.setImePosition", set_ime_position);
    api_manager.register_api("window.setBackgroundColor", set_background_color);
    api_manager.register_api("window.setFocusable", set_focusable);
    api_manager.register_api("window.setTitle", set_title);
    api_manager.register_api("window.title", title);
    api_manager.register_api("window.isVisible", is_visible);
    api_manager.register_api("window.setVisible", set_visible);
    api_manager.register_api("window.isFocused", is_focused);
    api_manager.register_api("window.setFocus", set_focus);
    api_manager.register_api("window.isResizable", is_resizable);
    api_manager.register_api("window.setResizable", set_resizable);
    api_manager.register_api("window.isMinimizable", is_minimizable);
    api_manager.register_api("window.setMinimizable", set_minimizable);
    api_manager.register_api("window.isMaximizable", is_maximizable);
    api_manager.register_api("window.setMaximizable", set_maximizable);
    api_manager.register_api("window.isClosable", is_closable);
    api_manager.register_api("window.setClosable", set_closable);
    api_manager.register_api("window.isMinimized", is_minimized);
    api_manager.register_api("window.setMinimized", set_minimized);
    api_manager.register_api("window.isMaximized", is_maximized);
    api_manager.register_api("window.setMaximized", set_maximized);
    api_manager.register_api("window.isDecorated", decorated);
    api_manager.register_api("window.setDecorated", set_decorated);
    api_manager.register_api("window.fullscreen", fullscreen);
    api_manager.register_api("window.setFullscreen", set_fullscreen);
    api_manager.register_api("window.setAlwaysOnTop", set_always_on_top);
    api_manager.register_api("window.setAlwaysOnBottom", set_always_on_bottom);
    api_manager.register_api("window.requestUserAttention", request_user_attention);
    api_manager.register_api("window.setContentProtection", set_content_protection);
    api_manager.register_api(
        "window.setVisibleOnAllWorkspaces",
        set_visible_on_all_workspaces,
    );
    api_manager.register_api("window.setCursorIcon", set_cursor_icon);
    api_manager.register_api("window.cursorPosition", cursor_position);
    api_manager.register_api("window.setCursorPosition", set_cursor_position);
    api_manager.register_api("window.setCursorGrab", set_cursor_grab);
    api_manager.register_api("window.setCursorVisible", set_cursor_visible);
    api_manager.register_api("window.dragWindow", drag_window);
    api_manager.register_api("window.setIgnoreCursorEvents", set_ignore_cursor_events);
    api_manager.register_api("window.theme", theme);
    api_manager.register_api("window.blockCloseRequested", block_close_requested);
}

async fn current(_app: Arc<NivaApp>, window: Arc<NivaWindow>, _request: ApiRequest) -> Result<u8> {
    Ok(window.id)
}

async fn open(app: Arc<NivaApp>, _window: Arc<NivaWindow>, request: ApiRequest) -> Result<u8> {
    let app2 = app.clone();
    run_on_main(&app, move |target, _control_flow| {
        let (options,) = request.args().optional::<(Option<NivaWindowOptions>,)>(1)?;
        let new_window = app2
            .window()?
            .open_window(&options.unwrap_or_default(), target)?;
        Ok(new_window.id)
    })
    .await
}

async fn close(app: Arc<NivaApp>, window: Arc<NivaWindow>, request: ApiRequest) -> Result<()> {
    let app2 = app.clone();
    run_on_main(&app, move |_target, control_flow| {
        let (id,) = request.args().optional::<(Option<u8>,)>(1)?;
        let id = id.unwrap_or(window.id);
        if id == 0 {
            *control_flow = ControlFlow::Exit;
            return Ok(());
        }
        let closed = app2.window()?.close_window(id)?;
        WindowManager::cleanup_window(&app2, &closed)
    })
    .await
}

async fn list(
    app: Arc<NivaApp>,
    _window: Arc<NivaWindow>,
    _request: ApiRequest,
) -> Result<Vec<Value>> {
    Ok(app
        .window()?
        .list_windows()
        .into_iter()
        .map(|w| json!({"id":w.id,"title":w.title(),"visible":w.is_visible(),}))
        .collect())
}

async fn send_message(
    app: Arc<NivaApp>,
    window: Arc<NivaWindow>,
    request: ApiRequest,
) -> Result<()> {
    let (message, id) = request.args().get::<(String, u8)>()?;
    let remote = app.window()?.get_window(id)?;
    remote.send_ipc_event(
        "window.message",
        json!({"from":window.id,"message":message,}),
    );
    Ok(())
}

async fn set_menu(app: Arc<NivaApp>, window: Arc<NivaWindow>, request: ApiRequest) -> Result<()> {
    let app2 = app.clone();
    run_on_main(&app, move |_target, _control_flow| {
        let (options, id) = request
            .args()
            .optional::<(Option<WindowMenuOptions>, Option<u8>)>(2)?;
        match_window!(app2, window, id);
        window.set_menu(&options);
        Ok(())
    })
    .await
}

async fn hide_menu(app: Arc<NivaApp>, window: Arc<NivaWindow>, request: ApiRequest) -> Result<()> {
    let app2 = app.clone();
    run_on_main(&app, move |_target, _control_flow| {
        let (id,) = request.args().optional::<(Option<u8>,)>(1)?;
        match_window!(app2, window, id);
        window.hide_menu();
        Ok(())
    })
    .await
}

async fn show_menu(app: Arc<NivaApp>, window: Arc<NivaWindow>, request: ApiRequest) -> Result<()> {
    let app2 = app.clone();
    run_on_main(&app, move |_target, _control_flow| {
        let (id,) = request.args().optional::<(Option<u8>,)>(1)?;
        match_window!(app2, window, id);
        window.show_menu();
        Ok(())
    })
    .await
}

async fn is_menu_visible(
    app: Arc<NivaApp>,
    window: Arc<NivaWindow>,
    request: ApiRequest,
) -> Result<bool> {
    let (id,) = request.args().optional::<(Option<u8>,)>(1)?;
    match_window!(app, window, id);
    Ok(window.is_menu_visible())
}

async fn scale_factor(
    app: Arc<NivaApp>,
    window: Arc<NivaWindow>,
    request: ApiRequest,
) -> Result<f64> {
    let (id,) = request.args().optional::<(Option<u8>,)>(1)?;
    match_window!(app, window, id);
    Ok(window.scale_factor())
}

async fn inner_position(
    app: Arc<NivaApp>,
    window: Arc<NivaWindow>,
    request: ApiRequest,
) -> Result<NivaPosition> {
    let (id,) = request.args().optional::<(Option<u8>,)>(1)?;
    match_window!(app, window, id);
    Ok(logical_try!(window, inner_position))
}

async fn outer_position(
    app: Arc<NivaApp>,
    window: Arc<NivaWindow>,
    request: ApiRequest,
) -> Result<NivaPosition> {
    let (id,) = request.args().optional::<(Option<u8>,)>(1)?;
    match_window!(app, window, id);
    Ok(logical_try!(window, outer_position))
}

async fn set_outer_position(
    app: Arc<NivaApp>,
    window: Arc<NivaWindow>,
    request: ApiRequest,
) -> Result<()> {
    let (position, id) = request.args().optional::<(NivaPosition, Option<u8>)>(2)?;
    match_window!(app, window, id);
    window.set_outer_position(position);
    Ok(())
}

async fn inner_size(
    app: Arc<NivaApp>,
    window: Arc<NivaWindow>,
    request: ApiRequest,
) -> Result<NivaSize> {
    let (id,) = request.args().optional::<(Option<u8>,)>(1)?;
    match_window!(app, window, id);
    #[cfg(target_os = "macos")]
    {
        // Wry replaces Tao's original NSView with its own content view. Tao's
        // inner_size reads the detached view and keeps returning the startup
        // size after a resize. Read the live NSWindow content rect instead.
        return run_on_main(&app, move |_target, _control_flow| {
            use objc2_app_kit::NSWindow;
            use tao::platform::macos::WindowExtMacOS;

            let pointer = window.ns_window().cast::<NSWindow>();
            let native = unsafe { pointer.as_ref() }
                .ok_or_else(|| anyhow!("window has no native NSWindow"))?;
            let content = native.contentRectForFrameRect(native.frame());
            Ok(NivaSize::new(content.size.width, content.size.height))
        })
        .await;
    }
    #[cfg(not(target_os = "macos"))]
    {
        Ok(logical!(window, inner_size))
    }
}

async fn set_inner_size(
    app: Arc<NivaApp>,
    window: Arc<NivaWindow>,
    request: ApiRequest,
) -> Result<()> {
    let (size, id) = request.args().optional::<(NivaSize, Option<u8>)>(2)?;
    match_window!(app, window, id);
    #[cfg(target_os = "macos")]
    {
        // Tao defers setContentSize to GCD even on the event-loop thread. In
        // this app that dispatch can remain queued while the window is live,
        // so the API resolves without changing its size. Execute the Cocoa
        // setter synchronously on the main event loop instead.
        return run_on_main(&app, move |_target, _control_flow| {
            use objc2_app_kit::NSWindow;
            use objc2_foundation::NSSize;
            use tao::platform::macos::WindowExtMacOS;

            let pointer = window.ns_window().cast::<NSWindow>();
            // The Arc<NivaWindow> captured above owns this NSWindow until the
            // main-thread closure returns.
            let native = unsafe { pointer.as_ref() }
                .ok_or_else(|| anyhow!("window has no native NSWindow"))?;
            native.setContentSize(NSSize::new(size.width, size.height));
            Ok(())
        })
        .await;
    }
    #[cfg(not(target_os = "macos"))]
    {
        window.set_inner_size(size);
        Ok(())
    }
}

async fn outer_size(
    app: Arc<NivaApp>,
    window: Arc<NivaWindow>,
    request: ApiRequest,
) -> Result<NivaSize> {
    let (id,) = request.args().optional::<(Option<u8>,)>(1)?;
    match_window!(app, window, id);
    Ok(logical!(window, outer_size))
}

async fn set_min_inner_size(
    app: Arc<NivaApp>,
    window: Arc<NivaWindow>,
    request: ApiRequest,
) -> Result<()> {
    if cfg!(any(target_os = "ios", target_os = "android")) {
        return Err(anyhow!(
            "window.setMinInnerSize is unsupported on this platform"
        ));
    }
    let (size, id) = request
        .args()
        .optional::<(Option<NivaSize>, Option<u8>)>(2)?;
    match_window!(app, window, id);
    #[cfg(target_os = "macos")]
    {
        return run_on_main(&app, move |_target, _control_flow| {
            window.set_min_inner_size(size);
            Ok(())
        })
        .await;
    }
    #[cfg(not(target_os = "macos"))]
    {
        window.set_min_inner_size(size);
        Ok(())
    }
}

async fn set_max_inner_size(
    app: Arc<NivaApp>,
    window: Arc<NivaWindow>,
    request: ApiRequest,
) -> Result<()> {
    if cfg!(any(target_os = "ios", target_os = "android")) {
        return Err(anyhow!(
            "window.setMaxInnerSize is unsupported on this platform"
        ));
    }
    let (size, id) = request
        .args()
        .optional::<(Option<NivaSize>, Option<u8>)>(2)?;
    match_window!(app, window, id);
    #[cfg(target_os = "macos")]
    {
        return run_on_main(&app, move |_target, _control_flow| {
            window.set_max_inner_size(size);
            Ok(())
        })
        .await;
    }
    #[cfg(not(target_os = "macos"))]
    {
        window.set_max_inner_size(size);
        Ok(())
    }
}

async fn set_window_icon(
    app: Arc<NivaApp>,
    window: Arc<NivaWindow>,
    request: ApiRequest,
) -> Result<()> {
    if cfg!(any(
        target_os = "macos",
        target_os = "ios",
        target_os = "android"
    )) {
        return Err(anyhow!(
            "window.setWindowIcon is unsupported on this platform"
        ));
    }
    let (icon_path, id) = request.args().optional::<(Option<String>, Option<u8>)>(2)?;
    let app2 = app.clone();
    run_on_main(&app, move |_target, _control_flow| {
        match_window!(app2, window, id);
        let icon = icon_path
            .as_deref()
            .map(|path| app2.resource().load_icon(path))
            .transpose()?;
        window.set_window_icon(icon);
        Ok(())
    })
    .await
}

async fn set_theme(app: Arc<NivaApp>, window: Arc<NivaWindow>, request: ApiRequest) -> Result<()> {
    if cfg!(any(target_os = "ios", target_os = "android")) {
        return Err(anyhow!("window.setTheme is unsupported on this platform"));
    }
    let (theme, id) = request.args().optional::<(Option<String>, Option<u8>)>(2)?;
    let theme = parse_theme(theme.as_deref())?;
    let app2 = app.clone();
    run_on_main(&app, move |_target, _control_flow| {
        match_window!(app2, window, id);
        window.set_theme(theme);
        Ok(())
    })
    .await
}

async fn drag_resize_window(
    app: Arc<NivaApp>,
    window: Arc<NivaWindow>,
    request: ApiRequest,
) -> Result<()> {
    let (direction, id) = request.args().optional::<(String, Option<u8>)>(2)?;
    let direction = parse_resize_direction(&direction)?;
    let app2 = app.clone();
    run_on_main(&app, move |_target, _control_flow| {
        match_window!(app2, window, id);
        window.drag_resize_window(direction)?;
        Ok(())
    })
    .await
}

async fn set_progress_bar(
    app: Arc<NivaApp>,
    window: Arc<NivaWindow>,
    request: ApiRequest,
) -> Result<()> {
    if cfg!(any(target_os = "ios", target_os = "android")) {
        return Err(anyhow!(
            "window.setProgressBar is unsupported on this platform"
        ));
    }
    let (options, id) = request
        .args()
        .optional::<(ProgressBarOptions, Option<u8>)>(2)?;
    let progress = parse_progress_bar(options)?;
    let app2 = app.clone();
    run_on_main(&app, move |_target, _control_flow| {
        match_window!(app2, window, id);
        window.set_progress_bar(progress);
        Ok(())
    })
    .await
}

async fn request_redraw(
    app: Arc<NivaApp>,
    window: Arc<NivaWindow>,
    request: ApiRequest,
) -> Result<()> {
    if cfg!(target_os = "android") {
        return Err(anyhow!("window.requestRedraw is unsupported on Android"));
    }
    let (id,) = request.args().optional::<(Option<u8>,)>(1)?;
    let app2 = app.clone();
    run_on_main(&app, move |_target, _control_flow| {
        match_window!(app2, window, id);
        window.request_redraw();
        Ok(())
    })
    .await
}

async fn set_ime_position(
    app: Arc<NivaApp>,
    window: Arc<NivaWindow>,
    request: ApiRequest,
) -> Result<()> {
    if cfg!(any(
        target_os = "linux",
        target_os = "ios",
        target_os = "android"
    )) {
        return Err(anyhow!(
            "window.setImePosition is unsupported on this platform"
        ));
    }
    let (position, id) = request.args().optional::<(NivaPosition, Option<u8>)>(2)?;
    let app2 = app.clone();
    run_on_main(&app, move |_target, _control_flow| {
        match_window!(app2, window, id);
        window.set_ime_position(position);
        Ok(())
    })
    .await
}

async fn set_background_color(
    app: Arc<NivaApp>,
    window: Arc<NivaWindow>,
    request: ApiRequest,
) -> Result<()> {
    if cfg!(any(target_os = "ios", target_os = "android")) {
        return Err(anyhow!(
            "window.setBackgroundColor is unsupported on this platform"
        ));
    }
    let (color, id) = request
        .args()
        .optional::<(Option<(u8, u8, u8, u8)>, Option<u8>)>(2)?;
    let app2 = app.clone();
    run_on_main(&app, move |_target, _control_flow| {
        match_window!(app2, window, id);
        window.set_background_color(color);
        Ok(())
    })
    .await
}

async fn set_focusable(
    app: Arc<NivaApp>,
    window: Arc<NivaWindow>,
    request: ApiRequest,
) -> Result<()> {
    if cfg!(any(target_os = "ios", target_os = "android")) {
        return Err(anyhow!(
            "window.setFocusable is unsupported on this platform"
        ));
    }
    let (focusable, id) = request.args().optional::<(bool, Option<u8>)>(2)?;
    let app2 = app.clone();
    run_on_main(&app, move |_target, _control_flow| {
        match_window!(app2, window, id);
        window.set_focusable(focusable);
        Ok(())
    })
    .await
}

async fn set_title(app: Arc<NivaApp>, window: Arc<NivaWindow>, request: ApiRequest) -> Result<()> {
    let (title, id) = request.args().optional::<(String, Option<u8>)>(2)?;
    match_window!(app, window, id);
    window.set_title(&title);
    Ok(())
}

async fn title(app: Arc<NivaApp>, window: Arc<NivaWindow>, request: ApiRequest) -> Result<String> {
    let (id,) = request.args().optional::<(Option<u8>,)>(1)?;
    match_window!(app, window, id);
    Ok(window.title())
}

async fn is_visible(
    app: Arc<NivaApp>,
    window: Arc<NivaWindow>,
    request: ApiRequest,
) -> Result<bool> {
    let (id,) = request.args().optional::<(Option<u8>,)>(1)?;
    match_window!(app, window, id);
    Ok(window.is_visible())
}

async fn set_visible(
    app: Arc<NivaApp>,
    window: Arc<NivaWindow>,
    request: ApiRequest,
) -> Result<()> {
    let (visible, id) = request.args().optional::<(bool, Option<u8>)>(2)?;
    match_window!(app, window, id);
    window.set_visible(visible);
    Ok(())
}

async fn is_focused(
    app: Arc<NivaApp>,
    window: Arc<NivaWindow>,
    request: ApiRequest,
) -> Result<bool> {
    let (id,) = request.args().optional::<(Option<u8>,)>(1)?;
    match_window!(app, window, id);
    Ok(window.is_focused())
}

async fn set_focus(app: Arc<NivaApp>, window: Arc<NivaWindow>, request: ApiRequest) -> Result<()> {
    let (id,) = request.args().optional::<(Option<u8>,)>(1)?;
    match_window!(app, window, id);
    window.set_focus();
    Ok(())
}

async fn is_resizable(
    app: Arc<NivaApp>,
    window: Arc<NivaWindow>,
    request: ApiRequest,
) -> Result<bool> {
    let (id,) = request.args().optional::<(Option<u8>,)>(1)?;
    match_window!(app, window, id);
    Ok(window.is_resizable())
}

async fn set_resizable(
    app: Arc<NivaApp>,
    window: Arc<NivaWindow>,
    request: ApiRequest,
) -> Result<()> {
    let (resizable, id) = request.args().optional::<(bool, Option<u8>)>(2)?;
    match_window!(app, window, id);
    window.set_resizable(resizable);
    Ok(())
}

async fn is_minimizable(
    app: Arc<NivaApp>,
    window: Arc<NivaWindow>,
    request: ApiRequest,
) -> Result<bool> {
    let (id,) = request.args().optional::<(Option<u8>,)>(1)?;
    match_window!(app, window, id);
    Ok(window.is_minimizable())
}

async fn set_minimizable(
    app: Arc<NivaApp>,
    window: Arc<NivaWindow>,
    request: ApiRequest,
) -> Result<()> {
    let (minimizable, id) = request.args().optional::<(bool, Option<u8>)>(2)?;
    match_window!(app, window, id);
    window.set_minimizable(minimizable);
    Ok(())
}

async fn is_maximizable(
    app: Arc<NivaApp>,
    window: Arc<NivaWindow>,
    request: ApiRequest,
) -> Result<bool> {
    let (id,) = request.args().optional::<(Option<u8>,)>(1)?;
    match_window!(app, window, id);
    Ok(window.is_maximizable())
}

async fn set_maximizable(
    app: Arc<NivaApp>,
    window: Arc<NivaWindow>,
    request: ApiRequest,
) -> Result<()> {
    let (maximizable, id) = request.args().optional::<(bool, Option<u8>)>(2)?;
    match_window!(app, window, id);
    window.set_maximizable(maximizable);
    Ok(())
}

async fn is_closable(
    app: Arc<NivaApp>,
    window: Arc<NivaWindow>,
    request: ApiRequest,
) -> Result<bool> {
    let (id,) = request.args().optional::<(Option<u8>,)>(1)?;
    match_window!(app, window, id);
    Ok(window.is_closable())
}

async fn set_closable(
    app: Arc<NivaApp>,
    window: Arc<NivaWindow>,
    request: ApiRequest,
) -> Result<()> {
    let (closable, id) = request.args().optional::<(bool, Option<u8>)>(2)?;
    match_window!(app, window, id);
    window.set_closable(closable);
    Ok(())
}

async fn is_minimized(
    app: Arc<NivaApp>,
    window: Arc<NivaWindow>,
    request: ApiRequest,
) -> Result<bool> {
    let (id,) = request.args().optional::<(Option<u8>,)>(1)?;
    match_window!(app, window, id);
    Ok(window.is_minimized())
}

async fn set_minimized(
    app: Arc<NivaApp>,
    window: Arc<NivaWindow>,
    request: ApiRequest,
) -> Result<()> {
    let (minimized, id) = request.args().optional::<(bool, Option<u8>)>(2)?;
    match_window!(app, window, id);
    #[cfg(target_os = "macos")]
    {
        return run_on_main(&app, move |_target, _control_flow| {
            window.set_minimized(minimized);
            Ok(())
        })
        .await;
    }
    #[cfg(not(target_os = "macos"))]
    {
        window.set_minimized(minimized);
        Ok(())
    }
}

async fn is_maximized(
    app: Arc<NivaApp>,
    window: Arc<NivaWindow>,
    request: ApiRequest,
) -> Result<bool> {
    let (id,) = request.args().optional::<(Option<u8>,)>(1)?;
    match_window!(app, window, id);
    Ok(window.is_maximized())
}

async fn set_maximized(
    app: Arc<NivaApp>,
    window: Arc<NivaWindow>,
    request: ApiRequest,
) -> Result<()> {
    let (maximized, id) = request.args().optional::<(bool, Option<u8>)>(2)?;
    match_window!(app, window, id);
    window.set_maximized(maximized);
    Ok(())
}

async fn decorated(
    app: Arc<NivaApp>,
    window: Arc<NivaWindow>,
    request: ApiRequest,
) -> Result<bool> {
    let (id,) = request.args().optional::<(Option<u8>,)>(1)?;
    match_window!(app, window, id);
    Ok(window.is_decorated())
}

async fn set_decorated(
    app: Arc<NivaApp>,
    window: Arc<NivaWindow>,
    request: ApiRequest,
) -> Result<()> {
    let (decorated, id) = request.args().optional::<(bool, Option<u8>)>(2)?;
    match_window!(app, window, id);
    window.set_decorations(decorated);
    Ok(())
}

async fn fullscreen(
    app: Arc<NivaApp>,
    window: Arc<NivaWindow>,
    request: ApiRequest,
) -> Result<bool> {
    let (id,) = request.args().optional::<(Option<u8>,)>(1)?;
    match_window!(app, window, id);
    Ok(window.fullscreen().is_some())
}

async fn set_fullscreen(
    app: Arc<NivaApp>,
    window: Arc<NivaWindow>,
    request: ApiRequest,
) -> Result<()> {
    let (is_fullscreen, monitor_name, id) = request
        .args()
        .optional::<(bool, Option<String>, Option<u8>)>(3)?;
    match_window!(app, window, id);
    if !is_fullscreen {
        window.set_fullscreen(None);
        return Ok(());
    }
    match monitor_name {
        Some(name) => {
            let monitor = window
                .available_monitors()
                .find(|m| m.name() == Some(name.clone()));
            if monitor.is_none() {
                return Err(anyhow!("Monitornotfound"));
            }
            window.set_fullscreen(Some(Fullscreen::Borderless(monitor)));
        }
        None => {
            window.set_fullscreen(Some(Fullscreen::Borderless(None)));
        }
    };
    Ok(())
}

async fn set_always_on_top(
    app: Arc<NivaApp>,
    window: Arc<NivaWindow>,
    request: ApiRequest,
) -> Result<()> {
    let (always_on_top, id) = request.args().optional::<(bool, Option<u8>)>(2)?;
    match_window!(app, window, id);
    window.set_always_on_top(always_on_top);
    Ok(())
}

async fn set_always_on_bottom(
    app: Arc<NivaApp>,
    window: Arc<NivaWindow>,
    request: ApiRequest,
) -> Result<()> {
    let (always_on_bottom, id) = request.args().optional::<(bool, Option<u8>)>(2)?;
    match_window!(app, window, id);
    window.set_always_on_bottom(always_on_bottom);
    Ok(())
}

async fn request_user_attention(
    app: Arc<NivaApp>,
    window: Arc<NivaWindow>,
    request: ApiRequest,
) -> Result<()> {
    let (level, id) = request.args().optional::<(String, Option<u8>)>(2)?;
    match_window!(app, window, id);
    match level.as_str() {
        "informational" => window.request_user_attention(Some(UserAttentionType::Informational)),
        "critical" => window.request_user_attention(Some(UserAttentionType::Critical)),
        _ => window.request_user_attention(None),
    }
    Ok(())
}

async fn set_content_protection(
    app: Arc<NivaApp>,
    window: Arc<NivaWindow>,
    request: ApiRequest,
) -> Result<()> {
    let (enabled, id) = request.args().optional::<(bool, Option<u8>)>(2)?;
    match_window!(app, window, id);
    #[cfg(target_os = "macos")]
    {
        return run_on_main(&app, move |_target, _control_flow| {
            window.set_content_protection(enabled);
            Ok(())
        })
        .await;
    }
    #[cfg(not(target_os = "macos"))]
    {
        window.set_content_protection(enabled);
        Ok(())
    }
}

async fn set_visible_on_all_workspaces(
    app: Arc<NivaApp>,
    window: Arc<NivaWindow>,
    request: ApiRequest,
) -> Result<()> {
    let (visible, id) = request.args().optional::<(bool, Option<u8>)>(2)?;
    match_window!(app, window, id);
    #[cfg(target_os = "macos")]
    {
        return run_on_main(&app, move |_target, _control_flow| {
            window.set_visible_on_all_workspaces(visible);
            Ok(())
        })
        .await;
    }
    #[cfg(not(target_os = "macos"))]
    {
        window.set_visible_on_all_workspaces(visible);
        Ok(())
    }
}

async fn set_cursor_icon(
    app: Arc<NivaApp>,
    window: Arc<NivaWindow>,
    request: ApiRequest,
) -> Result<()> {
    let (icon, id) = request.args().optional::<(String, Option<u8>)>(2)?;
    match_window!(app, window, id);
    let native_icon = match icon.as_str() {
        "default" => CursorIcon::Default,
        "crosshair" => CursorIcon::Crosshair,
        "hand" => CursorIcon::Hand,
        "arrow" => CursorIcon::Arrow,
        "move" => CursorIcon::Move,
        "text" => CursorIcon::Text,
        "wait" => CursorIcon::Wait,
        "help" => CursorIcon::Help,
        "progress" => CursorIcon::Progress,
        "not_allowed" => CursorIcon::NotAllowed,
        "context_menu" => CursorIcon::ContextMenu,
        "cell" => CursorIcon::Cell,
        "vertical_text" => CursorIcon::VerticalText,
        "alias" => CursorIcon::Alias,
        "copy" => CursorIcon::Copy,
        "no_drop" => CursorIcon::NoDrop,
        "grab" => CursorIcon::Grab,
        "grabbing" => CursorIcon::Grabbing,
        "all_scroll" => CursorIcon::AllScroll,
        "zoom_in" => CursorIcon::ZoomIn,
        "zoom_out" => CursorIcon::ZoomOut,
        "e_resize" => CursorIcon::EResize,
        "n_resize" => CursorIcon::NResize,
        "ne_resize" => CursorIcon::NeResize,
        "nw_resize" => CursorIcon::NwResize,
        "s_resize" => CursorIcon::SResize,
        "se_resize" => CursorIcon::SeResize,
        "sw_resize" => CursorIcon::SwResize,
        "w_resize" => CursorIcon::WResize,
        "ew_resize" => CursorIcon::EwResize,
        "ns_resize" => CursorIcon::NsResize,
        "nesw_resize" => CursorIcon::NeswResize,
        "nwse_resize" => CursorIcon::NwseResize,
        "col_resize" => CursorIcon::ColResize,
        "row_resize" => CursorIcon::RowResize,
        _ => CursorIcon::Arrow,
    };
    #[cfg(target_os = "macos")]
    {
        let css_icon = match icon.as_str() {
            "hand" => "pointer",
            "not_allowed" => "not-allowed",
            "context_menu" => "context-menu",
            "vertical_text" => "vertical-text",
            "no_drop" => "no-drop",
            "all_scroll" => "all-scroll",
            "zoom_in" => "zoom-in",
            "zoom_out" => "zoom-out",
            "e_resize" => "e-resize",
            "n_resize" => "n-resize",
            "ne_resize" => "ne-resize",
            "nw_resize" => "nw-resize",
            "s_resize" => "s-resize",
            "se_resize" => "se-resize",
            "sw_resize" => "sw-resize",
            "w_resize" => "w-resize",
            "ew_resize" => "ew-resize",
            "ns_resize" => "ns-resize",
            "nesw_resize" => "nesw-resize",
            "nwse_resize" => "nwse-resize",
            "col_resize" => "col-resize",
            "row_resize" => "row-resize",
            "default" | "arrow" => "default",
            "move" => "move",
            "text" => "text",
            "wait" => "wait",
            "help" => "help",
            "progress" => "progress",
            "cell" => "cell",
            "alias" => "alias",
            "copy" => "copy",
            "grab" => "grab",
            "grabbing" => "grabbing",
            "crosshair" => "crosshair",
            _ => "default",
        };
        return run_on_main(&app, move |_target, _control_flow| {
            window.set_cursor_icon(native_icon);
            update_macos_webview_cursor(&window, Some(css_icon), None)
        })
        .await;
    }
    #[cfg(not(target_os = "macos"))]
    {
        window.set_cursor_icon(native_icon);
        Ok(())
    }
}

#[cfg(target_os = "macos")]
fn update_macos_webview_cursor(
    window: &Arc<NivaWindow>,
    icon: Option<&str>,
    visible: Option<bool>,
) -> Result<()> {
    let icon = serde_json::to_string(&icon)?;
    let visible = serde_json::to_string(&visible)?;
    let script = format!(
        r#"(function() {{
          var state = window.__nivaCursorState || (window.__nivaCursorState = {{icon: 'default', visible: true}});
          var icon = {icon};
          var visible = {visible};
          if (icon !== null) state.icon = icon;
          if (visible !== null) state.visible = visible;
          var style = document.getElementById('__niva_cursor_style');
          if (!style) {{
            style = document.createElement('style');
            style.id = '__niva_cursor_style';
            (document.head || document.documentElement).appendChild(style);
          }}
          style.textContent = '* {{ cursor: ' + (state.visible ? state.icon : 'none') + ' !important; }}';
        }})();"#
    );
    window.webview.evaluate_script(&script)?;
    Ok(())
}

async fn cursor_position(
    app: Arc<NivaApp>,
    window: Arc<NivaWindow>,
    request: ApiRequest,
) -> Result<NivaPosition> {
    let (id,) = request.args().optional::<(Option<u8>,)>(1)?;
    match_window!(app, window, id);
    Ok(logical_try!(window, cursor_position))
}

async fn set_cursor_position(
    app: Arc<NivaApp>,
    window: Arc<NivaWindow>,
    request: ApiRequest,
) -> Result<()> {
    let (position, id) = request.args().optional::<(NivaPosition, Option<u8>)>(2)?;
    match_window!(app, window, id);
    window.set_cursor_position(position)?;
    Ok(())
}

async fn set_cursor_grab(
    app: Arc<NivaApp>,
    window: Arc<NivaWindow>,
    request: ApiRequest,
) -> Result<()> {
    let (grab, id) = request.args().optional::<(bool, Option<u8>)>(2)?;
    match_window!(app, window, id);
    window.set_cursor_grab(grab)?;
    Ok(())
}

async fn set_cursor_visible(
    app: Arc<NivaApp>,
    window: Arc<NivaWindow>,
    request: ApiRequest,
) -> Result<()> {
    let (visible, id) = request.args().optional::<(bool, Option<u8>)>(2)?;
    match_window!(app, window, id);
    #[cfg(target_os = "macos")]
    {
        return run_on_main(&app, move |_target, _control_flow| {
            window.set_cursor_visible(visible);
            update_macos_webview_cursor(&window, None, Some(visible))
        })
        .await;
    }
    #[cfg(not(target_os = "macos"))]
    {
        window.set_cursor_visible(visible);
        Ok(())
    }
}

async fn drag_window(
    app: Arc<NivaApp>,
    window: Arc<NivaWindow>,
    request: ApiRequest,
) -> Result<()> {
    let (id,) = request.args().optional::<(Option<u8>,)>(1)?;
    match_window!(app, window, id);
    #[cfg(target_os = "macos")]
    {
        drag_macos_window(&app, window).await
    }
    #[cfg(not(target_os = "macos"))]
    {
        run_on_main(&app, move |_target, _control_flow| {
            window.drag_window()?;
            Ok(())
        })
        .await
    }
}

#[cfg(target_os = "macos")]
async fn drag_macos_window(app: &Arc<NivaApp>, window: Arc<NivaWindow>) -> Result<()> {
    use objc2_app_kit::{NSEvent, NSWindow};
    use tao::platform::macos::WindowExtMacOS;

    // WebView calls cross the WebSocket bridge before reaching AppKit. By
    // then NSApp.currentEvent is the bridge's application event, not the
    // original mouseDown, so performWindowDragWithEvent silently does nothing.
    // Track the held button asynchronously; each native frame update happens
    // on the event-loop thread, which remains free between updates.
    let initial_window = window.clone();
    let (start_x, start_y, origin_x, origin_y) = run_on_main(app, move |_target, _control_flow| {
        let pointer = initial_window.ns_window().cast::<NSWindow>();
        let native =
            unsafe { pointer.as_ref() }.ok_or_else(|| anyhow!("window has no native NSWindow"))?;
        if NSEvent::pressedMouseButtons() & 1 == 0 {
            return Err(anyhow!(
                "window.dragWindow requires a pressed left mouse button"
            ));
        }
        let start = NSEvent::mouseLocation();
        let origin = native.frame().origin;
        Ok((start.x, start.y, origin.x, origin.y))
    })
    .await?;
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    loop {
        let step_window = window.clone();
        let held = run_on_main(app, move |_target, _control_flow| {
            let pointer = step_window.ns_window().cast::<NSWindow>();
            let native = unsafe { pointer.as_ref() }
                .ok_or_else(|| anyhow!("window has no native NSWindow"))?;
            let current = NSEvent::mouseLocation();
            native.setFrameOrigin(objc2_foundation::NSPoint::new(
                origin_x + current.x - start_x,
                origin_y + current.y - start_y,
            ));
            Ok(NSEvent::pressedMouseButtons() & 1 != 0)
        })
        .await?;
        if !held || std::time::Instant::now() >= deadline {
            break;
        }
        smol::Timer::after(std::time::Duration::from_millis(8)).await;
    }
    Ok(())
}

async fn set_ignore_cursor_events(
    app: Arc<NivaApp>,
    window: Arc<NivaWindow>,
    request: ApiRequest,
) -> Result<()> {
    let (ignore, id) = request.args().optional::<(bool, Option<u8>)>(2)?;
    match_window!(app, window, id);
    window.set_ignore_cursor_events(ignore)?;
    Ok(())
}

async fn theme(app: Arc<NivaApp>, window: Arc<NivaWindow>, request: ApiRequest) -> Result<String> {
    let (id,) = request.args().optional::<(Option<u8>,)>(1)?;
    match_window!(app, window, id);
    Ok(String::from(match window.theme() {
        Theme::Light => "light",
        Theme::Dark => "dark",
        _ => "system",
    }))
}

async fn block_close_requested(
    app: Arc<NivaApp>,
    window: Arc<NivaWindow>,
    request: ApiRequest,
) -> Result<()> {
    let (blocked, id) = request.args().optional::<(bool, Option<u8>)>(2)?;
    match_window!(app, window, id);
    let mut state = lock!(window.state)?;
    state.is_block_closed_requested = blocked;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_supported_runtime_themes() {
        assert_eq!(parse_theme(Some("light")).unwrap(), Some(Theme::Light));
        assert_eq!(parse_theme(Some("dark")).unwrap(), Some(Theme::Dark));
        assert_eq!(parse_theme(Some("system")).unwrap(), None);
        assert!(parse_theme(Some("sepia")).is_err());
    }

    #[test]
    fn parses_each_supported_resize_direction() {
        for (name, expected) in [
            ("east", ResizeDirection::East),
            ("north", ResizeDirection::North),
            ("northEast", ResizeDirection::NorthEast),
            ("northWest", ResizeDirection::NorthWest),
            ("south", ResizeDirection::South),
            ("southEast", ResizeDirection::SouthEast),
            ("southWest", ResizeDirection::SouthWest),
            ("west", ResizeDirection::West),
        ] {
            assert_eq!(parse_resize_direction(name).unwrap(), expected);
        }
        assert!(parse_resize_direction("center").is_err());
    }

    #[test]
    fn parses_progress_state_and_enforces_percent_range() {
        let options: ProgressBarOptions = serde_json::from_value(json!({
            "state": "error",
            "progress": 100,
            "desktopFilename": "sample.desktop"
        }))
        .unwrap();
        let parsed = parse_progress_bar(options).unwrap();
        assert!(matches!(parsed.state, Some(ProgressState::Error)));
        assert_eq!(parsed.progress, Some(100));
        assert_eq!(parsed.desktop_filename.as_deref(), Some("sample.desktop"));

        let options: ProgressBarOptions =
            serde_json::from_value(json!({ "progress": 101 })).unwrap();
        assert!(parse_progress_bar(options).is_err());
    }

    #[test]
    fn null_min_and_max_inner_sizes_deserialize_as_cleared_constraints() {
        let min_request: ApiRequest =
            serde_json::from_value(json!([1, "window.setMinInnerSize", [null, 3]])).unwrap();
        let (min_size, min_window_id) = min_request
            .args()
            .optional::<(Option<NivaSize>, Option<u8>)>(2)
            .unwrap();
        assert!(min_size.is_none());
        assert_eq!(min_window_id, Some(3));

        let max_request: ApiRequest =
            serde_json::from_value(json!([2, "window.setMaxInnerSize", [null]])).unwrap();
        let (max_size, max_window_id) = max_request
            .args()
            .optional::<(Option<NivaSize>, Option<u8>)>(2)
            .unwrap();
        assert!(max_size.is_none());
        assert_eq!(max_window_id, None);
    }
}
