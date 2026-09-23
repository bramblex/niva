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
/// asynchronous `niva://` protocol; ordinary HTTP static routes are reserved
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
        let node_compat = NodeCompat::from_option(&app.launch_info.options.node_compat)?;
        if node_compat.is_some() && !app.resource().exists("__niva_compat/node-compat.js") {
            return Err(anyhow!(
                "nodeCompat enabled but packaged adapter assets are missing"
            ));
        }
        let listener = smol::block_on(TcpListener::bind("127.0.0.1:0"))?;
        let port = listener.local_addr()?.port();

        let state = Arc::new(ServerState {
            app: app.clone(),
            resource: app.resource(),
            port,
            node_compat,
            debug_static_enabled: app.launch_info.arguments.debug_resource.is_some()
                || app.launch_info.arguments.debug_config.is_some()
                || app.launch_info.arguments.debug_entry.is_some(),
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
            headers.insert(name.trim().to_lowercase(), value.trim().to_string());
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

    if head.method != "GET" {
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
        && relative.starts_with("__niva_compat/")
    {
        if !state.debug_static_enabled {
            return write_response(&mut stream, 404, "Not Found", "text/plain", b"not found").await;
        }
        let Some(compat) = &state.node_compat else {
            return write_response(&mut stream, 404, "Not Found", "text/plain", b"not found").await;
        };
        if !compat.allows_asset(relative) {
            return write_response(&mut stream, 404, "Not Found", "text/plain", b"not found").await;
        }
        return match state.resource.load(relative) {
            Ok(content) => {
                let content = if relative == "__niva_compat/node-compat.js" {
                    compat.classic_script(&content)?
                } else {
                    content
                };
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
        let cors_origin = head
            .headers
            .get("origin")
            .filter(|origin| window.trusted_ws_origin.as_deref() == Some(origin.as_str()));
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
                eprintln!("[niva] fs response failed: {:?}", err.kind());
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
            eprintln!("[niva] static {path} -> 404: {err}");
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
            .and_then(|window| window.trusted_ws_origin.as_deref())
            .is_some_and(|trusted_origin| origin == Some(trusted_origin));
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
    let claimed_window_id: u8 = match ws.read()? {
        Message::Text(text) => parse_hello(&text).ok_or(anyhow!("Bad hello"))?,
        _ => return Err(anyhow!("Bad hello")),
    };

    if claimed_window_id != window.id {
        return Err(anyhow!("WS hello window id does not match credential"));
    }

    let (tx, rx) = mpsc::channel::<crate::app::window_manager::window::WsOut>();
    let connection_id = window.register_ws_sender(tx.clone());
    if window.id == 0
        && let Some(bridge) = state.app.stdio_bridge()
    {
        bridge.send_ready_once()?;
    }

    ws.get_ref()
        .set_read_timeout(Some(Duration::from_millis(200)))?;
    loop {
        match ws.read() {
            Ok(Message::Text(text)) => {
                handle_ws_request(state, &window, connection_id, &tx, &text);
            }
            Ok(Message::Binary(bytes)) => {
                handle_ws_binary(state, &window, connection_id, &bytes);
            }
            Ok(Message::Close(_)) => break,
            Ok(_) => {}
            Err(tungstenite::Error::Io(err))
                if err.kind() == std::io::ErrorKind::TimedOut
                    || err.kind() == std::io::ErrorKind::WouldBlock =>
            {
                // No inbound traffic: flush outbound queue.
                use crate::app::window_manager::window::WsOut;
                while let Ok(out) = rx.try_recv() {
                    let sent = match out {
                        WsOut::Text(text) => ws.send(Message::text(text)).is_ok(),
                        WsOut::Binary(bytes) => ws.send(Message::Binary(bytes.into())).is_ok(),
                    };
                    if !sent {
                        break;
                    }
                }
            }
            Err(tungstenite::Error::AlreadyClosed) | Err(tungstenite::Error::ConnectionClosed) => {
                break;
            }
            Err(_) => break,
        }
    }

    window.remove_ws_sender(connection_id);
    state.app.api().cancel_connection(window.id, connection_id);
    Ok(())
}

/// `{t:"hello", wid, v}` — first text frame on a fresh connection.
/// Version mismatches are refused loudly (fail fast, never half-talk).
fn parse_hello(text: &str) -> Option<u8> {
    use crate::app::api_manager::protocol::{ClientMsg, WIRE_VERSION};

    match serde_json::from_str::<ClientMsg>(text).ok()? {
        ClientMsg::Hello { wid, v } if v == WIRE_VERSION => Some(wid),
        ClientMsg::Hello { v, .. } => {
            eprintln!("[niva] ws hello wire version mismatch: got {v}");
            None
        }
        _ => None,
    }
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
