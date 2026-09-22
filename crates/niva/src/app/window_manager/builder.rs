use super::WindowManager;
use super::options::NivaWindowOptions;
use super::options::WindowMenuOptions;
use super::options::WindowRootMenu;
use crate::app::assets::INITIALIZE_SCRIPT;
use crate::app::menu::options::{MenuItemOption, MenuOptions};
use crate::app::menu::{build_menu as build_muda_menu, build_submenu};
use crate::app::utils::url_join;
use crate::app::window_manager::url::get_host_from_url;
use crate::app::window_manager::url::make_base_url;
use crate::{
    app::{NivaApp, NivaWindowTarget},
    log_err, log_if_err, set_property, set_property_some,
};
use anyhow::Result;
use anyhow::anyhow;
use serde_json::json;
use std::borrow::Cow;
use std::sync::Arc;
use tao::window::{Fullscreen, Theme, Window, WindowBuilder};
use wry::{DragDropEvent, WebContext, WebView, WebViewBuilder, http::Response};

pub struct NivaBuilder {}

impl NivaBuilder {
    /// Build the native window plus its muda menu (if any).
    /// The menu handle must be kept alive by the caller for as long as
    /// the menu should stay attached.
    pub fn build_window(
        app: &Arc<NivaApp>,
        manager: &WindowManager,
        _id: u8,
        options: &NivaWindowOptions,
        target: &NivaWindowTarget,
    ) -> Result<(Window, Option<muda::Menu>)> {
        let mut builder = WindowBuilder::new();

        set_property_some!(
            builder,
            with_title,
            options.title.clone().or(Some(app.launch_info.name.clone()))
        );

        set_property_some!(
            builder,
            with_theme,
            options
                .theme
                .clone()
                .map(|theme_str| match theme_str.as_str() {
                    "dark" => Some(Theme::Dark),
                    "light" => Some(Theme::Light),
                    _ => None,
                })
        );

        if let Some(icon_path) = &options.icon {
            let icon = app.resource().load_icon(icon_path)?;
            set_property!(builder, with_window_icon, Some(icon));
        }

        set_property_some!(
            builder,
            with_fullscreen,
            options.fullscreen.map(|b| {
                if b {
                    Some(Fullscreen::Borderless(None))
                } else {
                    None
                }
            })
        );

        set_property_some!(builder, with_inner_size, options.size);
        set_property_some!(builder, with_min_inner_size, options.min_size);
        set_property_some!(builder, with_max_inner_size, options.max_size);
        set_property_some!(builder, with_position, options.position);
        set_property_some!(builder, with_resizable, options.resizable);
        set_property_some!(builder, with_minimizable, options.minimizable);
        set_property_some!(builder, with_maximizable, options.maximizable);
        set_property_some!(builder, with_closable, options.closable);
        set_property_some!(builder, with_maximized, options.maximized);
        set_property_some!(builder, with_visible, options.visible);
        set_property_some!(builder, with_transparent, options.transparent);
        set_property_some!(builder, with_decorations, options.decorations);
        set_property_some!(builder, with_always_on_top, options.always_on_top);
        set_property_some!(builder, with_always_on_bottom, options.always_on_bottom);
        set_property_some!(
            builder,
            with_visible_on_all_workspaces,
            options.visible_on_all_workspaces
        );
        set_property_some!(builder, with_focused, options.focused);
        set_property_some!(builder, with_content_protection, options.content_protection);

        #[cfg(target_os = "macos")]
        if let Some(macos_extra) = &options.macos_extra {
            use tao::platform::macos::{WindowBuilderExtMacOS, WindowExtMacOS};

            if let Some(parent) = &macos_extra.parent_window {
                let parent = manager.get_window(*parent)?;
                let parent = parent.ns_window();
                set_property!(builder, with_parent_window, parent);
            }

            set_property_some!(
                builder,
                with_movable_by_window_background,
                macos_extra.movable_by_window_background
            );
            set_property_some!(
                builder,
                with_titlebar_transparent,
                macos_extra.title_bar_transparent
            );
            set_property_some!(builder, with_titlebar_hidden, macos_extra.title_bar_hidden);
            set_property_some!(
                builder,
                with_titlebar_buttons_hidden,
                macos_extra.title_bar_buttons_hidden
            );
            set_property_some!(builder, with_title_hidden, macos_extra.title_hidden);
            set_property_some!(
                builder,
                with_fullsize_content_view,
                macos_extra.full_size_content_view
            );
            set_property_some!(
                builder,
                with_resize_increments,
                macos_extra.resize_increments
            );
            set_property_some!(builder, with_disallow_hidpi, macos_extra.disallow_hi_dpi);
            set_property_some!(builder, with_has_shadow, macos_extra.has_shadow);
            set_property_some!(
                builder,
                with_automatic_window_tabbing,
                macos_extra.automatic_window_tabbing
            );
            set_property_some!(
                builder,
                with_tabbing_identifier,
                &macos_extra.tabbing_identifier
            );
        }

        #[cfg(target_os = "windows")]
        if let Some(windows_extra) = &options.windows_extra {
            use tao::platform::windows::{WindowBuilderExtWindows, WindowExtWindows};

            if let Some(parent) = &windows_extra.parent_window {
                let parent = manager.get_window(*parent)?;
                let parent = parent.hwnd() as _;
                set_property!(builder, with_parent_window, parent);
            }

            if let Some(owner) = &windows_extra.parent_window {
                let owner = manager.get_window(*owner)?;
                let owner = owner.hwnd() as _;
                set_property!(builder, with_owner_window, owner);
            }

            if let Some(icon_path) = &windows_extra.taskbar_icon {
                let icon = app.resource().load_icon(icon_path)?;
                set_property!(builder, with_taskbar_icon, Some(icon));
            }

            set_property_some!(builder, with_skip_taskbar, windows_extra.skip_taskbar);
            set_property_some!(
                builder,
                with_undecorated_shadow,
                windows_extra.undecorated_shadow
            );
        }

        let window = builder.build(target)?;

        // Attach the window menu (tao no longer owns menus; muda does).
        let menu = Self::build_menu(_id, app, &options.menu);
        if let Some(menu) = &menu {
            Self::attach_menu(&window, menu);
        }

        Ok((window, menu))
    }

