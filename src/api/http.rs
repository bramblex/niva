use std::collections::HashMap;

use super::{ApiRequest, ApiResponse};
use serde::Deserialize;
use serde_json::{json, Value};

pub async fn call(request: ApiRequest) -> ApiResponse {
    return match request.method.as_str() {
        "request" => request_method(request).await,
        _ => ApiResponse::err("Method not found".to_string()),
    };
}

#[derive(Deserialize)]
struct RequestOptions {
    pub method: String,
    pub url: String,
    pub headers: Option<HashMap<String, String>>,
    pub body: Option<String>,
}

pub async fn request_method(request: ApiRequest) -> ApiResponse {
    let data_result = serde_json::from_value::<RequestOptions>(request.data);
    if data_result.is_err() {
        return ApiResponse::err("Invalid options".to_string());
    }
    let options = data_result.unwrap();

    let mut builder = reqwest::Client::new().request(options.method.parse().unwrap(), options.url);
    if let Some(headers) = options.headers {
        for (key, value) in headers {
            builder = builder.header(key, value);
        }
    }
    if let Some(body) = options.body {
        builder = builder.body(body);
    }
    let response = builder.send().await.unwrap();
    let status = response.status();
    let headers = response.headers().clone();
    let body = response.text().await.unwrap();

    let mut response_headers = HashMap::new();
    for (key, value) in headers {
        response_headers.insert(
            key.unwrap().as_str().to_string(),
            value.to_str().unwrap().to_string(),
        );
    }

    return ApiResponse::ok(json!({
        "status": status.as_u16(),
        "headers": response_headers,
        "body": body,
    }));
}
