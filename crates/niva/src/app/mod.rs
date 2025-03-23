mod api;
pub mod resource;
mod shortcut;
mod tray;
mod window;

mod common;
mod arguments;
mod event;
mod launch_info;
mod options;

use anyhow::Result;
use launch_info::NivaLaunchInfo;
use resource::NivaResourceManager;
use std::sync::Arc;

use crate::{utils::arc_mut::ArcMut, with_lock};

use self::{
    api::NivaApiManager,
    event::{NivaEventLoop, NivaEventLoopProxy, NivaEvent},
    shortcut::NivaShortcutManager,
    tray::NivaTrayManager,
    window::NivaWindowManager,
};

pub struct NivaApp {
    pub launch_info: NivaLaunchInfo,
    pub event_loop: NivaEventLoopProxy,

    pub resource_manager: ArcMut<NivaResourceManager>,
    pub window_manager: ArcMut<NivaWindowManager>,
    pub tray_manager: ArcMut<NivaTrayManager>,
    pub shortcut_manager: ArcMut<NivaShortcutManager>,
    pub api_manager: ArcMut<NivaApiManager>,
}

pub type NivaAppRef = Arc<NivaApp>;

macro_rules! map_manager {
    ($app:expr, {$($manager_name:ident),+}, $name:ident => $body:block) => {
        $(with_lock!($name = &$app.$manager_name, $body);)+
    };
}

impl NivaApp {
    pub fn create_event_loop() -> NivaEventLoop {
        NivaEventLoop::with_user_event()
    }

    pub fn new(event_loop: &mut NivaEventLoop) -> Result<Arc<NivaApp>> {
        let launch_info = NivaLaunchInfo::new()?;

        let event_loop = event_loop.create_proxy();

        let resource_manager = NivaResourceManager::new(&launch_info)?;
        let window_manager = NivaWindowManager::new(&launch_info)?;
        let tray_manager = NivaTrayManager::new(&launch_info)?;
        let shortcut_manager = NivaShortcutManager::new(&launch_info)?;
        let api_manager = NivaApiManager::new(&launch_info)?;

        Ok(Arc::new(Self {
            launch_info,
            event_loop,

            resource_manager,
            window_manager,
            tray_manager,
            shortcut_manager,
            api_manager,
        }))
    }

    async fn init(self: &Arc<Self>) -> Result<()> {
        map_manager!(self, {
            resource_manager,
            window_manager,
            tray_manager,
            shortcut_manager,
            api_manager
        }, manager => { manager.init(self).await?; });
        Ok(())
    }

    pub fn run(self: &Arc<Self>, event_loop: NivaEventLoop) -> Result<()> {
        smol::block_on(async { self.init().await })?;
        map_manager!(self, {
            resource_manager,
            window_manager,
            tray_manager,
            shortcut_manager,
            api_manager
        }, manager => { manager.start(&event_loop)?; });

        let app = self.clone();
        event_loop.run(move |event, target, control_flow| {
            let mut err_list: Vec<anyhow::Error> = Vec::new();
            map_manager!(&app, {
                    resource_manager,
                    window_manager,
                    tray_manager,
                    shortcut_manager,
                    api_manager
                }, manager => {
                    if let Err(err) = manager.run(&event, target, control_flow) {
                        err_list.push(err);
                    }
            });
            for err in err_list {
                println!("{}", err);
            }
        });
    }

    pub fn emit(self: &Arc<NivaApp>, event: NivaEvent) -> Result<()>{
        Ok(self.event_loop.send_event(event)?)
    }
}
