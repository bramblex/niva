use std::{
    sync::{
        Arc, Condvar, Mutex,
        mpsc::{self, SyncSender, TrySendError},
    },
    thread,
    time::{Duration, Instant},
};

use anyhow::{Context, Result, ensure};
use wry::{
    RequestAsyncResponder,
    http::{
        Method, Request, Response, StatusCode, Uri,
        header::{ACCEPT_RANGES, CONTENT_LENGTH, CONTENT_RANGE, CONTENT_TYPE, RANGE},
    },
};

use super::{node_compat::NodeCompat, resource_manager::ResourceManager};

pub(crate) const SCHEME: &str = "niva";
#[cfg(not(target_os = "windows"))]
pub(crate) const MACOS_ORIGIN: &str = "niva://app";
#[cfg(target_os = "windows")]
pub(crate) const WINDOWS_ORIGIN: &str = "http://niva.app";

const WORKER_COUNT: usize = 4;
const QUEUE_CAPACITY: usize = 32;
const REQUEST_TIMEOUT: Duration = Duration::from_secs(15);
/// Wry currently accepts a complete `Vec<u8>` response. Bound each response
/// allocation and let clients request large media in valid byte ranges.
const MAX_RESPONSE_BYTES: usize = 32 * 1024 * 1024;

/// Return the exact origin emitted by the platform WebView for Niva's custom
/// scheme. Wry maps `niva://app/` to `http://niva.app/` in WebView2.
pub(crate) fn platform_origin() -> &'static str {
    #[cfg(target_os = "windows")]
    {
        WINDOWS_ORIGIN
    }
    #[cfg(not(target_os = "windows"))]
    {
        MACOS_ORIGIN
    }
}

/// Dispatches protocol loads to a fixed number of background workers. A full
/// queue is rejected immediately, so requests cannot create an unbounded
/// number of threads or block Wry's event loop.
#[derive(Clone)]
pub(crate) struct CustomProtocolDispatcher {
    sender: SyncSender<ProtocolJob>,
}

impl CustomProtocolDispatcher {
    pub(crate) fn new() -> Result<Self> {
        let (sender, receiver) = mpsc::sync_channel::<ProtocolJob>(QUEUE_CAPACITY);
        let receiver = Arc::new(Mutex::new(receiver));

        for worker_id in 0..WORKER_COUNT {
            let receiver = receiver.clone();
            thread::Builder::new()
                .name(format!("niva-protocol-{worker_id}"))
                .spawn(move || {
                    loop {
                        let job = {
                            let receiver = match receiver.lock() {
                                Ok(receiver) => receiver,
                                Err(_) => break,
                            };
                            match receiver.recv() {
                                Ok(job) => job,
                                Err(_) => break,
                            }
                        };
                        job.run();
                    }
                })?;
        }

        Ok(Self { sender })
    }

    pub(crate) fn dispatch(
        &self,
        resources: Arc<dyn ResourceManager>,
        node_compat: Option<NodeCompat>,
        server_port: u16,
        request: Request<Vec<u8>>,
        responder: RequestAsyncResponder,
    ) {
        let gate = Arc::new(ResponseGate::new(responder));
        let job = ProtocolJob {
            resources,
            node_compat,
            server_port,
            request,
            gate: gate.clone(),
        };
        match self.sender.try_send(job) {
            Ok(()) => {
                if let Err(err) = arm_timeout(gate.clone()) {
                    gate.respond(error_response(
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "Unable to schedule protocol response",
                    ));
                    crate::niva_log!(
                        crate::app::logging::Level::Error,
                        "unable to start protocol timeout: {err}"
                    );
                }
            }
            Err(TrySendError::Full(job)) => job.respond(error_response(
                StatusCode::SERVICE_UNAVAILABLE,
                "Local page service is busy",
            )),
            Err(TrySendError::Disconnected(job)) => job.respond(error_response(
                StatusCode::SERVICE_UNAVAILABLE,
                "Local page service is unavailable",
            )),
        }
    }
}

struct ResponseGate {
    responder: Mutex<Option<RequestAsyncResponder>>,
    changed: Condvar,
}

impl ResponseGate {
    fn new(responder: RequestAsyncResponder) -> Self {
        Self {
            responder: Mutex::new(Some(responder)),
            changed: Condvar::new(),
        }
    }

    fn is_pending(&self) -> bool {
        self.responder
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .is_some()
    }

    fn respond(&self, response: Response<Vec<u8>>) {
        let responder = self
            .responder
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .take();
        if let Some(responder) = responder {
            responder.respond(response);
            self.changed.notify_all();
        }
    }
}

fn arm_timeout(gate: Arc<ResponseGate>) -> std::io::Result<()> {
    thread::Builder::new()
        .name("niva-protocol-timeout".into())
        .spawn(move || {
            let deadline = Instant::now() + REQUEST_TIMEOUT;
            let mut pending = gate
                .responder
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            while pending.is_some() {
                let now = Instant::now();
                if now >= deadline {
                    let responder = pending.take();
                    drop(pending);
                    if let Some(responder) = responder {
                        responder.respond(error_response(
                            StatusCode::GATEWAY_TIMEOUT,
                            "Local page request timed out",
                        ));
                        gate.changed.notify_all();
                    }
                    return;
                }
                let wait = deadline.saturating_duration_since(now);
                let (next, timed) = gate
                    .changed
                    .wait_timeout(pending, wait)
                    .unwrap_or_else(std::sync::PoisonError::into_inner);
                pending = next;
                if timed.timed_out() && pending.is_some() {
                    continue;
                }
            }
        })
        .map(|_| ())
}

struct ProtocolJob {
    resources: Arc<dyn ResourceManager>,
    node_compat: Option<NodeCompat>,
    server_port: u16,
    request: Request<Vec<u8>>,
    gate: Arc<ResponseGate>,
}

impl ProtocolJob {
    fn run(self) {
        if !self.gate.is_pending() {
            return;
        }
        let response = response_for_request(
            &self.request,
            self.resources.as_ref(),
            self.node_compat.as_ref(),
            self.server_port,
        );
        self.respond(response);
    }

    fn respond(self, response: Response<Vec<u8>>) {
        self.gate.respond(response);
    }
}

