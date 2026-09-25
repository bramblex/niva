use std::{
    collections::HashMap,
    fs::File,
    io::{Read, Write},
    path::Path,
    sync::{Arc, mpsc},
    thread,
    time::Duration,
};

use anyhow::{Result, anyhow};
use smol::{net::TcpListener, prelude::*};

use super::{
    NivaApp,
    node_compat::NodeCompat,
    resource_manager::ResourceManager,
    utils::{ArcMut, error_page_html},
};
use crate::{lock, log_err};

/// Hand-written async loopback server hosting the API WebSocket and the
/// authenticated `__niva_fs` route. Packaged static assets use Wry's
/// asynchronous per-app custom protocol; ordinary HTTP static routes are reserved
/// for explicit debug launches.
///
/// Only GET + `Connection: close` is implemented; that is all the webview
/// needs. No tokio/axum/hyper: this crate plus tungstenite is everything.
pub struct NivaHttpServer {
    pub port: u16,
    _state: Arc<ServerState>,
    _driver: thread::JoinHandle<()>,
}

struct ServerState {
    app: Arc<NivaApp>,
    resource: Arc<dyn ResourceManager>,
    port: u16,
    node_compat: Option<NodeCompat>,
    debug_static_enabled: bool,
}

impl NivaHttpServer {
    pub fn start(app: &Arc<NivaApp>) -> Result<Arc<Self>> {
        let node_compat = Some(NodeCompat::new(app.launch_info.options.inject_esm));
        let listener = smol::block_on(TcpListener::bind("127.0.0.1:0"))?;
        let port = listener.local_addr()?.port();

        let state = Arc::new(ServerState {
            app: app.clone(),
            resource: app.resource(),
            port,
            node_compat,
            debug_static_enabled: app
                .launch_info
                .arguments
                .has_explicit_debug_entry(&app.launch_info.options),
        });

        // Single driver thread pumps the accept loop and all static-file
        // tasks; WebSocket pumps run on smol's unblock pool.
        let driver_state = state.clone();
        let driver = thread::Builder::new()
            .name("niva-http".into())
            .spawn(move || smol::block_on(accept_loop(driver_state, listener)))
            .map_err(|err| anyhow!("Failed to spawn http thread: {err}"))?;

        Ok(Arc::new(Self {
            port,
            _state: state,
            _driver: driver,
        }))
    }

    pub fn base_url(&self) -> String {
        format!("http://127.0.0.1:{}", self.port)
    }
}

async fn accept_loop(state: Arc<ServerState>, listener: TcpListener) {
    loop {
        let (stream, _) = match listener.accept().await {
            Ok(pair) => pair,
            Err(_) => continue,
        };
        let state = state.clone();
        smol::spawn(async move {
            if let Err(err) = handle_conn(&state, stream).await {
                log_err!(format!("http conn failed: {err}"));
            }
        })
        .detach();
    }
}

const MAX_HEAD: usize = 32 * 1024;

struct HttpHead {
    method: String,
    target: String,
    headers: HashMap<String, String>,
}

/// Byte source that first drains already-read head bytes, then the stream.
/// Lets routing peek at the HTTP head while tungstenite still sees the full
/// handshake.
struct PrependReader<R> {
    head: std::io::Cursor<Vec<u8>>,
    inner: R,
}

impl<R: Read> Read for PrependReader<R> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        if self.head.position() < self.head.get_ref().len() as u64 {
            self.head.read(buf)
        } else {
            self.inner.read(buf)
        }
    }
}

impl<R: Write> Write for PrependReader<R> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.inner.write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.inner.flush()
    }
}

type WsStream = PrependReader<std::net::TcpStream>;

impl WsStream {
    fn set_read_timeout(&self, timeout: Option<Duration>) -> std::io::Result<()> {
        self.inner.set_read_timeout(timeout)
    }
}

/// Read bytes until the end of the HTTP head (`\r\n\r\n`).
/// Reads one byte at a time so no unread bytes are left buffered; the raw
/// head is returned for re-feeding (WebSocket handshake).
async fn read_head(stream: &mut smol::net::TcpStream) -> Result<(HttpHead, Vec<u8>)> {
    let mut raw: Vec<u8> = Vec::with_capacity(512);
    let mut byte = [0u8; 1];
    loop {
        if raw.len() >= MAX_HEAD {
            return Err(anyhow!("HTTP head too large"));
        }
        stream.read_exact(&mut byte).await?;
        raw.push(byte[0]);
        if raw.len() >= 4 && raw[raw.len() - 4..] == *b"\r\n\r\n" {
            break;
        }
    }
    let (head, _) = parse_head(&raw)?;
    Ok((head, raw))
}

