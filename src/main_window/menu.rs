use crate::env::Config;
use wry::application::menu::{MenuBar, MenuItem};

pub fn create(config: &Config) -> MenuBar {
    let mut menu = MenuBar::new();

    let (native_menu_name, native_menu) = create_native_menu(config);
    menu.add_submenu(&native_menu_name, true, native_menu);

		// TODO: implement custom menu, shortcuts and event by user config
    menu
}

fn create_native_menu(config: &Config) -> (String, MenuBar) {
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