fn response_for_request(
    request: &Request<Vec<u8>>,
    resources: &dyn ResourceManager,
    node_compat: Option<&NodeCompat>,
    server_port: u16,
) -> Response<Vec<u8>> {
    let is_head = request.method() == Method::HEAD;
    let mut response = response_for_request_impl(request, resources, node_compat, server_port);
    if is_head {
        let length = response
            .headers()
            .get(CONTENT_LENGTH)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.parse::<u64>().ok())
            .unwrap_or(response.body().len() as u64);
        response
            .headers_mut()
            .insert(CONTENT_LENGTH, length.to_string().parse().unwrap());
        response.body_mut().clear();
    }
    response
}

fn response_for_request_impl(
    request: &Request<Vec<u8>>,
    resources: &dyn ResourceManager,
    node_compat: Option<&NodeCompat>,
    server_port: u16,
) -> Response<Vec<u8>> {
    let is_head = request.method() == Method::HEAD;
    if request.method() != Method::GET && !is_head {
        return Response::builder()
            .status(StatusCode::METHOD_NOT_ALLOWED)
            .header("allow", "GET, HEAD")
            .body(Vec::new())
            .unwrap_or_else(|_| {
                error_response(StatusCode::METHOD_NOT_ALLOWED, "GET or HEAD only")
            });
    }
    if !is_app_uri(request.uri()) {
        return error_response(StatusCode::NOT_FOUND, "Not found");
    }
    if request.body().len() > 64 * 1024 {
        return error_response(StatusCode::PAYLOAD_TOO_LARGE, "Request body too large");
    }

    let path = match asset_path(request.uri()) {
        Ok(path) => path,
        Err(_) => return error_response(StatusCode::BAD_REQUEST, "Invalid resource path"),
    };
    if path.starts_with("__niva_") {
        let is_allowed_runtime_asset = path.starts_with(RUNTIME_ASSET_PREFIX)
            && node_compat.is_some_and(|compat| compat.allows_asset(&path));
        if !is_allowed_runtime_asset {
            return error_response(StatusCode::NOT_FOUND, "Not found");
        }
    }

    let mime = mime_guess::from_path(&path)
        .first()
        .map(|mime| mime.to_string())
        .unwrap_or_else(|| "application/octet-stream".to_string());
    let is_runtime_asset = path.starts_with(RUNTIME_ASSET_PREFIX);
    if is_runtime_asset {
        let Some(body) = NodeCompat::embedded_asset(&path) else {
            return error_response(StatusCode::NOT_FOUND, "Not found");
        };
        let mut body = body;
        if mime == "text/html" && !has_range(request) && is_document_navigation(request) {
            body = match prepare_document(body, node_compat, server_port) {
                Ok(body) => body,
                Err(response) => return response,
            };
        }
        return response_for_memory_body(request, &mime, &body);
    }

    let total_len = match resources.resource_size(&path) {
        Ok(Some(length)) => length,
        Ok(None) | Err(_) => return error_response(StatusCode::NOT_FOUND, "Not found"),
    };
    let range = match requested_range(request, total_len) {
        Ok(range) => range,
        Err(_) => return range_not_satisfiable(total_len),
    };
    if let Some((start, end)) = range {
        let length = usize::try_from(end - start + 1).unwrap_or(usize::MAX);
        let body = if is_head {
            Vec::new()
        } else {
            let read = match resources.read_range(&path, start, length) {
                Ok(read) => read,
                Err(_) => {
                    return error_response(
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "Unable to read resource range",
                    );
                }
            };
            if read.total_len != total_len || read.bytes.len() != length {
                return error_response(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Resource changed during read",
                );
            }
            read.bytes
        };
        return ranged_response(
            StatusCode::PARTIAL_CONTENT,
            &mime,
            body,
            start,
            end,
            total_len,
            length,
        );
    }

    let needs_document_body = mime == "text/html" && is_document_navigation(request);
    if total_len > MAX_RESPONSE_BYTES as u64 && (!is_head || needs_document_body) {
        return Response::builder()
            .status(StatusCode::PAYLOAD_TOO_LARGE)
            .header(CONTENT_RANGE, format!("bytes */{total_len}"))
            .header(ACCEPT_RANGES, "bytes")
            .body(b"Resource exceeds the bounded WebView response; request a byte range.".to_vec())
            .unwrap_or_else(|_| {
                error_response(StatusCode::PAYLOAD_TOO_LARGE, "Resource too large")
            });
    }

    let mut body = if is_head && !(mime == "text/html" && is_document_navigation(request)) {
        Vec::new()
    } else {
        let read = match resources.read_range(&path, 0, total_len as usize) {
            Ok(read) => read,
            Err(_) => {
                return error_response(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Unable to read resource",
                );
            }
        };
        if read.total_len != total_len || read.bytes.len() != total_len as usize {
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Resource changed during read",
            );
        }
        read.bytes
    };
    if mime == "text/html" && is_document_navigation(request) {
        body = match prepare_document(body, node_compat, server_port) {
            Ok(body) => body,
            Err(response) => return response,
        };
    }
    let response_len = if is_head && body.is_empty() && total_len != 0 {
        total_len
    } else {
        body.len() as u64
    };
    full_response(request.method(), &mime, body, response_len)
}

fn has_range(request: &Request<Vec<u8>>) -> bool {
    request.headers().contains_key(RANGE)
}

/// Return a single bounded inclusive range. A valid request larger than the
/// Wry response cap is served as a smaller 206 range, with exact headers so a
/// media client can request the next segment. Multiple ranges are unsupported.
fn requested_range(
    request: &Request<Vec<u8>>,
    total_len: u64,
) -> anyhow::Result<Option<(u64, u64)>> {
    let Some(value) = request.headers().get(RANGE) else {
        return Ok(None);
    };
    let value = value.to_str()?;
    let spec = value
        .strip_prefix("bytes=")
        .context("unsupported range unit")?;
    ensure!(!spec.contains(',') && total_len > 0, "unsatisfiable range");
    let (start, requested_end) = if let Some(suffix) = spec.strip_prefix('-') {
        let suffix = suffix.parse::<u64>()?;
        ensure!(suffix > 0, "empty suffix range");
        (total_len.saturating_sub(suffix), total_len - 1)
    } else {
        let (start, end) = spec.split_once('-').context("invalid byte range")?;
        let start = start.parse::<u64>()?;
        let end = if end.is_empty() {
            total_len - 1
        } else {
            end.parse::<u64>()?.min(total_len - 1)
        };
        (start, end)
    };
    ensure!(
        start < total_len && requested_end >= start,
        "unsatisfiable range"
    );
    let cap_end = start.saturating_add(MAX_RESPONSE_BYTES as u64 - 1);
    Ok(Some((start, requested_end.min(cap_end))))
}

