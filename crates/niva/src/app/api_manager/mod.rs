pub(crate) mod protocol;

use std::{
    collections::HashMap,
    future::Future,
    pin::Pin,
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, AtomicUsize, Ordering},
        mpsc,
    },
    thread,
    time::Duration,
};

use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::{lock, unsafe_impl_sync_send};

use self::protocol::{ClientMsg, ServerMsg, decode_chunk, encode_chunk};
use super::{
    NivaApp,
    options::NivaOptions,
    window_manager::window::{NivaWindow, WsOut},
};

#[derive(Deserialize, Clone)]
pub struct ApiArguments(Value);

impl ApiArguments {
    pub fn get<T: serde::de::DeserializeOwned>(&self) -> Result<T> {
        Ok(serde_json::from_value(self.0.clone())?)
    }

    pub fn optional<T: serde::de::DeserializeOwned>(&self, args_size: usize) -> Result<T> {
        let mut args = serde_json::from_value::<Vec<serde_json::Value>>(self.0.clone())?;
        args.resize(args_size, json!(null));
        let args = json!(args);
        Ok(serde_json::from_value(args)?)
    }
}

#[derive(Deserialize, Clone)]
pub struct ApiRequest(pub u64, pub String, pub ApiArguments);

impl ApiRequest {
    pub fn err<C: Into<i32>, S: Into<String>>(&self, code: C, msg: S) -> ApiResponse {
        ApiResponse(self.0, code.into(), msg.into(), json!(null))
    }

    pub fn ok<D: Serialize>(&self, data: D) -> ApiResponse {
        ApiResponse(self.0, 0, "ok".to_string(), json!(data))
    }

    pub fn args(&self) -> &ApiArguments {
        &self.2
    }
}

pub type Code = i32;

#[derive(Serialize, Clone)]
pub struct ApiResponse(u64, Code, String, Value);

/// Boxed unary handler: compute data, the dispatcher delivers it.
pub type ApiFuture = Pin<Box<dyn Future<Output = ApiResponse> + Send>>;
pub type ApiHandler =
    Arc<dyn Fn(Arc<NivaApp>, Arc<NivaWindow>, ApiRequest) -> ApiFuture + Send + Sync>;

/// Boxed stateful handler: drives its own CallContext (streams, chunks).
pub type StreamFuture = Pin<Box<dyn Future<Output = Result<()>> + Send>>;
pub type StreamHandler = Arc<dyn Fn(CallContext, ApiRequest) -> StreamFuture + Send + Sync>;

enum HandlerKind {
    Unary(ApiHandler),
    Stream(StreamHandler),
}

struct HandlerEntry {
    handler: HandlerKind,
    /// None = dispatcher default, Some(None) = no timeout.
    timeout: Option<Option<Duration>>,
}

/// One binary frame from client to server, already header-decoded.
pub struct InboundChunk {
    /// Frame order within the call (_UPLOAD direction keeps sender order;
    /// present for consumers that reassemble out of order).
    #[allow(dead_code)]
    pub seq: u64,
    pub data: Vec<u8>,
    pub end: bool,
}

/// Handle to a live stateful call: push events, stream binary, answer once.
/// Cheaply cloneable; all clones observe the same cancellation.
#[derive(Clone)]
pub struct CallContext {
    inner: Arc<CallInner>,
}

/// Shared state behind CallContext. Public so stream handlers can reach
/// `app` / `window` / `id` through the Deref impl.
pub struct CallInner {
    pub app: Arc<NivaApp>,
    pub window: Arc<NivaWindow>,
    pub id: u64,
    pub connection_id: u64,
    pub ws_tx: mpsc::Sender<WsOut>,
    pub seq: AtomicU64,
    pub cancel_rx: async_channel::Receiver<()>,
    pub inbound_rx: async_channel::Receiver<InboundChunk>,
}