    /// Attach a muda menu to a native window.
    pub fn attach_menu(window: &Window, menu: &muda::Menu) {
        #[cfg(target_os = "macos")]
        {
            let _ = window;
            menu.init_for_nsapp();
        }
        #[cfg(target_os = "windows")]
        {
            use tao::platform::windows::WindowExtWindows;
            unsafe {
                log_if_err!(menu.init_for_hwnd(window.hwnd()));
            }
        }
        #[cfg(not(any(target_os = "macos", target_os = "windows")))]
        {
            let _ = (window, menu);
        }
    }

    /// Detach a muda menu from a native window.
    pub fn detach_menu(window: &Window, menu: &muda::Menu) {
        #[cfg(target_os = "macos")]
        {
            let _ = window;
            menu.remove_for_nsapp();
        }
        #[cfg(target_os = "windows")]
        {
            use tao::platform::windows::WindowExtWindows;
            unsafe {
                log_if_err!(menu.remove_for_hwnd(window.hwnd()));
            }
        }
        #[cfg(not(any(target_os = "macos", target_os = "windows")))]
        {
            let _ = (window, menu);
        }
    }

    pub fn build_webview(
        app: &Arc<NivaApp>,
        options: &NivaWindowOptions,
        window: &Window,
        web_context: &mut WebContext,
        window_id: tao::window::WindowId,
    ) -> Result<WebView> {
        let id_name = app.launch_info.id_name.clone();
        let protocol = "niva";

        let debug_entry = app.launch_info.arguments.debug_entry.clone();

        let base_url = debug_entry.unwrap_or(make_base_url(protocol, &id_name));

        let entry_url = url_join(&base_url, &options.entry.clone().unwrap_or_default());

        let mut builder = WebViewBuilder::new_with_web_context(web_context);

        set_property!(builder, with_initialization_script, INITIALIZE_SCRIPT);
        set_property!(builder, with_accept_first_mouse, true);
        set_property!(builder, with_clipboard, true);
        set_property_some!(builder, with_devtools, options.devtools);

        if options.transparent.unwrap_or(false) {
            set_property!(builder, with_background_color, (255, 255, 255, 0));
            set_property!(builder, with_transparent, true);
        }

        let prefix = get_host_from_url(&entry_url).unwrap_or(base_url);
        set_property!(builder, with_navigation_handler, move |url| url
            .starts_with(&prefix));

        let custom_protocol_app = app.clone();
        builder =
            builder.with_custom_protocol(protocol.to_string(), move |_webview_id, request| {
                let hostname = request.uri().host().unwrap_or(&id_name);

                let mut path = request.uri().path().to_string();

                if path.ends_with('/') {
                    path += "index.html";
                }

                let result = (|| -> Result<Vec<u8>> {
                    if hostname == id_name {
                        let path = path.strip_prefix('/').unwrap_or("index.html");
                        custom_protocol_app.resource().load(path)
                    } else if hostname == "filesystem" {
                        let file_path = path.strip_prefix('/').unwrap_or("index.html");
                        Ok(std::fs::read(file_path)?)
                    } else {
                        Err(anyhow!("Invalid hostname: {}", hostname))
                    }
                })();

                let origin =
                    get_host_from_url(&request.uri().to_string()).unwrap_or("*".to_string());
                let uri_string = request.uri().to_string();

                match result {
                    Err(err) => {
                        eprintln!("[niva] custom protocol {uri_string} -> 404: {err}");
                        error_page(404, "Not Found", &uri_string, &err.to_string())
                    }

                    Ok(content) => {
                        let mime_type = mime_guess::from_path(path)
                            .first()
                            .unwrap_or(mime_guess::mime::TEXT_PLAIN)
                            .to_string();

                        Response::builder()
                            .status(200)
                            .header("Content-Type", mime_type)
                            .header("Access-Control-Allow-Origin", origin)
                            .body(Cow::Owned(content))
                            .unwrap_or_else(|err| {
                                eprintln!("[niva] custom protocol {uri_string} -> 500: {err}");
                                error_page(
                                    500,
                                    "Internal Server Error",
                                    &uri_string,
                                    "Failed to build response",
                                )
                            })
                    }
                }
            });

        let drop_app = app.clone();
        builder = builder.with_drag_drop_handler(move |event| {
            let window_result = drop_app
                .window()
                .and_then(|w| w.get_window_inner(window_id));
            match window_result {
                Ok(window) => match event {
                    DragDropEvent::Enter { paths, position } => {
                        let position = physical_to_logical(position, window.scale_factor());
                        log_if_err!(window.send_ipc_event(
                            "fileDrop.hovered",
                            json!({
                                "paths": paths,
                                "position": position,
                            }),
                        ));
                    }
                    DragDropEvent::Drop { paths, position } => {
                        let position = physical_to_logical(position, window.scale_factor());
                        log_if_err!(window.send_ipc_event(
                            "fileDrop.dropped",
                            json!({
                                "paths": paths,
                                "position": position,
                            }),
                        ));
                    }
                    DragDropEvent::Leave => {
                        log_if_err!(window.send_ipc_event("fileDrop.cancelled", json!(null)));
                    }
                    // Over events fire continuously; avoid spamming the frontend.
                    DragDropEvent::Over { .. } => (),
                    _ => (),
                },
                Err(err) => {
                    log_err!(err);
                }
            }
            false
        });

        let ipc_app = app.clone();
        builder = builder.with_ipc_handler(move |request| {
            let request_str = request.body().clone();
            if let Err(err) = ipc_app.api().and_then(|w| w.call(window_id, request_str)) {
                let window = ipc_app.window().and_then(|w| w.get_window_inner(window_id));
                if let Ok(window) = window {
                    log_if_err!(window.send_ipc_callback(json!({
                        "ipc.error": err.to_string(),
                    })));
                }
            };
        });

        Ok(builder.with_url(entry_url).build(window)?)
    }

