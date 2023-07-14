use std::sync::Arc;

use anyhow::Result;
use tao::window::WindowBuilder;
use wry::webview::{WebView, WebViewBuilder};

use super::{event::NivaWindowTarget, NivaAppRef};

mod webview;
mod menu;
mod options;

pub struct NivaWindow {
    webview: WebView,
}

impl NivaWindow {
    pub fn new(
        app: NivaAppRef,
        target: &NivaWindowTarget,
        entry_url: String,
    ) -> Result<Arc<NivaWindow>> {
        let window_builder = WindowBuilder::new();
        let window = window_builder.build(target)?;
        let webview = WebViewBuilder::new(window)?.with_url(&entry_url)?.build()?;
        Ok(Arc::new(Self { webview }))
    }
}

pub struct NivaWindowManager {
}