fn parse_head(raw: &[u8]) -> Result<(HttpHead, Vec<u8>)> {
    let text = std::str::from_utf8(raw)?;
    let mut lines = text.split("\r\n");
    let request_line = lines.next().ok_or(anyhow!("Empty request"))?;
    let mut parts = request_line.split_whitespace();
    let method = parts.next().ok_or(anyhow!("Bad request line"))?.to_string();
    let target = parts.next().ok_or(anyhow!("Bad request line"))?.to_string();

    let mut headers = HashMap::new();
    for line in lines {
        if line.is_empty() {
            break;
        }
        if let Some((name, value)) = line.split_once(':') {
            let name = name.trim().to_lowercase();
            if headers.insert(name, value.trim().to_string()).is_some() {
                return Err(anyhow!("duplicate HTTP header"));
            }
        } else {
            return Err(anyhow!("malformed HTTP header"));
        }
    }
    Ok((
        HttpHead {
            method,
            target,
            headers,
        },
        Vec::new(),
    ))
}

fn split_target(target: &str) -> (&str, Option<&str>) {
    match target.split_once('?') {
        Some((path, query)) => (path, Some(query)),
        None => (target, None),
    }
}

fn query_value<'a>(query: &'a str, key: &str) -> Option<&'a str> {
    query.split('&').find_map(|pair| {
        let (k, v) = pair.split_once('=')?;
        (k == key).then_some(v)
    })
}

fn is_upgrade(headers: &HashMap<String, String>) -> bool {
    let upgrade = headers
        .get("upgrade")
        .map(|v| v.to_lowercase())
        .unwrap_or_default();
    let connection = headers
        .get("connection")
        .map(|v| v.to_lowercase())
        .unwrap_or_default();
    upgrade.contains("websocket") && connection.contains("upgrade")
}

/// True when the request is a browser document load (top-level page or
/// iframe), as opposed to JS fetching HTML as data via fetch()/XHR.
///
/// Detection order (header names are lowercased at parse):
/// 1. `Sec-Fetch-Mode: navigate` — Fetch Metadata, sent by Chromium/WebView2
///    and modern WebKit for real navigations. fetch()/XHR send
///    cors/no-cors/same-origin instead, so they never match.
/// 2. Fallback for engines without Fetch Metadata: `Accept` containing
///    `text/html`. Navigations always ask for HTML; fetch() defaults to
///    `*/*`, so an explicit text/html accept almost always means a document
///    load. (A page *could* fetch() with `Accept: text/html` by hand — that
///    corner is documented; such callers get the injected bytes too.)
///
/// Response rewriting (importmap injection for nodeCompat) only applies when
/// this returns true: data-fetched HTML must pass through byte-identical.
fn is_document_navigation(headers: &HashMap<String, String>) -> bool {
    if let Some(mode) = headers.get("sec-fetch-mode") {
        return mode.trim().to_lowercase() == "navigate";
    }
    headers
        .get("accept")
        .map(|v| v.to_lowercase().contains("text/html"))
        .unwrap_or(false)
}

const MAX_SYNC_BODY: usize = 16 * 1024 * 1024;