fn range_not_satisfiable(total_len: u64) -> Response<Vec<u8>> {
    Response::builder()
        .status(StatusCode::RANGE_NOT_SATISFIABLE)
        .header(CONTENT_RANGE, format!("bytes */{total_len}"))
        .header(ACCEPT_RANGES, "bytes")
        .header(CONTENT_LENGTH, "0")
        .body(Vec::new())
        .unwrap_or_else(|_| {
            error_response(StatusCode::RANGE_NOT_SATISFIABLE, "Range not satisfiable")
        })
}

fn response_for_memory_body(
    request: &Request<Vec<u8>>,
    mime: &str,
    body: &[u8],
) -> Response<Vec<u8>> {
    if body.len() > MAX_RESPONSE_BYTES {
        return error_response(StatusCode::PAYLOAD_TOO_LARGE, "Resource too large");
    }
    let total_len = body.len() as u64;
    let range = match requested_range(request, total_len) {
        Ok(range) => range,
        Err(_) => return range_not_satisfiable(total_len),
    };
    if let Some((start, end)) = range {
        let start = start as usize;
        let end = end as usize + 1;
        let response_body = if request.method() == Method::HEAD {
            Vec::new()
        } else {
            body[start..end].to_vec()
        };
        return ranged_response(
            StatusCode::PARTIAL_CONTENT,
            mime,
            response_body,
            start as u64,
            end as u64 - 1,
            total_len,
            end - start,
        );
    }
    let response_body = if request.method() == Method::HEAD {
        Vec::new()
    } else {
        body.to_vec()
    };
    full_response(request.method(), mime, response_body, total_len)
}

fn full_response(
    method: &Method,
    mime: &str,
    body: Vec<u8>,
    content_len: u64,
) -> Response<Vec<u8>> {
    let body = if *method == Method::HEAD {
        Vec::new()
    } else {
        body
    };
    Response::builder()
        .status(StatusCode::OK)
        .header(CONTENT_TYPE, mime)
        .header(CONTENT_LENGTH, content_len.to_string())
        .header(ACCEPT_RANGES, "bytes")
        .header("x-content-type-options", "nosniff")
        .body(body)
        .unwrap_or_else(|_| {
            error_response(StatusCode::INTERNAL_SERVER_ERROR, "Internal server error")
        })
}

fn ranged_response(
    status: StatusCode,
    mime: &str,
    body: Vec<u8>,
    start: u64,
    end: u64,
    total_len: u64,
    content_len: usize,
) -> Response<Vec<u8>> {
    Response::builder()
        .status(status)
        .header(CONTENT_TYPE, mime)
        .header(CONTENT_LENGTH, content_len.to_string())
        .header(CONTENT_RANGE, format!("bytes {start}-{end}/{total_len}"))
        .header(ACCEPT_RANGES, "bytes")
        .header("x-content-type-options", "nosniff")
        .body(body)
        .unwrap_or_else(|_| {
            error_response(StatusCode::INTERNAL_SERVER_ERROR, "Internal server error")
        })
}

fn prepare_document(
    body: Vec<u8>,
    node_compat: Option<&NodeCompat>,
    server_port: u16,
) -> std::result::Result<Vec<u8>, Response<Vec<u8>>> {
    let mut body = if let Some(compat) = node_compat {
        compat.rewrite_html(&body).map_err(|_| {
            error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Unable to prepare local page",
            )
        })?
    } else {
        body
    };
    match String::from_utf8(body) {
        Ok(mut html) => {
            if patch_document_csp(&mut html, server_port).is_err() {
                return Err(error_response(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Unable to prepare local page security policy",
                ));
            }
            body = html.into_bytes();
        }
        Err(_) if node_compat.is_some() => {
            return Err(error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "NodeCompat requires UTF-8 HTML",
            ));
        }
        Err(error) => body = error.into_bytes(),
    }
    if body.len() > MAX_RESPONSE_BYTES {
        return Err(error_response(
            StatusCode::PAYLOAD_TOO_LARGE,
            "Resource too large",
        ));
    }
    Ok(body)
}

fn is_app_uri(uri: &Uri) -> bool {
    let Some(authority) = uri.authority() else {
        return false;
    };
    uri.scheme_str() == Some(SCHEME)
        && authority.as_str().eq_ignore_ascii_case("app")
        && uri.path().starts_with('/')
}

fn asset_path(uri: &Uri) -> Result<String> {
    let path = uri.path();
    let relative = path.strip_prefix('/').unwrap_or(path);
    let decoded = super::resource_manager::percent_decode_path(relative)?;
    if decoded.is_empty() {
        return Ok("index.html".into());
    }
    let components = decoded.split('/').collect::<Vec<_>>();
    let trailing_slash = decoded.ends_with('/');
    if decoded.contains('\\')
        || decoded.starts_with('/')
        || components.iter().enumerate().any(|(index, component)| {
            matches!(*component, "." | "..")
                || (component.is_empty() && !(trailing_slash && index + 1 == components.len()))
        })
    {
        return Err(anyhow::anyhow!("invalid resource path"));
    }
    if decoded.ends_with('/') {
        return Ok(format!("{decoded}index.html"));
    }
    Ok(decoded)
}

fn is_document_navigation(request: &Request<Vec<u8>>) -> bool {
    if let Some(mode) = request.headers().get("sec-fetch-mode") {
        return mode
            .to_str()
            .is_ok_and(|mode| mode.trim().eq_ignore_ascii_case("navigate"));
    }
    request
        .headers()
        .get("accept")
        .and_then(|value| value.to_str().ok())
        .is_some_and(|accept| accept.to_ascii_lowercase().contains("text/html"))
}

fn response(status: StatusCode, content_type: &str, body: Vec<u8>) -> Response<Vec<u8>> {
    Response::builder()
        .status(status)
        .header(CONTENT_TYPE, content_type)
        .header("x-content-type-options", "nosniff")
        .body(body)
        .unwrap_or_else(|_| {
            Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(b"Internal server error".to_vec())
                .unwrap()
        })
}

fn error_response(status: StatusCode, message: &str) -> Response<Vec<u8>> {
    response(
        status,
        "text/plain; charset=utf-8",
        message.as_bytes().to_vec(),
    )
}

