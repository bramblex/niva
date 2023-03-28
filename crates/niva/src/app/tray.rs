use super::options::MenuItemOptions;
use super::options::MenuOptions;
use super::options::NivaTrayOptions;
use super::NivaApp;
use super::NivaEventLoop;
use crate::{set_property, set_property_some};
use anyhow::Result;
use tao::menu::ContextMenu;
use tao::menu::MenuId;
use tao::menu::MenuItem;
use tao::menu::MenuItemAttributes;
use std::sync::Arc;
use tao::system_tray::SystemTray;
use tao::system_tray::SystemTrayBuilder;

pub struct NivaTray {}

impl NivaTray {
    #[cfg(target_os = "macos")]
    pub fn build(
        app: &Arc<NivaApp>,
        options: &NivaTrayOptions,
        event_loop: &NivaEventLoop,
    ) -> Result<SystemTray> {
        use tao::platform::macos::SystemTrayBuilderExtMacOS;

        let icon = app.resource_manager.load_icon(options.icon.clone())?;

        let menu = match &options.menu {
            Some(menu_options) => Some(Self::build_menu(menu_options)),
            None => None,
        };

        let mut builder = SystemTrayBuilder::new(icon, menu);

        if let Some(title) = options.title.clone() {
            set_property!(builder, with_title, &title);
        }

        if let Some(tooltip) = options.tooltip.clone() {
            set_property!(builder, with_tooltip, &tooltip);
        }

        Ok(builder.build(event_loop)?)
    }

    #[cfg(target_os = "windows")]
    pub fn build(app: &Arc<NivaApp>, options: &NivaTrayOptions, event_loop: &NivaEventLoop) {
    }

    fn build_menu(menu_options: &MenuOptions) -> ContextMenu {
        let mut menu = ContextMenu::new();
        Self::build_custom_menu(&mut menu, &menu_options.0);
        menu
    }

    fn build_custom_menu(menu: &mut ContextMenu, menu_item_options_list: &Vec<MenuItemOptions>) {
        for options in menu_item_options_list {
            match options {
                MenuItemOptions::NativeItem(label) => {
                    Self::build_native_item(menu, label);
                }
                MenuItemOptions::MenuItem(label, id) => {
                    menu.add_item(MenuItemAttributes::new(label).with_id(MenuId(*id)));
                }
                MenuItemOptions::SubMenu(label, submenu_item_options_list) => {
                    let mut submenu = ContextMenu::new();
                    Self::build_custom_menu(&mut submenu, submenu_item_options_list);
                    menu.add_submenu(label, true, submenu);
                }
            }
        }
    }

    fn build_native_item(menu: &mut ContextMenu, label: &str) {
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

}
