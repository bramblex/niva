mod fs;
mod http;
mod os;
mod process;

pub mod window;
pub mod webview;
pub mod dialog;

use serde::{Deserialize, Serialize};

use crate::environment::EnvironmentRef;

#[derive(Debug, Deserialize)]
pub struct ApiRequest {
    pub namespace: String,
    pub method: String,
    pub data: serde_json::Value,
    pub callback_id: u32,
}

#[derive(Debug, Serialize)]
pub struct ApiResponse {
    pub code: i16,
    pub message: String,
    pub data: serde_json::Value,
    pub callback_id: u32,
}

impl From<ApiResponse> for serde_json::Value {
    fn from(val: ApiResponse) -> Self {
        serde_json::to_value(val).unwrap()
    }
}

unsafe impl Send for ApiResponse {}
unsafe impl Sync for ApiResponse {}

impl ApiResponse {
    pub fn ok<D>(callback_id: u32, data: D) -> ApiResponse
    where
        D: Into<serde_json::Value>,
    {
        ApiResponse {
            code: 0,
            message: String::new(),
            data: data.into(),
            callback_id,
        }
    }

    pub fn err<S>(callback_id: u32, message: S) -> ApiResponse
    where
        S: Into<String>,
    {
        ApiResponse {
            code: -1,
            message: message.into(),
            data: serde_json::Value::Null,
            callback_id,
        }
    }
}


pub fn call(env: EnvironmentRef, request: ApiRequest) -> ApiResponse {
    let response: ApiResponse = match request.namespace.as_str() {
        "fs" => fs::call(request),
        "http" => http::call(request),
        "os" => os::call(request),
        "process" => process::call(env, request),
        _ => ApiResponse::err(request.callback_id, "Namespace not found".to_string()),
    };

    response
}