impl std::ops::Deref for CallContext {
    type Target = CallInner;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl CallContext {
    /// Best-effort stream push tied to this call (dropped when socket gone).
    pub fn push(&self, name: &str, data: Value) {
        let seq = self.seq.fetch_add(1, Ordering::Relaxed);
        let msg = ServerMsg::event(Some(self.id), seq, name.to_string(), data);
        let _ = self.ws_tx.send(WsOut::Text(msg.encode()));
    }

    /// Best-effort binary chunk tied to this call (dropped when socket gone).
    /// `stderr` tags the exec stderr sub-stream; data streams leave it false.
    pub fn chunk(&self, data: &[u8], end: bool) {
        self.chunk_to(data, end, false);
    }

    pub fn chunk_stderr(&self, data: &[u8], end: bool) {
        self.chunk_to(data, end, true);
    }

    fn chunk_to(&self, data: &[u8], end: bool, stderr: bool) {
        let seq = self.seq.fetch_add(1, Ordering::Relaxed);
        // First chunk of a call opens with START (seq starts at 1).
        let frame = encode_chunk(self.id, seq, seq == 1, end, stderr, data);
        let _ = self.ws_tx.send(WsOut::Binary(frame));
    }

    /// Terminal response for this call.
    pub fn respond<T: Serialize>(&self, result: Result<T>) {
        let (code, message, data) = match result {
            Ok(data) => (0, "ok".to_string(), json!(data)),
            Err(err) => (-1, err.to_string(), native_error_data(&err)),
        };
        let msg = ServerMsg::result(self.id, code, message, data);
        if self.ws_tx.send(WsOut::Text(msg.encode())).is_err() {
            eprintln!("[niva] api response dropped, window socket gone");
        }
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancel_rx.is_closed()
    }

    pub async fn cancelled(&self) {
        let _ = self.cancel_rx.recv().await;
    }

    /// Next inbound binary chunk, or None when the client ends/cancels.
    /// (Used by interactive stream handlers; e.g. process stdin.)
    #[allow(dead_code)]
    pub async fn next_chunk(&self) -> Option<InboundChunk> {
        self.inbound_rx.recv().await.ok()
    }

    /// Blocking variant for pump threads (exec stdin); same semantics.
    pub fn next_chunk_blocking(&self) -> Option<InboundChunk> {
        self.inbound_rx.recv_blocking().ok()
    }

    /// Gather inbound chunks until END into one buffer.
    #[allow(dead_code)]
    pub async fn collect_all(&self) -> Vec<u8> {
        let mut out = Vec::new();
        while let Some(chunk) = self.next_chunk().await {
            out.extend_from_slice(&chunk.data);
            if chunk.end {
                break;
            }
        }
        out
    }
}

struct ActiveCall {
    cancel_tx: async_channel::Sender<()>,
    inbound_tx: async_channel::Sender<InboundChunk>,
}

struct DispatchJob {
    ctx: CallContext,
    request: ApiRequest,
    handler: HandlerKind,
    timeout: Option<Duration>,
}

unsafe_impl_sync_send!(ApiManager);
pub struct ApiManager {
    app: std::sync::OnceLock<Arc<NivaApp>>,
    handlers: HashMap<String, HandlerEntry>,
    dispatch_tx: async_channel::Sender<DispatchJob>,
    default_timeout: Duration,
    active: Arc<Mutex<HashMap<(u8, u64, u64), ActiveCall>>>,
    ipc_active: AtomicUsize,
    max_ipc_active: usize,
}

struct IpcPermit<'a>(&'a AtomicUsize);

impl Drop for IpcPermit<'_> {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::AcqRel);
    }
}

impl ApiManager {
    pub fn ipc_error_response(raw_body: &str, error: &str) -> String {
        let id = if raw_body.len() <= 256 * 1024 {
            serde_json::from_str::<Value>(raw_body)
                .ok()
                .and_then(|message| message.get("id").and_then(Value::as_u64))
                .unwrap_or(0)
        } else {
            0
        };
        ServerMsg::result(id, -1, error.to_string(), json!(null)).encode()
    }