    pub fn build_menu(
        window_id: u8,
        app: &Arc<NivaApp>,
        menu_options: &Option<WindowMenuOptions>,
    ) -> Option<muda::Menu> {
        #[cfg(target_os = "macos")]
        let default_menu = Self::macos_default_menu();
        #[cfg(target_os = "macos")]
        let menu_options = match &menu_options {
            Some(_) => menu_options,
            None => &default_menu,
        };

        if let Some(window_menu_options) = menu_options {
            let menu = muda::Menu::new();
            for WindowRootMenu {
                label,
                enabled,
                children,
            } in window_menu_options
            {
                let submenu =
                    build_submenu(window_id, app, label, enabled.unwrap_or(true), children);
                log_if_err!(menu.append(&submenu));
            }
            return Some(menu);
        }
        None
    }

    pub fn build_tray_menu(
        window_id: u8,
        app: &Arc<NivaApp>,
        menu_options: &MenuOptions,
    ) -> muda::Menu {
        build_muda_menu(window_id, app, menu_options)
    }

    #[cfg(target_os = "macos")]
    fn macos_default_menu() -> Option<WindowMenuOptions> {
        use crate::app::menu::options::NativeLabel;

        Some(vec![WindowRootMenu {
            label: "".to_string(),
            enabled: None,
            children: vec![
                MenuItemOption::Native {
                    label: NativeLabel::SelectAll,
                },
                MenuItemOption::Native {
                    label: NativeLabel::Copy,
                },
                MenuItemOption::Native {
                    label: NativeLabel::Paste,
                },
                MenuItemOption::Native {
                    label: NativeLabel::Cut,
                },
                MenuItemOption::Native {
                    label: NativeLabel::Undo,
                },
                MenuItemOption::Native {
                    label: NativeLabel::Separator,
                },
                MenuItemOption::Native {
                    label: NativeLabel::Quit,
                },
            ],
        }])
    }
}