fn sync_body_length(headers: &HashMap<String, String>) -> Result<usize> {
    if headers.contains_key("transfer-encoding") {
        return Err(anyhow!("synchronous requests require Content-Length"));
    }
    let length = headers
        .get("content-length")
        .ok_or_else(|| anyhow!("missing length"))?;
    if length.is_empty() || !length.bytes().all(|b| b.is_ascii_digit()) {
        return Err(anyhow!("invalid length"));
    }
    let length: usize = length.parse()?;
    if length > MAX_SYNC_BODY {
        return Err(anyhow!("request too large"));
    }
    Ok(length)
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct SyncEnvelope {
    token: String,
    session_id: String,
    request: super::api_manager::ApiRequest,
}

async fn handle_sync(
    state: &Arc<ServerState>,
    stream: &mut smol::net::TcpStream,
    head: &HttpHead,
) -> Result<()> {
    let Ok(length) = sync_body_length(&head.headers) else {
        return write_response(
            stream,
            400,
            "Bad Request",
            "text/plain",
            b"invalid request framing",
        )
        .await;
    };
    let mut body = vec![0; length];
    let read = smol::future::or(
        async {
            stream
                .read_exact(&mut body)
                .await
                .map_err(anyhow::Error::from)
        },
        async {
            smol::Timer::after(Duration::from_secs(10)).await;
            Err(anyhow!("request timeout"))
        },
    )
    .await;
    if read.is_err() {
        return Ok(());
    }
    let Ok(envelope) = serde_json::from_slice::<SyncEnvelope>(&body) else {
        return write_response(stream, 400, "Bad Request", "text/plain", b"invalid request").await;
    };
    let window = state
        .app
        .window()
        .ok()
        .and_then(|windows| windows.get_window_by_token(&envelope.token).ok());
    let Some(window) = window else {
        return write_response(stream, 403, "Forbidden", "text/plain", b"unauthorized").await;
    };
    let Some(origin) = head.headers.get("origin").filter(|origin| {
        is_trusted_origin(window.trusted_ws_origin.as_deref(), Some(origin.as_str()))
    }) else {
        return write_response(stream, 403, "Forbidden", "text/plain", b"unauthorized").await;
    };
    enum SyncOutcome {
        Response(Result<String>),
        ClientDisconnected,
    }
    let api = state.app.api();
    let window_id = window.id;
    let session_id = envelope.session_id;
    let request = envelope.request;
    let outcome = smol::future::or(
        async move { SyncOutcome::Response(api.sync_call(window_id, &session_id, request).await) },
        async {
            // A synchronous XHR cannot heartbeat while JavaScript is blocked.
            // Watching the request socket lets navigation/close abort owned
            // work immediately without inventing a lease timeout.
            let mut extra = [0u8; 1];
            let _ = stream.read(&mut extra).await;
            SyncOutcome::ClientDisconnected
        },
    )
    .await;
    let response = match outcome {
        SyncOutcome::Response(response) => response?,
        SyncOutcome::ClientDisconnected => return Ok(()),
    };
    write_response_headers(stream, 200, "OK", "application/json", response.as_bytes(),
        &format!("Access-Control-Allow-Origin: {origin}\r\nVary: Origin\r\nCache-Control: no-store\r\nX-Content-Type-Options: nosniff\r\n")).await
}

async fn handle_conn(state: &Arc<ServerState>, mut stream: smol::net::TcpStream) -> Result<()> {
    let (head, raw_head) = read_head(&mut stream).await?;

    let expected_host = format!("127.0.0.1:{}", state.port);
    if head
        .headers
        .get("host")
        .is_none_or(|host| host != &expected_host)
    {
        return write_response(&mut stream, 403, "Forbidden", "text/plain", b"bad host").await;
    }

    if head.method == "POST" && head.target == "/__niva_sync" {
        return handle_sync(state, &mut stream, &head).await;
    }

    if head.method != "GET" {
        // Drain bounded Content-Length and chunked bodies before replying.
        // Closing with unread bytes can reset the peer before it receives 405.
        let drain = smol::future::or(drain_rejected_body(&mut stream, &head.headers), async {
            smol::Timer::after(Duration::from_secs(3)).await;
            Err(anyhow!("request body timeout"))
        })
        .await;
        if !matches!(drain, Ok(true)) {
            let (status, reason) = if matches!(drain, Ok(false)) {
                (413, "Payload Too Large")
            } else {
                (400, "Bad Request")
            };
            return write_response(&mut stream, status, reason, "text/plain", reason.as_bytes())
                .await;
        }
        return write_response(
            &mut stream,
            405,
            "Method Not Allowed",
            "text/plain",
            b"GET only",
        )
        .await;
    }

    let (path, query) = split_target(&head.target);

    if let Some(relative) = path.strip_prefix('/')
        && relative.starts_with("__niva_runtime/")
    {
        let Some(compat) = &state.node_compat else {
            return write_response(&mut stream, 404, "Not Found", "text/plain", b"not found").await;
        };
        if !compat.allows_asset(relative) {
            return write_response(&mut stream, 404, "Not Found", "text/plain", b"not found").await;
        }
        return match NodeCompat::embedded_asset(relative)
            .ok_or_else(|| anyhow!("missing embedded asset"))
        {
            Ok(content) => {
                write_response_headers(
                    &mut stream, 200, "OK", "text/javascript; charset=utf-8", &content,
                    "Access-Control-Allow-Origin: *\r\nX-Content-Type-Options: nosniff\r\nCache-Control: no-store\r\n",
                ).await
            }
            Err(_) => {
                write_response(&mut stream, 404, "Not Found", "text/plain", b"not found").await
            }
        };
    }

    if path == "/__niva_ws" {
        if !is_upgrade(&head.headers) {
            return write_response(
                &mut stream,
                400,
                "Bad Request",
                "text/plain",
                b"WebSocket upgrade required",
            )
            .await;
        }
        if query.and_then(|q| query_value(q, "token")).is_none() {
            return write_response(
                &mut stream,
                403,
                "Forbidden",
                "text/plain",
                b"missing token",
            )
            .await;
        }
        // Hand the stream to a blocking pump thread with the consumed head
        // bytes fed back, so tungstenite sees the complete handshake.
        // NOTE: the fd comes back in nonblocking mode from the async
        // runtime; tungstenite's blocking handshake needs blocking mode.
        let std_stream = {
            let arc: Arc<smol::Async<std::net::TcpStream>> = stream.into();
            let std_stream = arc.get_ref().try_clone()?;
            std_stream.set_nonblocking(false)?;
            std_stream
        };
        let replay = PrependReader {
            head: std::io::Cursor::new(raw_head),
            inner: std_stream,
        };
        let state = state.clone();
        smol::unblock(move || ws_pump(state, replay)).await;
        return Ok(());
    }

    if let Some(rest) = path.strip_prefix("/__niva_fs/") {
        let Some((window_token, rest)) = rest.split_once('/') else {
            return write_response(
                &mut stream,
                403,
                "Forbidden",
                "text/plain",
                b"bad file credential",
            )
            .await;
        };
        let window = state
            .app
            .window()
            .ok()
            .and_then(|windows| windows.get_window_by_token(window_token).ok());
        let Some(window) = window else {
            return write_response(
                &mut stream,
                403,
                "Forbidden",
                "text/plain",
                b"bad file credential",
            )
            .await;
        };
        // A browser page at the exact origin of the credential's native
        // window may fetch this bearer URL. Other origins receive no CORS
        // permission, even if they somehow learn the URL.
        let cors_origin = head.headers.get("origin").filter(|origin| {
            is_trusted_origin(window.trusted_ws_origin.as_deref(), Some(origin.as_str()))
        });
        let rest = match super::resource_manager::percent_decode_path(rest) {
            Ok(rest) => rest,
            Err(_) => {
                return write_response(
                    &mut stream,
                    400,
                    "Bad Request",
                    "text/plain",
                    b"bad file path",
                )
                .await;
            }
        };
        let rest = rest.as_str();
        // Joined URLs may carry a leading slash before a Windows drive
        // letter (`/__niva_fs//C:/...`); strip it in that case only.
        let bytes = rest.as_bytes();
        let rest = if bytes.len() > 3
            && bytes[0] == b'/'
            && bytes[1].is_ascii_alphabetic()
            && bytes[2] == b':'
        {
            &rest[1..]
        } else {
            rest
        };
        #[cfg(target_os = "windows")]
        let file_path = rest.to_string();
        #[cfg(not(target_os = "windows"))]
        let file_path = format!("/{rest}");
        let load_path = file_path.clone();
        return match smol::unblock(move || read_limited_regular_file(Path::new(&load_path))).await {
            Ok(content) => {
                let mime = mime_guess::from_path(&file_path)
                    .first()
                    .map(|mime| mime.to_string())
                    .unwrap_or_else(|| "application/octet-stream".to_string());
                let mut extra_headers = String::from(
                    "Content-Security-Policy: sandbox\r\nX-Content-Type-Options: nosniff\r\n",
                );
                if let Some(origin) = cors_origin {
                    extra_headers.push_str(&format!(
                        "Access-Control-Allow-Origin: {origin}\r\nVary: Origin\r\n"
                    ));
                }
                write_response_headers(&mut stream, 200, "OK", &mime, &content, &extra_headers)
                    .await
            }
            Err(err) => {
                // The request path embeds the per-window credential.
                crate::niva_log!(
                    crate::app::logging::Level::Warn,
                    "fs response failed: {:?}",
                    err.kind()
                );
                let (_, body) = error_page_html(404, "Not Found", path, &err.to_string());
                write_response(
                    &mut stream,
                    404,
                    "Not Found",
                    "text/html; charset=utf-8",
                    &body,
                )
                .await
            }
        };
    }

    if !state.debug_static_enabled {
        return write_response(&mut stream, 404, "Not Found", "text/plain", b"not found").await;
    }

    // Directory paths serve index.html.
    let mut routed = path.to_string();
    if routed.ends_with('/') {
        routed += "index.html";
    }
    let rel = routed.strip_prefix('/').unwrap_or("index.html");

    match state.resource.load(rel) {
        Ok(content) => {
            let mime = mime_guess::from_path(rel)
                .first()
                .map(|mime| mime.to_string())
                .unwrap_or_else(|| "application/octet-stream".to_string());
            let content = if mime == "text/html" && is_document_navigation(&head.headers) {
                let content = if let Some(compat) = &state.node_compat {
                    match compat.rewrite_html(&content) {
                        Ok(content) => content,
                        Err(_) => {
                            return write_response(
                                &mut stream,
                                500,
                                "Internal Server Error",
                                "text/plain",
                                b"Unable to prepare local page",
                            )
                            .await;
                        }
                    }
                } else {
                    content
                };
                match String::from_utf8(content) {
                    Ok(mut html) => {
                        if super::custom_protocol::patch_document_csp(&mut html, state.port)
                            .is_err()
                        {
                            return write_response(
                                &mut stream,
                                500,
                                "Internal Server Error",
                                "text/plain",
                                b"Unable to prepare local page security policy",
                            )
                            .await;
                        }
                        html.into_bytes()
                    }
                    Err(_) if state.node_compat.is_some() => {
                        return write_response(
                            &mut stream,
                            500,
                            "Internal Server Error",
                            "text/plain",
                            b"NodeCompat requires UTF-8 HTML",
                        )
                        .await;
                    }
                    Err(error) => error.into_bytes(),
                }
            } else {
                content
            };
            write_response(&mut stream, 200, "OK", &mime, &content).await
        }
        Err(err) => {
            crate::niva_log!(
                crate::app::logging::Level::Warn,
                "static {path} -> 404: {err}"
            );
            let (_, body) = error_page_html(404, "Not Found", path, &err.to_string());
            write_response(
                &mut stream,
                404,
                "Not Found",
                "text/html; charset=utf-8",
                &body,
            )
            .await
        }
    }
}

/// Consume a rejected request body up to 1 MiB. `false` means it exceeded
/// the cap; malformed framing returns an error. This runs under a 3s deadline.
async fn drain_rejected_body<R: smol::io::AsyncRead + Unpin>(
    reader: &mut R,
    headers: &HashMap<String, String>,
) -> Result<bool> {
    const MAX_BODY: usize = 1024 * 1024;
    if headers.get("transfer-encoding").is_some_and(|value| {
        value
            .split(',')
            .any(|item| item.trim().eq_ignore_ascii_case("chunked"))
    }) {
        let mut total = 0usize;
        loop {
            let line = read_rejected_body_line(reader, 128).await?;
            let size_text = std::str::from_utf8(&line)?
                .split(';')
                .next()
                .unwrap_or_default();
            let size = usize::from_str_radix(size_text.trim(), 16)?;
            if size > MAX_BODY - total {
                return Ok(false);
            }
            if size == 0 {
                let mut trailer_bytes = 0usize;
                loop {
                    let trailer = read_rejected_body_line(reader, 1024).await?;
                    trailer_bytes += trailer.len() + 2;
                    if trailer_bytes > 8192 {
                        return Err(anyhow!("request trailers exceed 8 KiB"));
                    }
                    if trailer.is_empty() {
                        return Ok(true);
                    }
                }
            }
            drain_rejected_bytes(reader, size).await?;
            total += size;
            let mut ending = [0u8; 2];
            reader.read_exact(&mut ending).await?;
            if ending != *b"\r\n" {
                return Err(anyhow!("malformed chunk ending"));
            }
        }
    }
    if let Some(raw) = headers.get("content-length") {
        let length: usize = raw.parse()?;
        if length > MAX_BODY {
            return Ok(false);
        }
        drain_rejected_bytes(reader, length).await?;
    }
    Ok(true)
}

async fn drain_rejected_bytes<R: smol::io::AsyncRead + Unpin>(
    reader: &mut R,
    mut length: usize,
) -> Result<()> {
    let mut buffer = [0u8; 8192];
    while length > 0 {
        let chunk_len = length.min(buffer.len());
        reader.read_exact(&mut buffer[..chunk_len]).await?;
        length -= chunk_len;
    }
    Ok(())
}

async fn read_rejected_body_line<R: smol::io::AsyncRead + Unpin>(
    reader: &mut R,
    limit: usize,
) -> Result<Vec<u8>> {
    let mut line = Vec::new();
    loop {
        let mut byte = [0u8; 1];
        reader.read_exact(&mut byte).await?;
        line.push(byte[0]);
        if line.len() > limit + 2 {
            return Err(anyhow!("request body line exceeds limit"));
        }
        if line.ends_with(b"\r\n") {
            line.truncate(line.len() - 2);
            return Ok(line);
        }
    }
}

const MAX_FILE_RESPONSE: u64 = 32 * 1024 * 1024;

fn read_limited_regular_file(path: &Path) -> std::io::Result<Vec<u8>> {
    let file = File::open(path)?;
    let metadata = file.metadata()?;
    if !metadata.is_file() || metadata.len() > MAX_FILE_RESPONSE {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "file unavailable",
        ));
    }
    let mut content = Vec::with_capacity(metadata.len() as usize);
    file.take(MAX_FILE_RESPONSE + 1).read_to_end(&mut content)?;
    if content.len() as u64 > MAX_FILE_RESPONSE {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "file too large",
        ));
    }
    Ok(content)
}

