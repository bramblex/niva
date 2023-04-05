use crate::{log_if_err, unsafe_impl_sync_send};

use super::{
    api::register_api_instances,
    utils::{arc_mut, ArcMut, IdCounter},
    NivaEventLoop,
};
use anyhow::{anyhow, Result};
use serde::Deserialize;
use std::{collections::HashMap, str::FromStr};
use tao::{
    accelerator::{Accelerator, AcceleratorId},
    global_shortcut::{GlobalShortcut, ShortcutManager},
};

#[derive(Deserialize, Clone, Debug)]
pub struct ShortcutOption {
    pub accelerator: String,
    pub id: u16,
}

pub type ShortcutsOptions = Vec<ShortcutOption>;

unsafe_impl_sync_send!(NivaShortcutManager);
pub struct NivaShortcutManager {
    manager: ShortcutManager,
    shortcuts: HashMap<u16, (u16, String, GlobalShortcut)>,
    id_counter: IdCounter,
}

impl NivaShortcutManager {
    pub fn new(
        event_loop: &NivaEventLoop,
    ) -> ArcMut<NivaShortcutManager> {
        let manager = NivaShortcutManager {
            manager: ShortcutManager::new(event_loop),
            shortcuts: HashMap::new(),
            id_counter: IdCounter::new(),
        };
        arc_mut(manager)
    }

    pub fn get(&self, id: u16) -> Result<&(u16, String, GlobalShortcut)> {
        self.shortcuts
            .get(&id)
            .ok_or(anyhow!("Shortcut with id {} not found", id))
    }

    pub fn register_with_options(
        &mut self,
        window_id: u16,
        options: &ShortcutsOptions,
    ) -> Result<()> {
        for ShortcutOption { accelerator, id } in options {
            self.register_with_id(window_id, *id, accelerator.clone())?;
        }
        Ok(())
    }

    pub fn register_with_id(
        &mut self,
        window_id: u16,
        id: u16,
        accelerator_str: String,
    ) -> Result<()> {
        if self.shortcuts.contains_key(&id) {
            return Err(anyhow!("Shortcetet with id {} already registered", id));
        }

        let accelerator = Accelerator::from_str(&accelerator_str)
            .map_err(|err| anyhow!("{}", err.to_string()))?
            .with_id(AcceleratorId(id));
        let shortcut = self.manager.register(accelerator)?;

        self.shortcuts
            .insert(id, (window_id, accelerator_str, shortcut));
        Ok(())
    }

    pub fn register(&mut self, window_id: u16, accelerator_str: String) -> Result<u16> {
        let id = self.id_counter.next(&self.shortcuts)?;
        self.register_with_id(window_id, id, accelerator_str)?;
        Ok(id)
    }

    pub fn unregister(&mut self, window_id: u16, id: u16) -> Result<()> {
        let (owner_id, _, _) = self
            .shortcuts
            .get(&id)
            .ok_or(anyhow!("Shortcut with id {} not found", id))?;
        if window_id != *owner_id {
            return Err(anyhow!(
                "Shortcut with id {} can only unregister in window {}",
                id,
                owner_id
            ));
        }
        let (_, _, shortcut) = self
            .shortcuts
            .remove(&id)
            .ok_or(anyhow!("Shortcut with id {} not found", id))?;
        self.manager.unregister(shortcut)?;
        Ok(())
    }

    pub fn unregister_all(&mut self, window_id: u16) -> Result<()> {
        let shortcuts = self
            .shortcuts
            .iter()
            .filter(|(_, (owner_id, _, _))| *owner_id == window_id)
            .map(|(id, _)| *id)
            .collect::<Vec<_>>();
        for id in shortcuts {
            self.unregister(window_id, id)?;
        }
        Ok(())
    }

    pub fn list(&self, window_id: u16) -> Result<Vec<(u16, String)>> {
        Ok(self
            .shortcuts
            .iter()
            .filter(|(_, (owner_id, _, _))| *owner_id == window_id)
            .map(|(id, (_, accelerator_str, _))| (*id, accelerator_str.clone()))
            .collect())
    }
}
