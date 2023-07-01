
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

mod arguments;
mod launch_info;
mod options;
mod resource;
mod api;

use anyhow::Result;
use launch_info::NivaLaunchInfo;
use resource::NivaResourceManager;
use std::sync::Arc;

pub struct NivaApp {
    pub launch_info: NivaLaunchInfo,
    pub resource_manager: NivaResourceManager,
}

impl NivaApp {
    pub fn new() -> Result<Arc<NivaApp>> {
        let launch_info = NivaLaunchInfo::new()?;
        let resource_manager =
            NivaResourceManager::new(&launch_info.workspace, &launch_info.options.resource)?;

        Ok(Arc::new(Self {
            launch_info,
            resource_manager,
        }))
    }

    pub fn run(&self) -> Result<()> {
        Ok(())
    }
}