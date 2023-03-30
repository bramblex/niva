use anyhow::{anyhow, Result};
use serde_json::{json, Value};

use tao::{
    event_loop::ControlFlow,
    window::{CursorIcon, Fullscreen, Theme, UserAttentionType},
};

use crate::{
    app::{
        api_manager::ApiManager,
        options::MenuOptions,
        window_manager::options::{NivaPosition, NivaSize, NivaWindowOptions},
        NivaId,
    },
    args_match, logical, logical_try, opts_match,
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
    api_manager.register_api("window.current", |_app, window, _request| -> Result<NivaId> {
        Ok(window.id)
    });

    api_manager.register_event_api(
        "window.open",
        |app, _, request, target, _| -> Result<NivaId> {
            opts_match!(request, options: Option<NivaWindowOptions>);
            let new_window = app
                .window()?
                .open_window(&options.unwrap_or_default(), target)?;
            Ok(new_window.id)
        },
    );

    api_manager.register_event_api(
        "window.close",
        |app, window, request, _, control_flow| -> Result<()> {
            opts_match!(request, id: Option<NivaId>);
            let id = id.unwrap_or(window.id);

            if id == 0 {
                *control_flow = ControlFlow::Exit;
                return Ok(());
            }

            app.window()?.close_window(id)
        },
    );
    api_manager.register_api("window.list", |app, _, _| -> Result<Vec<Value>> {
        Ok(app
            .window()?
            .list_windows()
            .into_iter()
            .map(|w| {
                json!({
                    "id": w.id,
                    "title": w.title(),
                    "visible": w.is_visible(),
                })
            })
            .collect())
    });

    api_manager.register_api("window.sendMessage", |app, window, request| -> Result<()> {
        args_match!(request, message: String, id: NivaId);
        let remote = app.window()?.get_window(id)?;
        remote.send_ipc_event(
            "window.message",
            json!({
                "from": window.id,
                "message": message,
            }),
        )?;
        Ok(())
    });

    api_manager.register_api("window.setMenu", |app, window, request| -> Result<()> {
        opts_match!(request, options: Option<MenuOptions>, id: Option<NivaId>);
        match_window!(app, window, id);
        window.set_menu(&options);
        Ok(())
    });

    api_manager.register_api("window.hideMenu", |app, window, request| {
        opts_match!(request, id: Option<NivaId>);
        match_window!(app, window, id);
        window.hide_menu();
        Ok(())
    });

    api_manager.register_api("window.showMenu", |app, window, request| {
        opts_match!(request, id: Option<NivaId>);
        match_window!(app, window, id);
        window.hide_menu();
        Ok(())
    });

    api_manager.register_api("window.isMenuVisible", |app, window, request| {
        opts_match!(request, id: Option<NivaId>);
        match_window!(app, window, id);
        Ok(window.is_menu_visible())
    });

    api_manager.register_api("window.scaleFactor", |app, window, request| {
        opts_match!(request, id: Option<NivaId>);
        match_window!(app, window, id);
        Ok(window.scale_factor())
    });

    api_manager.register_api("window.innerPosition", |app, window, request| {
        opts_match!(request, id: Option<NivaId>);
        match_window!(app, window, id);
        Ok(logical_try!(window, inner_position))
    });

    api_manager.register_api("window.outerPosition", |app, window, request| {
        opts_match!(request, id: Option<NivaId>);
        match_window!(app, window, id);
        Ok(logical_try!(window, outer_position))
    });

    api_manager.register_api("window.setOuterPosition", |app, window, request| {
        opts_match!(request, position: NivaPosition, id: Option<NivaId>);
        match_window!(app, window, id);
        window.set_outer_position(position);
        Ok(())
    });

    api_manager.register_api("window.innerSize", |app, window, request| {
        opts_match!(request, id: Option<NivaId>);
        match_window!(app, window, id);
        Ok(logical!(window, inner_size))
    });

    api_manager.register_api("window.setInnerSize", |app, window, request| {
        opts_match!(request, size: NivaSize, id: Option<NivaId>);
        match_window!(app, window, id);
        window.set_inner_size(size);
        Ok(())
    });

    api_manager.register_api("window.outerSize", |app, window, request| {
        opts_match!(request, id: Option<NivaId>);
        match_window!(app, window, id);
        Ok(logical!(window, outer_size))
    });

    api_manager.register_api("window.setMinInnerSize", |app, window, request| {
        opts_match!(request, size: NivaSize, id: Option<NivaId>);
        match_window!(app, window, id);
        window.set_min_inner_size(Some(size));
        Ok(())
    });

    api_manager.register_api("window.setMaxInnerSize", |app, window, request| {
        opts_match!(request, size: NivaSize, id: Option<NivaId>);
        match_window!(app, window, id);
        window.set_max_inner_size(Some(size));
        Ok(())
    });

    api_manager.register_api("window.setTitle", |app, window, request| {
        opts_match!(request, title: String, id: Option<NivaId>);
        match_window!(app, window, id);
        window.set_title(&title);
        Ok(())
    });

    api_manager.register_api("window.title", |app, window, request| {
        opts_match!(request, id: Option<NivaId>);
        match_window!(app, window, id);
        Ok(window.title())
    });

    api_manager.register_api("window.isVisible", |app, window, request| {
        opts_match!(request, id: Option<NivaId>);
        match_window!(app, window, id);
        Ok(window.is_visible())
    });

    api_manager.register_api("window.setVisible", |app, window, request| {
        opts_match!(request, visible: bool, id: Option<NivaId>);
        match_window!(app, window, id);
        window.set_visible(visible);
        Ok(())
    });

    api_manager.register_api("window.isFocused", |app, window, request| {
        opts_match!(request, id: Option<NivaId>);
        match_window!(app, window, id);
        Ok(window.is_focused())
    });

    api_manager.register_api("window.setFocus", |app, window, request| {
        opts_match!(request, id: Option<NivaId>);
        match_window!(app, window, id);
        window.set_focus();
        Ok(())
    });

    api_manager.register_api("window.isResizable", |app, window, request| {
        opts_match!(request, id: Option<NivaId>);
        match_window!(app, window, id);
        Ok(window.is_resizable())
    });

    api_manager.register_api("window.setResizable", |app, window, request| {
        opts_match!(request, resizable: bool, id: Option<NivaId>);
        match_window!(app, window, id);
        window.set_resizable(resizable);
        Ok(())
    });

    api_manager.register_api("window.isMinimizable", |app, window, request| {
        opts_match!(request, id: Option<NivaId>);
        match_window!(app, window, id);
        Ok(window.is_minimizable())
    });

    api_manager.register_api("window.setMinimizable", |app, window, request| {
        opts_match!(request, minimizable: bool, id: Option<NivaId>);
        match_window!(app, window, id);
        window.set_minimizable(minimizable);
        Ok(())
    });

    api_manager.register_api("window.isMaximizable", |app, window, request| {
        opts_match!(request, id: Option<NivaId>);
        match_window!(app, window, id);
        Ok(window.is_maximizable())
    });

    api_manager.register_api("window.setMaximizable", |app, window, request| {
        opts_match!(request, maximizable: bool, id: Option<NivaId>);
        match_window!(app, window, id);
        window.set_maximizable(maximizable);
        Ok(())
    });

    api_manager.register_api("window.isClosable", |app, window, request| {
        opts_match!(request, id: Option<NivaId>);
        match_window!(app, window, id);
        Ok(window.is_closable())
    });

    api_manager.register_api("window.setClosable", |app, window, request| {
        opts_match!(request, closable: bool, id: Option<NivaId>);
        match_window!(app, window, id);
        window.set_closable(closable);
        Ok(())
    });

    api_manager.register_api("window.isMinimized", |app, window, request| {
        opts_match!(request, id: Option<NivaId>);
        match_window!(app, window, id);
        Ok(window.is_minimized())
    });

    api_manager.register_api("window.setMinimized", |app, window, request| {
        opts_match!(request, minimized: bool, id: Option<NivaId>);
        match_window!(app, window, id);
        window.set_minimized(minimized);
        Ok(())
    });

    api_manager.register_api("window.isMaximized", |app, window, request| {
        opts_match!(request, id: Option<NivaId>);
        match_window!(app, window, id);
        Ok(window.is_maximized())
    });

    api_manager.register_api("window.setMaximized", |app, window, request| {
        opts_match!(request, maximized: bool, id: Option<NivaId>);
        match_window!(app, window, id);
        window.set_maximized(maximized);
        Ok(())
    });

    api_manager.register_api("window.Decorated", |app, window, request| {
        opts_match!(request, id: Option<NivaId>);
        match_window!(app, window, id);
        Ok(window.is_decorated())
    });

    api_manager.register_api("window.setDecorated", |app, window, request| {
        opts_match!(request, decorated: bool, id: Option<NivaId>);
        match_window!(app, window, id);
        window.set_decorations(decorated);
        Ok(())
    });

    api_manager.register_api("window.fullscreen", |app, window, request| {
        opts_match!(request, id: Option<NivaId>);
        match_window!(app, window, id);
        Ok(window.fullscreen().is_some())
    });

    api_manager.register_api("window.setFullscreen", |app, window, request| {
        opts_match!(
            request,
            is_fullscreen: bool,
            monitor_name: Option<String>,
            id: Option<NivaId>
        );
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
                    return Err(anyhow!("Monitor not found"));
                }
                window.set_fullscreen(Some(Fullscreen::Borderless(monitor)));
            }
            None => {
                window.set_fullscreen(Some(Fullscreen::Borderless(None)));
            }
        };

        Ok(())
    });

    api_manager.register_api("window.setAlwaysOnTop", |app, window, request| {
        opts_match!(request, always_on_top: bool, id: Option<NivaId>);
        match_window!(app, window, id);
        window.set_always_on_top(always_on_top);
        Ok(())
    });

    api_manager.register_api("window.setAlwaysOnBottom", |app, window, request| {
        opts_match!(request, always_on_bottom: bool, id: Option<NivaId>);
        match_window!(app, window, id);
        window.set_always_on_bottom(always_on_bottom);
        Ok(())
    });

    api_manager.register_api("window.requestUserAttention", |app, window, request| {
        opts_match!(request, level: String, id: Option<NivaId>);
        match_window!(app, window, id);
        match level.as_str() {
            "informational" => {
                window.request_user_attention(Some(UserAttentionType::Informational))
            }
            "critical" => window.request_user_attention(Some(UserAttentionType::Critical)),
            _ => window.request_user_attention(None),
        }
        Ok(())
    });

    api_manager.register_api("theme", |app, window, request| {
        opts_match!(request, id: Option<NivaId>);
        match_window!(app, window, id);
        Ok(match window.theme() {
            Theme::Light => "light",
            Theme::Dark => "dark",
            _ => "system",
        })
    });

    api_manager.register_api("window.setContentProtection", |app, window, request| {
        opts_match!(request, enabled: bool, id: Option<NivaId>);
        match_window!(app, window, id);
        window.set_content_protection(enabled);
        Ok(())
    });

    api_manager.register_api(
        "window.setVisibleOnAllWorkspaces",
        |app, window, request| {
            opts_match!(request, visible: bool, id: Option<NivaId>);
            match_window!(app, window, id);
            window.set_visible_on_all_workspaces(visible);
            Ok(())
        },
    );

    api_manager.register_api("window.setCursorIcon", |app, window, request| {
        opts_match!(request, icon: String, id: Option<NivaId>);
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
    });

    api_manager.register_api("window.cursorPosition", |app, window, request| {
        opts_match!(request, id: Option<NivaId>);
        match_window!(app, window, id);
        Ok(logical_try!(window, cursor_position))
    });

    api_manager.register_api("window.setCursorPosition", |app, window, request| {
        opts_match!(request, position: NivaPosition, id: Option<NivaId>);
        match_window!(app, window, id);
        window.set_cursor_position(position)?;
        Ok(())
    });

    api_manager.register_api("window.setCursorGrab", |app, window, request| {
        opts_match!(request, grab: bool, id: Option<NivaId>);
        match_window!(app, window, id);
        window.set_cursor_grab(grab)?;
        Ok(())
    });

    api_manager.register_api("window.setCursorVisible", |app, window, request| {
        opts_match!(request, visible: bool, id: Option<NivaId>);
        match_window!(app, window, id);
        window.set_cursor_visible(visible);
        Ok(())
    });

    api_manager.register_api("window.dragWindow", |app, window, request| {
        opts_match!(request, id: Option<NivaId>);
        match_window!(app, window, id);
        window.drag_window()?;
        Ok(())
    });

    api_manager.register_api("window.setIgnoreCursorEvents", |app, window, request| {
        opts_match!(request, ignore : bool, id: Option<NivaId>);
        match_window!(app, window, id);
        window.set_ignore_cursor_events(ignore)?;
        Ok(())
    });
}
