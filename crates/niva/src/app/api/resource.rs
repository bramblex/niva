use anyhow::Result;
use niva_macros::niva_api;
use serde::Deserialize;

use crate::app::api_manager::ApiManager;

pub fn register_apis(api_manager: &mut ApiManager) {
    api_manager.register_async_api("resource.exists", exists);
    api_manager.register_async_api("resource.read", read);
    api_manager.register_async_api("resource.extract", extract);
}

#[niva_api]
fn exists(path: String) -> Result<bool> {
    Ok(app.resource().exists(&path))
}

#[derive(Deserialize)]
enum EncodeType {
    #[serde(rename = "utf8")]
    UTF8,
    #[serde(rename = "base64")]
    BASE64,
}

#[niva_api]
fn read(path: String, encode: Option<EncodeType>) -> Result<String> {
    let encode = encode.unwrap_or(EncodeType::UTF8);
    let content = app.resource().load(&path)?;
    let content = match encode {
        EncodeType::UTF8 => String::from_utf8(content)?,
        EncodeType::BASE64 => base64::encode_config(content, base64::STANDARD),
    };

    Ok(content)
}

#[niva_api]
fn extract(from: String, to: String) -> Result<()> {
    let content = app.resource().load(&from)?;
    std::fs::write(to, content)?;
    Ok(())
}