const NODE_COMPAT_MARKER: &str = "<!-- niva-runtime-importmap -->";
const RUNTIME_ASSET_PREFIX: &str = "__niva_runtime/";

/// Add the active loopback sources to existing CSP meta policies. When
/// Runtime injected an import map, give only that tag the runtime nonce and
/// place them after the policy so the nonce is actually enforced.
pub(crate) fn patch_document_csp(html: &mut String, server_port: u16) -> Result<()> {
    let node_compat_block = node_compat_block_range(html)?;
    if !csp_meta_ranges(html).is_empty() {
        let nonce = NodeCompat::csp_nonce()?;
        if node_compat_block.is_some() {
            move_node_compat_block_after_csp(html)?;
            add_node_compat_nonce(html, nonce)?;
        }
        // User CommonJS factories need the same nonce even when ESM injection
        // is disabled. This never enables unsafe-eval or arbitrary inline JS.
        patch_csp_script_nonce(html, nonce);
    }

    patch_csp_connect_source(html, &format!("ws://127.0.0.1:{server_port}"));
    patch_csp_connect_source(html, &format!("http://127.0.0.1:{server_port}"));
    let filesystem_origin = format!("http://127.0.0.1:{server_port}");
    for directive in ["img-src", "media-src", "font-src", "style-src"] {
        patch_csp_resource_source(html, directive, &filesystem_origin);
    }
    Ok(())
}

fn node_compat_block_range(html: &str) -> Result<Option<(usize, usize)>> {
    let Some(start) = html.find(NODE_COMPAT_MARKER) else {
        return Ok(None);
    };
    let tag_start = start + NODE_COMPAT_MARKER.len();
    let tag_end = html[tag_start..]
        .find('>')
        .map(|end| tag_start + end + 1)
        .ok_or_else(|| anyhow::anyhow!("Runtime importmap tag is unterminated"))?;
    let tag = &html[tag_start..tag_end];
    let is_map = tag.starts_with("<script")
        && tag_attribute_range(tag, "type")
            .is_some_and(|(a, b)| tag[a..b].eq_ignore_ascii_case("importmap"));
    if !is_map {
        return Err(anyhow::anyhow!("Runtime marker must precede its importmap"));
    }
    let close_end = html[tag_end..]
        .find("</script>")
        .map(|end| tag_end + end + "</script>".len())
        .ok_or_else(|| anyhow::anyhow!("Runtime importmap is unterminated"))?;
    Ok(Some((start, close_end)))
}

fn move_node_compat_block_after_csp(html: &mut String) -> Result<()> {
    let Some((block_start, block_end)) = node_compat_block_range(html)? else {
        return Ok(());
    };
    let meta_ranges = csp_meta_ranges(html);
    let Some((meta_start, meta_end, _, _)) = meta_ranges
        .iter()
        .copied()
        .filter(|(start, end, _, _)| *start >= block_end && *end <= head_end(html))
        .last()
    else {
        return Ok(());
    };

    if has_executable_script_between(html, block_end, meta_start) {
        return Err(anyhow::anyhow!(
            "NodeCompat requires CSP meta to precede page scripts"
        ));
    }

    let block = html[block_start..block_end].to_string();
    html.replace_range(block_start..block_end, "");
    let insertion = meta_end - (block_end - block_start);
    html.insert_str(insertion, &block);
    Ok(())
}

fn head_end(html: &str) -> usize {
    html.to_ascii_lowercase()
        .find("</head>")
        .unwrap_or(html.len())
}

fn has_executable_script_between(html: &str, start: usize, end: usize) -> bool {
    let lower = html.to_ascii_lowercase();
    let mut cursor = start;
    while cursor < end {
        let Some(relative) = lower[cursor..end].find("<script") else {
            break;
        };
        let tag_start = cursor + relative;
        let Some(tag_end) = lower[tag_start..end].find('>') else {
            break;
        };
        let tag_end = tag_start + tag_end + 1;
        let script_type = tag_attribute_range(&html[tag_start..tag_end], "type").map(
            |(value_start, value_end)| {
                html[tag_start + value_start..tag_start + value_end].to_ascii_lowercase()
            },
        );
        if script_type.as_deref().is_none_or(|script_type| {
            !matches!(
                script_type,
                "application/json" | "application/ld+json" | "importmap" | "speculationrules"
            )
        }) {
            return true;
        }
        cursor = tag_end;
    }
    false
}

fn add_node_compat_nonce(html: &mut String, nonce: &str) -> Result<()> {
    let Some((start, _)) = node_compat_block_range(html)? else {
        return Ok(());
    };
    let tag_start = start + NODE_COMPAT_MARKER.len();
    let tag_end = html[tag_start..]
        .find('>')
        .map(|end| tag_start + end)
        .ok_or_else(|| anyhow::anyhow!("Runtime importmap tag is unterminated"))?;
    if let Some((a, b)) = tag_attribute_range(&html[tag_start..=tag_end], "nonce") {
        html.replace_range(tag_start + a..tag_start + b, nonce);
    } else {
        html.insert_str(tag_end, &format!(" nonce=\"{nonce}\""));
    }
    Ok(())
}

fn patch_csp_script_nonce(html: &mut String, nonce: &str) {
    let ranges = csp_meta_ranges(html);
    for (_, _, content_start, content_end) in ranges.into_iter().rev() {
        let original = html[content_start..content_end].to_string();
        let updated = add_script_nonce(&original, nonce);
        if updated != original {
            html.replace_range(content_start..content_end, &updated);
        }
    }
}

/// Extend every enforcing CSP meta policy with one loopback source. When a
/// directive is omitted, CSP falls back to default-src; copy that source list
/// before making the directive explicit so other destinations keep their
/// prior behavior.
fn patch_csp_connect_source(html: &mut String, source: &str) {
    patch_csp_source(html, "connect-src", source);
}

/// Allow Niva's token-scoped loopback filesystem URL for non-executable page
/// resources while keeping script and object policies untouched.
fn patch_csp_resource_source(html: &mut String, directive: &str, source: &str) {
    patch_csp_source(html, directive, source);
}

fn patch_csp_source(html: &mut String, directive: &str, source: &str) {
    let ranges = csp_meta_ranges(html);
    for (_, _, content_start, content_end) in ranges.into_iter().rev() {
        let original = html[content_start..content_end].to_string();
        let updated = add_csp_source(&original, directive, source);
        if updated != original {
            html.replace_range(content_start..content_end, &updated);
        }
    }
}

