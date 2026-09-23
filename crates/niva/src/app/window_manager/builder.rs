use super::WindowManager;
use super::options::NivaWindowOptions;
use super::options::WindowMenuOptions;
use super::options::WindowRootMenu;
use crate::app::assets::INITIALIZE_SCRIPT;
use crate::app::custom_protocol::{self, CustomProtocolDispatcher};
use crate::app::http_server::app_server;
use crate::app::menu::options::{MenuItemOption, MenuOptions};
use crate::app::menu::{build_menu as build_muda_menu, build_submenu};
use crate::app::node_compat::NodeCompat;
use crate::app::resource_manager::ResourceManager;
use crate::app::utils::url_join;
use crate::{
    app::{NivaApp, NivaEvent, NivaWindowTarget},
    log_err, log_if_err, set_property, set_property_some,
};
use anyhow::Result;
use serde_json::json;
use std::sync::Arc;
use tao::window::{Fullscreen, Theme, Window, WindowBuilder};
use wry::{
    DragDropEvent, NewWindowResponse, PermissionKind, PermissionResponse, WebContext, WebView,
    WebViewBuilder,
};

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
                with_traffic_light_inset,
                macos_extra.traffic_light_inset
            );
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

            if let Some(owner) = &windows_extra.owner_window {
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
        id: u8,
        options: &NivaWindowOptions,
        window: &Window,
        web_context: &mut WebContext,
        window_id: tao::window::WindowId,
        window_token: &str,
    ) -> Result<(WebView, Option<String>)> {
        let server_port = app_server(app)?;
        let server_origin = format!("http://127.0.0.1:{server_port}");

        // A packaged app must never honor a debug.entry embedded in niva.json.
        // Only an explicit debug launch may load a cross-port development page.
        let explicit_debug = !app.launch_info.arguments.build_mode
            && (app.launch_info.arguments.debug_config.is_some()
                || app.launch_info.arguments.debug_entry.is_some());
        let debug_entry = explicit_debug
            .then(|| {
                app.launch_info.arguments.debug_entry.clone().or_else(|| {
                    app.launch_info
                        .options
                        .debug
                        .as_ref()
                        .and_then(|debug| debug.entry.clone())
                })
            })
            .flatten();

        let configured_entry = options.entry.as_deref().unwrap_or_default();
        let absolute_entry = url::Url::parse(configured_entry).ok();
        let entry_is_niva_app = absolute_entry.as_ref().is_some_and(is_niva_app_url);
        let entry_is_external = absolute_entry.is_some() && !entry_is_niva_app;
        // Packaged resources use the fixed custom origin. A filesystem-backed
        // debug resource and explicit development launch keep the loopback
        // route; absolute remote entries continue to load as remote pages.
        let use_custom_protocol = app.launch_info.arguments.debug_resource.is_none()
            && !explicit_debug
            && !entry_is_external;

        let entry_url = if use_custom_protocol {
            custom_protocol_entry(configured_entry)?
        } else if let Some(absolute_entry) = absolute_entry {
            absolute_entry.to_string()
        } else {
            let base_url = debug_entry.clone().unwrap_or(server_origin.clone());
            resolve_entry_url(&base_url, configured_entry)
        };

        let mut builder = WebViewBuilder::new_with_web_context(web_context);
        // Only an initially local window receives a credential, and only its
        // main frame stores it. Same-origin child frames may obtain it through
        // their parent; cross-origin frames cannot read the parent's globals.
        let initial_url = url::Url::parse(&entry_url).ok();
        let initial_origin = if use_custom_protocol {
            Some(custom_protocol::platform_origin().to_string())
        } else {
            initial_url
                .as_ref()
                .map(|url| url.origin().ascii_serialization())
        };
        let debug_loopback = explicit_debug
            && debug_entry.is_some()
            && initial_url.as_ref().is_some_and(|url| {
                url.scheme() == "http" && matches!(url.host_str(), Some("localhost" | "127.0.0.1"))
            });
        let trusted_ws_origin = if use_custom_protocol {
            initial_origin
        } else {
            initial_origin.filter(|origin| origin == &server_origin || debug_loopback)
        };
        if let Some(trusted_origin) = &trusted_ws_origin {
            let trusted_bootstrap = format!(
                "if(window.top===window && location.origin==={:?} && !location.pathname.startsWith('/__niva_fs/')){{\
                window.__niva_ws_url={:?};window.__niva_window_id={id};\
                window.__niva_token={:?};}}",
                trusted_origin,
                format!("ws://127.0.0.1:{server_port}/__niva_ws"),
                window_token,
            );
            builder = builder.with_initialization_script_for_main_only(trusted_bootstrap, true);
        }
        let init_script = if use_custom_protocol {
            format!(
                "window.__niva_server_origin={:?};window.__niva_compat_origin=location.origin;{INITIALIZE_SCRIPT}",
                server_origin,
            )
        } else {
            format!(
                "window.__niva_server_origin={:?};{INITIALIZE_SCRIPT}",
                server_origin,
            )
        };
        builder = builder.with_initialization_script_for_main_only(init_script, false);

        if use_custom_protocol {
            let dispatcher: CustomProtocolDispatcher = app._custom_protocol.clone();
            let resources: Arc<dyn ResourceManager> = app.resource();
            let node_compat = NodeCompat::from_option(&app.launch_info.options.node_compat)?;
            builder = builder.with_asynchronous_custom_protocol(
                custom_protocol::SCHEME.to_string(),
                move |_webview_id, request, responder| {
                    dispatcher.dispatch(
                        resources.clone(),
                        node_compat.clone(),
                        server_port,
                        request,
                        responder,
                    );
                },
            );
        }

        #[cfg(target_os = "windows")]
        {
            let ipc_app = app.clone();
            builder = builder.with_ipc_handler(move |request| {
                let source_url = request.uri().to_string();
                let body = request.into_body();
                let app = ipc_app.clone();
                smol::spawn(async move {
                    let response = match app.api().ipc_call(id, &source_url, &body).await {
                        Ok(response) => response,
                        Err(err) => {
                            eprintln!("[niva] rejected IPC request: {err}");
                            crate::app::api_manager::ApiManager::ipc_error_response(
                                &body,
                                &err.to_string(),
                            )
                        }
                    };
                    let app_for_main = app.clone();
                    if let Err(err) = crate::app::main_exec::run_on_main(&app, move |_, _| {
                        use wry::WebViewExtWindows;
                        let window = app_for_main.window()?.get_window(id)?;
                        let current = window.webview.url()?;
                        let current_origin =
                            url::Url::parse(&current)?.origin().ascii_serialization();
                        let request_origin =
                            url::Url::parse(&source_url)?.origin().ascii_serialization();
                        if current_origin == request_origin {
                            unsafe {
                                window.webview.webview().PostWebMessageAsString(
                                    &windows::core::HSTRING::from(response.clone()),
                                )?;
                            }
                        }
                        Ok(())
                    })
                    .await
                    {
                        eprintln!("[niva] unable to send IPC response: {err}");
                    }
                })
                .detach();
            });
        }
        set_property!(builder, with_accept_first_mouse, true);
        set_property!(builder, with_clipboard, true);
        set_property_some!(builder, with_devtools, options.devtools);

        if options.transparent.unwrap_or(false) {
            set_property!(builder, with_background_color, (255, 255, 255, 0));
            set_property!(builder, with_transparent, true);
        }

        let title_app = app.clone();
        builder = builder.with_document_title_changed_handler(move |title| {
            let event_app = title_app.clone();
            if let Err(err) = title_app.event_loop_proxy.send_event(NivaEvent::new(
                move |_target, _control_flow| {
                    event_app.window()?.get_window(id)?.set_title(&title);
                    Ok(())
                },
            )) {
                eprintln!("[niva] unable to enqueue webview title update: {err}");
            }
        });

        let load_app = app.clone();
        let trusted_load_origin = trusted_ws_origin.clone();
        let custom_origin = use_custom_protocol;
        builder = builder.with_on_page_load_handler(move |event, url| {
            let page_origin = page_origin(&url, custom_origin);
            if matches!(event, wry::PageLoadEvent::Finished)
                && trusted_load_origin
                    .as_ref()
                    .is_some_and(|origin| page_origin.as_ref() == Some(origin))
            {
                let event_app = load_app.clone();
                smol::spawn(async move {
                    // The page can finish before its local API WebSocket has
                    // completed the handshake. Keep this event until a frame
                    // connects, then stop after a short window for IPC-only
                    // remote pages which do not receive pushed events.
                    for _ in 0..40 {
                        if let Ok(window) = event_app
                            .window()
                            .and_then(|manager| manager.get_window(id))
                            && window.send_ipc_event("webview.loaded", json!({ "url": url }))
                        {
                            break;
                        }
                        smol::Timer::after(std::time::Duration::from_millis(50)).await;
                    }
                })
                .detach();
            }
        });

        // Wry's callbacks expose the requested target/download URL, but not
        // the initiating frame URL. Include the current top-level page URL as
        // context and deny operations that would create a new native surface
        // or write a file without an explicit Niva policy.
        let new_window_app = app.clone();
        builder = builder.with_new_window_req_handler(move |url, _features| {
            queue_webview_request_event(
                &new_window_app,
                id,
                "webview.newWindowRequested",
                json!({ "url": url, "decision": "denied" }),
            );
            NewWindowResponse::Deny
        });

        let download_app = app.clone();
        builder = builder.with_download_started_handler(move |url, _path| {
            queue_webview_request_event(
                &download_app,
                id,
                "webview.downloadStarted",
                json!({ "url": url, "decision": "denied" }),
            );
            false
        });

        let permission_app = app.clone();
        builder = builder.with_permission_handler(move |kind| {
            let response = permission_request_response(kind);
            if response == PermissionResponse::Deny {
                queue_webview_request_event(
                    &permission_app,
                    id,
                    "webview.permissionDenied",
                    json!({ "kind": kind.to_string(), "decision": "denied" }),
                );
            }
            response
        });

        // Frame navigation is checked again at every native API entry point.
        // Wry's macOS navigation callback also sees iframe navigations, so a
        // top-level-only origin allowlist here would break remote iframes.

        let drop_app = app.clone();
        builder = builder.with_drag_drop_handler(move |event| {
            let window_result = drop_app
                .window()
                .and_then(|w| w.get_window_inner(window_id));
            match window_result {
                Ok(window) => match event {
                    DragDropEvent::Enter { paths, position } => {
                        let position = physical_to_logical(position, window.scale_factor());
                        window.send_ipc_event(
                            "fileDrop.hovered",
                            json!({
                                "paths": paths,
                                "position": position,
                            }),
                        );
                    }
                    DragDropEvent::Drop { paths, position } => {
                        let position = physical_to_logical(position, window.scale_factor());
                        window.send_ipc_event(
                            "fileDrop.dropped",
                            json!({
                                "paths": paths,
                                "position": position,
                            }),
                        );
                    }
                    DragDropEvent::Leave => {
                        window.send_ipc_event("fileDrop.cancelled", json!(null));
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

        // NOTE: API traffic goes over our own WebSocket now
        // (see http_server + initialize_script.js); wry's ipc handler is gone.

        Ok((
            builder.with_url(entry_url).build(window)?,
            trusted_ws_origin,
        ))
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

fn custom_protocol_entry(entry: &str) -> Result<String> {
    let base = url::Url::parse("niva://app/")?;
    Ok(base.join(entry)?.to_string())
}

fn resolve_entry_url(base: &str, entry: &str) -> String {
    if entry.is_empty() {
        return base.to_string();
    }
    if entry.starts_with('/') {
        format!("{}{}", base.trim_end_matches('/'), entry)
    } else {
        url_join(base, entry)
    }
}

fn page_origin(raw_url: &str, is_custom_protocol: bool) -> Option<String> {
    let parsed = url::Url::parse(raw_url).ok()?;
    if is_custom_protocol && is_niva_app_url(&parsed) {
        return Some(custom_protocol::platform_origin().to_string());
    }
    #[cfg(target_os = "windows")]
    if is_custom_protocol
        && parsed.scheme() == "http"
        && parsed
            .host_str()
            .is_some_and(|host| host.eq_ignore_ascii_case("niva.app"))
        && parsed.port().is_none()
        && parsed.username().is_empty()
        && parsed.password().is_none()
    {
        return Some(custom_protocol::platform_origin().to_string());
    }
    Some(parsed.origin().ascii_serialization())
}

fn is_niva_app_url(url: &url::Url) -> bool {
    url.scheme() == custom_protocol::SCHEME
        && url
            .host_str()
            .is_some_and(|host| host.eq_ignore_ascii_case("app"))
        && url.port().is_none()
        && url.username().is_empty()
        && url.password().is_none()
}

fn physical_to_logical(position: (i32, i32), scale_factor: f64) -> tao::dpi::LogicalPosition<f64> {
    tao::dpi::PhysicalPosition::new(position.0 as f64, position.1 as f64).to_logical(scale_factor)
}

fn permission_request_response(kind: PermissionKind) -> PermissionResponse {
    match kind {
        // These requests can expose media, location, or other privileged
        // capabilities. Wry's documented default can show a platform prompt;
        // keep the default policy explicit until Niva has a per-origin policy.
        PermissionKind::Camera
        | PermissionKind::Geolocation
        | PermissionKind::Microphone
        | PermissionKind::DisplayCapture
        | PermissionKind::Other => PermissionResponse::Deny,
        _ => PermissionResponse::Default,
    }
}

fn queue_webview_request_event(
    app: &Arc<NivaApp>,
    id: u8,
    event_name: &'static str,
    payload: serde_json::Value,
) {
    let event_app = app.clone();
    if let Err(err) =
        app.event_loop_proxy
            .send_event(NivaEvent::new(move |_target, _control_flow| {
                let window = event_app.window()?.get_window(id)?;
                let page_url = window.webview.url().ok();
                let mut payload = payload.clone();
                if let Some(payload) = payload.as_object_mut() {
                    payload.insert("pageUrl".into(), json!(page_url));
                }
                window.send_ipc_event(event_name, payload);
                Ok(())
            }))
    {
        eprintln!("[niva] unable to enqueue webview request event: {err}");
    }
}

#[cfg(test)]
mod tests {
    use crate::app::utils::error_page_html;

    #[test]
    fn packaged_entry_and_origin_use_platform_custom_protocol_origin() {
        assert_eq!(
            super::custom_protocol_entry("index.html?view=main").unwrap(),
            "niva://app/index.html?view=main"
        );
        assert_eq!(
            super::custom_protocol_entry("/index.html").unwrap(),
            "niva://app/index.html"
        );
        assert_eq!(
            super::page_origin("niva://app/index.html", true).as_deref(),
            Some(crate::app::custom_protocol::platform_origin())
        );
        assert!(!super::is_niva_app_url(
            &url::Url::parse("niva://user@app/index.html").unwrap()
        ));

        #[cfg(target_os = "windows")]
        assert_eq!(
            super::page_origin("http://niva.app/index.html", true).as_deref(),
            Some("http://niva.app")
        );
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn windows_owner_option_deserializes_separately_from_parent() {
        use crate::app::window_manager::options::NivaWindowOptions;

        let options: NivaWindowOptions =
            serde_json::from_value(serde_json::json!({"parentWindow": 1, "ownerWindow": 2}))
                .unwrap();
        let windows_extra = options.windows_extra.unwrap();

        assert_eq!(windows_extra.parent_window, Some(1));
        assert_eq!(windows_extra.owner_window, Some(2));
    }

    #[test]
    fn error_page_renders_status_uri_and_escaped_detail() {
        let (status, body) = error_page_html(
            404,
            "Not Found",
            "http://127.0.0.1:9/missing.html",
            "No such file <x> & \"y\" (os error 2)",
        );
        assert_eq!(status, 404);
        let body = std::str::from_utf8(&body).unwrap();
        assert!(body.contains("<h1>404 Not Found</h1>"), "{body}");
        assert!(body.contains("http://127.0.0.1:9/missing.html"), "{body}");
        assert!(body.contains("No such file &lt;x&gt; &amp;"), "{body}");
        assert!(!body.contains("<x>"), "{body}");
    }

    #[test]
    fn sensitive_webview_permission_requests_are_denied_by_default() {
        use wry::{PermissionKind, PermissionResponse};

        for kind in [
            PermissionKind::Camera,
            PermissionKind::Geolocation,
            PermissionKind::Microphone,
            PermissionKind::DisplayCapture,
            PermissionKind::Other,
        ] {
            assert_eq!(
                super::permission_request_response(kind),
                PermissionResponse::Deny,
                "{kind:?}"
            );
        }

        // Keep the existing Wry/Windows clipboard opt-in and other native
        // browser flows intact; these use the platform's documented default.
        assert_eq!(
            super::permission_request_response(PermissionKind::ClipboardRead),
            PermissionResponse::Default
        );
    }
}
