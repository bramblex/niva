use std::{
    collections::HashMap,
    hash::Hash,
    sync::{Arc, Mutex},
};

use anyhow::{anyhow, Result};
use tao::{event::Event, event_loop::ControlFlow, window::WindowId};
use wry::webview::WebContext;

use crate::utils::id_container::IdContainer;

use self::{
    options::NivaWindowOptions,
    window::{NivaWindow, NivaWindowRef},
};

use super::{
    event::{NivaEvent, NivaEventLoop, NivaWindowTarget},
    launch_info::NivaLaunchInfo,
    NivaAppRef,
};

pub mod options;
mod webview;
mod window;

pub struct NivaWindowIpcEvent {
    window_id: u32,
    event: String,
    data: serde_json::Value,
}

pub struct NivaWindowManager {
    app: Option<NivaAppRef>,
    windows: IdContainer<u32, NivaWindowRef>,
    id_map: HashMap<WindowId, u32>,
    context: WebContext,
}

impl NivaWindowManager {
    pub fn new(launch_info: &NivaLaunchInfo) -> Result<Arc<Mutex<NivaWindowManager>>> {
        Ok(Arc::new(Mutex::new(Self {
            app: None,
            context: WebContext::new(Some(launch_info.cache_dir.clone())),
            windows: IdContainer::new(0, 1),
            id_map: HashMap::new(),
        })))
    }

    pub async fn init(&mut self, app: &NivaAppRef) -> Result<()> {
        self.app = Some(app.clone());
        Ok(())
    }

    pub fn start(&mut self, event_loop: &NivaEventLoop) -> Result<()> {
        let app = self.app.clone().ok_or(anyhow!(""))?;
        smol::block_on(async {
            self.open(&app.launch_info.options.window, &event_loop)
                .await
        })?;
        Ok(())
    }

    pub fn run(
        &mut self,
        event: &Event<NivaEvent>,
        target: &NivaWindowTarget,
        control_flow: &mut ControlFlow,
    ) -> Result<()> {
        Ok(())
    }

    pub async fn open(
        &mut self,
        options: &NivaWindowOptions,
        target: &NivaWindowTarget,
    ) -> Result<NivaWindowRef> {
        let app = self.app.clone().ok_or(anyhow!(""))?;
        let id = self.windows.next_id()?;
        let window = NivaWindow::new(&app, self, target, id, options).await?;

        self.windows.insert_with_id(id, window.clone());
        self.id_map.insert(window.window_id , window.id);

        Ok(window)
    }

    pub fn get(&self, id: &u32) -> Result<&NivaWindowRef> {
        todo!()
    }

    pub fn get_by_window_id(&self, window_id: &WindowId) -> Result<&NivaWindowRef> {
        let id = self.id_map.get(&window_id).ok_or(anyhow!(""))?;
        self.get(id)
    }

    pub fn close(&mut self, id: u32) -> Result<()> {
        Ok(())
    }

    pub fn close_by_window_id(&mut self,  window_id: &WindowId) -> Result<()> {
        let window = self.get_by_window_id(window_id)?;
        self.close(window.id);
        Ok(())
    }

    pub fn get_all(&self) {}
}
