use std::{
    collections::HashMap,
    io::{Read, Write},
    sync::{Arc, mpsc},
    thread,
    time::Duration,
};

use anyhow::{Result, anyhow};
use smol::{net::TcpListener, prelude::*};

use super::{
    NivaApp,
    resource_manager::ResourceManager,
    utils::{ArcMut, error_page_html},
};
use crate::{lock, log_err};

/// Hand-written async HTTP server (smol) replacing the old `niva://` custom
/// protocol (custom schemes break iframes; plain http over loopback does
/// not). Serves static files and hosts the API WebSocket.
///
/// Only GET + `Connection: close` is implemented; that is all the webview
/// needs. No tokio/axum/hyper: this crate plus tungstenite is everything.
pub struct NivaHttpServer {
    pub port: u16,
    pub token: String,
    _state: Arc<ServerState>,
    _driver: thread::JoinHandle<()>,
}

struct ServerState {
    app: Arc<NivaApp>,
    resource: Arc<dyn ResourceManager>,
    token: String,
}

impl NivaHttpServer {
    pub fn start(app: &Arc<NivaApp>) -> Result<Arc<Self>> {
        let token = random_token()?;
        let listener = smol::block_on(TcpListener::bind("127.0.0.1:0"))?;
        let port = listener.local_addr()?.port();

        let state = Arc::new(ServerState {
            app: app.clone(),
            resource: app.resource(),
            token: token.clone(),
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
            token,
            _state: state,
            _driver: driver,
        }))
    }

    pub fn base_url(&self) -> String {
        format!("http://127.0.0.1:{}", self.port)
    }
}

fn random_token() -> Result<String> {
    let mut bytes = [0u8; 16];
    getrandom::fill(&mut bytes).map_err(|err| anyhow!("RNG failed: {err}"))?;
    Ok(bytes.iter().map(|b| format!("{b:02x}")).collect())
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

async fn handle_conn(state: &Arc<ServerState>, mut stream: smol::net::TcpStream) -> Result<()> {
    let (head, raw_head) = read_head(&mut stream).await?;

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
        let token_ok = query
            .and_then(|q| query_value(q, "token"))
            .map(|t| t == state.token)
            .unwrap_or(false);
        if !token_ok {
            return write_response(&mut stream, 403, "Forbidden", "text/plain", b"bad token").await;
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
        return match std::fs::read(rest) {
            Ok(content) => {
                let mime = mime_guess::from_path(rest)
                    .first()
                    .map(|mime| mime.to_string())
                    .unwrap_or_else(|| "application/octet-stream".to_string());
                write_response(&mut stream, 200, "OK", &mime, &content).await
            }
            Err(err) => {
                eprintln!("[niva] fs {path} -> 404: {err}");
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

async fn write_response(
    stream: &mut smol::net::TcpStream,
    status: u16,
    reason: &str,
    content_type: &str,
    body: &[u8],
) -> Result<()> {
    let head = format!(
        "HTTP/1.1 {status} {reason}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
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

    let token = state.token.clone();
    // NB: HandshakeError holds the stream type (not Send/Sync-friendly),
    // so map handshake failures to strings immediately.
    let mut ws = accept_hdr(stream, move |req: &Request, response: Response| {
        let path_ok = req.uri().path() == "/__niva_ws";
        let query_ok = req
            .uri()
            .query()
            .and_then(|q| query_value(q, "token"))
            .map(|t| t == token)
            .unwrap_or(false);
        if path_ok && query_ok {
            Ok(response)
        } else {
            let mut rejection = ErrorResponse::new(Some("forbidden".to_string()));
            *rejection.status_mut() = tungstenite::http::StatusCode::FORBIDDEN;
            Err(rejection)
        }
    })
    .map_err(|err| anyhow!("ws handshake failed: {err}"))?;

    ws.get_ref()
        .set_read_timeout(Some(Duration::from_secs(5)))?;
    let window_id: u8 = match ws.read()? {
        Message::Text(text) => parse_hello(&text).ok_or(anyhow!("Bad hello"))?,
        _ => return Err(anyhow!("Bad hello")),
    };

    let window = state
        .app
        .window()
        .and_then(|windows| windows.get_window(window_id))?;

    let (tx, rx) = mpsc::channel::<String>();
    window.set_ws_sender(Some(tx));

    ws.get_ref()
        .set_read_timeout(Some(Duration::from_millis(200)))?;
    loop {
        match ws.read() {
            Ok(Message::Text(text)) => {
                handle_ws_request(state, &window, &text);
            }
            Ok(Message::Close(_)) => break,
            Ok(_) => {}
            Err(tungstenite::Error::Io(err))
                if err.kind() == std::io::ErrorKind::TimedOut
                    || err.kind() == std::io::ErrorKind::WouldBlock =>
            {
                // No inbound traffic: flush outbound queue.
                while let Ok(text) = rx.try_recv() {
                    if ws.send(Message::text(text)).is_err() {
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

    window.set_ws_sender(None);
    Ok(())
}

/// `["hello", <window_id:number>]`
fn parse_hello(text: &str) -> Option<u8> {
    let value: serde_json::Value = serde_json::from_str(text).ok()?;
    let arr = value.as_array()?;
    if arr.first()?.as_str()? != "hello" {
        return None;
    }
    arr.get(1)?.as_u64()?.try_into().ok()
}

fn handle_ws_request(
    state: &Arc<ServerState>,
    window: &Arc<crate::app::window_manager::window::NivaWindow>,
    text: &str,
) {
    let request_id = serde_json::from_str::<serde_json::Value>(text)
        .ok()
        .and_then(|v| v.as_array()?.first()?.as_u64())
        .unwrap_or(0);

    let result = (|| -> Result<()> {
        let api = state.app.api()?;
        api.call(window.id, text.to_string())
    })();

    if let Err(err) = result {
        // Deliver the error through the normal ipc.callback channel so the
        // pending JS promise rejects instead of hanging.
        let payload = serde_json::json!([request_id, -1, err.to_string(), null]);
        let envelope = serde_json::json!(["event", "ipc.callback", payload]);
        if !window.send_ws_envelope(&envelope.to_string()) {
            let _ = window.send_envelope_eval(&envelope);
        }
    }
}

/// Server accessor stored on the app (populated after the Arc exists).
pub type HttpServerSlot = ArcMut<Option<Arc<NivaHttpServer>>>;

pub fn app_server(app: &Arc<NivaApp>) -> Result<(u16, String)> {
    let slot = app.http_slot();
    let slot = lock!(slot)?;
    let server = slot.as_ref().ok_or(anyhow!("HTTP server not started"))?;
    Ok((server.port, server.token.clone()))
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
}