fn csp_meta_ranges(html: &str) -> Vec<(usize, usize, usize, usize)> {
    let lower = html.to_ascii_lowercase();
    let mut cursor = 0;
    let mut ranges = Vec::new();
    let head_end = head_end(html);
    while cursor < head_end
        && let Some(relative) = lower[cursor..head_end].find("<meta")
    {
        let start = cursor + relative;
        let Some(end_relative) = lower[start..head_end].find('>') else {
            break;
        };
        let end = start + end_relative + 1;
        let tag = &html[start..end];
        if let Some((content_start, content_end)) = csp_content_range(tag) {
            ranges.push((start, end, start + content_start, start + content_end));
        }
        cursor = end;
    }
    ranges
}

fn csp_content_range(tag: &str) -> Option<(usize, usize)> {
    let mut cursor = "<meta".len();
    let mut http_equiv = None::<String>;
    let mut content = None;
    while cursor < tag.len() {
        let bytes = tag.as_bytes();
        while cursor < tag.len() && (bytes[cursor].is_ascii_whitespace() || bytes[cursor] == b'/') {
            cursor += 1;
        }
        if cursor >= tag.len() || bytes[cursor] == b'>' {
            break;
        }
        let name_start = cursor;
        while cursor < tag.len()
            && !bytes[cursor].is_ascii_whitespace()
            && !matches!(bytes[cursor], b'=' | b'/' | b'>')
        {
            cursor += 1;
        }
        if cursor == name_start {
            cursor += 1;
            continue;
        }
        let name = tag[name_start..cursor].to_ascii_lowercase();
        while cursor < tag.len() && bytes[cursor].is_ascii_whitespace() {
            cursor += 1;
        }
        if cursor >= tag.len() || bytes[cursor] != b'=' {
            continue;
        }
        cursor += 1;
        while cursor < tag.len() && bytes[cursor].is_ascii_whitespace() {
            cursor += 1;
        }
        if cursor >= tag.len() {
            break;
        }
        let quote = if matches!(bytes[cursor], b'\'' | b'"') {
            let quote = bytes[cursor];
            cursor += 1;
            Some(quote)
        } else {
            None
        };
        let value_start = cursor;
        while cursor < tag.len()
            && if let Some(quote) = quote {
                bytes[cursor] != quote
            } else {
                !bytes[cursor].is_ascii_whitespace() && bytes[cursor] != b'>'
            }
        {
            cursor += 1;
        }
        let value_end = cursor;
        let value = &tag[value_start..value_end];
        if name == "http-equiv" {
            http_equiv = Some(value.to_string());
        } else if name == "content" {
            content = Some((value_start, value_end));
        }
        if quote.is_some() && cursor < tag.len() {
            cursor += 1;
        }
    }

    http_equiv
        .is_some_and(|value| value.eq_ignore_ascii_case("content-security-policy"))
        .then_some(content)
        .flatten()
}

fn tag_attribute_range(tag: &str, target_name: &str) -> Option<(usize, usize)> {
    let bytes = tag.as_bytes();
    let mut cursor = "<script".len();
    while cursor < tag.len() {
        while cursor < tag.len() && (bytes[cursor].is_ascii_whitespace() || bytes[cursor] == b'/') {
            cursor += 1;
        }
        if cursor >= tag.len() || bytes[cursor] == b'>' {
            break;
        }
        let name_start = cursor;
        while cursor < tag.len()
            && !bytes[cursor].is_ascii_whitespace()
            && !matches!(bytes[cursor], b'=' | b'/' | b'>')
        {
            cursor += 1;
        }
        if cursor == name_start {
            cursor += 1;
            continue;
        }
        let name = &tag[name_start..cursor];
        while cursor < tag.len() && bytes[cursor].is_ascii_whitespace() {
            cursor += 1;
        }
        if cursor >= tag.len() || bytes[cursor] != b'=' {
            continue;
        }
        cursor += 1;
        while cursor < tag.len() && bytes[cursor].is_ascii_whitespace() {
            cursor += 1;
        }
        if cursor >= tag.len() {
            break;
        }
        let quote = if matches!(bytes[cursor], b'\'' | b'"') {
            let quote = bytes[cursor];
            cursor += 1;
            Some(quote)
        } else {
            None
        };
        let value_start = cursor;
        while cursor < tag.len()
            && if let Some(quote) = quote {
                bytes[cursor] != quote
            } else {
                !bytes[cursor].is_ascii_whitespace() && bytes[cursor] != b'>'
            }
        {
            cursor += 1;
        }
        let value_end = cursor;
        if name.eq_ignore_ascii_case(target_name) {
            return Some((value_start, value_end));
        }
        if quote.is_some() && cursor < tag.len() {
            cursor += 1;
        }
    }
    None
}

fn add_script_nonce(policy: &str, nonce: &str) -> String {
    let mut directives = policy
        .split(';')
        .map(str::trim)
        .filter(|directive| !directive.is_empty())
        .map(str::to_string)
        .collect::<Vec<_>>();
    let target_directive = if directives.iter().any(|directive| {
        directive
            .split_whitespace()
            .next()
            .is_some_and(|name| name.eq_ignore_ascii_case("script-src-elem"))
    }) {
        "script-src-elem"
    } else if directives.iter().any(|directive| {
        directive
            .split_whitespace()
            .next()
            .is_some_and(|name| name.eq_ignore_ascii_case("script-src"))
    }) {
        "script-src"
    } else if directives.iter().any(|directive| {
        directive
            .split_whitespace()
            .next()
            .is_some_and(|name| name.eq_ignore_ascii_case("default-src"))
    }) {
        "script-src"
    } else {
        return policy.to_string();
    };
    let nonce_source = format!("'nonce-{nonce}'");
    if let Some(directive) = directives.iter_mut().find(|directive| {
        directive
            .split_whitespace()
            .next()
            .is_some_and(|name| name.eq_ignore_ascii_case(target_directive))
    }) {
        let mut sources = directive
            .split_whitespace()
            .skip(1)
            .filter(|source| !source.eq_ignore_ascii_case("'none'"))
            .map(str::to_string)
            .collect::<Vec<_>>();
        if !sources.iter().any(|source| source == &nonce_source) {
            sources.push(nonce_source);
        }
        *directive = format!("{target_directive} {}", sources.join(" "));
    } else {
        let mut sources = directives
            .iter()
            .find_map(|directive| {
                let mut words = directive.split_whitespace();
                words
                    .next()
                    .is_some_and(|name| name.eq_ignore_ascii_case("default-src"))
                    .then(|| {
                        words
                            .filter(|source| !source.eq_ignore_ascii_case("'none'"))
                            .map(str::to_string)
                            .collect::<Vec<_>>()
                    })
            })
            .unwrap_or_default();
        sources.push(nonce_source);
        directives.push(format!("script-src {}", sources.join(" ")));
    }
    directives.join("; ")
}

