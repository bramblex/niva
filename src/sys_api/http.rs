use std::{collections::HashMap, sync::Arc};

use super::{ApiRequest, ApiResponse};
use serde::Deserialize;
use serde_json::json;

pub fn call(request: ApiRequest) -> ApiResponse {
    return match request.method.as_str() {
        "request" => request_method(request),
        _ => ApiResponse::err(request.callback_id, "Method not found"),
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
        return ApiResponse::err(request.callback_id, "Invalid options");
    }
    let options = data_result.unwrap();

    let agent = ureq::AgentBuilder::new()
        .tls_connector(Arc::new(native_tls::TlsConnector::new().unwrap()))
        .build();

    let mut http_request = agent.request(&options.method, options.url.as_str());

    if let Some(headers) = options.headers {
        for (key, value) in headers {
            http_request = http_request.set(key.as_str(), value.as_str());
        }
    };

    let result = if let Some(body) = options.body {
        http_request.send_string(body.as_str())
    } else {
        http_request.call()
    };

    if let Ok(http_response) = result {
        let status = http_response.status();
        let header_names = http_response.headers_names();

        let mut response_headers = HashMap::new();
        for name in header_names {
            let value = http_response.header(&name).unwrap();
            response_headers.insert(name, value.to_string());
        }

        let body = http_response.into_string().unwrap();
        return ApiResponse::ok(
            request.callback_id,
            json!({
                "status": status,
                "headers": response_headers,
                "body": body,
            }),
        );
    }

    ApiResponse::err(request.callback_id, "request error")
}
