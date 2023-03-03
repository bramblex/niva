use super::{event::EventContent, menu};
use crate::env::Config;
use wry::application::{
    event_loop::EventLoop,
    window::{Fullscreen, Icon, Theme, Window, WindowBuilder},
};

static DEFAULT_ICON: &[u8] = include_bytes!("../assets/icon.bitmap");

pub fn create(config: &Config, event_loop: &EventLoop<EventContent>) -> Window {
    // window config
    let mut window_builder = WindowBuilder::new();

    let title = config.title.clone().unwrap_or(config.name.clone());
    window_builder = window_builder.with_title(title);

    let icon = (|| {
        if let Some(window_icon) = &config.window_icon {
            let result = std::fs::read(window_icon);
            if let Ok(data) = result {
                let icon = Icon::from_rgba(data, 32, 32);
                if let Ok(icon) = icon {
                    return Some(icon);
                }
            }
        }
        let icon = Icon::from_rgba(DEFAULT_ICON.to_vec(), 32, 32);
        if let Ok(icon) = icon {
            return Some(icon);
        }
        return None;
    })();
    window_builder = window_builder.with_window_icon(icon);

    if let Some(theme_str) = &config.theme {
        match theme_str.as_str() {
            "dark" => {
                window_builder = window_builder.with_theme(Some(Theme::Dark));
            }
            "light" => {
                window_builder = window_builder.with_theme(Some(Theme::Light));
            }
            _ => {}
        }
    }

    if let Some(size) = &config.size {
        window_builder = window_builder.with_inner_size(size.to_dpi_size());
    }

    if let Some(min_size) = &config.min_size {
        window_builder = window_builder.with_min_inner_size(min_size.to_dpi_size());
    }

    if let Some(max_size) = &config.max_size {
        window_builder = window_builder.with_max_inner_size(max_size.to_dpi_size());
    }

    if let Some(position) = &config.position {
        window_builder = window_builder.with_position(position.to_dpi_position());
    }

    if let Some(resizable) = &config.resizable {
        window_builder = window_builder.with_resizable(*resizable);
    }

    if let Some(minimizable) = &config.minimizable {
        window_builder = window_builder.with_minimizable(*minimizable);
    }

    if let Some(maximizable) = &config.maximizable {
        window_builder = window_builder.with_maximizable(*maximizable);
    }

    if let Some(closable) = &config.closable {
        window_builder = window_builder.with_closable(*closable);
    }

    if config.fullscreen.is_some() {
        window_builder = window_builder.with_fullscreen(Some(Fullscreen::Borderless(None)));
    }

    if let Some(maximized) = &config.maximized {
        window_builder = window_builder.with_maximized(*maximized);
    }

    if let Some(visible) = &config.visible {
        window_builder = window_builder.with_visible(*visible);
    }

    if let Some(transparent) = &config.transparent {
        window_builder = window_builder.with_transparent(*transparent);
    }

    if let Some(decorations) = &config.decorations {
        window_builder = window_builder.with_decorations(*decorations);
    }

    if let Some(always_on_top) = &config.always_on_top {
        window_builder = window_builder.with_always_on_top(*always_on_top);
    }

    if let Some(always_on_bottom) = &config.always_on_bottom {
        window_builder = window_builder.with_always_on_bottom(*always_on_bottom);
    }

    if let Some(visible_on_all_workspaces) = &config.visible_on_all_workspaces {
        window_builder = window_builder.with_visible_on_all_workspaces(*visible_on_all_workspaces);
    }

    if let Some(focused) = &config.focused {
        window_builder = window_builder.with_focused(*focused);
    }

    if let Some(content_protection) = &config.content_protection {
        window_builder = window_builder.with_content_protection(*content_protection);
    }

    let menu = menu::create(config);
    window_builder = window_builder.with_menu(menu);

    window_builder.build(event_loop).unwrap()
}
