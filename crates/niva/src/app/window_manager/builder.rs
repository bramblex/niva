use super::{options::NivaWindowOptions, window::NivaWindow};
use crate::{
    app::{
        options::{MenuItemOptions, MenuOptions},
        NivaApp, NivaId, NivaWindowTarget,
    },
    log_err, log_if_err, set_property, set_property_some,
};
use anyhow::Result;
use serde_json::json;
use std::{borrow::Cow, sync::Arc};
use tao::{
    menu::{MenuBar, MenuId, MenuItem, MenuItemAttributes},
    platform::macos::WindowBuilderExtMacOS,
    window::{Fullscreen, Theme, Window, WindowBuilder},
};
use wry::{
    http::Response,
    webview::{FileDropEvent, WebContext, WebView, WebViewBuilder},
};

static INITIALIZE_SCRIPT: &str = include_str!("../../../assets/initialize_script.js");

pub struct NivaBuilder {}

impl NivaBuilder {
    pub fn build_window(
        app: &Arc<NivaApp>,
        _id: NivaId,
        parent: Option<Arc<NivaWindow>>,
        options: &NivaWindowOptions,
        target: &NivaWindowTarget,
    ) -> Result<Window> {
        let mut builder = WindowBuilder::new();

        #[cfg(target_os = "macos")]
        if let Some(parent) = parent {
            use tao::platform::macos::WindowExtMacOS;
            set_property!(builder, with_parent_window, parent.ns_window());
        }

        #[cfg(target_os = "windows")]
        if let Some(parent) = parent {
            use tao::platform::windows::WindowExtWindows;
            use windows::Win32::Foundation::HWND;
            set_property!(builder, with_parent_window, HWND(main_window.hwnd() as _));
        }

        set_property_some!(
            builder,
            with_title,
            options.title.clone().or(Some(app.launch_info.name.clone()))
        );

        #[cfg(target_os = "windows")]
        set_property_some!(
            builder,
            with_window_icon,
            options
                .icon
                .clone()
                .map(move |icon_path| app.resource.load_icon(icon_path).ok())
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

        if let Some(menu) = Self::build_menu(&options.menu) {
            set_property!(builder, with_menu, menu);
        }

        Ok(builder.build(target)?)
    }

    #[cfg(target_os = "macos")]
    pub fn build_menu(menu_options: &Option<MenuOptions>) -> Option<MenuBar> {
        let menu_options = menu_options.clone().unwrap_or(MenuOptions(vec![]));
        let mut menu = MenuBar::new();
        let (native_menu_name, native_menu) = Self::build_default_menu();
        menu.add_submenu(&native_menu_name, true, native_menu);
        Self::build_custom_menu(&mut menu, &menu_options.0);
        Some(menu)
    }

    #[cfg(target_os = "windows")]
    pub fn build_menu(menu_options: &Option<MenuOptions>) -> Option<MenuBar> {
        if let Some(menu_options) = menu_options {
            let mut menu = MenuBar::new();
            Self::build_custom_menu(&mut menu, &menu_options.0);
            return Some(menu);
        }
        None
    }

    fn build_custom_menu(menu: &mut MenuBar, menu_item_options_list: &Vec<MenuItemOptions>) {
        for options in menu_item_options_list {
            match options {
                MenuItemOptions::NativeItem(label) => {
                    Self::build_native_item(menu, label);
                }
                MenuItemOptions::MenuItem(label, id) => {
                    menu.add_item(MenuItemAttributes::new(label).with_id(MenuId(*id)));
                }
                MenuItemOptions::SubMenu(label, submenu_item_options_list) => {
                    let mut submenu = MenuBar::new();
                    Self::build_custom_menu(&mut submenu, submenu_item_options_list);
                    menu.add_submenu(label, true, submenu);
                }
            }
        }
    }

    fn build_native_item(menu: &mut MenuBar, label: &str) {
        match label {
            "---" => {
                menu.add_native_item(MenuItem::Separator);
            }
            "Separator" => {
                menu.add_native_item(MenuItem::Separator);
            }
            _ => (),
        }
    }

    #[cfg(target_os = "macos")]
    fn build_default_menu() -> (String, MenuBar) {
        let native_menu_name = "File".to_string();
        let mut native_menu = MenuBar::new();

        native_menu.add_native_item(MenuItem::SelectAll);
        native_menu.add_native_item(MenuItem::Copy);
        native_menu.add_native_item(MenuItem::Paste);
        native_menu.add_native_item(MenuItem::Cut);
        native_menu.add_native_item(MenuItem::Undo);
        native_menu.add_native_item(MenuItem::Redo);

        native_menu.add_native_item(MenuItem::Separator);

        native_menu.add_native_item(MenuItem::Quit);

        (native_menu_name, native_menu)
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
        #[cfg(target_os = "macos")]
        let base_url = debug_entry.unwrap_or(format!("{}://{}", protocol, id_name));
        #[cfg(target_os = "windows")]
        let base_url = debug_entry.unwrap_or(format!("https://{}.{}", protocol, id_name));

        let entry_url = format!(
            "{}/{}",
            base_url,
            options.entry.clone().unwrap_or("".to_string())
        );

        let mut builder = WebViewBuilder::new(window)?;

        set_property!(builder, with_web_context, web_context);
        set_property!(builder, with_initialization_script, INITIALIZE_SCRIPT);
        set_property!(builder, with_accept_first_mouse, true);
        set_property!(builder, with_clipboard, true);
        set_property!(builder, with_background_color, (255, 255, 255, 0));
        set_property_some!(builder, with_devtools, options.devtools);
        set_property_some!(builder, with_transparent, options.transparent);

        let prefix = base_url;
        set_property!(builder, with_navigation_handler, move |url| url
            .starts_with(&prefix));

        let resource_manager = app.resource.clone();
        builder = builder.with_custom_protocol(protocol.to_string(), move |request| {
            let mut path = request.uri().path().to_string();
            if path.ends_with('/') {
                path += "index.html";
            }
            let path = path.strip_prefix('/').unwrap_or("index.html");
            let result = resource_manager.load(path.to_string());

            match result {
                Err(err) => Ok(Response::builder()
                    .status(404)
                    .body(Cow::Owned(err.to_string().into_bytes()))?),
                Ok(content) => {
                    let mime_type = mime_guess::from_path(path)
                        .first()
                        .unwrap_or(mime_guess::mime::TEXT_PLAIN)
                        .to_string();

                    Ok(Response::builder()
                        .status(200)
                        .header("Content-Type", mime_type)
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
                        log_if_err!(window.send_ipc_event(
                            "fileDrop.hovered",
                            json!({
                                "paths": paths,
                                "position": (position.x, position.y),
                            }),
                        ));
                    }
                    FileDropEvent::Dropped { paths, position } => {
                        log_if_err!(window.send_ipc_event(
                            "fileDrop.dropped",
                            json!({
                                "paths": paths,
                                "position": (position.x, position.y),
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
}
