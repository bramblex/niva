// TODO: window api

use crate::sys_api::{ApiRequest, ApiResponse};
use serde::Deserialize;
use serde_json::json;
use wry::application::dpi::{PhysicalPosition, PhysicalSize};
use wry::application::window::{CursorIcon, Fullscreen, Theme, UserAttentionType, Window};

pub fn call(window: &Window, request: ApiRequest) -> ApiResponse {
    match request.method.as_str() {
        "scaleFactor" => scale_factor(window, request),
        "innerPosition" => inner_position(window, request),
        "outerPosition" => outer_position(window, request),
        "setOuterPosition" => set_outer_position(window, request),
        "innerSize" => inner_size(window, request),
        "setInnerSize" => set_inner_size(window, request),
        "outerSize" => outer_size(window, request),
        "setMinInnerSize" => set_min_inner_size(window, request),
        "setMaxInnerSize" => set_max_inner_size(window, request),
        "setTitle" => set_title(window, request),
        "title" => title(window, request),
        "setVisible" => set_visible(window, request),
        "setFocus" => set_focus(window, request),
        "isFocused" => is_focused(window, request),
        "setResizable" => set_resizable(window, request),
        "setMinimizable" => set_minimizable(window, request),
        "setMaximizable" => set_maximizable(window, request),
        "setClosable" => set_closable(window, request),
        "setMinimized" => set_minimized(window, request),
        "setMaximized" => set_maximized(window, request),
        "isMinimized" => is_minimized(window, request),
        "isMaximized" => is_maximized(window, request),
        "isVisible" => is_visible(window, request),
        "isResizable" => is_resizable(window, request),
        "isMinimizable" => is_minimizable(window, request),
        "isMaximizable" => is_maximizable(window, request),
        "isClosable" => is_closable(window, request),
        "isDecorated" => is_decorated(window, request),
        "setFullscreen" => set_fullscreen(window, request),
        "fullscreen" => fullscreen(window, request),
        "setDecorations" => set_decorations(window, request),
        "setAlwaysOnBottom" => set_always_on_bottom(window, request),
        "setAlwaysOnTop" => set_always_on_top(window, request),
        "requestUserAttention" => request_user_attention(window, request),
        "hideMenu" => hide_menu(window, request),
        "showMenu" => show_menu(window, request),
        "isMenuVisible" => is_menu_visible(window, request),
        "theme" => theme(window, request),
        "setContentProtection" => set_content_protection(window, request),
        "setVisibleOnAllWorkspaces" => set_visible_on_all_workspaces(window, request),
        "setCursorIcon" => set_cursor_icon(window, request),
        "setCursorPosition" => set_cursor_position(window, request),
        "setCursorGrab" => set_cursor_grab(window, request),
        "setCursorVisible" => set_cursor_visible(window, request),
        "dragWindow" => drag_window(window, request),
        "setIgnoreCursorEvents" => set_ignore_cursor_events(window, request),
        "cursorPosition" => cursor_position(window, request),
        _ => ApiResponse::err(request.callback_id, "method not found"),
    }
}

#[derive(Deserialize)]
struct SizeOptions {
    pub width: u32,
    pub height: u32,
}

#[derive(Deserialize)]
struct PositionOptions {
    pub x: i32,
    pub y: i32,
}

// dpi
fn scale_factor(window: &Window, request: ApiRequest) -> ApiResponse {
    ApiResponse::ok(request.callback_id, window.scale_factor())
}

// position and size
fn inner_position(window: &Window, request: ApiRequest) -> ApiResponse {
    if let Ok(position) = window.inner_position() {
        return ApiResponse::ok(
            request.callback_id,
            json!({
                "x": position.x,
                "y": position.y,
            }),
        );
    }
    ApiResponse::err(request.callback_id, "failed to get inner position")
}

fn outer_position(window: &Window, request: ApiRequest) -> ApiResponse {
    if let Ok(position) = window.outer_position() {
        return ApiResponse::ok(
            request.callback_id,
            json!({
                "x": position.x,
                "y": position.y,
            }),
        );
    }
    ApiResponse::err(request.callback_id, "failed to get outer position")
}

fn set_outer_position(window: &Window, request: ApiRequest) -> ApiResponse {
    let options_result = serde_json::from_value::<PositionOptions>(request.data);
    if options_result.is_err() {
        return ApiResponse::err(request.callback_id, "invalid options");
    }
    let options = options_result.unwrap();
    window.set_outer_position(PhysicalPosition::new(options.x, options.y));
    ApiResponse::ok(request.callback_id, json!({}))
}

fn inner_size(window: &Window, request: ApiRequest) -> ApiResponse {
    let size = window.inner_size();
    ApiResponse::ok(
        request.callback_id,
        json!({
            "width": size.width,
            "height": size.height,
        }),
    )
}

