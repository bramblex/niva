use crate::{
    api_manager::{ApiManager, ApiRequest},
    environment::EnvironmentRef,
};
use anyhow::Result;
use serde::Deserialize;
use serde_json::{Value, json};
use std::{collections::HashMap, sync::Arc};

pub fn register_apis(api_manager: &mut ApiManager) {
    api_manager.register_async_api("http.request", request);
    api_manager.register_async_api("http.get", get);
    api_manager.register_async_api("http.post", post);
}

type Headers = HashMap<String, String>;

#[derive(Deserialize)]
struct RequestOptions {
    pub method: String,
    pub url: String,
    pub headers: Option<Headers>,
    pub body: Option<String>,
}

fn _request(options: RequestOptions) -> Result<Value> {
    let agent = ureq::AgentBuilder::new()
        .tls_connector(Arc::new(native_tls::TlsConnector::new().unwrap()))
        .build();

    let mut http_request = agent.request(&options.method, options.url.as_str());

    if let Some(headers) = options.headers {
        for (key, value) in headers {
            http_request = http_request.set(key.as_str(), value.as_str());
        }
    };

    let http_response = if let Some(body) = options.body {
        http_request.send_string(body.as_str())?
    } else {
        http_request.call()?
    };

    let status = http_response.status();
    let header_names = http_response.headers_names();

    let mut response_headers = HashMap::new();
    for name in header_names {
        let value = http_response.header(&name).unwrap();
        response_headers.insert(name, value.to_string());
    }

    let body = http_response.into_string()?;

    Ok(json!({
        "status": status,
        "headers": response_headers,
        "body": body,
    }))
}

fn request(_: EnvironmentRef, request: ApiRequest) -> Result<Value> {
    let options = request.args().get_single::<RequestOptions>()?;
    _request(options)
}

fn get(_: EnvironmentRef, request: ApiRequest) -> Result<Value> {
    let (url, headers) = request.args().get_with_optional::<(String, Option<Headers>)>(2)?;
    _request(RequestOptions{
        method: "GET".to_string(),
        url,
        headers,
        body: None,
    })
}

fn post(_: EnvironmentRef, request: ApiRequest) -> Result<Value> {
    let (url, body, headers) = request.args().get_with_optional::<(String, String, Option<Headers>)>(3)?;
    _request(RequestOptions{
        method: "POST".to_string(),
        url,
        headers,
        body: Some(body),
    })
}