async fn write_response(
    stream: &mut smol::net::TcpStream,
    status: u16,
    reason: &str,
    content_type: &str,
    body: &[u8],
) -> Result<()> {
    write_response_headers(stream, status, reason, content_type, body, "").await
}

async fn write_response_headers(
    stream: &mut smol::net::TcpStream,
    status: u16,
    reason: &str,
    content_type: &str,
    body: &[u8],
    extra_headers: &str,
) -> Result<()> {
    let head = format!(
        "HTTP/1.1 {status} {reason}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\n{extra_headers}Connection: close\r\n\r\n",
        body.len()
    );
    stream.write_all(head.as_bytes()).await?;
    stream.write_all(body).await?;
    stream.flush().await?;
    Ok(())
}

/// Blocking WebSocket pump (one unblock thread per window connection).
fn ws_pump(state: Arc<ServerState>, stream: WsStream) {
    if let Err(err) = ws_pump_inner(&state, stream) {
        log_err!(format!("ws pump failed: {err}"));
    }
}

fn ws_pump_inner(state: &Arc<ServerState>, stream: WsStream) -> Result<()> {
    use tungstenite::{
        Message, accept_hdr,
        handshake::server::{ErrorResponse, Request, Response},
    };

    let app = state.app.clone();
    let expected_host = format!("127.0.0.1:{}", state.port);
    let selected_window = Arc::new(std::sync::Mutex::new(None));
    let selected_in_handshake = selected_window.clone();
    // NB: HandshakeError holds the stream type (not Send/Sync-friendly),
    // so map handshake failures to strings immediately.
    let mut ws = accept_hdr(stream, move |req: &Request, response: Response| {
        let path_ok = req.uri().path() == "/__niva_ws";
        let token = req.uri().query().and_then(|q| query_value(q, "token"));
        let origin = req
            .headers()
            .get("origin")
            .and_then(|value| value.to_str().ok());
        let host_ok = req
            .headers()
            .get("host")
            .and_then(|v| v.to_str().ok())
            .is_some_and(|host| host == expected_host);
        let window = token.and_then(|token| {
            app.window()
                .ok()
                .and_then(|windows| windows.get_window_by_token(token).ok())
        });
        let origin_ok = window
            .as_ref()
            .is_some_and(|window| is_trusted_origin(window.trusted_ws_origin.as_deref(), origin));
        if path_ok
            && origin_ok
            && host_ok
            && let Some(window) = window
        {
            *selected_in_handshake.lock().unwrap() = Some(window);
            Ok(response)
        } else {
            let mut rejection = ErrorResponse::new(Some("forbidden".to_string()));
            *rejection.status_mut() = tungstenite::http::StatusCode::FORBIDDEN;
            Err(rejection)
        }
    })
    .map_err(|err| anyhow!("ws handshake failed: {err}"))?;

    let window = selected_window
        .lock()
        .unwrap()
        .take()
        .ok_or_else(|| anyhow!("missing authenticated window"))?;

    ws.get_ref()
        .set_read_timeout(Some(Duration::from_secs(5)))?;
    let hello_text = match ws.read()? {
        Message::Text(text) => text.to_string(),
        _ => return Err(anyhow!("Bad hello")),
    };

    let (tx, rx) = mpsc::channel::<crate::app::window_manager::window::WsOut>();
    let connection_id = window.register_ws_sender(tx.clone());
    let api = state.app.api();
    if let Err(error) = register_ws_hello(&api, window.id, connection_id, &hello_text) {
        window.remove_ws_sender(connection_id);
        api.cancel_connection(window.id, connection_id);
        return Err(error);
    }
    ws.get_ref()
        .set_read_timeout(Some(Duration::from_millis(10)))?;
    // Drain on every iteration, including while uploads are active. Previously
    // continuously arriving frames starved all write acknowledgements until a
    // read timeout, preventing socket backpressure from making progress.
    'pump: loop {
        use crate::app::window_manager::window::WsOut;
        for _ in 0..64 {
            let Ok(out) = rx.try_recv() else {
                break;
            };
            let sent = match out {
                WsOut::Text(text) => ws.send(Message::text(text)),
                WsOut::Binary(bytes) => ws.send(Message::Binary(bytes.into())),
            };
            if sent.is_err() {
                break 'pump;
            }
        }
        match ws.read() {
            Ok(Message::Text(text)) => handle_ws_request(state, &window, connection_id, &tx, &text),
            Ok(Message::Binary(bytes)) => handle_ws_binary(state, &window, connection_id, &bytes),
            Ok(Message::Close(_)) => break,
            Ok(_) => {}
            Err(tungstenite::Error::Io(err))
                if matches!(
                    err.kind(),
                    std::io::ErrorKind::TimedOut | std::io::ErrorKind::WouldBlock
                ) => {}
            Err(_) => break,
        }
    }

    window.remove_ws_sender(connection_id);
    state.app.api().cancel_connection(window.id, connection_id);
    Ok(())
}

