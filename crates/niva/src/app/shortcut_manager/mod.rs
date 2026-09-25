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
    manager: Option<GlobalHotKeyManager>,
    manager_initialization_error: Option<String>,
    /// hotkey id (u32 from global-hotkey) -> (window id, action id, hotkey)
    shortcuts: HashMap<u32, (u8, u8, HotKey)>,
    /// (window id, action id) -> hotkey id, for per-window unregister/list
    index: HashMap<(u8, u8), u32>,
}

impl NivaShortcutManager {
    pub fn new() -> ArcMut<NivaShortcutManager> {
        let (manager, manager_initialization_error) = match GlobalHotKeyManager::new() {
            Ok(manager) => (Some(manager), None),
            Err(err) => {
                let reason = err.to_string();
                crate::niva_log!(
                    crate::app::logging::Level::Warn,
                    "[niva] global shortcuts are disabled because manager initialization failed: {reason}"
                );
                (None, Some(reason))
            }
        };

        let manager = NivaShortcutManager {
            manager,
            manager_initialization_error,
            shortcuts: HashMap::new(),
            index: HashMap::new(),
        };
        arc_mut(manager)
    }

    fn global_manager(&self) -> Result<&GlobalHotKeyManager> {
        self.manager.as_ref().ok_or_else(|| {
            anyhow!(
                "Global shortcuts are unavailable because manager initialization failed: {}",
                self.manager_initialization_error
                    .as_deref()
                    .unwrap_or("unknown initialization error")
            )
        })
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
        if options.is_empty() {
            return Ok(());
        }
        self.global_manager()?;
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
        let manager = self.global_manager()?;
        if self.index.contains_key(&(window_id, id)) {
            return Err(anyhow!("Shortcut with id {} already registered", id));
        }

        let hotkey = HotKey::from_str(&accelerator_str).map_err(|err| anyhow!("{err}"))?;
        let hotkey_id = hotkey.id();
        if self.shortcuts.contains_key(&hotkey_id) {
            return Err(anyhow!("Shortcut {} already registered", accelerator_str));
        }
        manager.register(hotkey)?;

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
        self.global_manager()?;
        let hotkey_id = self
            .index
            .remove(&(window_id, id))
            .ok_or(anyhow!("Shortcut with id {} not found", id))?;
        let (_, _, hotkey) = self
            .shortcuts
            .remove(&hotkey_id)
            .ok_or(anyhow!("Shortcut with id {} not found", id))?;
        self.global_manager()?.unregister(hotkey)?;
        Ok(())
    }

    pub fn unregister_all(&mut self, window_id: u8) -> Result<()> {
        if !self
            .index
            .keys()
            .any(|(owner_id, _)| *owner_id == window_id)
        {
            return Ok(());
        }
        self.global_manager()?;
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
        self.global_manager()?;
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

#[cfg(test)]
mod tests {
    use super::*;

    fn unavailable_manager() -> NivaShortcutManager {
        NivaShortcutManager {
            manager: None,
            manager_initialization_error: Some("test initialization failure".to_string()),
            shortcuts: HashMap::new(),
            index: HashMap::new(),
        }
    }

    fn assert_unavailable<T>(result: Result<T>) {
        let Err(err) = result else {
            panic!("expected unavailable shortcut manager error");
        };
        assert!(err.to_string().contains("test initialization failure"));
    }

    #[test]
    fn unavailable_manager_returns_clear_shortcut_api_errors() {
        let mut manager = unavailable_manager();
        assert_unavailable(manager.register(1, "invalid accelerator".to_string()));
        assert!(manager.register_with_options(1, &Vec::new()).is_ok());
        assert_unavailable(manager.unregister(1, 0));
        assert!(manager.unregister_all(1).is_ok());
        assert_unavailable(manager.list(1));
    }
}