    pub fn new(options: &NivaOptions) -> Self {
        let api_options = options.api.clone().unwrap_or_default();
        let default_timeout = Duration::from_millis(api_options.timeout_ms.unwrap_or(30_000));
        let max_queue = api_options.max_queue.unwrap_or(64);

        let (dispatch_tx, dispatch_rx) = async_channel::bounded::<DispatchJob>(max_queue);
        let active = Arc::new(Mutex::new(HashMap::new()));

        // Dedicated driver thread: pumps the dispatch queue, one smol task
        // per request. Handlers never block it (blocking goes to unblock).
        let loop_active = active.clone();
        thread::Builder::new()
            .name("niva-api".into())
            .spawn(move || {
                smol::block_on(async move {
                    while let Ok(job) = dispatch_rx.recv().await {
                        smol::spawn(run_job(job, loop_active.clone())).detach();
                    }
                })
            })
            .expect("Failed to spawn api driver thread");

        ApiManager {
            app: std::sync::OnceLock::new(),
            handlers: HashMap::new(),
            dispatch_tx,
            default_timeout,
            active,
            ipc_active: AtomicUsize::new(0),
            max_ipc_active: max_queue.max(1),
        }
    }

    /// Set once during startup; lock-free reads afterwards.
    pub fn bind_app(&self, app: Arc<NivaApp>) {
        let _ = self.app.set(app);
    }

    /// Execute one JSON-only call from a remote document. The native WebView
    /// supplies both the window and source URL; neither is taken from the JS
    /// payload. Stateful/stream handlers have no IPC representation.
    pub async fn ipc_call(
        &self,
        window_id: u8,
        source_url: &str,
        raw_body: &str,
    ) -> Result<String> {
        const MAX_IPC_BYTES: usize = 256 * 1024;
        if raw_body.len() > MAX_IPC_BYTES {
            return Err(anyhow!("IPC request exceeds 256 KiB"));
        }
        let msg: ClientMsg = serde_json::from_str(raw_body)?;
        let ClientMsg::Call { id, method, args } = msg else {
            return Err(anyhow!("IPC only accepts unary calls"));
        };
        let request = ApiRequest(id, method, ApiArguments(args));
        let previous = self.ipc_active.fetch_add(1, Ordering::AcqRel);
        if previous >= self.max_ipc_active {
            self.ipc_active.fetch_sub(1, Ordering::AcqRel);
            return Ok(ServerMsg::result(id, -3, "IPC server busy".into(), json!(null)).encode());
        }
        let _permit = IpcPermit(&self.ipc_active);
        let app = self
            .app
            .get()
            .cloned()
            .ok_or_else(|| anyhow!("API manager not ready"))?;
        let window = app.window()?.get_window(window_id)?;
        let local_origin = format!("http://127.0.0.1:{}", app.server_info()?);
        let source_origin = url::Url::parse(source_url)
            .map_err(|_| anyhow!("invalid IPC source URL"))?
            .origin()
            .ascii_serialization();
        let denied = |message: &str| {
            let response = request.err(-4, message);
            ServerMsg::result(response.0, response.1, response.2, response.3).encode()
        };
        if source_origin == local_origin {
            return Ok(denied("local pages use the WebSocket bridge"));
        }
        if !window.permissions.allows(source_url, &request.1)
            || matches!(
                request.1.as_str(),
                "window.open" | "webview.baseFileSystemUrl"
            )
        {
            return Ok(denied("permission denied"));
        }
        let Some(entry) = self.handlers.get(&request.1) else {
            return Ok(denied("api not found"));
        };
        let HandlerKind::Unary(handler) = &entry.handler else {
            return Ok(denied("stream APIs are unavailable over IPC"));
        };
        let handler = handler.clone();
        let timeout = entry.timeout.unwrap_or(Some(self.default_timeout));
        let work = handler(app, window, request.clone());
        let response = match timeout {
            Some(duration) => {
                smol::future::or(work, async {
                    smol::Timer::after(duration).await;
                    request.err(-2, "API timeout")
                })
                .await
            }
            None => work.await,
        };
        let encoded = ServerMsg::result(response.0, response.1, response.2, response.3).encode();
        if encoded.len() > MAX_IPC_BYTES {
            return Ok(ServerMsg::result(
                id,
                -1,
                "IPC response exceeds 256 KiB".into(),
                json!(null),
            )
            .encode());
        }
        Ok(encoded)
    }

