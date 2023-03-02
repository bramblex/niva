mod fs;
mod http;
mod os;
mod process;

use serde::{Deserialize, Serialize};
use serde_json;

#[derive(Debug, Serialize, Deserialize)]
pub struct ApiRequest {
    namespace: String,
    method: String,
    data: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ApiResponse {
    pub code: i16,
    pub message: String,
    pub data: serde_json::Value,
}
unsafe impl Send for ApiResponse {}
unsafe impl Sync for ApiResponse {}

impl ApiResponse {
    pub fn ok(data: serde_json::Value) -> ApiResponse {
        return ApiResponse {
            code: 0,
            message: String::new(),
            data,
        };
    }

    pub fn err(message: String) -> ApiResponse {
        return ApiResponse {
            code: -1,
            message,
            data: serde_json::Value::Null,
        };
    }

    pub fn to_string(&self) -> String {
        return serde_json::to_string::<ApiResponse>(self).unwrap();
    }
}

pub async fn call(request_str: String) -> String {
    let request_result = serde_json::from_str::<ApiRequest>(request_str.as_str());
    if request_result.is_err() {
        return ApiResponse::err("Invalid request".to_string()).to_string();
    }

    let request = request_result.unwrap();
    let response: ApiResponse = match request.namespace.as_str() {
        "fs" => fs::call(request).await,
        "http" => http::call(request).await,
        "os" => os::call(request).await,
        "process" => process::call(request).await,
        _ => ApiResponse::err("Namespace not found".to_string()),
    };

		return response.to_string();
}