fn set_inner_size(window: &Window, request: ApiRequest) -> ApiResponse {
    let options_result = serde_json::from_value::<SizeOptions>(request.data);
    if options_result.is_err() {
        return ApiResponse::err(request.callback_id, "invalid options");
    }
    let options = options_result.unwrap();
    window.set_inner_size(PhysicalSize::new(options.width, options.height));
    ApiResponse::ok(request.callback_id, json!({}))
}

fn outer_size(window: &Window, request: ApiRequest) -> ApiResponse {
    let size = window.outer_size();
    ApiResponse::ok(
        request.callback_id,
        json!({
            "width": size.width,
            "height": size.height,
        }),
    )
}

fn set_min_inner_size(window: &Window, request: ApiRequest) -> ApiResponse {
    let options_result = serde_json::from_value::<SizeOptions>(request.data);
    if options_result.is_err() {
        return ApiResponse::err(request.callback_id, "invalid options");
    }
    let options = options_result.unwrap();
    window.set_min_inner_size(Some(PhysicalSize::new(options.width, options.height)));
    ApiResponse::ok(request.callback_id, json!({}))
}

fn set_max_inner_size(window: &Window, request: ApiRequest) -> ApiResponse {
    let options_result = serde_json::from_value::<SizeOptions>(request.data);
    if options_result.is_err() {
        return ApiResponse::err(request.callback_id, "invalid options");
    }
    let options = options_result.unwrap();
    window.set_max_inner_size(Some(PhysicalSize::new(options.width, options.height)));
    ApiResponse::ok(request.callback_id, json!({}))
}

// misc options

fn set_title(window: &Window, request: ApiRequest) -> ApiResponse {
    let title_result = serde_json::from_value::<String>(request.data);
    if title_result.is_err() {
        return ApiResponse::err(request.callback_id, "invalid title");
    }
    let title = title_result.unwrap();
    window.set_title(&title);
    ApiResponse::ok(request.callback_id, json!({}))
}

fn title(window: &Window, request: ApiRequest) -> ApiResponse {
    ApiResponse::ok(request.callback_id, window.title())
}

fn set_visible(window: &Window, request: ApiRequest) -> ApiResponse {
    let visible_result = serde_json::from_value::<bool>(request.data);
    if visible_result.is_err() {
        return ApiResponse::err(request.callback_id, "invalid visible");
    }
    let visible = visible_result.unwrap();
    window.set_visible(visible);
    ApiResponse::ok(request.callback_id, json!({}))
}

fn set_focus(window: &Window, request: ApiRequest) -> ApiResponse {
    window.set_focus();
    ApiResponse::ok(request.callback_id, json!({}))
}

fn is_focused(window: &Window, request: ApiRequest) -> ApiResponse {
    ApiResponse::ok(request.callback_id, window.is_focused())
}

fn set_resizable(window: &Window, request: ApiRequest) -> ApiResponse {
    let resizable_result = serde_json::from_value::<bool>(request.data);
    if resizable_result.is_err() {
        return ApiResponse::err(request.callback_id, "invalid resizable");
    }
    let resizable = resizable_result.unwrap();
    window.set_resizable(resizable);
    ApiResponse::ok(request.callback_id, json!({}))
}

fn set_minimizable(window: &Window, request: ApiRequest) -> ApiResponse {
    let minimizable_result = serde_json::from_value::<bool>(request.data);
    if minimizable_result.is_err() {
        return ApiResponse::err(request.callback_id, "invalid minimizable");
    }
    let minimizable = minimizable_result.unwrap();
    window.set_minimizable(minimizable);
    ApiResponse::ok(request.callback_id, json!({}))
}

fn set_maximizable(window: &Window, request: ApiRequest) -> ApiResponse {
    let maximizable_result = serde_json::from_value::<bool>(request.data);
    if maximizable_result.is_err() {
        return ApiResponse::err(request.callback_id, "invalid maximizable");
    }
    let maximizable = maximizable_result.unwrap();
    window.set_maximizable(maximizable);
    ApiResponse::ok(request.callback_id, json!({}))
}

fn set_closable(window: &Window, request: ApiRequest) -> ApiResponse {
    let closable_result = serde_json::from_value::<bool>(request.data);
    if closable_result.is_err() {
        return ApiResponse::err(request.callback_id, "invalid closable");
    }
    let closable = closable_result.unwrap();
    window.set_closable(closable);
    ApiResponse::ok(request.callback_id, json!({}))
}