    /// Called only after the loopback HTTP endpoint authenticates window token
    /// and exact origin. UI/main-thread handlers must never run while XHR blocks
    /// that thread; socket streams likewise require their WS connection identity.
    pub async fn sync_call(&self, window_id: u8, request: ApiRequest) -> Result<String> {
        let app = self
            .app
            .get()
            .cloned()
            .ok_or_else(|| anyhow!("API manager not ready"))?;
        let window = app.window()?.get_window(window_id)?;
        let response = if !sync_method_allowed(&request.1) {
            request.err(-4, "API unavailable over synchronous XHR")
        } else if let Some(HandlerEntry {
            handler: HandlerKind::Unary(handler),
            ..
        }) = self.handlers.get(&request.1)
        {
            let previous = self.ipc_active.fetch_add(1, Ordering::AcqRel);
            if previous >= self.max_ipc_active {
                self.ipc_active.fetch_sub(1, Ordering::AcqRel);
                request.err(-3, "API server busy")
            } else {
                let _permit = IpcPermit(&self.ipc_active);
                // Blocking OS calls run on the existing blocking pool. A
                // timeout is not cancellation and could leave a mutation
                // running, so wait for a definitive result here.
                handler(app, window, request.clone()).await
            }
        } else {
            request.err(-4, "API is not a synchronous unary handler")
        };
        Ok(serde_json::to_string(&response)?)
    }

    /// Register a unary API: plain `async fn`, result auto-delivered.
    pub fn register_api<S, F, Fut, T>(&mut self, name: S, f: F)
    where
        S: Into<String>,
        F: Fn(Arc<NivaApp>, Arc<NivaWindow>, ApiRequest) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<T>> + Send + 'static,
        T: Serialize + Send + 'static,
    {
        self.register_api_with(name, None, f);
    }

    /// Unary registration with timeout override (`None` = no timeout).
    pub fn register_api_with<S, F, Fut, T>(
        &mut self,
        name: S,
        timeout: Option<Option<Duration>>,
        f: F,
    ) where
        S: Into<String>,
        F: Fn(Arc<NivaApp>, Arc<NivaWindow>, ApiRequest) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<T>> + Send + 'static,
        T: Serialize + Send + 'static,
    {
        let f = Arc::new(f);
        let handler: ApiHandler = Arc::new(move |app, window, request| {
            let f = f.clone();
            let fut = f(app, window, request.clone());
            Box::pin(async move {
                match fut.await {
                    Ok(data) => request.ok(data),
                    Err(err) => {
                        ApiResponse(request.0, -1, err.to_string(), native_error_data(&err))
                    }
                }
            }) as ApiFuture
        });
        self.handlers.insert(
            name.into(),
            HandlerEntry {
                handler: HandlerKind::Unary(handler),
                timeout,
            },
        );
    }

    /// Blocking registration: plain sync body, executed on smol's elastic
    /// thread pool by construction (impossible to stall the driver or forget
    /// the wrapper). For syscalls, subprocess waits, file IO.
    pub fn register_blocking_api<S, F, T>(&mut self, name: S, f: F)
    where
        S: Into<String>,
        F: Fn(Arc<NivaApp>, Arc<NivaWindow>, ApiRequest) -> Result<T> + Send + Sync + 'static,
        T: Serialize + Send + 'static,
    {
        self.register_blocking_api_with(name, None, f);
    }

