use super::options::WindowMenuOptions;
use super::options::WindowRootMenu;
use super::WindowManager;
use super::{options::NivaWindowOptions, window::NivaWindow};
use crate::app::assets::INITIALIZE_SCRIPT;
use crate::app::menu::options::MenuItemOption;
use crate::app::menu::options::MenuOptions;
use crate::app::menu::{self, build_native_item};
use crate::app::utils::url_join;
use crate::app::window_manager::url::get_host_from_url;
use crate::app::window_manager::url::make_base_url;
use crate::{
    app::{NivaApp, NivaWindowTarget},
    log_err, log_if_err, set_property, set_property_some,
};
use anyhow::anyhow;
use anyhow::Result;
use serde_json::json;
use std::default;
use std::str::FromStr;
use std::{borrow::Cow, sync::Arc};
use tao::accelerator::Accelerator;
use tao::{
    menu::{MenuBar, MenuId, MenuItem, MenuItemAttributes},
    window::{Fullscreen, Theme, Window, WindowBuilder},
};
use wry::http::HeaderValue;
use wry::{
    http::Response,
    webview::{FileDropEvent, WebContext, WebView, WebViewBuilder},
};

pub struct NivaBuilder {}

impl NivaBuilder {
    pub fn build_window(
        app: &Arc<NivaApp>,
        manager: &WindowManager,
        _id: u16,
        options: &NivaWindowOptions,
        target: &NivaWindowTarget,
    ) -> Result<Window> {
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

        if let Some(menu) = Self::build_menu(app, &options.menu) {
            set_property!(builder, with_menu, menu);
        }

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
            use windows::Win32::Foundation::HWND;

            if let Some(parent) = &windows_extra.parent_window {
                let parent = manager.get_window(*parent)?;
                let parent = HWND(parent.hwnd() as _);
                set_property!(builder, with_parent_window, parent);
            }

            if let Some(owner) = &windows_extra.parent_window {
                let owner = manager.get_window(*owner)?;
                let owner = HWND(owner.hwnd() as _);
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

        Ok(builder.build(target)?)
    }

    pub fn build_webview(
        app: &Arc<NivaApp>,
        options: &NivaWindowOptions,
        window: Window,
        web_context: &mut WebContext,
    ) -> Result<WebView> {
        let id_name = app.launch_info.id_name.clone();
        let protocol = "niva";

        let debug_entry = app.launch_info.arguments.debug_entry.clone();

        let base_url = debug_entry.unwrap_or(make_base_url(protocol, &id_name));

        let entry_url = url_join(
            &base_url,
            &options.entry.clone().unwrap_or_default()
        );

        let mut builder = WebViewBuilder::new(window)?;

        set_property!(builder, with_web_context, web_context);
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
        builder = builder.with_custom_protocol(protocol.to_string(), move |request| {
            let hostname = request.uri().host().unwrap_or(&id_name);

            let mut path = request.uri().path().to_string();

            if path.ends_with('/') {
                path += "index.html";
            }

            let result = (|| -> Result<Vec<u8>> {
                if hostname == &id_name {
                    let path = path.strip_prefix('/').unwrap_or("index.html");
                    custom_protocol_app.resource().load(&path.to_string())
                } else if hostname == "filesystem" {
                    #[cfg(target_os = "windows")]
                    let path = path.strip_prefix('/').unwrap_or("index.html");
                    Ok(std::fs::read(&path)?)
                } else {
                    Err(anyhow!("Invalid hostname: {}", hostname))
                }
            })();
            
            let origin = get_host_from_url(&request.uri().to_string()).unwrap_or("*".to_string());

            match result {
                Err(err) => Ok(Response::builder()
                    .status(404)
                    .header("Content-Type", "text/plain; charset=utf-8")
                    .body(Cow::Owned(err.to_string().into_bytes()))?),

                Ok(content) => {
                    let mime_type = mime_guess::from_path(path)
                        .first()
                        .unwrap_or(mime_guess::mime::TEXT_PLAIN)
                        .to_string();

                    Ok(Response::builder()
                        .status(200)
                        .header("Content-Type", mime_type)
                        .header("Access-Control-Allow-Origin", origin)
                        .body(Cow::Owned(content))?)
                }
            }
        });

        let drop_app = app.clone();
        set_property!(builder, with_file_drop_handler, move |window, event| {
            let window_result = drop_app
                .window()
                .and_then(|w| w.get_window_inner(window.id()));
            match window_result {
                Ok(window) => match event {
                    FileDropEvent::Hovered { paths, position } => {
                        let position = position.to_logical::<f64>(window.scale_factor());
                        log_if_err!(window.send_ipc_event(
                            "fileDrop.hovered",
                            json!({
                                "paths": paths,
                                "position": position,
                            }),
                        ));
                    }
                    FileDropEvent::Dropped { paths, position } => {
                        let position = position.to_logical::<f64>(window.scale_factor());
                        log_if_err!(window.send_ipc_event(
                            "fileDrop.dropped",
                            json!({
                                "paths": paths,
                                "position": position,
                            }),
                        ));
                    }
                    FileDropEvent::Cancelled => {
                        log_if_err!(window.send_ipc_event("fileDrop.cancelled", json!(null)));
                    }
                    _ => (),
                },
                Err(err) => {
                    log_err!(err);
                }
            }
            false
        });

        let ipc_app = app.clone();
        set_property!(builder, with_ipc_handler, move |window, request_str| {
            if let Err(err) = ipc_app.api().and_then(|w| w.call(window, request_str)) {
                let window = ipc_app
                    .window()
                    .and_then(|w| w.get_window_inner(window.id()));
                if let Ok(window) = window {
                    log_if_err!(window.send_ipc_callback(json!({
                        "ipc.error": err.to_string(),
                    })));
                }
            };
        });

        Ok(builder.with_url(&entry_url)?.build()?)
    }

    pub fn build_menu(
        app: &Arc<NivaApp>,
        menu_options: &Option<WindowMenuOptions>,
    ) -> Option<MenuBar> {
        #[cfg(target_os = "macos")]
        let default_menu = Self::macos_default_menu();
        #[cfg(target_os = "macos")]
        let menu_options = match &menu_options {
            Some(_) => menu_options,
            None => &default_menu,
        };

        if let Some(window_menu_options) = menu_options {
            let mut menu = MenuBar::new();
            for WindowRootMenu {
                label,
                enabled,
                children,
            } in window_menu_options
            {
                let mut root_menu = MenuBar::new();
                Self::build_custom_menu(app, &mut root_menu, children);
                menu.add_submenu(&label, enabled.unwrap_or(true), root_menu);
            }
            return Some(menu);
        }
        None
    }

    fn build_custom_menu(app: &Arc<NivaApp>, menu: &mut MenuBar, options: &MenuOptions) {
        for option in options {
            match option {
                MenuItemOption::Native { label } => {
                    menu.add_native_item(build_native_item(label));
                }
                MenuItemOption::Item {
                    label,
                    id,
                    enabled,
                    selected,
                    icon,
                    accelerator,
                } => {
                    let mut attr = MenuItemAttributes::new(label).with_id(MenuId(*id));
                    set_property_some!(attr, with_enabled, enabled);
                    set_property_some!(attr, with_selected, selected);

                    #[cfg(target_os = "macos")]
                    if let Some(accelerator) = accelerator {
                        if let Ok(accelerator) = Accelerator::from_str(accelerator) {
                            set_property!(attr, with_accelerators, &accelerator);
                        }
                    }

                    let mut item = menu.add_item(attr);

                    #[cfg(target_os = "macos")]
                    if let Some(icon) = icon {
                        let icon = app.resource().load_icon(icon);
                        if let Ok(icon) = icon {
                            item.set_icon(icon);
                        }
                    }
                }
                MenuItemOption::Menu {
                    label,
                    enabled,
                    children,
                } => {
                    let mut submenu = MenuBar::new();
                    Self::build_custom_menu(app, &mut submenu, children);
                    menu.add_submenu(label, enabled.unwrap_or(true), submenu);
                }
            }
        }
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
