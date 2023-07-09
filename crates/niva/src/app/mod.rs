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
mod window;

mod arguments;
mod launch_info;
mod options;

use anyhow::Result;
use launch_info::NivaLaunchInfo;
use resource::NivaResourceManager;
use smol::lock::Mutex;
use std::sync::Arc;

pub struct NivaApp {
    pub launch_info: NivaLaunchInfo,
    pub resource_manager: Arc<Mutex<NivaResourceManager>>,
}

impl NivaApp {
    pub async fn new() -> Result<Arc<NivaApp>> {
        let launch_info = NivaLaunchInfo::new()?;
        let resource_manager = Arc::new(Mutex::new(
            NivaResourceManager::new(&launch_info.workspace, &launch_info.options.resource).await?,
        ));

        Ok(Arc::new(Self {
            launch_info,
            resource_manager,
        }))
    }

    pub fn run(&self) -> Result<()> {
        Ok(())
    }
}
