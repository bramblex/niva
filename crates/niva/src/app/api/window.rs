use anyhow::{Result, anyhow};
use serde_json::{Value, json};

use tao::{
    event_loop::ControlFlow,
    window::{CursorIcon, Fullscreen, Theme, UserAttentionType},
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
    api_manager.register_api("window.Decorated", decorated);
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
    let (options, id) = request
        .args()
        .optional::<(Option<WindowMenuOptions>, Option<u8>)>(2)?;
    match_window!(app, window, id);
    window.set_menu(&options);
    Ok(())
}

async fn hide_menu(app: Arc<NivaApp>, window: Arc<NivaWindow>, request: ApiRequest) -> Result<()> {
    let (id,) = request.args().optional::<(Option<u8>,)>(1)?;
    match_window!(app, window, id);
    window.hide_menu();
    Ok(())
}

async fn show_menu(app: Arc<NivaApp>, window: Arc<NivaWindow>, request: ApiRequest) -> Result<()> {
    let (id,) = request.args().optional::<(Option<u8>,)>(1)?;
    match_window!(app, window, id);
    window.show_menu();
    Ok(())
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
    Ok(logical!(window, inner_size))
}

async fn set_inner_size(
    app: Arc<NivaApp>,
    window: Arc<NivaWindow>,
    request: ApiRequest,
) -> Result<()> {
    let (size, id) = request.args().optional::<(NivaSize, Option<u8>)>(2)?;
    match_window!(app, window, id);
    window.set_inner_size(size);
    Ok(())
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
    let (size, id) = request.args().optional::<(NivaSize, Option<u8>)>(2)?;
    match_window!(app, window, id);
    window.set_min_inner_size(Some(size));
    Ok(())
}

async fn set_max_inner_size(
    app: Arc<NivaApp>,
    window: Arc<NivaWindow>,
    request: ApiRequest,
) -> Result<()> {
    let (size, id) = request.args().optional::<(NivaSize, Option<u8>)>(2)?;
    match_window!(app, window, id);
    window.set_max_inner_size(Some(size));
    Ok(())
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
    window.set_minimized(minimized);
    Ok(())
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
    window.set_content_protection(enabled);
    Ok(())
}

async fn set_visible_on_all_workspaces(
    app: Arc<NivaApp>,
    window: Arc<NivaWindow>,
    request: ApiRequest,
) -> Result<()> {
    let (visible, id) = request.args().optional::<(bool, Option<u8>)>(2)?;
    match_window!(app, window, id);
    window.set_visible_on_all_workspaces(visible);
    Ok(())
}

async fn set_cursor_icon(
    app: Arc<NivaApp>,
    window: Arc<NivaWindow>,
    request: ApiRequest,
) -> Result<()> {
    let (icon, id) = request.args().optional::<(String, Option<u8>)>(2)?;
    match_window!(app, window, id);
    window.set_cursor_icon(match icon.as_str() {
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
    });
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
    window.set_cursor_visible(visible);
    Ok(())
}

async fn drag_window(
    app: Arc<NivaApp>,
    window: Arc<NivaWindow>,
    request: ApiRequest,
) -> Result<()> {
    let app2 = app.clone();
    run_on_main(&app, move |_target, _control_flow| {
        let (id,) = request.args().optional::<(Option<u8>,)>(1)?;
        match_window!(app2, window, id);
        window.drag_window()?;
        Ok(())
    })
    .await
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
