use crate::lock;
use crate::set_property;
use crate::unsafe_impl_sync_send;

use super::utils::Counter;
use super::{
    options::{MenuItemOptions, MenuOptions},
    utils::{arc_mut, ArcMut},
    NivaApp, NivaWindowTarget,
};
use tao::TrayId;

use anyhow::{anyhow, Ok, Result};
use serde::Deserialize;
use std::{collections::HashMap, sync::Arc};
use tao::system_tray::SystemTray;
use tao::{
    menu::{ContextMenu, MenuId, MenuItem, MenuItemAttributes},
    system_tray::SystemTrayBuilder,
};

#[cfg(target_os = "macos")]
use tao::platform::macos::{SystemTrayBuilderExtMacOS, SystemTrayExtMacOS};

#[derive(Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct NivaTrayOptions {
    pub icon: String,
    pub title: Option<String>,
    pub tooltip: Option<String>,
    pub menu: Option<MenuOptions>,
}

#[derive(Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct NivaTrayUpdateOptions {
    pub icon: Option<String>,
    pub title: Option<String>,
    pub tooltip: Option<String>,
    pub menu: Option<MenuOptions>,
}

unsafe_impl_sync_send!(NivaTrayManager);
pub struct NivaTrayManager {
    counter: Counter<u16>,
    app: Option<Arc<NivaApp>>,
    trays: HashMap<u16, ArcMut<SystemTray>>,
}

impl NivaTrayManager {
    pub fn new() -> ArcMut<NivaTrayManager> {
        arc_mut(NivaTrayManager {
            counter: Counter::<u16>::new(0),
            app: None,
            trays: HashMap::new(),
        })
    }

    pub fn bind_app(&mut self, app: Arc<NivaApp>) {
        self.app = Some(app);
    }

    pub fn create(&mut self, options: &NivaTrayOptions, target: &NivaWindowTarget) -> Result<u16> {
        let id = self.counter.next();
        let tray = self.build_tray(id, options, target)?;
        self.trays.insert(id, tray);
        Ok(id)
    }

    pub fn destroy(&mut self, id: u16) -> Result<()> {
        let _tray = self
            .trays
            .remove(&id)
            .ok_or(anyhow!("Tray with id {} not found", id))?;

        #[cfg(target_os = "windows")]
        drop(lock!(_tray));

        Ok(())
    }

    pub fn destroy_all(&mut self) -> Result<()> {
        #[cfg(target_os = "windows")]
        for tray in self.trays.values() {
            drop(lock!(tray));
        }
        self.trays.clear();
        Ok(())
    }

    pub fn list(&self) -> Result<Vec<u16>> {
        Ok(self.trays.keys().copied().collect())
    }

    pub fn update(&mut self, id: u16, options: &NivaTrayUpdateOptions) -> Result<()> {
        let tray = self
            .trays
            .get(&id)
            .ok_or(anyhow!("Tray with id {} not found", id))?;
        let mut tray = lock!(tray);

        if let Some(icon) = options.icon.clone() {
            let icon = self
                .app
                .clone()
                .ok_or(anyhow!("App not bound to tray manager"))?
                .resource
                .load_icon(icon)?;
            tray.set_icon(icon);
        }

        #[cfg(target_os = "macos")]
        if let Some(title) = options.title.clone() {
            tray.set_title(&title);
        }

        Ok(())
    }

    fn build_tray(
        &self,
        id: u16,
        options: &NivaTrayOptions,
        target: &NivaWindowTarget,
    ) -> Result<ArcMut<SystemTray>> {
        let app = self
            .app
            .clone()
            .ok_or(anyhow!("App not bound to tray manager"))?;

        let icon = app.resource.load_icon(options.icon.clone())?;

        let menu = options.menu.as_ref().map(Self::build_menu);

        let mut builder = SystemTrayBuilder::new(icon, menu);
        set_property!(builder, with_id, TrayId(id));

        #[cfg(target_os = "macos")]
        if let Some(title) = options.title.clone() {
            set_property!(builder, with_title, &title);
        }

        if let Some(tooltip) = options.tooltip.clone() {
            set_property!(builder, with_tooltip, &tooltip);
        }

        Ok(arc_mut(builder.build(target)?))
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