fn set_minimized(window: &Window, request: ApiRequest) -> ApiResponse {
    let minimized_result = serde_json::from_value::<bool>(request.data);
    if minimized_result.is_err() {
        return ApiResponse::err(request.callback_id, "invalid minimized");
    }
    let minimized = minimized_result.unwrap();
    window.set_minimized(minimized);
    ApiResponse::ok(request.callback_id, json!({}))
}

fn set_maximized(window: &Window, request: ApiRequest) -> ApiResponse {
    let maximized_result = serde_json::from_value::<bool>(request.data);
    if maximized_result.is_err() {
        return ApiResponse::err(request.callback_id, "invalid maximized");
    }
    let maximized = maximized_result.unwrap();
    window.set_maximized(maximized);
    ApiResponse::ok(request.callback_id, json!({}))
}

fn is_maximized(window: &Window, request: ApiRequest) -> ApiResponse {
    ApiResponse::ok(request.callback_id, window.is_maximized())
}

fn is_minimized(window: &Window, request: ApiRequest) -> ApiResponse {
    ApiResponse::ok(request.callback_id, window.is_minimized())
}

fn is_visible(window: &Window, request: ApiRequest) -> ApiResponse {
    ApiResponse::ok(request.callback_id, window.is_visible())
}

fn is_resizable(window: &Window, request: ApiRequest) -> ApiResponse {
    ApiResponse::ok(request.callback_id, window.is_resizable())
}

fn is_minimizable(window: &Window, request: ApiRequest) -> ApiResponse {
    ApiResponse::ok(request.callback_id, window.is_minimizable())
}

fn is_maximizable(window: &Window, request: ApiRequest) -> ApiResponse {
    ApiResponse::ok(request.callback_id, window.is_maximizable())
}

fn is_closable(window: &Window, request: ApiRequest) -> ApiResponse {
    ApiResponse::ok(request.callback_id, window.is_closable())
}

fn is_decorated(window: &Window, request: ApiRequest) -> ApiResponse {
    ApiResponse::ok(request.callback_id, window.is_decorated())
}

fn set_fullscreen(window: &Window, request: ApiRequest) -> ApiResponse {
    let fullscreen_result = serde_json::from_value::<bool>(request.data);
    if fullscreen_result.is_err() {
        return ApiResponse::err(request.callback_id, "invalid fullscreen");
    }
    let fullscreen = fullscreen_result.unwrap();
    window.set_fullscreen(if fullscreen {
        Some(Fullscreen::Borderless(None))
    } else {
        None
    });
    ApiResponse::ok(request.callback_id, json!({}))
}

fn fullscreen(window: &Window, request: ApiRequest) -> ApiResponse {
    ApiResponse::ok(request.callback_id, window.fullscreen().is_some())
}

fn set_decorations(window: &Window, request: ApiRequest) -> ApiResponse {
    let decorations_result = serde_json::from_value::<bool>(request.data);
    if decorations_result.is_err() {
        return ApiResponse::err(request.callback_id, "invalid decorations");
    }
    let decorations = decorations_result.unwrap();
    window.set_decorations(decorations);
    ApiResponse::ok(request.callback_id, json!({}))
}

fn set_always_on_bottom(window: &Window, request: ApiRequest) -> ApiResponse {
    let always_on_bottom_result = serde_json::from_value::<bool>(request.data);
    if always_on_bottom_result.is_err() {
        return ApiResponse::err(request.callback_id, "invalid always_on_bottom");
    }
    let always_on_bottom = always_on_bottom_result.unwrap();
    window.set_always_on_top(always_on_bottom);
    ApiResponse::ok(request.callback_id, json!({}))
}

fn set_always_on_top(window: &Window, request: ApiRequest) -> ApiResponse {
    let always_on_top_result = serde_json::from_value::<bool>(request.data);
    if always_on_top_result.is_err() {
        return ApiResponse::err(request.callback_id, "invalid always_on_top");
    }
    let always_on_top = always_on_top_result.unwrap();
    window.set_always_on_top(always_on_top);
    ApiResponse::ok(request.callback_id, json!({}))
}

fn request_user_attention(window: &Window, request: ApiRequest) -> ApiResponse {
    let always_on_top_result = serde_json::from_value::<String>(request.data);
    if always_on_top_result.is_err() {
        return ApiResponse::err(request.callback_id, "invalid request_user_attention");
    }
    match always_on_top_result.unwrap().as_str() {
        "none" => window.request_user_attention(None),
        "informational" => window.request_user_attention(Some(UserAttentionType::Informational)),
        "critical" => window.request_user_attention(Some(UserAttentionType::Critical)),
        _ => return ApiResponse::err(request.callback_id, "invalid request_user_attention"),
    }
    ApiResponse::ok(request.callback_id, json!({}))
}

fn hide_menu(window: &Window, request: ApiRequest) -> ApiResponse {
    window.hide_menu();
    ApiResponse::ok(request.callback_id, json!({}))
}

