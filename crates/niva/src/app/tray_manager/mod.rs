use crate::app::resource_manager::image_utils::png_to_tray_icon;
use crate::app::utils::merge_id;
use crate::app::window_manager::builder::NivaBuilder;
use crate::lock;
use crate::unsafe_impl_sync_send;

use super::menu::options::MenuOptions;
use super::utils::IdCounter;
use super::{
    NivaApp, NivaWindowTarget,
    utils::{ArcMut, arc_mut},
};

use anyhow::{Result, anyhow};
use serde::Deserialize;
use std::collections::HashSet;
use std::{collections::HashMap, sync::Arc};
use tray_icon::{TrayIcon, TrayIconBuilder};

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
    trays: HashMap<u8, (u8, HashSet<u8>, ArcMut<TrayIcon>)>,
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
        for (window_id, menu_ids, _) in self.trays.values() {
            if menu_ids.contains(&menu_id) {
                return Some(*window_id);
            }
        }
        None
    }

    /// Look up the owning window id by tray-icon id string.
    pub fn get_window_id_by_tray_id(&self, tray_id: &str) -> Option<(u8, u8)> {
        for (id, (window_id, _, _)) in &self.trays {
            if tray_icon_id(*window_id, *id) == tray_id {
                return Some((*window_id, *id));
            }
        }
        None
    }

    pub fn create(
        &mut self,
        window_id: u8,
        options: &NivaTrayOptions,
        _target: &NivaWindowTarget,
    ) -> Result<u8> {
        let id = self.id_counter.next(&self.trays)?;
        let tray = self.build_tray(id, window_id, options)?;
        let menu_ids = if let Some(options) = &options.menu {
            Self::get_menu_ids(options)
        } else {
            HashSet::new()
        };

        self.trays.insert(id, (window_id, menu_ids, tray));
        Ok(id)
    }

    pub fn get(&self, id: u8) -> Result<&(u8, HashSet<u8>, ArcMut<TrayIcon>)> {
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

        self.trays
            .remove(&id)
            .ok_or(anyhow!("Tray with id {} not found", id))?;

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

        let tray = lock!(tray)?;

        if let Some(icon_path) = options.icon.clone() {
            let app = self
                .app
                .clone()
                .ok_or(anyhow!("App not bound to tray manager"))?;
            let data = app.resource().load(&icon_path)?;
            let icon = png_to_tray_icon(&data)?;
            tray.set_icon(Some(icon))?;
        }

        if let Some(tooltip) = options.tooltip.clone() {
            tray.set_tooltip(Some(tooltip))?;
        }

        #[cfg(target_os = "macos")]
        if let Some(title) = options.title.clone() {
            tray.set_title(Some(title));
        }
        #[cfg(not(target_os = "macos"))]
        let _ = &options.title;

        if let Some(menu_options) = &options.menu {
            let app = self
                .app
                .clone()
                .ok_or(anyhow!("App not bound to tray manager"))?;
            let menu = NivaBuilder::build_tray_menu(window_id, &app, menu_options);
            tray.set_menu(Some(Box::new(menu)));
        }

        Ok(())
    }

    fn build_tray(
        &self,
        id: u8,
        window_id: u8,
        options: &NivaTrayOptions,
    ) -> Result<ArcMut<TrayIcon>> {
        let app = self
            .app
            .clone()
            .ok_or(anyhow!("App not bound to tray manager"))?;

        let icon_data = app.resource().load(&options.icon)?;
        let icon = png_to_tray_icon(&icon_data)?;

        let mut builder = TrayIconBuilder::new()
            .with_id(tray_icon_id(window_id, id))
            .with_icon(icon);

        if let Some(menu_options) = &options.menu {
            let menu = NivaBuilder::build_tray_menu(window_id, &app, menu_options);
            builder = builder.with_menu(Box::new(menu));
        }

        if let Some(tooltip) = options.tooltip.clone() {
            builder = builder.with_tooltip(tooltip);
        }

        #[cfg(target_os = "macos")]
        if let Some(title) = options.title.clone() {
            builder = builder.with_title(title);
        }

        let tray = builder.build()?;
        Ok(arc_mut(tray))
    }

    fn get_menu_ids(options: &MenuOptions) -> HashSet<u8> {
        use super::menu::options::MenuItemOption;

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

fn tray_icon_id(window_id: u8, id: u8) -> String {
    merge_id(window_id, id).to_string()
}
