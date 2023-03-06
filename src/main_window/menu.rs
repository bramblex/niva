use crate::environment::{Config, MenuConfig, MenuItemConfig};
use wry::application::menu::{MenuBar, MenuId, MenuItem, MenuItemAttributes};

pub fn create(config: &Config) -> Option<MenuBar> {
    if let Some(MenuConfig(menu_item_config_list)) = &config.menu {
        let mut menu = MenuBar::new();

        #[cfg(target_os = "macos")]
        {
            let (native_menu_name, native_menu) = create_default_menu(config);
            menu.add_submenu(&native_menu_name, true, native_menu);
        }

        append_custom_menu(&mut menu, menu_item_config_list);
        return Some(menu);
    }

    #[cfg(target_os = "macos")]
    {
        let (native_menu_name, native_menu) = create_default_menu(config);
        let mut menu = MenuBar::new();
        menu.add_submenu(&native_menu_name, true, native_menu);
        Some(menu)
    }
    #[cfg(not(target_os = "macos"))]
    {
        None
    }
}

fn append_custom_menu(menu: &mut MenuBar, menu_item_config_list: &Vec<MenuItemConfig>) {
    for config in menu_item_config_list {
        match config {
            MenuItemConfig::NativeItem(label) => {
                append_native_item(menu, label);
            }
            MenuItemConfig::MenuItem(label, id) => {
                menu.add_item(MenuItemAttributes::new(label).with_id(MenuId(*id)));
            }
            MenuItemConfig::SubMenu(label, submenu_item_config_list) => {
                let mut submenu = MenuBar::new();
                append_custom_menu(&mut submenu, submenu_item_config_list);
                menu.add_submenu(label, true, submenu);
            }
        }
    }
}

fn append_native_item(menu: &mut MenuBar, label: &str) {
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
fn create_default_menu(config: &Config) -> (String, MenuBar) {
    let native_menu_name = config.title.clone().unwrap_or(config.name.clone());
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
