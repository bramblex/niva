pub mod options;

use self::options::{MenuItemOption, NativeLabel};
use crate::app::NivaApp;
use crate::app::resource_manager::image_utils::png_to_muda_icon;
use crate::app::utils::merge_id;
use crate::log_if_err;
use muda::{CheckMenuItem, IconMenuItem, IsMenuItem, Menu, MenuItem, PredefinedMenuItem, Submenu};
use std::str::FromStr;
use std::sync::Arc;

pub fn build_native_item(label: &NativeLabel) -> PredefinedMenuItem {
    match label {
        NativeLabel::Hide => PredefinedMenuItem::hide(None),
        NativeLabel::Services => PredefinedMenuItem::services(None),
        NativeLabel::HideOthers => PredefinedMenuItem::hide_others(None),
        NativeLabel::ShowAll => PredefinedMenuItem::show_all(None),
        NativeLabel::CloseWindow => PredefinedMenuItem::close_window(None),
        NativeLabel::Quit => PredefinedMenuItem::quit(None),
        NativeLabel::Copy => PredefinedMenuItem::copy(None),
        NativeLabel::Cut => PredefinedMenuItem::cut(None),
        NativeLabel::Undo => PredefinedMenuItem::undo(None),
        NativeLabel::Redo => PredefinedMenuItem::redo(None),
        NativeLabel::SelectAll => PredefinedMenuItem::select_all(None),
        NativeLabel::Paste => PredefinedMenuItem::paste(None),
        NativeLabel::EnterFullScreen => PredefinedMenuItem::fullscreen(None),
        NativeLabel::Minimize => PredefinedMenuItem::minimize(None),
        NativeLabel::Zoom => PredefinedMenuItem::zoom(None),
        NativeLabel::Separator => PredefinedMenuItem::separator(),
    }
}

fn parse_accelerator(accelerator: &str) -> Option<muda::accelerator::Accelerator> {
    muda::accelerator::Accelerator::from_str(accelerator).ok()
}

/// Append menu options into any muda menu-like container (`Menu` or `Submenu`).
fn append_options(
    append: &dyn Fn(&dyn IsMenuItem) -> muda::Result<()>,
    window_id: u8,
    app: &Arc<NivaApp>,
    options: &options::MenuOptions,
) {
    for option in options {
        match option {
            MenuItemOption::Native { label } => {
                let item = build_native_item(label);
                log_if_err!(append(&item));
            }
            MenuItemOption::Item {
                label,
                id,
                enabled,
                selected,
                icon,
                accelerator,
            } => {
                let menu_id = merge_id(window_id, *id).to_string();
                let is_enabled = enabled.unwrap_or(true);
                #[cfg(target_os = "macos")]
                let accel = accelerator.as_deref().and_then(parse_accelerator);
                #[cfg(not(target_os = "macos"))]
                let accel: Option<muda::accelerator::Accelerator> = None;

                if selected.is_some() {
                    let item = CheckMenuItem::with_id(
                        menu_id,
                        label,
                        is_enabled,
                        selected.unwrap_or(false),
                        accel,
                    );
                    log_if_err!(append(&item));
                } else if let Some(icon_path) = icon {
                    #[cfg(target_os = "macos")]
                    {
                        match app
                            .resource()
                            .load(icon_path)
                            .ok()
                            .and_then(|data| png_to_muda_icon(&data).ok())
                        {
                            Some(icon) => {
                                let item = IconMenuItem::with_id(
                                    menu_id,
                                    label,
                                    is_enabled,
                                    Some(icon),
                                    accel,
                                );
                                log_if_err!(append(&item));
                            }
                            None => {
                                let item = MenuItem::with_id(menu_id, label, is_enabled, accel);
                                log_if_err!(append(&item));
                            }
                        }
                    }
                    #[cfg(not(target_os = "macos"))]
                    {
                        let _ = icon_path;
                        let item = MenuItem::with_id(menu_id, label, is_enabled, accel);
                        log_if_err!(append(&item));
                    }
                } else {
                    let item = MenuItem::with_id(menu_id, label, is_enabled, accel);
                    log_if_err!(append(&item));
                }
            }
            MenuItemOption::Menu {
                label,
                enabled,
                children,
            } => {
                let submenu = Submenu::new(label, enabled.unwrap_or(true));
                append_options(&|item| submenu.append(item), window_id, app, children);
                log_if_err!(append(&submenu));
            }
        }
    }
}

/// Build a top-level muda menu from window menu options.
pub fn build_menu(window_id: u8, app: &Arc<NivaApp>, options: &options::MenuOptions) -> Menu {
    let menu = Menu::new();
    append_options(&|item| menu.append(item), window_id, app, options);
    menu
}

/// Build a submenu entry (used for window root menus).
pub fn build_submenu(
    window_id: u8,
    app: &Arc<NivaApp>,
    label: &str,
    enabled: bool,
    children: &options::MenuOptions,
) -> Submenu {
    let submenu = Submenu::new(label, enabled);
    append_options(&|item| submenu.append(item), window_id, app, children);
    submenu
}
