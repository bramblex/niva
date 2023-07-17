/*
NivaApp {
    NivaWindow {
        SystemTray
        GlobalShortcut
        Menu
        Webview
        Api
    }

    NivaAppEventLoop {
        NivaWindow.handle()
    }
}
 */

mod api;
pub mod resource;
mod shortcut;
mod tray;
pub mod window;

mod arguments;
pub mod event;
mod launch_info;
pub mod manager;
mod options;

use anyhow::Result;
use launch_info::NivaLaunchInfo;
use resource::NivaResourceManager;
use std::{
    any::TypeId,
    collections::HashMap,
    ops::DerefMut,
    sync::{Arc, MutexGuard},
};

use crate::{app::manager::NivaManager, lock};

use self::{event::NivaEventLoop, manager::NivaManagerRef};

pub struct NivaApp {
    pub launch_info: NivaLaunchInfo,
    managers: HashMap<TypeId, NivaManagerRef>,
}

pub type NivaAppRef = Arc<NivaApp>;

impl NivaApp {
    pub fn new() -> Result<Arc<NivaApp>> {
        let launch_info = NivaLaunchInfo::new()?;
        let mut managers = HashMap::new();

        managers.insert(
            TypeId::of::<NivaResourceManager>(),
            NivaResourceManager::new(&launch_info)?,
        );

        Ok(Arc::new(Self {
            launch_info,
            managers, // resource_manager,
        }))
    }

    pub fn get_manager<T: NivaManager + 'static>(&self) -> Option<NivaManagerRef> {
        let type_id = TypeId::of::<T>();
        self.managers.get(&type_id).cloned()
    }

    async fn init(self: &Arc<Self>) -> Result<()> {
        for (_, manager) in &self.managers {
            lock!(manager, { manager.init(self).await? });
        }
        Ok(())
    }

    pub fn run(self: &Arc<Self>, event_loop: NivaEventLoop) -> Result<()> {
        smol::block_on(async { self.init().await })?;
        for (_, manager) in &self.managers {
            lock!(manager, { manager.start(&event_loop)? });
        }
        let app = self.clone();
        event_loop.run(move |event, target, control_flow| {
            for (_, manager) in &app.managers {
                lock!(manager, {
                    manager.tick(&event, target, control_flow);
                });
            }
        });
    }
}
