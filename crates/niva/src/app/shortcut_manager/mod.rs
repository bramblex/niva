use super::{
    options::ShortcutsOptions,
    utils::{arc, Counter},
    NivaEventLoop, NivaId,
};
use anyhow::{anyhow, Result};
use std::{collections::HashMap, result, str::FromStr, sync::Arc};
use tao::{
    accelerator::{Accelerator, AcceleratorId},
    global_shortcut::ShortcutManager,
};

pub struct NivaShortcutManager {}

impl NivaShortcutManager {
    pub fn build(options: &ShortcutsOptions, event_loop: &NivaEventLoop) -> ShortcutManager {
        let mut manager = ShortcutManager::new(event_loop);
        for (shortcut, id) in options.0.clone() {
            let result = Self::register(&mut manager, shortcut, id);
            if let Err(err) = result {
                println!("{}", err.to_string());
            }
        }
        manager
    }

    fn register(manager: &mut ShortcutManager, shortcut: String, id: u16) -> Result<()> {
        let accelerator =
            Accelerator::from_str(&shortcut).map_err(|err| anyhow!("{}", err.to_string()))?;
        manager.register(accelerator.with_id(AcceleratorId(id)))?;
        Ok(())
    }
}
