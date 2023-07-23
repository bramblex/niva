use std::sync::Arc;

use anyhow::Result;
use tao::window::WindowBuilder;
use wry::webview::{WebView, WebViewBuilder};

use crate::app::{event::NivaWindowTarget, NivaAppRef};

use super::options::NivaWindowOptions;

pub struct NivaWindow {
    id: u8,
    webview: WebView,
}

pub type NivaWindowRef = Arc<NivaWindow>;

impl NivaWindow {
    pub fn new(
        app: NivaAppRef,
        target: &NivaWindowTarget,
        id: u8,
        options: NivaWindowOptions,
    ) -> Result<NivaWindowRef> {
        let window_builder = WindowBuilder::new();
        let window = window_builder.build(target)?;
        let webview = WebViewBuilder::new(window)?
            .with_url(&options.entry)?
            .build()?;
        Ok(Arc::new(Self { id, webview }))
    }
}
