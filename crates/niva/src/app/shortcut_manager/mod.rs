use super::utils::arc;
use std::{str::FromStr, sync::Arc};
use tao::accelerator::Accelerator;

pub struct NivaShortcutManager {}

impl NivaShortcutManager {
    pub fn new() -> Arc<Self> {
        // let a = Accelerator::from_str("command+shift+o").;
        // let id = a.id().0;
        arc(Self {})
    }
}
