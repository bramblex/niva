pub(crate) mod builder;
pub mod options;
pub mod permissions;
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
    token_map: HashMap<String, u8>,
    id_map: HashMap<WindowId, u8>,
}

impl WindowManager {
    pub fn new(launch_info: &NivaLaunchInfo) -> ArcMut<Self> {
        arc_mut(Self {
            app: None,
            id_counter: IdCounter::new(),
            web_context: WebContext::new(Some(launch_info.data_dir.clone())),
            windows: HashMap::new(),
            token_map: HashMap::new(),
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
        self.token_map
            .insert(niva_window.token.clone(), niva_window.id);
        self.windows.insert(niva_window.id, niva_window.clone());

        Ok(niva_window)
    }

    pub fn get_window(&self, id: u8) -> Result<Arc<NivaWindow>> {
        self.windows
            .get(&id)
            .cloned()
            .ok_or(anyhow!("Window {id} not found"))
    }

    pub fn get_window_by_token(&self, token: &str) -> Result<Arc<NivaWindow>> {
        let id = self
            .token_map
            .get(token)
            .ok_or(anyhow!("Unknown window token"))?;
        self.get_window(*id)
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
        self.token_map.remove(&niva_window.token);
        self.id_map
            .remove(&niva_window.window_id)
            .ok_or(anyhow!("Window {id} not found"))?;
        Ok(niva_window)
    }

    pub fn list_windows(&self) -> Vec<&Arc<NivaWindow>> {
        self.windows.values().collect()
    }

    /// Final application shutdown must visit every owner even when one native
    /// cleanup fails. Snapshot Arcs before taking any other manager lock.
    pub fn close_all_and_cleanup(app: &Arc<NivaApp>) -> Result<()> {
        let windows: Vec<_> = app.window()?.list_windows().into_iter().cloned().collect();
        let mut failures = Vec::new();
        for window in windows {
            if let Err(error) = Self::close_window_and_cleanup(app, window.id, Some(window.clone()))
            {
                failures.push(format!("window {}: {error:#}", window.id));
            }
        }
        if failures.is_empty() {
            Ok(())
        } else {
            Err(anyhow!(failures.join("; ")))
        }
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
        run_cleanup_steps(
            window.id,
            || {
                let mut shortcuts = app.shortcut()?;
                shortcuts.unregister_all(window.id)
            },
            || {
                let mut tray = app.tray()?;
                tray.destroy_all(window.id)
            },
            || app.api().cancel_window(window.id),
        )
    }

    /// Remove and clean a window even if one of its cleanup steps fails.
    /// `fallback` lets a close event finish cleanup from the Arc it already
    /// resolved, even when removal from the manager maps reports an error.
    pub fn close_window_and_cleanup(
        app: &Arc<NivaApp>,
        id: u8,
        fallback: Option<Arc<NivaWindow>>,
    ) -> Result<()> {
        let target = match fallback.filter(|window| window.id == id) {
            Some(window) => Ok(window),
            None => app.window().and_then(|manager| manager.get_window(id)),
        };
        let removal = app
            .window()
            .and_then(|mut manager| manager.close_window(id).map(|_| ()));

        let mut failures = Vec::new();
        if let Err(error) = removal {
            failures.push(format!("remove window: {error:#}"));
        }
        match target {
            Ok(window) => {
                if let Err(error) = Self::cleanup_window(app, &window) {
                    failures.push(format!("release window resources: {error:#}"));
                }
            }
            Err(error) => failures.push(format!("resolve window for cleanup: {error:#}")),
        }
        finish_window_cleanup(id, failures)
    }
}

fn run_cleanup_steps(
    window_id: u8,
    unregister_shortcuts: impl FnOnce() -> Result<()>,
    destroy_trays: impl FnOnce() -> Result<()>,
    cancel_api_calls: impl FnOnce(),
) -> Result<()> {
    let mut failures = Vec::new();
    if let Err(error) = unregister_shortcuts() {
        failures.push(format!("shortcuts: {error:#}"));
    }
    if let Err(error) = destroy_trays() {
        failures.push(format!("tray: {error:#}"));
    }
    cancel_api_calls();
    finish_window_cleanup(window_id, failures)
}

fn finish_window_cleanup(window_id: u8, failures: Vec<String>) -> Result<()> {
    if failures.is_empty() {
        return Ok(());
    }
    Err(anyhow!(
        "window {window_id} cleanup failed: {}",
        failures.join("; ")
    ))
}

#[cfg(test)]
mod cleanup_tests {
    use super::*;
    use std::sync::Mutex;

    #[test]
    fn cleanup_runs_every_owner_step_and_keeps_all_errors() {
        let completed = Mutex::new(Vec::new());
        let result = run_cleanup_steps(
            7,
            || {
                completed.lock().unwrap().push("shortcuts");
                Err(anyhow!("unregister failed"))
            },
            || {
                completed.lock().unwrap().push("tray");
                Err(anyhow!("destroy failed"))
            },
            || completed.lock().unwrap().push("api calls"),
        );
        let errors = result.unwrap_err().to_string();
        assert!(errors.contains("shortcuts: unregister failed"));
        assert!(errors.contains("tray: destroy failed"));
        assert_eq!(
            *completed.lock().unwrap(),
            ["shortcuts", "tray", "api calls"]
        );
    }
}