#[derive(Debug, PartialEq, Eq)]
struct WsHello {
    window_id: u8,
    session_id: String,
}

/// `{t:"hello", wid, v, sessionId}` — first text frame on a fresh connection.
/// Version mismatches are refused loudly (fail fast, never half-talk).
fn parse_hello(text: &str) -> Option<WsHello> {
    use crate::app::api_manager::protocol::{ClientMsg, WIRE_VERSION};

    match serde_json::from_str::<ClientMsg>(text).ok()? {
        ClientMsg::Hello { wid, v, session_id } if v == WIRE_VERSION => Some(WsHello {
            window_id: wid,
            session_id,
        }),
        ClientMsg::Hello { v, .. } => {
            crate::niva_log!(
                crate::app::logging::Level::Warn,
                "ws hello wire version mismatch: got {v}"
            );
            None
        }
        _ => None,
    }
}

/// Bind the authenticated transport's first frame to the ApiManager before
/// allowing any following call frame on this connection.
fn register_ws_hello(
    api: &crate::app::api_manager::ApiManager,
    window_id: u8,
    connection_id: u64,
    text: &str,
) -> Result<()> {
    let hello = parse_hello(text).ok_or_else(|| anyhow!("Bad hello"))?;
    if hello.window_id != window_id {
        return Err(anyhow!("WS hello window id does not match credential"));
    }
    if !api.bind_ws_session(window_id, connection_id, &hello.session_id) {
        return Err(anyhow!("WS hello has an invalid session id"));
    }
    Ok(())
}

