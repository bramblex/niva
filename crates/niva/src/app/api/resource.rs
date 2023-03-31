use anyhow::Result;
use serde::Deserialize;
use std::sync::Arc;

use crate::{app::{
    api_manager::{ApiManager, ApiRequest},
    window_manager::window::NivaWindow,
    NivaApp,
}, args_match, opts_match};

pub fn register_apis(api_manager: &mut ApiManager) {
    api_manager.register_async_api("resource.exists", exists);
    api_manager.register_async_api("resource.read", read);
    api_manager.register_async_api("resource.extract", extract);
}

fn exists(app: Arc<NivaApp>, _: Arc<NivaWindow>, request: ApiRequest) -> Result<bool> {
    args_match!(request, path: String);
    Ok(app.resource().exists(&path))
}

#[derive(Deserialize)]
enum EncodeType {
    #[serde(rename = "utf8")]
    UTF8,
    #[serde(rename = "base64")]
    BASE64,
}

fn read(app: Arc<NivaApp>, _: Arc<NivaWindow>, request: ApiRequest) -> Result<String> {
    opts_match!(request, path: String, encode: Option<EncodeType>);

    let encode = encode.unwrap_or(EncodeType::UTF8);
    let content = app.resource().load(&path)?;
    let content = match encode {
        EncodeType::UTF8 => String::from_utf8(content)?,
        EncodeType::BASE64 => base64::encode_config(content, base64::STANDARD),
    };

    Ok(content)
}

fn extract(env: Arc<NivaApp>, _: Arc<NivaWindow>, request: ApiRequest) -> Result<()> {
    args_match!(request, from: String, to: String);

    let content = env.resource().load(&from)?;
    std::fs::write(to, content)?;
    Ok(())
}
