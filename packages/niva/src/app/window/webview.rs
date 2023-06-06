use std::ops::Deref;

use crate::{app::NivaAppRef, set_property, set_property_some, with_lock};
use anyhow::{anyhow, Result};
use tao::window::Window;
use wry::webview::{WebView, WebViewBuilder};

use super::{options::NivaWindowOptions, NivaWindowManager};

pub struct NivaWebview(pub WebView);

impl NivaWebview {
    fn escape_code_string(code: &str) -> Result<String> {
        Ok(serde_json::to_string(&serde_json::json!(code))?)
    }

    fn null() -> String {
        "null".to_string()
    }

    async fn generate_init_script(app: &NivaAppRef, options: &NivaWindowOptions) -> Result<String> {
        let api_code_str = include_str!("../../../assets/api.js");
        let env_code_str = serde_json::to_string(&options.env)?;
        let preload_code_str = if let Some(preload) = &options.preload {
            with_lock!(
                m = app.resource_manager,
                {
                    let content = m.load_by_resource_url(preload).await?;
                    let content = String::from_utf8(content)?;
                    Self::escape_code_string(&content)?
                },
                Self::null()
            )
        } else {
            Self::null()
        };

        let script = format!(
            "(function(){{ 
                window.__ENV__ = {}; 
                {}; 
                eval({}); 
            }})();",
            env_code_str, api_code_str, preload_code_str
        );

        Ok(script)
    }

    pub async fn new(
        app: &NivaAppRef,
        manager: &mut NivaWindowManager,
        window: Window,
        options: &NivaWindowOptions,
    ) -> Result<NivaWebview> {
        let mut builder = WebViewBuilder::new(window)?;

        set_property!(builder, with_web_context, &mut manager.context);
        set_property!(builder, with_accept_first_mouse, true);
        set_property!(builder, with_clipboard, true);
        set_property_some!(builder, with_devtools, options.devtools);

        if options.transparent.unwrap_or(false) {
            set_property!(builder, with_background_color, (255, 255, 255, 0));
            set_property!(builder, with_transparent, true);
        }

        let entry = with_lock!(
            rm = app.resource_manager,
            { rm.transform_resource_url(&options.entry) },
            Err(anyhow!("Unexpected entry url."))
        )?;

        let host_str = entry
            .host_str()
            .ok_or(anyhow!("Unexpected entry url."))?
            .to_string();

        set_property!(builder, with_navigation_handler, move |url| {
            let url = url::Url::parse(&url);
            if let Ok(url) = url {
                if let Some(_host_str) = url.host_str() {
                    return _host_str == host_str;
                }
            }
            false
        });

        // let drop_app = app.clone();
        // set_property!(builder, with_file_drop_handler, move |window, event| {
        //     let window_result = drop_app
        //         .window()
        //         .and_then(|w| w.get_window_inner(window.id()));
        //     match window_result {
        //         Ok(window) => match event {
        //             FileDropEvent::Hovered { paths, position } => {
        //                 let position = position.to_logical::<f64>(window.scale_factor());
        //                 log_if_err!(window.send_ipc_event(
        //                     "fileDrop.hovered",
        //                     json!({
        //                         "paths": paths,
        //                         "position": position,
        //                     }),
        //                 ));
        //             }
        //             FileDropEvent::Dropped { paths, position } => {
        //                 let position = position.to_logical::<f64>(window.scale_factor());
        //                 log_if_err!(window.send_ipc_event(
        //                     "fileDrop.dropped",
        //                     json!({
        //                         "paths": paths,
        //                         "position": position,
        //                     }),
        //                 ));
        //             }
        //             FileDropEvent::Cancelled => {
        //                 log_if_err!(window.send_ipc_event("fileDrop.cancelled", json!(null)));
        //             }
        //             _ => (),
        //         },
        //         Err(err) => {
        //             log_err!(err);
        //         }
        //     }
        //     false
        // });

        // let ipc_app = app.clone();
        // set_property!(builder, with_ipc_handler, move |window, request_str| {
        // if let Err(err) = ipc_app.api().and_then(|w| w.call(window, request_str)) {
        //     let window = ipc_app
        //         .window()
        //         .and_then(|w| w.get_window_inner(window.id()));
        //     if let Ok(window) = window {
        //         log_if_err!(window.send_ipc_callback(json!({
        //             "ipc.error": err.to_string(),
        //         })));
        //     }
        // };
        // });

        // 初始化 webview 代码
        set_property!(
            builder,
            with_initialization_script,
            &Self::generate_init_script(app, options).await?
        );
        let entry = entry.to_string();
        let webview = builder.with_url(&entry)?.build()?;

        Ok(Self(webview))
    }
}

impl Deref for NivaWebview {
    type Target = WebView;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
