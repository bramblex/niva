use anyhow::{anyhow, Result};
use std::{borrow::Cow, ops::Deref, sync::Arc};

use serde::Serialize;
use serde_json::json;
use tao::{
    event_loop::{self, ControlFlow},
    window::{Window, WindowBuilder, WindowId},
};
use wry::{
    http::{Request, Response},
    webview::{WebContext, WebView, WebViewBuilder},
};

use crate::{
    niva_app::{
        resource_manager,
        utils::{arc, png_to_icon},
        NivaApp, NivaEvent, NivaEventLoopProxy, NivaId, NivaWindowTarget,
    },
    unsafe_impl_sync_send,
};

use super::options::NivaWindowOptions;

static INITIALIZE_SCRIPT: &str = include_str!("../../../assets/initialize_script.js");

pub struct NivaWindow {
    pub id: NivaId,
    pub window_id: WindowId,
    pub webview: WebView,

    event_loop_proxy: NivaEventLoopProxy,
}

unsafe_impl_sync_send!(NivaWindow);
impl Deref for NivaWindow {
    type Target = Window;
    fn deref(&self) -> &Self::Target {
        self.webview.window()
    }
}

impl NivaWindow {
    pub fn new(
        app: Arc<NivaApp>,
        id: NivaId,
        options: &NivaWindowOptions,
        web_context: &mut WebContext,
        target: &NivaWindowTarget,
    ) -> Result<Arc<NivaWindow>> {
        let id_name = app.launch_info.id_name.clone();

        let window = WindowBuilder::new()
            .with_window_icon(Some(
                png_to_icon(&app.resource_manager.read("icon.png".to_string()).unwrap()).unwrap(),
            ))
            .build(target)?;

        let event_loop_proxy = app.event_loop_proxy.clone();
        let resource_manager = app.resource_manager.clone();

        let webview = WebViewBuilder::new(window)
            .unwrap()
            .with_web_context(web_context)
            .with_initialization_script(INITIALIZE_SCRIPT)
            .with_custom_protocol("niva".to_string(), move |request| {
                let mut path = request.uri().path().to_string();
                if path.ends_with("/") {
                    path += "index.html";
                }
                let path = path.strip_prefix("/").unwrap_or("index.html");
                let result = resource_manager.read(path.to_string());

                match result {
                    Err(err) => Ok(Response::builder()
                        .status(404)
                        .body(Cow::Owned(err.to_string().into_bytes()))?),
                    Ok(content) => {
                        let mime_type = mime_guess::from_path(path)
                            .first()
                            .unwrap_or(mime_guess::mime::TEXT_PLAIN)
                            .to_string();

                        Ok(Response::builder()
                            .status(200)
                            .header("Content-Type", mime_type)
                            .body(Cow::Owned(content))?)
                    }
                }
            })
            .with_ipc_handler(move |window, request_str| {
                if let Err(err) = app.call_api(window, request_str) {
                    let window = app.get_window_inner(window.id());
                    if let Ok(window) = window {
                        window.send_ipc_callback(json!({
                            "ipc.error": err.to_string(),
                        }));
                    }
                };
            })
            .with_url(format!("niva://{id_name}/").as_str())?
            .build()?;

        Ok(arc(Self {
            id,
            window_id: webview.window().id(),
            webview,
            event_loop_proxy,
        }))
    }

    pub fn send_event<F: Fn(&NivaWindowTarget, &mut ControlFlow) -> Result<()> + Send + 'static>(
        self: &Arc<Self>,
        f: F,
    ) -> Result<()> {
        self.event_loop_proxy
            .send_event(NivaEvent::new(f))
            .map_err(|_| anyhow!("Failed to send event"))
    }

    pub fn send_ipc_event<E: Into<String>, P: Serialize>(
        self: &Arc<Self>,
        event: E,
        payload: P,
    ) -> Result<()> {
        let event: String = event.into();
        let payload = serde_json::to_string(&payload)?;
        let _self = self.clone();
        self.send_event(move |_, _| {
            _self
                .webview
                .evaluate_script(&format!("Niva.__emit__(\"{event}\", {payload})"))?;
            Ok(())
        })
    }

    pub fn send_ipc_callback<D: Serialize>(self: &Arc<Self>, data: D) -> Result<()> {
        self.send_ipc_event("ipc.callback", json!(data))
    }
}
