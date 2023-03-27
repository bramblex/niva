use wry::application::window::{CursorIcon, Fullscreen, Theme, UserAttentionType};

use crate::{
    api_manager::ApiManager,
    window_manager::options::{Position, Size},
};
pub fn register_apis(api_manager: &mut ApiManager) {
    api_manager.register_api("window.scaleFactor", |_, window, _| {
        Ok(window.scale_factor())
    });

    api_manager.register_api("window.innerPosition", |_, window, _| {
        Ok(window.inner_position()?)
    });

    api_manager.register_api("window.outerPosition", |_, window, _| {
        Ok(window.outer_position()?)
    });

    api_manager.register_api("window.setOuterPosition", |_, window, request| {
        let position = request.args().get_single::<Position>()?;
        window.set_outer_position(position);
        Ok(())
    });

    api_manager.register_api("window.innerSize", |_, window, _| Ok(window.inner_size()));

    api_manager.register_api("window.setInnerSize", |_, window, request| {
        let size = request.args().get_single::<Size>()?;
        window.set_inner_size(size);
        Ok(())
    });

    api_manager.register_api("window.outerSize", |_, window, _| Ok(window.outer_size()));

    api_manager.register_api("window.setMinInnerSize", |_, window, request| {
        let size = request.args().get_single::<Size>()?;
        window.set_min_inner_size(Some(size));
        Ok(())
    });

    api_manager.register_api("window.setMaxInnerSize", |_, window, request| {
        let size = request.args().get_single::<Size>()?;
        window.set_max_inner_size(Some(size));
        Ok(())
    });

    api_manager.register_api("window.setTitle", |_, window, request| {
        let title = request.args().get_single::<String>()?;
        window.set_title(&title);
        Ok(())
    });

    api_manager.register_api("window.title", |_, window, _| Ok(window.title()));

    api_manager.register_api("window.isVisible", |_, window, _| Ok(window.is_visible()));

    api_manager.register_api("window.setVisible", |_, window, request| {
        let visible = request.args().get_single::<bool>()?;
        window.set_visible(visible);
        Ok(())
    });

    api_manager.register_api("window.isFocused", |_, window, _| Ok(window.is_focused()));

    api_manager.register_api("window.setFocus", |_, window, _| {
        window.set_focus();
        Ok(())
    });

    api_manager.register_api("window.isResizable", |_, window, _| {
        Ok(window.is_resizable())
    });

    api_manager.register_api("window.setResizable", |_, window, request| {
        let resizable = request.args().get_single::<bool>()?;
        window.set_resizable(resizable);
        Ok(())
    });

    api_manager.register_api("window.isMinimizable", |_, window, _| {
        Ok(window.is_minimizable())
    });

    api_manager.register_api("window.setMinimizable", |_, window, request| {
        let minimizable = request.args().get_single::<bool>()?;
        window.set_minimizable(minimizable);
        Ok(())
    });

    api_manager.register_api("window.isMaximizable", |_, window, _| {
        Ok(window.is_maximizable())
    });

    api_manager.register_api("window.setMaximizable", |_, window, request| {
        let maximizable = request.args().get_single::<bool>()?;
        window.set_maximizable(maximizable);
        Ok(())
    });

    api_manager.register_api("window.isClosable", |_, window, _| Ok(window.is_closable()));

    api_manager.register_api("window.setClosable", |_, window, request| {
        let closable = request.args().get_single::<bool>()?;
        window.set_closable(closable);
        Ok(())
    });

    api_manager.register_api("window.isMinimized", |_, window, _| {
        Ok(window.is_minimized())
    });

    api_manager.register_api("window.setMinimized", |_, window, request| {
        let minimized = request.args().get_single::<bool>()?;
        window.set_minimized(minimized);
        Ok(())
    });

    api_manager.register_api("window.isMaximized", |_, window, _| {
        Ok(window.is_maximized())
    });

    api_manager.register_api("window.setMaximized", |_, window, request| {
        let maximized = request.args().get_single::<bool>()?;
        window.set_maximized(maximized);
        Ok(())
    });

    api_manager.register_api("window.Decorated", |_, window, _| Ok(window.is_decorated()));

    api_manager.register_api("window.setDecorated", |_, window, request| {
        let decorated = request.args().get_single::<bool>()?;
        window.set_decorations(decorated);
        Ok(())
    });

    api_manager.register_api("window.fullscreen", |_, window, _| {
        Ok(window.fullscreen().is_some())
    });

    api_manager.register_api("window.setFullscreen", |_, window, request| {
        let fullscreen = request.args().get_single::<bool>()?;
        window.set_fullscreen(if fullscreen {
            Some(Fullscreen::Borderless(None))
        } else {
            None
        });
        Ok(())
    });

    api_manager.register_api("window.setAlwaysOnTop", |_, window, request| {
        let always_on_top = request.args().get_single::<bool>()?;
        window.set_always_on_top(always_on_top);
        Ok(())
    });

    api_manager.register_api("window.setAlwaysOnBottom", |_, window, request| {
        let always_on_bottom = request.args().get_single::<bool>()?;
        window.set_always_on_bottom(always_on_bottom);
        Ok(())
    });

    api_manager.register_api("window.requestUserAttention", |_, window, request| {
        let level = request.args().get_single::<String>()?;
        match level.as_str() {
            "informational" => {
                window.request_user_attention(Some(UserAttentionType::Informational))
            }
            "critical" => window.request_user_attention(Some(UserAttentionType::Critical)),
            _ => window.request_user_attention(None),
        }
        Ok(())
    });

    api_manager.register_api("theme", |_, window, _| {
        Ok(match window.theme() {
            Theme::Light => "light",
            Theme::Dark => "dark",
            _ => "system",
        })
    });

    api_manager.register_api("window.setContentProtection", |_, window, request| {
        let enabled = request.args().get_single::<bool>()?;
        window.set_content_protection(enabled);
        Ok(())
    });

    api_manager.register_api("window.setVisibleOnAllWorkspaces", |_, window, request| {
        let visible = request.args().get_single::<bool>()?;
        window.set_visible_on_all_workspaces(visible);
        Ok(())
    });

    api_manager.register_api("window.setCursorIcon", |_, window, request| {
        let icon = request.args().get_single::<String>()?;
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
    });

    api_manager.register_api("cursorPosition", |_, window, _| {
        Ok(window.cursor_position()?)
    });

    api_manager.register_api("window.setCursorPosition", |_, window, request| {
        let position = request.args().get_single::<Position>()?;
        window.set_cursor_position(position)?;
        Ok(())
    });

    api_manager.register_api("window.setCursorGrab", |_, window, request| {
        let grab = request.args().get_single::<bool>()?;
        window.set_cursor_grab(grab)?;
        Ok(())
    });

    api_manager.register_api("window.setCursorVisible", |_, window, request| {
        let visible = request.args().get_single::<bool>()?;
        window.set_cursor_visible(visible);
        Ok(())
    });

    api_manager.register_api("window.dragWindow", |_, window, _| {
        window.drag_window()?;
        Ok(())
    });

    api_manager.register_api("window.setIgnoreCursorEvents", |_, window, request| {
        let ignore = request.args().get_single::<bool>()?;
        window.set_ignore_cursor_events(ignore)?;
        Ok(())
    });


}