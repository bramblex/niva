use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use anyhow::Result;
use tao::{event::Event, event_loop::ControlFlow, window::WindowBuilder};
use wry::webview::{WebContext, WebView, WebViewBuilder};

use crate::utils::arc_mut::ArcMut;

use self::{options::NivaWindowOptions, window::NivaWindowRef};

use super::{
    event::{NivaEvent, NivaEventLoop, NivaWindowTarget},
    launch_info::NivaLaunchInfo,
    NivaAppRef,
};

pub mod options;
mod webview;
mod window;

pub struct NivaWindowIpcEvent {
    window_id: u8,
    event: String,
    data: serde_json::Value,
}

pub struct NivaWindowManager {
    app: Option<NivaAppRef>,
    windows: HashMap<u8, NivaWindowRef>,
    context: WebContext,
}

impl NivaWindowManager {
    pub fn new(launch_info: &NivaLaunchInfo) -> Result<Arc<Mutex<NivaWindowManager>>> {
        Ok(Arc::new(Mutex::new(Self {
            app: None,
            windows: HashMap::new(),
            context: WebContext::new(Some(launch_info.cache_dir.clone())),
        })))
    }

    pub async fn init(&mut self, app: &NivaAppRef) -> Result<()> {
        self.app = Some(app.clone());
        // self.open();
        Ok(())
    }

    pub fn start(&mut self, event_loop: &NivaEventLoop) -> Result<()> {
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

    pub fn open(&mut self, options: &NivaWindowOptions, target: &NivaWindowTarget) {}

    pub fn close(&mut self, id: u8) {}

    pub fn get(&self, id: u8) -> Result<&NivaWindowRef> {
        todo!()
    }

    pub fn get_all(&self) {}
}