fn html_escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/// Render a debuggable HTML error page instead of a bare status text, so a
/// missing entry/resource shows *what* is missing right in the window.
fn error_page(status: u16, title: &str, uri: &str, detail: &str) -> Response<Cow<'static, [u8]>> {
    let body = format!(
        "<!DOCTYPE html><html><head><meta charset=\"utf-8\">\
        <title>{status} {title}</title></head><body>\
        <h1>{status} {title}</h1>\
        <p>URI: <code>{}</code></p>\
        <p>{}</p>\
        <p style=\"color:#888\">Niva custom protocol: check that the entry file \
        exists in the resource directory.</p>\
        </body></html>",
        html_escape(uri),
        html_escape(detail),
    );
    Response::builder()
        .status(status)
        .header("Content-Type", "text/html; charset=utf-8")
        .body(Cow::Owned(body.into_bytes()))
        .unwrap_or_else(|_| Response::new(Cow::Borrowed(&[][..])))
}

fn physical_to_logical(position: (i32, i32), scale_factor: f64) -> tao::dpi::LogicalPosition<f64> {
    tao::dpi::PhysicalPosition::new(position.0 as f64, position.1 as f64).to_logical(scale_factor)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_page_renders_status_uri_and_escaped_detail() {
        let response = error_page(
            404,
            "Not Found",
            "niva://demo_12345678/missing.html",
            "No such file <x> & \"y\" (os error 2)",
        );
        assert_eq!(response.status(), 404);
        let body = response.body();
        let body = std::str::from_utf8(body).unwrap();
        assert!(body.contains("<h1>404 Not Found</h1>"), "{body}");
        assert!(body.contains("niva://demo_12345678/missing.html"), "{body}");
        assert!(body.contains("No such file &lt;x&gt; &amp;"), "{body}");
        assert!(!body.contains("<x>"), "{body}");
    }
}
