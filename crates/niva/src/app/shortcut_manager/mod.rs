use crate::unsafe_impl_sync_send;

use super::utils::{ArcMut, arc_mut};
use anyhow::{Result, anyhow};
use global_hotkey::{GlobalHotKeyManager, hotkey::HotKey};
use serde::Deserialize;
use std::{collections::HashMap, str::FromStr};

#[derive(Deserialize, Clone, Debug)]
pub struct ShortcutOption {
    pub accelerator: String,
    pub id: u8,
}

pub type NivaShortcutsOptions = Vec<ShortcutOption>;

unsafe_impl_sync_send!(NivaShortcutManager);
pub struct NivaShortcutManager {
    manager: GlobalHotKeyManager,
    /// hotkey id (u32 from global-hotkey) -> (window id, action id, hotkey)
    shortcuts: HashMap<u32, (u8, u8, HotKey)>,
    /// (window id, action id) -> hotkey id, for per-window unregister/list
    index: HashMap<(u8, u8), u32>,
}

impl NivaShortcutManager {
    pub fn new() -> ArcMut<NivaShortcutManager> {
        let manager = NivaShortcutManager {
            manager: GlobalHotKeyManager::new().expect("Failed to create global hotkey manager"),
            shortcuts: HashMap::new(),
            index: HashMap::new(),
        };
        arc_mut(manager)
    }

    pub fn lookup(&self, hotkey_id: u32) -> Option<(u8, u8)> {
        self.shortcuts
            .get(&hotkey_id)
            .map(|(window_id, id, _)| (*window_id, *id))
    }

    pub fn register_with_options(
        &mut self,
        window_id: u8,
        options: &NivaShortcutsOptions,
    ) -> Result<()> {
        for ShortcutOption { accelerator, id } in options {
            self.register_with_id(window_id, *id, accelerator.clone())?;
        }
        Ok(())
    }

    pub fn register_with_id(
        &mut self,
        window_id: u8,
        id: u8,
        accelerator_str: String,
    ) -> Result<()> {
        if self.index.contains_key(&(window_id, id)) {
            return Err(anyhow!("Shortcut with id {} already registered", id));
        }

        let hotkey = HotKey::from_str(&accelerator_str).map_err(|err| anyhow!("{err}"))?;
        let hotkey_id = hotkey.id();
        if self.shortcuts.contains_key(&hotkey_id) {
            return Err(anyhow!("Shortcut {} already registered", accelerator_str));
        }
        self.manager.register(hotkey)?;

        self.shortcuts.insert(hotkey_id, (window_id, id, hotkey));
        self.index.insert((window_id, id), hotkey_id);
        Ok(())
    }

    pub fn register(&mut self, window_id: u8, accelerator_str: String) -> Result<u8> {
        // find a free action id for this window
        let mut id = 0u8;
        while self.index.contains_key(&(window_id, id)) {
            id = id.wrapping_add(1);
            if id == 0 {
                return Err(anyhow!("No free shortcut id"));
            }
        }
        self.register_with_id(window_id, id, accelerator_str)?;
        Ok(id)
    }

    pub fn unregister(&mut self, window_id: u8, id: u8) -> Result<()> {
        let hotkey_id = self
            .index
            .remove(&(window_id, id))
            .ok_or(anyhow!("Shortcut with id {} not found", id))?;
        let (_, _, hotkey) = self
            .shortcuts
            .remove(&hotkey_id)
            .ok_or(anyhow!("Shortcut with id {} not found", id))?;
        self.manager.unregister(hotkey)?;
        Ok(())
    }

    pub fn unregister_all(&mut self, window_id: u8) -> Result<()> {
        let shortcuts = self
            .index
            .keys()
            .filter(|(owner_id, _)| *owner_id == window_id)
            .cloned()
            .collect::<Vec<_>>();
        for (window_id, id) in shortcuts {
            self.unregister(window_id, id)?;
        }
        Ok(())
    }

    pub fn list(&self, window_id: u8) -> Result<Vec<(u8, String)>> {
        let mut result = Vec::new();
        for ((owner_id, id), hotkey_id) in &self.index {
            if *owner_id == window_id
                && let Some((_, _, hotkey)) = self.shortcuts.get(hotkey_id)
            {
                result.push((*id, (*hotkey).into_string()));
            }
        }
        Ok(result)
    }
}