fn handle_ws_request(
    state: &Arc<ServerState>,
    window: &Arc<crate::app::window_manager::window::NivaWindow>,
    connection_id: u64,
    tx: &mpsc::Sender<crate::app::window_manager::window::WsOut>,
    text: &str,
) {
    state.app.api().on_text(window.id, connection_id, tx, text);
}

fn handle_ws_binary(
    state: &Arc<ServerState>,
    window: &Arc<crate::app::window_manager::window::NivaWindow>,
    connection_id: u64,
    bytes: &[u8],
) {
    state.app.api().on_binary(window.id, connection_id, bytes);
}

fn is_trusted_origin(expected: Option<&str>, actual: Option<&str>) -> bool {
    expected.is_some_and(|expected| actual == Some(expected))
}

/// Server accessor stored on the app (populated after the Arc exists).
pub type HttpServerSlot = ArcMut<Option<Arc<NivaHttpServer>>>;

pub fn app_server(app: &Arc<NivaApp>) -> Result<u16> {
    let slot = app.http_slot();
    let slot = lock!(slot)?;
    let server = slot.as_ref().ok_or(anyhow!("HTTP server not started"))?;
    Ok(server.port)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trusted_http_origins_are_exact_per_app() {
        let expected =
            crate::app::custom_protocol::origin_for_scheme("niva-a51c1728d17442d48f577d296c966b51");
        let other =
            crate::app::custom_protocol::origin_for_scheme("niva-b61c1728d17442d48f577d296c966b51");
        assert!(is_trusted_origin(Some(&expected), Some(&expected)));
        assert!(!is_trusted_origin(Some(&expected), Some(&other)));
        assert!(!is_trusted_origin(Some(&expected), None));
        assert!(!is_trusted_origin(None, Some(&expected)));
    }

    #[test]
    fn ws_handshake_binds_hello_session_before_forwarding_calls_to_api_manager() {
        use crate::app::{
            api_manager::{ApiManager, protocol::WIRE_VERSION},
            options::NivaOptions,
            window_manager::window::WsOut,
        };

        let options: NivaOptions = serde_json::from_value(serde_json::json!({
            "name": "ws-handshake-test",
            "uuid": "a51c1728-d174-42d4-8f57-7d296c966b51"
        }))
        .unwrap();
        let manager = ApiManager::new(&options);
        let session_id = "0123456789abcdef0123456789abcdef";
        let hello = serde_json::json!({
            "t": "hello",
            "wid": 7,
            "v": WIRE_VERSION,
            "sessionId": session_id
        })
        .to_string();

        register_ws_hello(&manager, 7, 44, &hello).unwrap();
        assert!(manager.ws_session_matches(7, 44, session_id));

        // Exercise the same ApiManager receive entry used immediately after
        // ws_pump consumes Hello. A valid session reaches normal dispatch;
        // a different session is rejected at the transport boundary.
        let (tx, rx) = mpsc::channel();
        manager.on_text(
            7,
            44,
            &tx,
            &serde_json::json!({
                "t": "call",
                "id": 1,
                "method": "test.unregistered",
                "args": [],
                "sessionId": session_id
            })
            .to_string(),
        );
        let WsOut::Text(accepted) = rx.recv().unwrap() else {
            panic!("expected a unary text response");
        };
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&accepted).unwrap()["message"],
            "API manager not ready"
        );

        manager.on_text(
            7,
            44,
            &tx,
            &serde_json::json!({
                "t": "call",
                "id": 2,
                "method": "test.unregistered",
                "args": [],
                "sessionId": "abcdef0123456789abcdef0123456789"
            })
            .to_string(),
        );
        let WsOut::Text(rejected) = rx.recv().unwrap() else {
            panic!("expected a unary text response");
        };
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&rejected).unwrap()["message"],
            "invalid WebSocket session id"
        );

        manager.cancel_connection(7, 44);
        assert!(!manager.ws_session_matches(7, 44, session_id));
    }

    #[test]
    fn ws_handshake_rejects_bad_or_mismatched_hello_before_binding() {
        use crate::app::{api_manager::ApiManager, options::NivaOptions};

        let options: NivaOptions = serde_json::from_value(serde_json::json!({
            "name": "ws-handshake-reject-test",
            "uuid": "a51c1728-d174-42d4-8f57-7d296c966b51"
        }))
        .unwrap();
        let manager = ApiManager::new(&options);
        let valid_session = "0123456789abcdef0123456789abcdef";
        let mismatch = serde_json::json!({
            "t": "hello", "wid": 8, "v": 1, "sessionId": valid_session
        })
        .to_string();
        assert!(register_ws_hello(&manager, 7, 45, &mismatch).is_err());

        let invalid_session = serde_json::json!({
            "t": "hello", "wid": 7, "v": 1, "sessionId": "not-a-session"
        })
        .to_string();
        assert!(register_ws_hello(&manager, 7, 46, &invalid_session).is_err());
        assert!(!manager.ws_session_matches(7, 45, valid_session));
        assert!(!manager.ws_session_matches(7, 46, "not-a-session"));
    }

    #[test]
    fn rejected_post_body_drain_accepts_length_and_chunked_framing() {
        let mut length_headers = HashMap::new();
        length_headers.insert("content-length".into(), "5".into());
        let mut length_body = smol::io::Cursor::new(b"helloNEXT".to_vec());
        assert!(smol::block_on(drain_rejected_body(&mut length_body, &length_headers)).unwrap());
        let mut next = [0u8; 4];
        smol::block_on(length_body.read_exact(&mut next)).unwrap();
        assert_eq!(&next, b"NEXT");

        let mut chunked_headers = HashMap::new();
        chunked_headers.insert("transfer-encoding".into(), "chunked".into());
        let mut chunked_body =
            smol::io::Cursor::new(b"5\r\nhello\r\n0\r\nX-Check: yes\r\n\r\nNEXT".to_vec());
        assert!(smol::block_on(drain_rejected_body(&mut chunked_body, &chunked_headers)).unwrap());
        smol::block_on(chunked_body.read_exact(&mut next)).unwrap();
        assert_eq!(&next, b"NEXT");

        length_headers.insert("content-length".into(), (1024 * 1024 + 1).to_string());
        assert!(!smol::block_on(drain_rejected_body(&mut length_body, &length_headers)).unwrap());
    }

    #[test]
    fn sync_rejects_ambiguous_or_unbounded_framing() {
        assert!(
            parse_head(
                b"POST /__niva_sync HTTP/1.1\r\nContent-Length: 1\r\nContent-Length: 2\r\n\r\n"
            )
            .is_err()
        );
        let mut headers = HashMap::from([("content-length".into(), "12".into())]);
        assert_eq!(sync_body_length(&headers).unwrap(), 12);
        headers.insert("transfer-encoding".into(), "chunked".into());
        assert!(sync_body_length(&headers).is_err());
        headers.remove("transfer-encoding");
        headers.insert("content-length".into(), (MAX_SYNC_BODY + 1).to_string());
        assert!(sync_body_length(&headers).is_err());
        headers.insert("content-length".into(), "+12".into());
        assert!(sync_body_length(&headers).is_err());
    }

    #[test]
    fn parse_head_splits_method_target_headers() {
        let raw = b"GET /__niva_ws?token=abc HTTP/1.1\r\nHost: x\r\nUpgrade: websocket\r\nConnection: Upgrade\r\n\r\n";
        let (head, _) = parse_head(raw).unwrap();
        assert_eq!(head.method, "GET");
        assert_eq!(head.target, "/__niva_ws?token=abc");
        assert_eq!(head.headers.get("upgrade").unwrap(), "websocket");
        let (path, query) = split_target(&head.target);
        assert_eq!(path, "/__niva_ws");
        assert_eq!(query_value(query.unwrap(), "token"), Some("abc"));
        assert!(is_upgrade(&head.headers));
    }

    #[test]
    fn split_target_without_query() {
        let (path, query) = split_target("/assets/a.js");
        assert_eq!(path, "/assets/a.js");
        assert!(query.is_none());
    }

    fn head_with(headers: &[(&str, &str)]) -> HttpHead {
        HttpHead {
            method: "GET".into(),
            target: "/index.html".into(),
            headers: headers
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect(),
        }
    }

    #[test]
    fn navigation_detection_prefers_fetch_metadata() {
        // Real navigation (top-level or iframe): inject.
        let nav = head_with(&[
            ("sec-fetch-mode", "navigate"),
            ("accept", "text/html,application/xhtml+xml"),
        ]);
        assert!(is_document_navigation(&nav.headers));
        // fetch()/XHR: pass through byte-identical, even for HTML.
        let fetch = head_with(&[("sec-fetch-mode", "cors"), ("accept", "text/html")]);
        assert!(!is_document_navigation(&fetch.headers));
        let xhr = head_with(&[("sec-fetch-mode", "same-origin"), ("accept", "*/*")]);
        assert!(!is_document_navigation(&xhr.headers));
    }

    #[test]
    fn navigation_detection_falls_back_to_accept() {
        // Engines without Fetch Metadata: navigations ask for text/html…
        let nav = head_with(&[("accept", "text/html,application/xhtml+xml")]);
        assert!(is_document_navigation(&nav.headers));
        // …while fetch() defaults to */*.
        let fetch = head_with(&[("accept", "*/*")]);
        assert!(!is_document_navigation(&fetch.headers));
        let none = head_with(&[]);
        assert!(!is_document_navigation(&none.headers));
    }
}
