
use anyhow::Result;
use serde_json::{json, Value};
use std::sync::Arc;
use tao::{
    event_loop::ControlFlow,
    window::{CursorIcon, Fullscreen, Theme, UserAttentionType},
};

use crate::app::{
    api_manager::{ApiManager, ApiRequest},
    options::MenuOptions,
    window_manager::{
        options::{NivaPosition, NivaSize, NivaWindowOptions},
        window::NivaWindow,
    },
    NivaApp, NivaId, NivaWindowTarget,
};

pub fn register_api_instances(api_manager: &mut ApiManager) {
    api_manager.register_api(
        "monitor.current",
        |_, window, request| -> Result<Value> {
            Ok(json!(null))
            // let (options,) = request.args().optional::<(Option<NivaWindowOptions>,)>(1)?;
            // let new_window = app
            //     .window()?
            //     .open_window(&options.unwrap_or_default(), target)?;
            // Ok(new_window.id)
        },
    );

}
