use anyhow::Result;
use serde::Deserialize;
use std::sync::Arc;

use crate::app::{
    api_manager::{ApiManager, ApiRequest},
    window_manager::window::NivaWindow,
    NivaApp,
};

pub fn register_apis(api_manager: &mut ApiManager) {
    api_manager.register_async_api("resource.exists", exists);
    api_manager.register_async_api("resource.read", read);
    api_manager.register_async_api("resource.extract", extract);
}

fn exists(app: Arc<NivaApp>, _: Arc<NivaWindow>, request: ApiRequest) -> Result<bool> {
    let path = request.args().single::<String>()?;
    Ok(app._resource.exists(path))
}

#[derive(Deserialize)]
enum EncodeType {
    #[serde(rename = "utf8")]
    UTF8,
    #[serde(rename = "base64")]
    BASE64,
}

fn read(app: Arc<NivaApp>, _: Arc<NivaWindow>, request: ApiRequest) -> Result<String> {
    let (path, encode) = request.args().optional::<(String, Option<EncodeType>)>(2)?;

    let encode = encode.unwrap_or(EncodeType::UTF8);
    let content = app._resource.load(path)?;
    let content = match encode {
        EncodeType::UTF8 => String::from_utf8(content)?,
        EncodeType::BASE64 => base64::encode_config(content, base64::STANDARD),
    };

    Ok(content)
}

fn extract(env: Arc<NivaApp>, _: Arc<NivaWindow>, request: ApiRequest) -> Result<()> {
    let (from, to) = request.args().get::<(String, String)>()?;
    let content = env._resource.load(from)?;
    std::fs::write(to, content)?;
    Ok(())
}
