pub(crate) mod builder;
pub mod options;
pub mod url;
pub mod window;

use anyhow::{Result, anyhow};

use std::{collections::HashMap, sync::Arc};
use tao::window::WindowId;
use wry::WebContext;

use crate::unsafe_impl_sync_send;

use self::{options::NivaWindowOptions, window::NivaWindow};
use super::{
    NivaApp, NivaLaunchInfo, NivaWindowTarget,
    utils::{ArcMut, IdCounter, arc_mut},
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

    /// Remove a window from the maps and hand it back. Per the crate-wide
    /// lock discipline this never touches other managers while the
    /// WindowManager guard is held — callers run shortcut/tray cleanup
    /// afterwards via [`Self::cleanup_window`].
    pub fn close_window(&mut self, id: u8) -> Result<Arc<NivaWindow>> {
        let niva_window = self
            .windows
            .remove(&id)
            .ok_or(anyhow!("Window {id} not found"))?;
        self.id_map
            .remove(&niva_window.window_id)
            .ok_or(anyhow!("Window {id} not found"))?;
        Ok(niva_window)
    }

    pub fn list_windows(&self) -> Vec<&Arc<NivaWindow>> {
        self.windows.values().collect()
    }

    pub fn close_window_inner(&mut self, window_id: WindowId) -> Result<Arc<NivaWindow>> {
        let id = self
            .id_map
            .get(&window_id)
            .cloned()
            .ok_or(anyhow!("Window not found"))?;
        self.close_window(id)
    }

    /// Owner cleanup after removal (shortcuts, trays). Takes the app, NOT a
    /// manager guard, so locks are always acquired leaf-first and released
    /// before the next one — no nesting, no inversion.
    pub fn cleanup_window(app: &Arc<NivaApp>, window: &Arc<NivaWindow>) -> Result<()> {
        app.shortcut()?.unregister_all(window.id)?;
        app.tray()?.destroy_all(window.id)?;
        // Cancel in-flight stateful API calls bound to the closed window.
        app.api().cancel_window(window.id);
        Ok(())
    }
}
