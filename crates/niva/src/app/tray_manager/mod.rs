use crate::app::utils::merge_id;
use crate::lock;
use crate::set_property;
use crate::set_property_some;
use crate::unsafe_impl_sync_send;

use super::menu::build_native_item;
use super::menu::options::MenuItemOption;
use super::menu::options::MenuOptions;
use super::utils::IdCounter;
use super::{
    utils::{arc_mut, ArcMut},
    NivaApp, NivaWindowTarget,
};
use tao::TrayId;

use anyhow::{anyhow, Result};
use serde::Deserialize;
use std::any::Any;
use std::collections::HashSet;
use std::option;
use std::str::FromStr;
use std::{collections::HashMap, sync::Arc};
use tao::accelerator::Accelerator;
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
    id_counter: IdCounter,
    app: Option<Arc<NivaApp>>,
    trays: HashMap<u8, (u8, HashSet<u8>, ArcMut<SystemTray>)>,
}

impl NivaTrayManager {
    pub fn new() -> ArcMut<NivaTrayManager> {
        arc_mut(NivaTrayManager {
            id_counter: IdCounter::new(),
            app: None,
            trays: HashMap::new(),
        })
    }

    pub fn bind_app(&mut self, app: Arc<NivaApp>) {
        self.app = Some(app);
    }

    pub fn get_window_id_by_menu_id(&self, menu_id: u8) -> Option<u8> {
        for (_, (window_id, menu_ids, _)) in &self.trays {
            if menu_ids.contains(&menu_id) {
                return Some(*window_id);
            }
        }
        None
    }

    pub fn create(
        &mut self,
        window_id: u8,
        options: &NivaTrayOptions,
        target: &NivaWindowTarget,
    ) -> Result<u8> {
        let id = self.id_counter.next(&self.trays)?;
        let tray = self.build_tray(id, window_id, options, target)?;
        let menu_ids = if let Some(options) = &options.menu {
            Self::get_menu_ids(options)
        } else {
            HashSet::new()
        };

        self.trays.insert(id, (window_id, menu_ids, tray));
        Ok(id)
    }

    pub fn get(&self, id: u8) -> Result<&(u8, HashSet<u8>, ArcMut<SystemTray>)> {
        self.trays
            .get(&id)
            .ok_or(anyhow!("Tray with id {} not found", id))
    }

    pub fn destroy(&mut self, window_id: u8, id: u8) -> Result<()> {
        let (owner_id, _, _tray) = self
            .trays
            .get(&id)
            .ok_or(anyhow!("Tray with id {} not found", id))?;

        if window_id != *owner_id {
            return Err(anyhow!(
                "Tray with id {} can only unregister in window {}",
                id,
                owner_id
            ));
        }

        let (_, _, _tray) = self
            .trays
            .remove(&id)
            .ok_or(anyhow!("Tray with id {} not found", id))?;

        #[cfg(target_os = "windows")]
        drop(lock!(_tray));

        Ok(())
    }

    pub fn destroy_all(&mut self, window_id: u8) -> Result<()> {
        let trays = self
            .trays
            .iter()
            .filter(|(_, (owner_id, _, _))| *owner_id == window_id)
            .map(|(id, _)| *id)
            .collect::<Vec<_>>();
        for id in trays {
            self.destroy(window_id, id)?;
        }
        Ok(())
    }

    pub fn list(&self, window_id: u8) -> Result<Vec<u8>> {
        Ok(self
            .trays
            .iter()
            .filter(|(_, (owner_id, _, _))| *owner_id == window_id)
            .map(|(id, _)| *id)
            .collect())
    }

    pub fn update(&mut self, window_id: u8, id: u8, options: &NivaTrayUpdateOptions) -> Result<()> {
        let (owner_id, _, tray) = self
            .trays
            .get(&id)
            .ok_or(anyhow!("Tray with id {} not found", id))?;

        if window_id != *owner_id {
            return Err(anyhow!(
                "Tray with id {} can only update in window {}",
                id,
                owner_id
            ));
        }

        let mut tray = lock!(tray)?;

        if let Some(icon) = options.icon.clone() {
            let icon = self
                .app
                .clone()
                .ok_or(anyhow!("App not bound to tray manager"))?
                .resource()
                .load_icon(&icon)?;
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
        id: u8,
        window_id: u8,
        options: &NivaTrayOptions,
        target: &NivaWindowTarget,
    ) -> Result<ArcMut<SystemTray>> {
        let app = self
            .app
            .clone()
            .ok_or(anyhow!("App not bound to tray manager"))?;

        let icon = app.resource().load_icon(&options.icon)?;

        let menu = options
            .menu
            .as_ref()
            .map(|m| Self::build_menu(window_id, &app, m));
        let mut builder = SystemTrayBuilder::new(icon, menu);

        set_property!(builder, with_id, TrayId(merge_id(window_id, id)));

        #[cfg(target_os = "macos")]
        if let Some(title) = options.title.clone() {
            set_property!(builder, with_title, &title);
        }

        if let Some(tooltip) = options.tooltip.clone() {
            set_property!(builder, with_tooltip, &tooltip);
        }

        Ok(arc_mut(builder.build(target)?))
    }

    fn build_menu(window_id: u8, app: &Arc<NivaApp>, menu_options: &MenuOptions) -> ContextMenu {
        let mut menu = ContextMenu::new();
        Self::build_custom_menu(window_id, app, &mut menu, &menu_options);
        menu
    }

    fn build_custom_menu(
        window_id: u8,
        app: &Arc<NivaApp>,
        menu: &mut ContextMenu,
        options: &MenuOptions,
    ) {
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
                    let mut attr =
                        MenuItemAttributes::new(label).with_id(MenuId(merge_id(window_id, *id)));
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
                    let mut submenu = ContextMenu::new();
                    Self::build_custom_menu(window_id, app, &mut submenu, children);
                    menu.add_submenu(label, enabled.unwrap_or(true), submenu);
                }
            }
        }
    }

    fn get_menu_ids(options: &MenuOptions) -> HashSet<u8> {
        let mut ids = HashSet::<u8>::new();

        fn get_menu_item_id(item: &MenuItemOption, ids: &mut HashSet<u8>) {
            match item {
                MenuItemOption::Item { id, .. } => {
                    ids.insert(*id);
                }
                MenuItemOption::Menu { children, .. } => {
                    for child in children {
                        get_menu_item_id(child, ids);
                    }
                }
                _ => {}
            }
        }

        for item in options {
            get_menu_item_id(item, &mut ids)
        }

        ids
    }
}