    /// Blocking registration with timeout override (`None` = no timeout).
    pub fn register_blocking_api_with<S, F, T>(
        &mut self,
        name: S,
        timeout: Option<Option<Duration>>,
        f: F,
    ) where
        S: Into<String>,
        F: Fn(Arc<NivaApp>, Arc<NivaWindow>, ApiRequest) -> Result<T> + Send + Sync + 'static,
        T: Serialize + Send + 'static,
    {
        let f = Arc::new(f);
        self.register_api_with(name, timeout, move |app, window, request| {
            let f = f.clone();
            async move { crate::blocking!(f(app, window, request)).await }
        });
    }

    /// Register a stateful (streaming) API: drives its own CallContext.
    pub fn register_stream_api<S, F, Fut>(&mut self, name: S, f: F)
    where
        S: Into<String>,
        F: Fn(CallContext, ApiRequest) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<()>> + Send + 'static,
    {
        self.register_stream_api_with(name, None, f);
    }

    pub fn register_stream_api_with<S, F, Fut>(
        &mut self,
        name: S,
        timeout: Option<Option<Duration>>,
        f: F,
    ) where
        S: Into<String>,
        F: Fn(CallContext, ApiRequest) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<()>> + Send + 'static,
    {
        let handler: StreamHandler =
            Arc::new(move |ctx, request| Box::pin(f(ctx, request)) as StreamFuture);
        self.handlers.insert(
            name.into(),
            HandlerEntry {
                handler: HandlerKind::Stream(handler),
                timeout,
            },
        );
    }

    /// Entry for text frames from the transport (non-blocking).
    pub fn on_text(&self, window_id: u8, connection_id: u64, tx: &mpsc::Sender<WsOut>, text: &str) {
        let msg: ClientMsg = match serde_json::from_str(text) {
            Ok(msg) => msg,
            Err(err) => {
                eprintln!("[niva] api bad message: {err}");
                return;
            }
        };
        match msg {
            ClientMsg::Hello { .. } => {}
            ClientMsg::Cancel { id } => self.cancel_call(window_id, connection_id, id),
            ClientMsg::Call { id, method, args } => self.dispatch(
                window_id,
                connection_id,
                tx.clone(),
                ApiRequest(id, method, ApiArguments(args)),
            ),
        }
    }

    /// Entry for binary frames from the transport (non-blocking).
    pub fn on_binary(&self, window_id: u8, connection_id: u64, frame: &[u8]) {
        let (header, payload) = match decode_chunk(frame) {
            Ok(pair) => pair,
            Err(err) => {
                eprintln!("[niva] api bad chunk: {err}");
                return;
            }
        };
        let active = lock!(self.active);
        let active = match active {
            Ok(active) => active,
            Err(err) => {
                eprintln!("[niva] api active table poisoned: {err}");
                return;
            }
        };
        match active.get(&(window_id, connection_id, header.id)) {
            Some(call) => {
                if call
                    .inbound_tx
                    .try_send(InboundChunk {
                        seq: header.seq,
                        data: payload.to_vec(),
                        end: header.end,
                    })
                    .is_err()
                {
                    eprintln!("[niva] api inbound overflow, chunk dropped");
                }
            }
            None => {
                eprintln!("[niva] api chunk for unknown call {}", header.id);
            }
        }
    }

    /// Abort one call (client cancel). Silent when unknown.
    pub fn cancel_call(&self, window_id: u8, connection_id: u64, id: u64) {
        let mut active = match lock!(self.active) {
            Ok(active) => active,
            Err(_) => return,
        };
        if let Some(call) = active.remove(&(window_id, connection_id, id)) {
            call.cancel_tx.close();
        }
    }

    /// Disconnecting one frame must not cancel calls from sibling frames.
    pub fn cancel_connection(&self, window_id: u8, connection_id: u64) {
        let mut active = match lock!(self.active) {
            Ok(active) => active,
            Err(_) => return,
        };
        let keys: Vec<_> = active
            .keys()
            .filter(|(wid, cid, _)| *wid == window_id && *cid == connection_id)
            .copied()
            .collect();
        for key in keys {
            if let Some(call) = active.remove(&key) {
                call.cancel_tx.close();
            }
        }
    }

