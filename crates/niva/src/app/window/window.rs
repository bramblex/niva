use std::{ops::Deref, sync::Arc};

use anyhow::Result;
use tao::window::{Fullscreen, Theme, Window, WindowBuilder};
use wry::webview::{WebView, WebViewBuilder};

use crate::{
    app::{event::NivaWindowTarget, NivaAppRef},
    set_property, set_property_some,
};

use super::{options::NivaWindowOptions, webview::NivaWebview, NivaWindowManager};

pub struct NivaWindow {
    pub id: u8,
    pub webview: NivaWebview,
    pub custom_block_request: bool,
}

pub type NivaWindowRef = Arc<NivaWindow>;

impl NivaWindow {
    pub async fn new(
        app: &NivaAppRef,
        manager: &mut NivaWindowManager,
        target: &NivaWindowTarget,
        id: u8,
        options: &NivaWindowOptions,
    ) -> Result<NivaWindowRef> {
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

        // if let Some(icon_path) = &options.icon {
        //     let icon = app.resource().load_icon(icon_path)?;
        //     set_property!(builder, with_window_icon, Some(icon));
        // }

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

        // if let Some(menu) = Self::build_menu(_id, app, &options.menu) {
        //     set_property!(builder, with_menu, menu);
        // }

        #[cfg(target_os = "macos")]
        if let Some(macos_extra) = &options.macos_extra {
            use tao::platform::macos::{WindowBuilderExtMacOS, WindowExtMacOS};

            if let Some(parent) = &macos_extra.parent_window {
                let parent = manager.get(*parent)?;
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

            // if let Some(icon_path) = &windows_extra.taskbar_icon {
            //     let icon = app.resource().load_icon(icon_path)?;
            //     set_property!(builder, with_taskbar_icon, Some(icon));
            // }

            set_property_some!(builder, with_skip_taskbar, windows_extra.skip_taskbar);
            set_property_some!(
                builder,
                with_undecorated_shadow,
                windows_extra.undecorated_shadow
            );
        }

        let window = builder.build(target)?;
        let webview = NivaWebview::new(app, manager, window, options).await?;
        Ok(Arc::new(Self {
            id,
            webview,
            custom_block_request: options.custom_close_request.unwrap_or(false),
        }))
    }
}

impl Deref for NivaWindow {
    type Target = Window;
    fn deref(&self) -> &Self::Target {
        &self.webview.window()
    }
}
