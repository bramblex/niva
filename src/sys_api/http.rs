use std::{collections::HashMap, sync::Arc};

use super::{ApiRequest, ApiResponse};
use serde::Deserialize;
use serde_json::json;

pub fn call(request: ApiRequest) -> ApiResponse {
    return match request.method.as_str() {
        "request" => request_method(request),
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

pub fn request_method(request: ApiRequest) -> ApiResponse {
    let data_result = serde_json::from_value::<RequestOptions>(request.data);
    if data_result.is_err() {
        return ApiResponse::err("Invalid options".to_string());
    }
    let options = data_result.unwrap();

    let agent = ureq::AgentBuilder::new()
        .tls_connector(Arc::new(native_tls::TlsConnector::new().unwrap()))
        .build();

    let mut request = agent.request(&options.method, options.url.as_str());

    if let Some(headers) = options.headers {
        for (key, value) in headers {
            request = request.set(key.as_str(), value.as_str());
        }
    };

    let result = if let Some(body) = options.body {
        request.send_string(body.as_str())
    } else {
        request.call()
    };

    if let Ok(response) = result {
        let status = response.status();
        let header_names = response.headers_names();

        let mut response_headers = HashMap::new();
        for name in header_names {
            let value = response.header(&name).unwrap();
            response_headers.insert(name, value.to_string());
        }

        let body = response.into_string().unwrap();
        return ApiResponse::ok(json!({
            "status": status,
            "headers": response_headers,
            "body": body,
        }));
    }

    ApiResponse::err("request error".to_string())
}