    /// Abort all calls of a window (socket drop / window close).
    pub fn cancel_window(&self, window_id: u8) {
        let mut active = match lock!(self.active) {
            Ok(active) => active,
            Err(_) => return,
        };
        let keys: Vec<_> = active
            .keys()
            .filter(|(wid, _, _)| *wid == window_id)
            .copied()
            .collect();
        for key in keys {
            if let Some(call) = active.remove(&key) {
                call.cancel_tx.close();
            }
        }
    }

    pub(crate) fn dispatch(
        &self,
        window_id: u8,
        connection_id: u64,
        tx: mpsc::Sender<WsOut>,
        request: ApiRequest,
    ) {
        let app = match self.app.get().cloned() {
            Some(app) => app,
            None => {
                eprintln!("[niva] api dispatch before bind_app");
                return;
            }
        };
        let window = match app.window().and_then(|w| w.get_window(window_id)) {
            Ok(window) => window,
            Err(err) => {
                eprintln!("[niva] api call for unknown window {window_id}: {err}");
                return;
            }
        };
        let entry = match self.handlers.get(&request.1) {
            Some(entry) => entry,
            None => {
                respond(&tx, request.err(-1, "api not found".to_string()));
                return;
            }
        };
        let timeout = match entry.timeout {
            Some(custom) => custom,
            None => Some(self.default_timeout),
        };
        let handler = match &entry.handler {
            HandlerKind::Unary(h) => HandlerKind::Unary(h.clone()),
            HandlerKind::Stream(h) => HandlerKind::Stream(h.clone()),
        };
        let (cancel_tx, cancel_rx) = async_channel::bounded::<()>(1);
        let (inbound_tx, inbound_rx) = async_channel::bounded::<InboundChunk>(64);
        if let Ok(mut active) = lock!(self.active) {
            active.insert(
                (window_id, connection_id, request.0),
                ActiveCall {
                    cancel_tx: cancel_tx.clone(),
                    inbound_tx,
                },
            );
        }
        let ctx = CallContext {
            inner: Arc::new(CallInner {
                app,
                window,
                id: request.0,
                connection_id,
                ws_tx: tx,
                seq: AtomicU64::new(1),
                cancel_rx,
                inbound_rx,
            }),
        };
        let job = DispatchJob {
            ctx,
            request,
            handler,
            timeout,
        };
        if let Err(err) = self.dispatch_tx.try_send(job) {
            let job = err.into_inner();
            eprintln!("[niva] api dispatch queue full, rejecting");
            respond(
                &job.ctx.ws_tx,
                job.request.err(-3, "server busy".to_string()),
            );
        }
    }
}

/// Deliver a terminal response over the window socket; log-and-drop when gone.
fn respond(tx: &mpsc::Sender<WsOut>, response: ApiResponse) {
    let msg = ServerMsg::result(response.0, response.1, response.2, response.3);
    if tx.send(WsOut::Text(msg.encode())).is_err() {
        eprintln!("[niva] api response dropped, window socket gone");
    }
}

#[derive(PartialEq)]
enum End {
    Done,
    Timeout,
    Cancelled,
}

