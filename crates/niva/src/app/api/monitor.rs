
use anyhow::Result;
use serde_json::{json, Value};
use std::sync::Arc;
use tao::{
    event_loop::ControlFlow,
    window::{CursorIcon, Fullscreen, Theme, UserAttentionType}, monitor::MonitorHandle,
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
        "monitor.list",
        |_, window, request| -> Result<Vec<Value>> {
            Ok(vec![])
        },
    );

    api_manager.register_api(
        "monitor.current",
        |_, window, request| -> Result<Vec<Value>> {
            Ok(vec![])
        },
    );

    api_manager.register_api(
        "monitor.primary",
        |_, window, request| -> Result<Vec<Value>> {
            Ok(vec![])
        },
    );
}