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
mod resource;
mod shortcut;
mod tray;
pub mod window;

mod arguments;
pub mod event;
mod launch_info;
mod manager;
mod options;

use anyhow::Result;
use launch_info::NivaLaunchInfo;
use resource::NivaResourceManager;
use std::{collections::HashMap, hash::Hash, sync::Arc};

use crate::utils::arc_mut::ArcMut;

use self::{
    event::NivaEventLoop,
    manager::{NivaManager, NivaManagers},
    window::NivaWindow,
};

pub struct NivaApp {
    pub launch_info: NivaLaunchInfo,
    managers: NivaManagers, // pub resource_manager: ArcMut<NivaResourceManager>,
}

pub type NivaAppRef = Arc<NivaApp>;

impl NivaApp {
    pub fn new() -> Result<Arc<NivaApp>> {
        let launch_info = NivaLaunchInfo::new()?;
        let mut managers = NivaManagers::new();

        managers.insert("resource", NivaResourceManager::new(&launch_info)?);
        // let resource_manager = NivaResourceManager::new(&launch_info)?;

        Ok(Arc::new(Self {
            launch_info,
            managers, // resource_manager,
        }))
    }

    async fn init(self: &Arc<Self>) -> Result<()> {
        // self.resource_manager.lock().await.init(self).await?;

        Ok(())
    }

    pub fn run(self: &Arc<Self>, event_loop: NivaEventLoop) -> Result<()> {
        smol::block_on(async { self.init().await })?;
        event_loop.run(|event, target, control_flow| {});
    }
}