async fn run_job(job: DispatchJob, active: Arc<Mutex<HashMap<(u8, u64, u64), ActiveCall>>>) {
    let DispatchJob {
        ctx,
        request,
        handler,
        timeout,
    } = job;
    let wid = ctx.window.id;
    let connection_id = ctx.connection_id;
    let id = ctx.id;
    let watch = ctx.clone();

    let reply = ctx.clone();
    let work = async {
        match handler {
            HandlerKind::Unary(f) => {
                let response = f(ctx.app.clone(), ctx.window.clone(), request.clone()).await;
                respond(&ctx.ws_tx, response);
            }
            HandlerKind::Stream(f) => {
                if let Err(err) = f(ctx, request.clone()).await {
                    // Stateful handlers answer themselves; a bare Err means
                    // they never did: reject instead of hanging.
                    reply.respond::<Value>(Err(err));
                }
            }
        }
        End::Done
    };

    let end = match timeout {
        Some(duration) => {
            smol::future::or(work, async {
                smol::future::or(
                    async {
                        smol::Timer::after(duration).await;
                        End::Timeout
                    },
                    async {
                        watch.cancelled().await;
                        End::Cancelled
                    },
                )
                .await
            })
            .await
        }
        None => {
            smol::future::or(work, async {
                watch.cancelled().await;
                End::Cancelled
            })
            .await
        }
    };

    // Always close the cancel channel on task end: unblocks ctx waiters
    // (collect loops, stdin pumps) even when nobody cancelled.
    if let Ok(mut active) = lock!(active)
        && let Some(call) = active.remove(&(wid, connection_id, id))
    {
        call.cancel_tx.close();
    }
    if end == End::Timeout {
        respond(&reply.ws_tx, request.err(-2, "API timeout".to_string()));
    }
    // Cancelled: client went away, nothing to answer.
}

fn native_error_data(error: &anyhow::Error) -> Value {
    use std::io::ErrorKind;
    if let Some(error) = error.downcast_ref::<std::io::Error>() {
        let code = match error.kind() {
            ErrorKind::NotFound => "ENOENT",
            ErrorKind::PermissionDenied => "EACCES",
            ErrorKind::AlreadyExists => "EEXIST",
            ErrorKind::NotADirectory => "ENOTDIR",
            ErrorKind::IsADirectory => "EISDIR",
            ErrorKind::DirectoryNotEmpty => "ENOTEMPTY",
            ErrorKind::InvalidInput => "EINVAL",
            ErrorKind::BrokenPipe => "EPIPE",
            ErrorKind::ConnectionRefused => "ECONNREFUSED",
            ErrorKind::ConnectionReset => "ECONNRESET",
            ErrorKind::TimedOut => "ETIMEDOUT",
            ErrorKind::AddrInUse => "EADDRINUSE",
            ErrorKind::AddrNotAvailable => "EADDRNOTAVAIL",
            ErrorKind::NotConnected => "ENOTCONN",
            ErrorKind::ConnectionAborted => "ECONNABORTED",
            ErrorKind::WouldBlock => "EAGAIN",
            ErrorKind::Unsupported => "ENOTSUP",
            _ => "EIO",
        };
        json!({"code":code,"errno":error.raw_os_error()})
    } else {
        json!(null)
    }
}

fn sync_method_allowed(method: &str) -> bool {
    method.starts_with("fs.")
        || method.starts_with("os.")
        || matches!(
            method,
            "process.currentDir"
                | "process.setCurrentDir"
                | "process.execSync"
                | "process.execFileSync"
                | "process.spawnSync"
                | "process.uptime"
                | "process.memoryUsage"
                | "process.cpuUsage"
        )
}

#[cfg(test)]
mod ipc_tests {
    use super::*;

    #[test]
    fn synchronous_transport_excludes_ui_and_connection_owned_apis() {
        for method in [
            "window.open",
            "process.exit",
            "socket.control",
            "socket.tcpConnect",
            "webview.eval",
            "process.env",
        ] {
            assert!(!sync_method_allowed(method), "{method}");
        }
        for method in [
            "fs.stat",
            "fs.readFile",
            "os.freemem",
            "process.currentDir",
            "process.spawnSync",
        ] {
            assert!(sync_method_allowed(method), "{method}");
        }
    }

    #[test]
    fn error_reply_keeps_the_call_id() {
        let reply = ApiManager::ipc_error_response(
            r#"{"t":"call","id":42,"method":"os.info","args":[]}"#,
            "bad request",
        );
        let decoded: Value = serde_json::from_str(&reply).unwrap();
        assert_eq!(decoded["t"], "result");
        assert_eq!(decoded["id"], 42);
        assert_eq!(decoded["code"], -1);
    }
}
