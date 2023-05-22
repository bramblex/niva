mod builder;
pub mod options;
pub mod url;
pub mod window;

use anyhow::{anyhow, Result};

use std::{collections::HashMap, sync::Arc};
use tao::window::WindowId;
use wry::webview::WebContext;

use crate::unsafe_impl_sync_send;

use self::{options::NivaWindowOptions, window::NivaWindow};
use super::{
    utils::{arc_mut, ArcMut, IdCounter},
    NivaApp, NivaLaunchInfo, NivaWindowTarget,
};

unsafe_impl_sync_send!(WindowManager);
pub struct WindowManager {
    app: Option<Arc<NivaApp>>,
    id_counter: IdCounter,
    web_context: WebContext,
    windows: HashMap<u8, Arc<NivaWindow>>,
    id_map: HashMap<WindowId, u8>,
}

impl WindowManager {
    pub fn new(launch_info: &NivaLaunchInfo) -> ArcMut<Self> {
        arc_mut(Self {
            app: None,
            id_counter: IdCounter::new(),
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
        let id = self.id_counter.next(&self.windows)?;
        let app = self.app.clone().ok_or(anyhow!("App not found"))?;

        let niva_window = NivaWindow::new(app, self, id, options, target)?;

        self.id_map.insert(niva_window.window_id, niva_window.id);
        self.windows.insert(niva_window.id, niva_window.clone());

        Ok(niva_window)
    }

    pub fn get_window(&self, id: u8) -> Result<Arc<NivaWindow>> {
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

    pub fn close_window(&mut self, id: u8) -> Result<()> {
        let niva_window = self
            .windows
            .remove(&id)
            .ok_or(anyhow!("Window {id} not found"))?;
        self.id_map
            .remove(&niva_window.window_id)
            .ok_or(anyhow!("Window {id} not found"))?;

        let app = self.app.clone().ok_or(anyhow!("App not found"))?;
        app.shortcut()?.unregister_all(id)?;
        app.tray()?.destroy_all(id)?;
        Ok(())
    }

    pub fn list_windows<'a>(&'a self) -> Vec<&'a Arc<NivaWindow>> {
        self.windows.values().collect()
    }

    pub fn close_window_inner(&mut self, window_id: WindowId) -> Result<()> {
        let id = self
            .id_map
            .get(&window_id)
            .ok_or(anyhow!("Window not found"))?;
        self.close_window(*id)
    }
}
