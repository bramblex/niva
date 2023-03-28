use anyhow::Result;
use base64::Engine;
use serde::Deserialize;

use crate::{
    api_manager::{ApiManager, ApiRequest},
    environment::EnvironmentRef,
};

pub fn register_apis(api_manager: &mut ApiManager) {
    api_manager.register_async_api("resource.exists", exists);
    api_manager.register_async_api("resource.read", read);
    api_manager.register_async_api("resource.extract", extract);
}

fn exists(env: EnvironmentRef, request: ApiRequest) -> Result<bool> {
    let path = request.args().get_single::<String>()?;
    Ok(env.resource.exists(path))
}

#[derive(Deserialize)]
enum EncodeType {
    #[serde(rename = "utf8")]
    UTF8,
    #[serde(rename = "base64")]
    BASE64,
}

fn read(env: EnvironmentRef, request: ApiRequest) -> Result<String> {
    let (path, encode) = request
        .args()
        .get_with_optional::<(String, Option<EncodeType>)>(2)?;

    let encode = encode.unwrap_or(EncodeType::UTF8);
    let content = env.resource.read(path)?;
    let content = match encode {
        EncodeType::UTF8 => String::from_utf8(content)?,
        EncodeType::BASE64 => base64::engine::general_purpose::STANDARD.encode(content),
    };

    Ok(content)
}

fn extract(env: EnvironmentRef, request: ApiRequest) -> Result<()> {
    let (from, to) = request
        .args()
        .get::<(String, String)>()?;
    let content = env.resource.read(from)?;
    std::fs::write(to, content)?;
    Ok(())
}