use anyhow::{anyhow, Result};
use niva_macros::{niva_api, niva_event_api};
use serde_json::{json, Value};

use tao::{
    event_loop::ControlFlow,
    window::{CursorIcon, Fullscreen, Theme, UserAttentionType},
};

use crate::{
    app::{
        api_manager::ApiManager,
        window_manager::options::{NivaPosition, NivaSize, NivaWindowOptions, WindowMenuOptions},
    },
    logical, logical_try,
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
    api_manager.register_event_api("window.open", open);
    api_manager.register_event_api("window.close", close);
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
}

#[niva_api]
fn current() -> Result<u16> {
    Ok(window.id)
}

#[niva_event_api]
fn open(options: Option<NivaWindowOptions>) -> Result<u16> {
    let new_window = app
        .window()?
        .open_window(&options.unwrap_or_default(), target)?;
    Ok(new_window.id)
}

#[niva_event_api]
fn close(id: Option<u16>) -> Result<()> {
    let id = id.unwrap_or(window.id);
    if id == 0 {
        *control_flow = ControlFlow::Exit;
        return Ok(());
    }
    app.window()?.close_window(id)
}

#[niva_api]
fn list() -> Result<Vec<Value>> {
    Ok(app
        .window()?
        .list_windows()
        .into_iter()
        .map(|w| json!({"id":w.id,"title":w.title(),"visible":w.is_visible(),}))
        .collect())
}

#[niva_api]
fn send_message(message: String, id: u16) -> Result<()> {
    let remote = app.window()?.get_window(id)?;
    remote.send_ipc_event(
        "window.message",
        json!({"from":window.id,"message":message,}),
    )?;
    Ok(())
}

#[niva_api]
fn set_menu(options: Option<WindowMenuOptions>, id: Option<u16>) -> Result<()> {
    match_window!(app, window, id);
    window.set_menu(&options);
    Ok(())
}

#[niva_api]
fn hide_menu(id: Option<u16>) -> Result<()> {
    match_window!(app, window, id);
    window.hide_menu();
    Ok(())
}

#[niva_api]
fn show_menu(id: Option<u16>) -> Result<()> {
    match_window!(app, window, id);
    window.hide_menu();
    Ok(())
}

#[niva_api]
fn is_menu_visible(id: Option<u16>) -> Result<bool> {
    match_window!(app, window, id);
    Ok(window.is_menu_visible())
}

#[niva_api]
fn scale_factor(id: Option<u16>) -> Result<f64> {
    match_window!(app, window, id);
    Ok(window.scale_factor())
}

#[niva_api]
fn inner_position(id: Option<u16>) -> Result<NivaPosition> {
    match_window!(app, window, id);
    Ok(logical_try!(window, inner_position))
}

#[niva_api]
fn outer_position(id: Option<u16>) -> Result<NivaPosition> {
    match_window!(app, window, id);
    Ok(logical_try!(window, outer_position))
}

#[niva_api]
fn set_outer_position(position: NivaPosition, id: Option<u16>) -> Result<()> {
    match_window!(app, window, id);
    window.set_outer_position(position);
    Ok(())
}

#[niva_api]
fn inner_size(id: Option<u16>) -> Result<NivaSize> {
    match_window!(app, window, id);
    Ok(logical!(window, inner_size))
}

#[niva_api]
fn set_inner_size(size: NivaSize, id: Option<u16>) -> Result<()> {
    match_window!(app, window, id);
    window.set_inner_size(size);
    Ok(())
}

#[niva_api]
fn outer_size(id: Option<u16>) -> Result<NivaSize> {
    match_window!(app, window, id);
    Ok(logical!(window, outer_size))
}

#[niva_api]
fn set_min_inner_size(size: NivaSize, id: Option<u16>) -> Result<()> {
    match_window!(app, window, id);
    window.set_min_inner_size(Some(size));
    Ok(())
}

#[niva_api]
fn set_max_inner_size(size: NivaSize, id: Option<u16>) -> Result<()> {
    match_window!(app, window, id);
    window.set_max_inner_size(Some(size));
    Ok(())
}

#[niva_api]
fn set_title(title: String, id: Option<u16>) -> Result<()> {
    match_window!(app, window, id);
    window.set_title(&title);
    Ok(())
}

#[niva_api]
fn title(id: Option<u16>) -> Result<String> {
    match_window!(app, window, id);
    Ok(window.title())
}

#[niva_api]
fn is_visible(id: Option<u16>) -> Result<bool> {
    match_window!(app, window, id);
    Ok(window.is_visible())
}

#[niva_api]
fn set_visible(visible: bool, id: Option<u16>) -> Result<()> {
    match_window!(app, window, id);
    window.set_visible(visible);
    Ok(())
}

#[niva_api]
fn is_focused(id: Option<u16>) -> Result<bool> {
    match_window!(app, window, id);
    Ok(window.is_focused())
}

#[niva_api]
fn set_focus(id: Option<u16>) -> Result<()> {
    match_window!(app, window, id);
    window.set_focus();
    Ok(())
}

#[niva_api]
fn is_resizable(id: Option<u16>) -> Result<bool> {
    match_window!(app, window, id);
    Ok(window.is_resizable())
}

#[niva_api]
fn set_resizable(resizable: bool, id: Option<u16>) -> Result<()> {
    match_window!(app, window, id);
    window.set_resizable(resizable);
    Ok(())
}