fn show_menu(window: &Window, request: ApiRequest) -> ApiResponse {
    window.show_menu();
    ApiResponse::ok(request.callback_id, json!({}))
}

fn is_menu_visible(window: &Window, request: ApiRequest) -> ApiResponse {
    ApiResponse::ok(request.callback_id, window.is_menu_visible())
}

fn theme(window: &Window, request: ApiRequest) -> ApiResponse {
    ApiResponse::ok(
        request.callback_id,
        match window.theme() {
            Theme::Light => "light",
            Theme::Dark => "dark",
            _ => "system",
        },
    )
}

fn set_content_protection(window: &Window, request: ApiRequest) -> ApiResponse {
    let content_protection_result = serde_json::from_value::<bool>(request.data);
    if content_protection_result.is_err() {
        return ApiResponse::err(request.callback_id, "invalid content_protection");
    }
    let content_protection = content_protection_result.unwrap();
    window.set_content_protection(content_protection);
    ApiResponse::ok(request.callback_id, json!({}))
}

fn set_visible_on_all_workspaces(window: &Window, request: ApiRequest) -> ApiResponse {
    let visible_on_all_workspaces_result = serde_json::from_value::<bool>(request.data);
    if visible_on_all_workspaces_result.is_err() {
        return ApiResponse::err(request.callback_id, "invalid visible_on_all_workspaces");
    }
    let visible_on_all_workspaces = visible_on_all_workspaces_result.unwrap();
    window.set_visible_on_all_workspaces(visible_on_all_workspaces);
    ApiResponse::ok(request.callback_id, json!({}))
}

fn set_cursor_icon(window: &Window, request: ApiRequest) -> ApiResponse {
    let cursor_icon_result = serde_json::from_value::<String>(request.data);
    if cursor_icon_result.is_err() {
        return ApiResponse::err(request.callback_id, "invalid cursor_icon");
    }
    let cursor_icon = cursor_icon_result.unwrap();
    window.set_cursor_icon(match cursor_icon.as_str() {
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
    ApiResponse::ok(request.callback_id, json!({}))
}

fn set_cursor_position(window: &Window, request: ApiRequest) -> ApiResponse {
    let cursor_position_result = serde_json::from_value::<PositionOptions>(request.data);
    if cursor_position_result.is_err() {
        return ApiResponse::err(request.callback_id, "invalid cursor_position");
    }
    let cursor_position = cursor_position_result.unwrap();
    window.set_cursor_position(PhysicalPosition::new(cursor_position.x, cursor_position.y)).unwrap();
    ApiResponse::ok(request.callback_id, json!({}))
}

fn set_cursor_grab(window: &Window, request: ApiRequest) -> ApiResponse {
    let cursor_grab_result = serde_json::from_value::<bool>(request.data);
    if cursor_grab_result.is_err() {
        return ApiResponse::err(request.callback_id, "invalid cursor_grab");
    }
    let cursor_grab = cursor_grab_result.unwrap();
    window.set_cursor_grab(cursor_grab).unwrap();
    ApiResponse::ok(request.callback_id, json!({}))
}

fn set_cursor_visible(window: &Window, request: ApiRequest) -> ApiResponse {
    let cursor_visible_result = serde_json::from_value::<bool>(request.data);
    if cursor_visible_result.is_err() {
        return ApiResponse::err(request.callback_id, "invalid cursor_visible");
    }
    let cursor_visible = cursor_visible_result.unwrap();
    window.set_cursor_visible(cursor_visible);
    ApiResponse::ok(request.callback_id, json!({}))
}

fn drag_window(window: &Window, request: ApiRequest) -> ApiResponse {
    window.drag_window().unwrap();
    ApiResponse::ok(request.callback_id, json!({}))
}

fn set_ignore_cursor_events(window: &Window, request: ApiRequest) -> ApiResponse {
    let ignore_cursor_events_result = serde_json::from_value::<bool>(request.data);
    if ignore_cursor_events_result.is_err() {
        return ApiResponse::err(request.callback_id, "invalid ignore_cursor_events");
    }
    let ignore_cursor_events = ignore_cursor_events_result.unwrap();
    window.set_ignore_cursor_events(ignore_cursor_events).unwrap();
    ApiResponse::ok(request.callback_id, json!({}))
}

fn cursor_position(window: &Window, request: ApiRequest) -> ApiResponse {
    let cursor_position = window.cursor_position();
    if let Ok(cursor_position) = cursor_position {
        return ApiResponse::ok(
            request.callback_id,
            json!({
                "x": cursor_position.x,
                "y": cursor_position.y,
            }),
        );
    }
    ApiResponse::err(request.callback_id, "cursor position not available")
}
