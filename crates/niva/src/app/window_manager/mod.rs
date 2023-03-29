mod builder;
pub mod options;
pub mod window;

use anyhow::{anyhow, Result};

use std::{collections::HashMap, sync::Arc};
use tao::window::WindowId;
use wry::webview::WebContext;

use crate::unsafe_impl_sync_send;

use self::{options::NivaWindowOptions, window::NivaWindow};
use super::{
    utils::{arc_mut, ArcMut, Counter},
    NivaApp, NivaId, NivaLaunchInfo, NivaWindowTarget,
};

unsafe_impl_sync_send!(WindowManager);
pub struct WindowManager {
    app: Option<Arc<NivaApp>>,

    id_counter: Counter<u32>,
    web_context: WebContext,
    windows: HashMap<NivaId, Arc<NivaWindow>>,
    id_map: HashMap<WindowId, NivaId>,
}

impl WindowManager {
    pub fn new(launch_info: &NivaLaunchInfo) -> ArcMut<Self> {
        arc_mut(Self {
            app: None,
            id_counter: Counter::<u32>::new(0),
            web_context: WebContext::new(Some(launch_info.data_dir.clone())),
            windows: HashMap::new(),
            id_map: HashMap::new(),
        })
    }

    pub fn bind_app(&mut self, app: Arc<NivaApp>) {
        self.app = Some(app);
    }

    pub fn open_window(
        &mut self,
        options: &NivaWindowOptions,
        target: &NivaWindowTarget,
    ) -> Result<Arc<NivaWindow>> {
        let id = self.id_counter.next();
        let app = self.app.clone().ok_or(anyhow!("App not found"))?;

        let niva_window = NivaWindow::new(app, id, options, &mut self.web_context, target)?;

        self.id_map.insert(niva_window.window_id, niva_window.id);
        self.windows.insert(niva_window.id, niva_window.clone());

        Ok(niva_window)
    }

    pub fn get_window(&self, id: NivaId) -> Result<Arc<NivaWindow>> {
        self.windows
            .get(&id)
            .cloned()
            .ok_or(anyhow!("Window {id} not found"))
    }

    pub fn get_window_inner(&self, window_id: WindowId) -> Result<Arc<NivaWindow>> {
        let id = self
            .id_map
            .get(&window_id)
            .cloned()
            .ok_or(anyhow!("Window not found"))?;
        self.get_window(id)
    }

    pub fn close_window(&mut self, id: NivaId) -> Result<()> {
        let niva_window = self
            .windows
            .remove(&id)
            .ok_or(anyhow!("Window {id} not found"))?;
        self.id_map
            .remove(&niva_window.window_id)
            .ok_or(anyhow!("Window {id} not found"))?;

        Ok(())
    }

    // pub fn close_window_inner(&mut self, window_id: WindowId) -> Result<()> {
    //     let id = self
    //         .id_map
    //         .remove(&window_id)
    //         .ok_or(anyhow!("Window not found"))?;
    //     self.close_window(id)
    // }

    // pub fn broadcast<E: Into<String>, P: Serialize>(self: Arc<Self>, event: E, payload: P) {
    //     let event: String = event.into();
    //     let payload: Value = serde_json::to_value(payload).unwrap();
    //     for (_, window) in self.windows.iter() {
    //         let window = window.clone();
    //         log_if_err!(window.send_ipc_event(event.clone(), payload.clone()));
    //     }
    // }
}