fn add_csp_source(policy: &str, target_directive: &str, source: &str) -> String {
    let mut directives = policy
        .split(';')
        .map(str::trim)
        .filter(|directive| !directive.is_empty())
        .map(str::to_string)
        .collect::<Vec<_>>();
    let mut found = false;
    for directive in &mut directives {
        let mut words = directive.split_whitespace();
        let Some(name) = words.next() else {
            continue;
        };
        if name.eq_ignore_ascii_case(target_directive) {
            found = true;
            let mut sources = words
                .filter(|source| !source.eq_ignore_ascii_case("'none'"))
                .map(str::to_string)
                .collect::<Vec<_>>();
            if !sources.iter().any(|item| item == source) {
                sources.push(source.to_string());
            }
            *directive = format!("{target_directive} {}", sources.join(" "));
        }
    }
    if !found {
        let inherited = directives.iter().find_map(|directive| {
            let mut words = directive.split_whitespace();
            words
                .next()
                .is_some_and(|name| name.eq_ignore_ascii_case("default-src"))
                .then(|| {
                    words
                        .filter(|source| !source.eq_ignore_ascii_case("'none'"))
                        .map(str::to_string)
                        .collect::<Vec<_>>()
                })
        });
        let Some(mut sources) = inherited else {
            // With neither the target directive nor default-src, the policy
            // does not restrict this resource type. Adding a narrow directive
            // would break existing destinations without improving Niva access.
            return policy.to_string();
        };
        if !sources.iter().any(|item| item == source) {
            sources.push(source.to_string());
        }
        directives.push(format!("{target_directive} {}", sources.join(" ")));
    }
    directives.join("; ")
}

#[cfg(test)]
mod tests {
    use crate::app::resource_manager::ResourceRead;
    use std::{collections::HashMap, path::Path, sync::Mutex};

    use tao::window::Icon;

    use super::*;

    #[derive(Debug)]
    struct TestResources(HashMap<String, Vec<u8>>);

    impl ResourceManager for TestResources {
        fn exists(&self, path: &str) -> bool {
            self.0.contains_key(path)
        }

        fn load(&self, path: &str) -> Result<Vec<u8>> {
            self.0
                .get(path)
                .cloned()
                .ok_or_else(|| anyhow::anyhow!("missing"))
        }

        fn extract(&self, _from: &str, _to: &Path) -> Result<()> {
            unreachable!()
        }

        fn load_icon(&self, _path: &str) -> Result<Icon> {
            unreachable!()
        }
    }

    fn resources() -> TestResources {
        TestResources(HashMap::from([
            ("index.html".into(), b"<html><head><meta http-equiv=\"Content-Security-Policy\" content=\"default-src 'self'; script-src 'self'\"></head></html>".to_vec()),
            ("asset.js".into(), b"console.log('ok')".to_vec()),
            ("__niva_runtime/dist/bootstrap.js".into(), b"window.__adapter_loaded=true;".to_vec()),
            ("__niva_runtime/dist/esm/path.mjs".into(), b"export default {};".to_vec()),
        ]))
    }

    fn get_request(uri: &str, headers: &[(&str, &str)]) -> Request<Vec<u8>> {
        request(Method::GET, uri, headers)
    }

    fn head_request(uri: &str, headers: &[(&str, &str)]) -> Request<Vec<u8>> {
        request(Method::HEAD, uri, headers)
    }

    fn request(method: Method, uri: &str, headers: &[(&str, &str)]) -> Request<Vec<u8>> {
        let mut builder = Request::builder().method(method).uri(uri);
        for (name, value) in headers {
            builder = builder.header(*name, *value);
        }
        builder.body(Vec::new()).unwrap()
    }

    #[derive(Debug)]
    struct SyntheticLargeResource {
        size: u64,
        reads: Mutex<Vec<(u64, usize)>>,
    }

    impl ResourceManager for SyntheticLargeResource {
        fn exists(&self, path: &str) -> bool {
            path == "large.bin"
        }

        fn load(&self, _path: &str) -> Result<Vec<u8>> {
            Err(anyhow::anyhow!("full read is forbidden in this fixture"))
        }

        fn resource_size(&self, path: &str) -> Result<Option<u64>> {
            Ok(self.exists(path).then_some(self.size))
        }

        fn read_range(&self, path: &str, offset: u64, length: usize) -> Result<ResourceRead> {
            if path != "large.bin" || offset > self.size {
                return Err(anyhow::anyhow!("bad synthetic range"));
            }
            self.reads.lock().unwrap().push((offset, length));
            Ok(ResourceRead {
                total_len: self.size,
                bytes: vec![b'x'; length],
            })
        }

        fn extract(&self, _from: &str, _to: &Path) -> Result<()> {
            unreachable!()
        }

        fn load_icon(&self, _path: &str) -> Result<Icon> {
            unreachable!()
        }
    }

    #[test]
    fn protocol_origin_is_platform_specific() {
        #[cfg(target_os = "windows")]
        assert_eq!(platform_origin(), WINDOWS_ORIGIN);
        #[cfg(not(target_os = "windows"))]
        assert_eq!(platform_origin(), MACOS_ORIGIN);
    }

