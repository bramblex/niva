use anyhow::Result;
use serde::Deserialize;
use serde_json::{json, Value};
use std::{collections::HashMap, sync::Arc};

use crate::{
    app::{
        api_manager::{ApiManager, ApiRequest},
        window_manager::window::NivaWindow,
        NivaApp,
    },
    args_match, opts_match,
};

pub fn register_api_instances(api_manager: &mut ApiManager) {
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
    pub proxy: Option<String>,
}

fn _request(options: RequestOptions) -> Result<Value> {
    let mut agent_builder =
        ureq::AgentBuilder::new().tls_connector(Arc::new(native_tls::TlsConnector::new()?));

    if let Some(proxy) = options.proxy {
        let proxy = ureq::Proxy::new(proxy)?;
        agent_builder = agent_builder.proxy(proxy);
    }

    let agent = agent_builder.build();

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
        if let Some(value) = http_response.header(&name) {
            response_headers.insert(name, value.to_string());
        }
    }

    let body = http_response.into_string()?;

    Ok(json!({
        "status": status,
        "headers": response_headers,
        "body": body,
    }))
}

fn request(_: Arc<NivaApp>, _: Arc<NivaWindow>, request: ApiRequest) -> Result<Value> {
    args_match!(request, options: RequestOptions);
    _request(options)
}

fn get(_: Arc<NivaApp>, _: Arc<NivaWindow>, request: ApiRequest) -> Result<Value> {
    opts_match!(request, url: String, headers: Option<Headers>);
    _request(RequestOptions {
        method: "GET".to_string(),
        url,
        headers,
        body: None,
        proxy: None,
    })
}

fn post(_: Arc<NivaApp>, _: Arc<NivaWindow>, request: ApiRequest) -> Result<Value> {
    opts_match!(request, url: String, body: String, headers: Option<Headers>);
    _request(RequestOptions {
        method: "POST".to_string(),
        url,
        headers,
        body: Some(body),
        proxy: None,
    })
}
