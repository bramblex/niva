use anyhow::{Result, anyhow};
use serde::Deserialize;
use serde_json::json;
use std::collections::HashMap;
use std::net::{Ipv4Addr, Ipv6Addr, SocketAddr};
use url::Url;

use crate::app::api_manager::{ApiManager, ApiRequest, CallContext};

pub fn register_api_instances(api_manager: &mut ApiManager) {
    api_manager.register_stream_api("http.requestStream", request_stream);
}

type Headers = HashMap<String, String>;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RequestOptions {
    pub method: String,
    pub url: String,
    pub headers: Option<Headers>,
    pub body: Option<String>,
}

/// Streaming HTTP request: `head` event carries `{status, headers}`, body
/// arrives as binary chunks, terminal result repeats `{status}`.
/// RequestOptions shape matches the old unary `http.request`.
async fn request_stream(ctx: CallContext, request: ApiRequest) -> Result<()> {
    let (options,) = request.args().get::<(RequestOptions,)>()?;
    let RequestOptions {
        method,
        url,
        headers,
        body,
    } = options;
    let request_url = validate_request_url(&url)?;
    let niva_http_port = ctx.app.server_info()?;

    // ureq is blocking: entire exchange on the elastic pool.
    crate::blocking!({
        use std::io::Read;

        let agent = request_agent(niva_http_port);

        let mut builder = ureq::http::Request::builder()
            .method(method.as_str())
            .uri(request_url.as_str());
        if let Some(headers) = headers {
            for (key, value) in &headers {
                builder = builder.header(key.as_str(), value.as_str());
            }
        }
        let http_response = match body {
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

fn validate_request_url(raw: &str) -> Result<Url> {
    let raw_authority = raw_url_authority(raw)
        .ok_or_else(|| anyhow!("URL must use an absolute http or https address"))?;
    if raw_authority.is_empty() {
        return Err(anyhow!("URL must use an absolute http or https address"));
    }
    if raw_authority.contains('@') {
        return Err(anyhow!("userinfo is not allowed in HTTP URLs"));
    }
    let url = Url::parse(raw).map_err(|error| anyhow!("invalid HTTP URL: {error}"))?;
    validate_url(&url).map_err(|message| anyhow!(message))?;
    Ok(url)
}

fn raw_url_authority(raw: &str) -> Option<&str> {
    raw.split_once("://").map(|(_, remainder)| {
        remainder
            .split(|character| matches!(character, '/' | '?' | '#' | '\\'))
            .next()
            .unwrap_or_default()
    })
}

fn validate_url(url: &Url) -> std::result::Result<(), &'static str> {
    if !matches!(url.scheme(), "http" | "https") || url.host().is_none() {
        return Err("URL must use an absolute http or https address");
    }

    // Url::username() alone cannot distinguish absent userinfo from an empty
    // userinfo component such as `http://@example.test`.
    let authority = url.as_str().split_once("://").map(|(_, remainder)| {
        remainder
            .split(|character| matches!(character, '/' | '?' | '#'))
            .next()
            .unwrap_or_default()
    });
    if !url.username().is_empty()
        || url.password().is_some()
        || authority.is_none_or(|authority| authority.contains('@'))
    {
        return Err("userinfo is not allowed in HTTP URLs");
    }

    Ok(())
}

#[derive(Debug)]
struct GuardedResolver {
    inner: ureq::unversioned::resolver::DefaultResolver,
    niva_http_port: u16,
}

impl ureq::unversioned::resolver::Resolver for GuardedResolver {
    fn resolve(
        &self,
        uri: &ureq::http::Uri,
        config: &ureq::config::Config,
        timeout: ureq::unversioned::transport::NextTimeout,
    ) -> std::result::Result<ureq::unversioned::resolver::ResolvedSocketAddrs, ureq::Error> {
        // ureq invokes the resolver for each redirect hop too. Validate the
        // redirect URI here instead of trusting only the original URL.
        let parsed_url = parse_ureq_uri(uri)?;
        let candidates = self.inner.resolve(uri, config, timeout)?;
        vetted_addresses(uri, &parsed_url, &candidates, self.niva_http_port)
    }
}

fn parse_ureq_uri(uri: &ureq::http::Uri) -> std::result::Result<Url, ureq::Error> {
    validate_request_url(&uri.to_string()).map_err(|error| ureq::Error::BadUri(error.to_string()))
}

fn request_agent_config() -> ureq::config::Config {
    ureq::Agent::config_builder()
        // HTTP error statuses are valid responses. Callers need the status,
        // headers, and body for Node-style IncomingMessage behavior.
        .http_status_as_error(false)
        .tls_config(
            ureq::tls::TlsConfig::builder()
                .provider(ureq::tls::TlsProvider::NativeTls)
                .build(),
        )
        // Config::default() picks up proxy environment variables. They would
        // let a proxy resolve the destination outside this policy.
        .proxy(None)
        .build()
}

fn request_agent(niva_http_port: u16) -> ureq::Agent {
    ureq::Agent::with_parts(
        request_agent_config(),
        ureq::unversioned::transport::DefaultConnector::new(),
        GuardedResolver {
            inner: ureq::unversioned::resolver::DefaultResolver::default(),
            niva_http_port,
        },
    )
}

fn vetted_addresses(
    uri: &ureq::http::Uri,
    parsed_url: &Url,
    candidates: &ureq::unversioned::resolver::ResolvedSocketAddrs,
    niva_http_port: u16,
) -> std::result::Result<ureq::unversioned::resolver::ResolvedSocketAddrs, ureq::Error> {
    let mut vetted = ureq::unversioned::resolver::ResolvedSocketAddrs::from_fn(|_| {
        SocketAddr::from(([0, 0, 0, 0], 0))
    });
    if candidates.is_empty() {
        return Err(ureq::Error::HostNotFound);
    }

    // Fail the entire DNS result if any candidate is forbidden. The connector
    // receives only the exact vetted SocketAddrs; it does not resolve again.
    for address in candidates.iter().copied() {
        if !address_allowed(uri, parsed_url, address, niva_http_port) {
            return Err(ureq::Error::HostNotFound);
        }
        vetted.push(address);
    }

    Ok(vetted)
}

fn address_allowed(
    uri: &ureq::http::Uri,
    parsed_url: &Url,
    address: SocketAddr,
    niva_http_port: u16,
) -> bool {
    let is_current_niva_service = uri.scheme_str() == Some("http")
        && parsed_url.scheme() == "http"
        && parsed_url.host_str() == Some("127.0.0.1")
        && parsed_url.port_or_known_default() == Some(niva_http_port)
        && address == SocketAddr::from(([127, 0, 0, 1], niva_http_port));

    is_current_niva_service || is_public_destination(address)
}

fn is_public_destination(address: SocketAddr) -> bool {
    match address {
        SocketAddr::V4(address) => !is_special_use_v4(*address.ip()),
        SocketAddr::V6(address) => address.scope_id() == 0 && is_public_v6(*address.ip()),
    }
}

fn is_special_use_v4(address: Ipv4Addr) -> bool {
    const BLOCKED: &[(Ipv4Addr, u8)] = &[
        (Ipv4Addr::new(0, 0, 0, 0), 8),      // this network
        (Ipv4Addr::new(10, 0, 0, 0), 8),     // private use
        (Ipv4Addr::new(100, 64, 0, 0), 10),  // shared address space; includes Alibaba metadata
        (Ipv4Addr::new(127, 0, 0, 0), 8),    // loopback
        (Ipv4Addr::new(169, 254, 0, 0), 16), // link-local and common cloud metadata
        (Ipv4Addr::new(172, 16, 0, 0), 12),  // private use
        (Ipv4Addr::new(192, 0, 0, 0), 24),   // protocol assignments
        (Ipv4Addr::new(192, 0, 2, 0), 24),   // documentation
        (Ipv4Addr::new(192, 88, 99, 0), 24), // deprecated 6to4 relay anycast
        (Ipv4Addr::new(192, 168, 0, 0), 16), // private use
        (Ipv4Addr::new(198, 18, 0, 0), 15),  // benchmarking
        (Ipv4Addr::new(198, 51, 100, 0), 24),
        (Ipv4Addr::new(203, 0, 113, 0), 24),
        (Ipv4Addr::new(224, 0, 0, 0), 4), // multicast
        (Ipv4Addr::new(240, 0, 0, 0), 4), // reserved and limited broadcast
    ];

    BLOCKED
        .iter()
        .any(|(network, prefix)| v4_in_prefix(address, *network, *prefix))
        || address.is_broadcast()
}

fn is_public_v6(address: Ipv6Addr) -> bool {
    // Accept global-unicast space only, with conservative exclusions for
    // special-purpose, transition, and documentation ranges. NAT64's
    // well-known /96 is accepted only when its embedded IPv4 target is public.
    const GLOBAL_UNICAST: (Ipv6Addr, u8) = (Ipv6Addr::new(0x2000, 0, 0, 0, 0, 0, 0, 0), 3);
    const NAT64_WELL_KNOWN: (Ipv6Addr, u8) = (Ipv6Addr::new(0x0064, 0xff9b, 0, 0, 0, 0, 0, 0), 96);
    const BLOCKED: &[(Ipv6Addr, u8)] = &[
        (Ipv6Addr::LOCALHOST, 128),
        (Ipv6Addr::UNSPECIFIED, 128),
        (Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0xffff, 0), 96), // IPv4-mapped
        (Ipv6Addr::new(0x0100, 0, 0, 0, 0, 0, 0, 0), 64),
        (Ipv6Addr::new(0x0100, 0, 0, 1, 0, 0, 0, 0), 64),
        (Ipv6Addr::new(0x2001, 0, 0, 0, 0, 0, 0, 0), 23), // protocol assignments
        (Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 0), 32),
        (Ipv6Addr::new(0x2002, 0, 0, 0, 0, 0, 0, 0), 16), // 6to4
        (Ipv6Addr::new(0x3fff, 0, 0, 0, 0, 0, 0, 0), 20), // documentation
        (Ipv6Addr::new(0x5f00, 0, 0, 0, 0, 0, 0, 0), 16), // SRv6 SIDs
        (Ipv6Addr::new(0xfc00, 0, 0, 0, 0, 0, 0, 0), 7),  // unique-local
        (Ipv6Addr::new(0xfe80, 0, 0, 0, 0, 0, 0, 0), 10), // link-local
        (Ipv6Addr::new(0xff00, 0, 0, 0, 0, 0, 0, 0), 8),  // multicast
        (Ipv6Addr::new(0x0064, 0xff9b, 0x0001, 0, 0, 0, 0, 0), 48), // local-use NAT64
    ];

    if v6_in_prefix(address, NAT64_WELL_KNOWN.0, NAT64_WELL_KNOWN.1) {
        let octets = address.octets();
        let embedded = Ipv4Addr::new(octets[12], octets[13], octets[14], octets[15]);
        return !is_special_use_v4(embedded);
    }

    v6_in_prefix(address, GLOBAL_UNICAST.0, GLOBAL_UNICAST.1)
        && !BLOCKED
            .iter()
            .any(|(network, prefix)| v6_in_prefix(address, *network, *prefix))
}

fn v4_in_prefix(address: Ipv4Addr, network: Ipv4Addr, prefix: u8) -> bool {
    debug_assert!((1..=32).contains(&prefix));
    (u32::from(address) ^ u32::from(network)) >> (32 - prefix) == 0
}

fn v6_in_prefix(address: Ipv6Addr, network: Ipv6Addr, prefix: u8) -> bool {
    debug_assert!((1..=128).contains(&prefix));
    (u128::from(address) ^ u128::from(network)) >> (128 - prefix) == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn uri(raw: &str) -> (ureq::http::Uri, Url) {
        let url = validate_request_url(raw).unwrap();
        (raw.parse().unwrap(), url)
    }

    fn candidates(addresses: &[SocketAddr]) -> ureq::unversioned::resolver::ResolvedSocketAddrs {
        let mut out = ureq::unversioned::resolver::ResolvedSocketAddrs::from_fn(|_| {
            SocketAddr::from(([0, 0, 0, 0], 0))
        });
        for address in addresses {
            out.push(*address);
        }
        out
    }

    #[test]
    fn accepts_only_http_urls_without_userinfo() {
        for raw in [
            "file:///etc/passwd",
            "ftp://example.test/file",
            "http:example.test/path",
            "http:///example.test/path",
            "http://user@example.test/",
            "https://user:pass@example.test/",
            "http://@example.test/",
        ] {
            assert!(validate_request_url(raw).is_err(), "{raw}");
        }
        assert!(validate_request_url("https://example.test/path").is_ok());
    }

    #[test]
    fn blocks_private_and_special_use_addresses() {
        for raw in [
            "0.1.2.3",
            "10.0.0.1",
            "100.100.100.200",
            "127.0.0.2",
            "169.254.169.254",
            "172.16.0.1",
            "192.0.2.1",
            "192.168.1.1",
            "198.18.0.1",
            "198.51.100.1",
            "203.0.113.1",
            "224.0.0.1",
            "240.0.0.1",
            "::",
            "::1",
            "::ffff:127.0.0.1",
            "64:ff9b::a00:1",
            "100::1",
            "2001:db8::1",
            "2002:7f00:1::1",
            "fd00:ec2::254",
            "fe80::1",
            "ff02::1",
        ] {
            let ip = raw.parse().unwrap();
            let address = SocketAddr::new(ip, 443);
            assert!(!is_public_destination(address), "{raw}");
        }

        for raw in ["8.8.8.8", "2606:4700::1111", "64:ff9b::808:808"] {
            let ip = raw.parse().unwrap();
            assert!(is_public_destination(SocketAddr::new(ip, 443)), "{raw}");
        }
    }

    #[test]
    fn allows_only_the_exact_current_niva_loopback_endpoint() {
        let port = 43123;
        let address = SocketAddr::from(([127, 0, 0, 1], port));
        let (niva_uri, niva_url) = uri(&format!("http://127.0.0.1:{port}/"));
        assert!(address_allowed(&niva_uri, &niva_url, address, port));

        for raw in [
            format!("http://127.0.0.1:{}/", port + 1),
            format!("https://127.0.0.1:{port}/"),
            format!("http://localhost:{port}/"),
            format!("http://127.0.0.2:{port}/"),
            format!("http://example.test:{port}/"),
        ] {
            let (uri, url) = uri(&raw);
            assert!(!address_allowed(&uri, &url, address, port), "{raw}");
        }
    }

    #[test]
    fn rejects_a_mixed_dns_answer_set_and_revalidates_redirect_uris() {
        let port = 43123;
        let (safe_uri, safe_url) = uri("https://example.test/");
        let mixed = candidates(&[
            SocketAddr::from(([8, 8, 8, 8], 443)),
            SocketAddr::from(([169, 254, 169, 254], 443)),
        ]);
        assert!(matches!(
            vetted_addresses(&safe_uri, &safe_url, &mixed, port),
            Err(ureq::Error::HostNotFound)
        ));

        let safe = candidates(&[SocketAddr::from(([8, 8, 8, 8], 443))]);
        let vetted = vetted_addresses(&safe_uri, &safe_url, &safe, port).unwrap();
        assert_eq!(&vetted[..], &safe[..]);

        let redirect_uri: ureq::http::Uri = "https://user@example.test/".parse().unwrap();
        assert!(parse_ureq_uri(&redirect_uri).is_err());
        let empty_userinfo_redirect: ureq::http::Uri = "https://@example.test/".parse().unwrap();
        assert!(parse_ureq_uri(&empty_userinfo_redirect).is_err());
    }

    #[test]
    fn request_agent_config_disables_environment_proxy() {
        let config = request_agent_config();
        assert!(config.proxy().is_none());
        assert!(!config.http_status_as_error());
    }

    #[test]
    fn request_options_reject_proxy_overrides() {
        let options = json!({
            "method": "GET",
            "url": "https://example.test/",
            "proxy": "http://127.0.0.1:8080"
        });
        assert!(serde_json::from_value::<RequestOptions>(options).is_err());
    }

    #[test]
    fn follows_local_redirect_and_returns_404_status_and_body() {
        use std::io::{BufRead, BufReader, Read, Write};
        use std::net::TcpListener;
        use std::thread;

        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let server = thread::spawn(move || {
            for expected_path in ["/start", "/missing"] {
                let (mut stream, _) = listener.accept().unwrap();
                let mut reader = BufReader::new(stream.try_clone().unwrap());
                let mut request_line = String::new();
                reader.read_line(&mut request_line).unwrap();
                assert_eq!(request_line.split_whitespace().nth(1), Some(expected_path));

                let response = if expected_path == "/start" {
                    "HTTP/1.1 302 Found\r\nLocation: /missing\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
                } else {
                    "HTTP/1.1 404 Not Found\r\nContent-Type: text/plain\r\nContent-Length: 5\r\nConnection: close\r\n\r\nnope!"
                };
                stream.write_all(response.as_bytes()).unwrap();
            }
        });

        let url = format!("http://127.0.0.1:{port}/start");
        let response = request_agent(port).get(&url).call().unwrap();
        let status = response.status().as_u16();
        let mut reader = response.into_body().into_reader();
        let mut body = String::new();
        reader.read_to_string(&mut body).unwrap();
        assert_eq!(status, 404);
        assert_eq!(body, "nope!");
        server.join().unwrap();
    }
}
