mod fs;
mod http;
mod os;
mod process;

use serde::{Deserialize, Serialize};
use serde_json;

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

unsafe impl Send for ApiResponse {}
unsafe impl Sync for ApiResponse {}

impl ApiResponse {
    pub fn ok(data: serde_json::Value) -> ApiResponse {
        return ApiResponse {
            code: 0,
            message: String::new(),
            data,
            callback_id: 0,
        };
    }

    pub fn err(message: String) -> ApiResponse {
        return ApiResponse {
            code: -1,
            message,
            data: serde_json::Value::Null,
            callback_id: 0,
        };
    }

    pub fn to_string(&self) -> String {
        return serde_json::to_string::<ApiResponse>(self).unwrap();
    }
}

pub fn call(request_str: String) -> String {
    let request_result = serde_json::from_str::<ApiRequest>(request_str.as_str());
    if request_result.is_err() {
        return ApiResponse::err("Invalid request".to_string()).to_string();
    }

    let request = request_result.unwrap();
    let callback_id = request.callback_id;
    let mut response: ApiResponse = match request.namespace.as_str() {
        "fs" => fs::call(request),
        "http" => http::call(request),
        "os" => os::call(request),
        "process" => process::call(request),
        _ => ApiResponse::err("Namespace not found".to_string()),
    };

    response.callback_id = callback_id;
    return response.to_string();
}
