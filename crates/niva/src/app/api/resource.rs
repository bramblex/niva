use anyhow::Result;

use crate::app::NivaApp;
use crate::app::api_manager::ApiRequest;
use crate::app::api_manager::{ApiManager, CallContext};
use crate::app::window_manager::window::NivaWindow;
use std::sync::Arc;

pub fn register_apis(api_manager: &mut ApiManager) {
    api_manager.register_blocking_api("resource.exists", exists);
    api_manager.register_blocking_api("resource.extract", extract);
    api_manager.register_stream_api("resource.readStream", read_stream);
}

fn exists(app: Arc<NivaApp>, _window: Arc<NivaWindow>, request: ApiRequest) -> Result<bool> {
    let (path,) = request.args().get::<(String,)>()?;
    Ok(app.resource().exists(&path))
}

fn extract(app: Arc<NivaApp>, _window: Arc<NivaWindow>, request: ApiRequest) -> Result<()> {
    let (from, to) = request.args().get::<(String, String)>()?;
    let content = app.resource().load(&from)?;
    std::fs::write(to, content)?;
    Ok(())
}

/// Streaming embedded-resource read: binary chunks + terminal `{size}`.
async fn read_stream(ctx: CallContext, request: ApiRequest) -> Result<()> {
    use serde_json::json;

    let (path,): (String,) = request.args().get()?;
    let data = ctx.app.resource().load(&path)?;

    const CHUNK: usize = 65536;
    let mut offset = 0;
    while offset < data.len() {
        if ctx.is_cancelled() {
            return Ok(());
        }
        let end = (offset + CHUNK).min(data.len());
        ctx.chunk(&data[offset..end], end == data.len());
        offset = end;
    }
    if data.is_empty() {
        ctx.chunk(&[], true);
    }
    ctx.respond(Ok(json!({ "size": data.len() })));
    Ok(())
}