#[niva_api]
fn is_minimizable(id: Option<u16>) -> Result<bool> {
    match_window!(app, window, id);
    Ok(window.is_minimizable())
}

#[niva_api]
fn set_minimizable(minimizable: bool, id: Option<u16>) -> Result<()> {
    match_window!(app, window, id);
    window.set_minimizable(minimizable);
    Ok(())
}

#[niva_api]
fn is_maximizable(id: Option<u16>) -> Result<bool> {
    match_window!(app, window, id);
    Ok(window.is_maximizable())
}

#[niva_api]
fn set_maximizable(maximizable: bool, id: Option<u16>) -> Result<()> {
    match_window!(app, window, id);
    window.set_maximizable(maximizable);
    Ok(())
}

#[niva_api]
fn is_closable(id: Option<u16>) -> Result<bool> {
    match_window!(app, window, id);
    Ok(window.is_closable())
}

#[niva_api]
fn set_closable(closable: bool, id: Option<u16>) -> Result<()> {
    match_window!(app, window, id);
    window.set_closable(closable);
    Ok(())
}

#[niva_api]
fn is_minimized(id: Option<u16>) -> Result<bool> {
    match_window!(app, window, id);
    Ok(window.is_minimized())
}

#[niva_api]
fn set_minimized(minimized: bool, id: Option<u16>) -> Result<()> {
    match_window!(app, window, id);
    window.set_minimized(minimized);
    Ok(())
}

#[niva_api]
fn is_maximized(id: Option<u16>) -> Result<bool> {
    match_window!(app, window, id);
    Ok(window.is_maximized())
}

#[niva_api]
fn set_maximized(maximized: bool, id: Option<u16>) -> Result<()> {
    match_window!(app, window, id);
    window.set_maximized(maximized);
    Ok(())
}

#[niva_api]
fn decorated(id: Option<u16>) -> Result<bool> {
    match_window!(app, window, id);
    Ok(window.is_decorated())
}

#[niva_api]
fn set_decorated(decorated: bool, id: Option<u16>) -> Result<()> {
    match_window!(app, window, id);
    window.set_decorations(decorated);
    Ok(())
}

#[niva_api]
fn fullscreen(id: Option<u16>) -> Result<bool> {
    match_window!(app, window, id);
    Ok(window.fullscreen().is_some())
}

#[niva_api]
fn set_fullscreen(
    is_fullscreen: bool,
    monitor_name: Option<String>,
    id: Option<u16>,
) -> Result<()> {
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

#[niva_api]
fn set_always_on_top(always_on_top: bool, id: Option<u16>) -> Result<()> {
    match_window!(app, window, id);
    window.set_always_on_top(always_on_top);
    Ok(())
}

#[niva_api]
fn set_always_on_bottom(always_on_bottom: bool, id: Option<u16>) -> Result<()> {
    match_window!(app, window, id);
    window.set_always_on_bottom(always_on_bottom);
    Ok(())
}

#[niva_api]
fn request_user_attention(level: String, id: Option<u16>) -> Result<()> {
    match_window!(app, window, id);
    match level.as_str() {
        "informational" => window.request_user_attention(Some(UserAttentionType::Informational)),
        "critical" => window.request_user_attention(Some(UserAttentionType::Critical)),
        _ => window.request_user_attention(None),
    }
    Ok(())
}

#[niva_api]
fn set_content_protection(enabled: bool, id: Option<u16>) -> Result<()> {
    match_window!(app, window, id);
    window.set_content_protection(enabled);
    Ok(())
}

#[niva_api]
fn set_visible_on_all_workspaces(visible: bool, id: Option<u16>) -> Result<()> {
    match_window!(app, window, id);
    window.set_visible_on_all_workspaces(visible);
    Ok(())
}

#[niva_api]
fn set_cursor_icon(icon: String, id: Option<u16>) -> Result<()> {
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

#[niva_api]
fn cursor_position(id: Option<u16>) -> Result<NivaPosition> {
    match_window!(app, window, id);
    Ok(logical_try!(window, cursor_position))
}

#[niva_api]
fn set_cursor_position(position: NivaPosition, id: Option<u16>) -> Result<()> {
    match_window!(app, window, id);
    window.set_cursor_position(position)?;
    Ok(())
}

#[niva_api]
fn set_cursor_grab(grab: bool, id: Option<u16>) -> Result<()> {
    match_window!(app, window, id);
    window.set_cursor_grab(grab)?;
    Ok(())
}

#[niva_api]
fn set_cursor_visible(visible: bool, id: Option<u16>) -> Result<()> {
    match_window!(app, window, id);
    window.set_cursor_visible(visible);
    Ok(())
}

#[niva_api]
fn drag_window(id: Option<u16>) -> Result<()> {
    match_window!(app, window, id);
    window.drag_window()?;
    Ok(())
}

#[niva_api]
fn set_ignore_cursor_events(ignore: bool, id: Option<u16>) -> Result<()> {
    match_window!(app, window, id);
    window.set_ignore_cursor_events(ignore)?;
    Ok(())
}

#[niva_api]
fn theme(id: Option<u16>) -> Result<String> {
    match_window!(app, window, id);
    Ok(String::from(match window.theme() {
        Theme::Light => "light",
        Theme::Dark => "dark",
        _ => "system",
    }))
}