    #[test]
    fn serves_root_html_and_preserves_csp_sources_while_allowing_ws() {
        let response = response_for_request(
            &get_request("niva://app/", &[("accept", "text/html")]),
            &resources(),
            None,
            43123,
        );
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response.headers()[CONTENT_TYPE], "text/html");
        let html = String::from_utf8(response.into_body()).unwrap();
        assert!(html.contains("script-src 'self'"));
        assert!(html.contains("connect-src 'self' ws://127.0.0.1:43123 http://127.0.0.1:43123"));
    }

    #[test]
    fn serves_assets_and_refuses_internal_routes_and_bad_authority() {
        let resources = resources();
        let asset = response_for_request(
            &get_request("niva://app/asset.js", &[]),
            &resources,
            None,
            43123,
        );
        assert_eq!(asset.status(), StatusCode::OK);
        assert_eq!(asset.body(), b"console.log('ok')");

        for internal_path in [
            "__niva_fs/secret/file.txt",
            "__niva_ws",
            "__niva_other/asset.js",
            "__niva_runtime",
        ] {
            let response = response_for_request(
                &get_request(&format!("niva://app/{internal_path}"), &[]),
                &resources,
                None,
                43123,
            );
            assert_eq!(response.status(), StatusCode::NOT_FOUND, "{internal_path}");
        }

        let bad_host = response_for_request(
            &get_request("niva://other/asset.js", &[]),
            &resources,
            None,
            43123,
        );
        assert_eq!(bad_host.status(), StatusCode::NOT_FOUND);
    }

    #[test]
    fn serves_head_and_single_byte_ranges_with_exact_headers() {
        let resources = resources();
        let partial = response_for_request(
            &get_request("niva://app/asset.js", &[("range", "bytes=0-3")]),
            &resources,
            None,
            43123,
        );
        assert_eq!(partial.status(), StatusCode::PARTIAL_CONTENT);
        assert_eq!(partial.headers()[CONTENT_RANGE], "bytes 0-3/17");
        assert_eq!(partial.body(), b"cons");

        let head = response_for_request(
            &head_request("niva://app/asset.js", &[]),
            &resources,
            None,
            43123,
        );
        assert_eq!(head.status(), StatusCode::OK);
        assert_eq!(head.headers()[CONTENT_LENGTH], "17");
        assert!(head.body().is_empty());

        let invalid = response_for_request(
            &get_request("niva://app/asset.js", &[("range", "bytes=17-")]),
            &resources,
            None,
            43123,
        );
        assert_eq!(invalid.status(), StatusCode::RANGE_NOT_SATISFIABLE);
        assert_eq!(invalid.headers()[CONTENT_RANGE], "bytes */17");
    }

    #[test]
    fn large_resources_require_range_and_range_responses_stay_bounded() {
        let resources = SyntheticLargeResource {
            size: MAX_RESPONSE_BYTES as u64 + 4096,
            reads: Mutex::new(Vec::new()),
        };
        let full = response_for_request(
            &get_request("niva://app/large.bin", &[]),
            &resources,
            None,
            43123,
        );
        assert_eq!(full.status(), StatusCode::PAYLOAD_TOO_LARGE);
        assert!(resources.reads.lock().unwrap().is_empty());

        let metadata = response_for_request(
            &head_request("niva://app/large.bin", &[]),
            &resources,
            None,
            43123,
        );
        assert_eq!(metadata.status(), StatusCode::OK);
        assert_eq!(metadata.headers()[CONTENT_LENGTH], "33558528");
        assert!(metadata.body().is_empty());
        assert!(resources.reads.lock().unwrap().is_empty());

        let head = response_for_request(
            &head_request("niva://app/large.bin", &[("range", "bytes=0-999999999")]),
            &resources,
            None,
            43123,
        );
        assert_eq!(head.status(), StatusCode::PARTIAL_CONTENT);
        assert_eq!(head.headers()[CONTENT_RANGE], "bytes 0-33554431/33558528");
        assert_eq!(head.headers()[CONTENT_LENGTH], "33554432");
        assert!(head.body().is_empty());
        assert!(resources.reads.lock().unwrap().is_empty());

        let small = response_for_request(
            &get_request("niva://app/large.bin", &[("range", "bytes=4-7")]),
            &resources,
            None,
            43123,
        );
        assert_eq!(small.status(), StatusCode::PARTIAL_CONTENT);
        assert_eq!(small.headers()[CONTENT_RANGE], "bytes 4-7/33558528");
        assert_eq!(small.body(), b"xxxx");
        assert_eq!(&*resources.reads.lock().unwrap(), &[(4, 4)]);
    }

    #[test]
    fn rejects_encoded_traversal_and_rewrites_only_document_navigation() {
        assert!(asset_path(&"niva://app/%2e%2e/outside".parse().unwrap()).is_err());
        assert!(asset_path(&"niva://app/folder%2f..%2foutside".parse().unwrap()).is_err());

        let response = response_for_request(
            &get_request(
                "niva://app/",
                &[("sec-fetch-mode", "cors"), ("accept", "text/html")],
            ),
            &resources(),
            None,
            43123,
        );
        assert_eq!(response.status(), StatusCode::OK);
        let html = String::from_utf8(response.into_body()).unwrap();
        assert!(!html.contains("connect-src"));
    }

    #[test]
    fn runtime_importmaps_and_facades_are_served_but_bootstrap_is_private() {
        let compat = NodeCompat::new(true);
        let resources = resources();
        let page = response_for_request(
            &get_request("niva://app/", &[("accept", "text/html")]),
            &resources,
            Some(&compat),
            43123,
        );
        assert_eq!(page.status(), StatusCode::OK);
        let html = String::from_utf8(page.into_body()).unwrap();
        assert!(!html.contains("bootstrap.js"));
        assert!(html.contains("/__niva_runtime/esm/path.mjs"));

        for path in [
            "bootstrap.js",
            "__bootstrap__/bootstrap.js",
            "dist/bootstrap.js",
        ] {
            let response = response_for_request(
                &get_request(&format!("niva://app/__niva_runtime/{path}"), &[]),
                &resources,
                Some(&compat),
                43123,
            );
            assert_eq!(response.status(), StatusCode::NOT_FOUND);
        }

        let selected = response_for_request(
            &get_request("niva://app/__niva_runtime/esm/path.mjs", &[]),
            &resources,
            Some(&compat),
            43123,
        );
        assert_eq!(selected.status(), StatusCode::OK);
        assert!(!selected.body().is_empty());

        let disabled = response_for_request(
            &get_request("niva://app/__niva_runtime/esm/fs.mjs", &[]),
            &resources,
            Some(&NodeCompat::new(false)),
            43123,
        );
        assert_eq!(disabled.status(), StatusCode::NOT_FOUND);
    }

    #[test]
    fn csp_template_adds_bridge_and_filesystem_origins_and_nonces_only_niva_scripts() {
        let compat = NodeCompat::new(true);
        let mut html = String::from(
            "<!doctype html><html><head><meta charset=\"UTF-8\"><meta http-equiv=\"Content-Security-Policy\" content=\"default-src 'self'; script-src 'self'; style-src 'self'; img-src 'self' data:; font-src 'self'; connect-src 'self'; object-src 'none'; base-uri 'self'; form-action 'self'\"><script type=\"module\" src=\"./index.js\"></script></head><body></body></html>",
        );
        html = String::from_utf8(compat.rewrite_html(html.as_bytes()).unwrap()).unwrap();
        assert!(
            html.find(NODE_COMPAT_MARKER).unwrap() < html.find("Content-Security-Policy").unwrap()
        );

        patch_document_csp(&mut html, 43123).unwrap();

        let policy_range = csp_meta_ranges(&html)[0];
        let policy = &html[policy_range.2..policy_range.3];
        for source in ["ws://127.0.0.1:43123", "http://127.0.0.1:43123"] {
            assert!(policy.contains(source), "missing {source} in {policy}");
        }
        for directive in ["img-src", "media-src", "font-src", "style-src"] {
            let sources = policy
                .split(';')
                .find_map(|entry| {
                    let (name, sources) = entry.trim().split_once(' ')?;
                    name.eq_ignore_ascii_case(directive).then_some(sources)
                })
                .unwrap_or_else(|| panic!("missing {directive} in {policy}"));
            assert!(sources.contains("'self'"), "{directive}: {sources}");
            assert!(
                sources.contains("http://127.0.0.1:43123"),
                "{directive}: {sources}"
            );
        }
        assert!(!policy.contains("unsafe-inline"));

        let map_start = html.find("<script type=\"importmap\"").unwrap();
        let map_end = html[map_start..].find('>').unwrap() + map_start + 1;
        let map_tag = &html[map_start..map_end];
        let (map_nonce_start, map_nonce_end) = tag_attribute_range(map_tag, "nonce").unwrap();
        let nonce = &map_tag[map_nonce_start..map_nonce_end];
        assert_eq!(nonce.len(), 32);
        assert!(policy.contains(&format!("'nonce-{nonce}'")));

        assert_eq!(nonce, NodeCompat::csp_nonce().unwrap());
        assert!(!html.contains("bootstrap.js"));
        assert!(html.find("Content-Security-Policy").unwrap() < map_start);
        assert!(map_start < html.find("src=\"./index.js\"").unwrap());
        let once = html.clone();
        patch_document_csp(&mut html, 43123).unwrap();
        assert_eq!(html, once);
    }

    #[test]
    fn commonjs_factory_nonce_is_allowed_without_esm_injection() {
        let mut html = String::from(
            "<html><head><meta http-equiv=\"Content-Security-Policy\" content=\"default-src 'self'; script-src 'self'\"><script src=\"app.js\"></script></head></html>",
        );
        patch_document_csp(&mut html, 43123).unwrap();
        assert!(html.contains(&format!("'nonce-{}'", NodeCompat::csp_nonce().unwrap())));
        assert!(!html.contains("unsafe-eval"));
        assert!(!html.contains("unsafe-inline"));
        assert!(!html.contains("type=\"importmap\""));
        assert!(html.contains("<script src=\"app.js\"></script>"));
    }

    #[test]
    fn csp_patch_does_not_force_a_policy_or_nonce_on_pages_without_csp() {
        let mut html = String::from(
            "<html><head><script type=\"importmap\">{}</script><script type=\"module\" src=\"./index.js\"></script></head></html>",
        );
        let original = html.clone();

        patch_document_csp(&mut html, 43123).unwrap();

        assert_eq!(html, original);
        assert!(
            !html
                .to_ascii_lowercase()
                .contains("content-security-policy")
        );
        assert!(html.contains("<script type=\"importmap\">"));
    }

    #[test]
    fn csp_patch_fails_closed_for_malformed_node_compat_blocks() {
        let csp = "<meta http-equiv=\"Content-Security-Policy\" content=\"script-src 'self'\">";
        for injected in [
            "<!-- niva-runtime-importmap -->",
            "<!-- niva-runtime-importmap --><script type=\"importmap\">{}",
            "<!-- niva-runtime-importmap --><script src=\"/other.js\"></script>",
        ] {
            let mut html = format!("<html><head>{csp}{injected}</head></html>");
            assert!(patch_document_csp(&mut html, 43123).is_err(), "{injected}");
        }
    }

    #[test]
    fn csp_nonce_does_not_change_author_importmaps_outside_node_compat_block() {
        let mut html = String::from(
            "<html><head><meta http-equiv=\"Content-Security-Policy\" content=\"script-src 'self'\"><script type=\"importmap\">{\"imports\":{\"author\":\"./author.js\"}}</script><!-- niva-runtime-importmap --><script type=\"importmap\">{\"imports\":{\"path\":\"/__niva_runtime/esm/path.mjs\"}}</script></head></html>",
        );

        patch_document_csp(&mut html, 43123).unwrap();

        assert!(html.contains(
            "<script type=\"importmap\">{\"imports\":{\"author\":\"./author.js\"}}</script>"
        ));
        assert!(html.contains("<script type=\"importmap\" nonce=\""));
        let author_map = html.find("./author.js").unwrap();
        let author_start = html[..author_map].rfind("<script").unwrap();
        let author_end = html[author_map..].find('>').unwrap() + author_map + 1;
        assert!(!html[author_start..author_end].contains("nonce="));
    }

    #[test]
    fn csp_nonce_extends_script_element_policy_without_unsafe_inline() {
        let policy = add_script_nonce(
            "default-src 'self'; script-src 'none'; script-src-elem 'none'; img-src data:",
            "0123456789abcdef0123456789abcdef",
        );
        assert!(policy.contains("script-src 'none'"));
        assert!(policy.contains("script-src-elem 'nonce-0123456789abcdef0123456789abcdef'"));
        assert!(!policy.contains("unsafe-inline"));
    }

    #[test]
    fn adding_connect_source_keeps_default_sources_and_avoids_duplicates() {
        let source = "ws://127.0.0.1:43123";
        let once = add_csp_source("default-src 'self'; img-src data:", "connect-src", source);
        assert!(once.contains("connect-src 'self' ws://127.0.0.1:43123"));
        assert_eq!(add_csp_source(&once, "connect-src", source), once);
        assert_eq!(
            add_csp_source("connect-src 'none'", "connect-src", source),
            "connect-src ws://127.0.0.1:43123"
        );
        assert_eq!(
            add_csp_source("script-src 'self'", "connect-src", source),
            "script-src 'self'"
        );
    }
}
