use anyhow::Result;
use serde::Deserialize;
use serde_json::json;
use std::collections::HashMap;

use crate::app::api_manager::{ApiManager, ApiRequest, CallContext};

pub fn register_api_instances(api_manager: &mut ApiManager) {
    api_manager.register_stream_api("http.requestStream", request_stream);
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

/// Streaming HTTP request: `head` event carries `{status, headers}`, body
/// arrives as binary chunks, terminal result repeats `{status}`.
/// RequestOptions shape matches the old unary `http.request`.
async fn request_stream(ctx: CallContext, request: ApiRequest) -> Result<()> {
    use crate::app::api_manager::ApiRequest;

    let (options,) = request.args().get::<(RequestOptions,)>()?;
    let RequestOptions {
        method,
        url,
        headers,
        body,
        proxy,
    } = options;

    // ureq is blocking: entire exchange on the elastic pool.
    crate::blocking!({
        use std::io::Read;

        let mut config = ureq::Agent::config_builder().tls_config(
            ureq::tls::TlsConfig::builder()
                .provider(ureq::tls::TlsProvider::NativeTls)
                .build(),
        );
        if let Some(proxy) = proxy {
            config = config.proxy(Some(ureq::Proxy::new(&proxy)?));
        }
        let agent: ureq::Agent = config.build().into();

        let mut builder = ureq::http::Request::builder()
            .method(method.as_str())
            .uri(url.as_str());
        if let Some(headers) = headers {
            for (key, value) in &headers {
                builder = builder.header(key.as_str(), value.as_str());
            }
        }
        let mut http_response = match body {
            Some(body) => agent.run(builder.body(body)?)?,
            None => agent.run(builder.body(())?)?,
        };

        let mut response_headers = HashMap::new();
        for (name, value) in http_response.headers().iter() {
            response_headers.insert(
                name.to_string(),
                value
                    .to_str()
                    .map(|s| s.to_string())
                    .unwrap_or_else(|_| String::from_utf8_lossy(value.as_bytes()).into_owned()),
            );
        }
        let status = http_response.status().as_u16();
        ctx.push(
            "head",
            json!({ "status": status, "headers": response_headers }),
        );

        let mut reader = http_response.into_body().into_reader();
        let mut buf = vec![0u8; 65536];
        loop {
            if ctx.is_cancelled() {
                return Ok(());
            }
            let n = reader.read(&mut buf)?;
            if n == 0 {
                break;
            }
            ctx.chunk(&buf[..n], false);
        }
        ctx.chunk(&[], true);
        ctx.respond(Ok(json!({ "status": status })));
        Ok(())
    })
    .await?;
    Ok(())
}
