pub(crate) mod protocol;

use std::{
    collections::HashMap,
    future::Future,
    pin::Pin,
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
    },
    thread,
    time::Duration,
};

use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::{lock, unsafe_impl_sync_send};

use self::protocol::{ClientMsg, ServerMsg, decode_chunk, encode_chunk};
use super::{NivaApp, options::NivaOptions, window_manager::window::NivaWindow};

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
        self.window.send_ws_envelope(&msg.encode());
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
        self.window.send_ws_binary(&frame);
    }

    /// Terminal response for this call.
    pub fn respond<T: Serialize>(&self, result: Result<T>) {
        let (code, message, data) = match result {
            Ok(data) => (0, "ok".to_string(), json!(data)),
            Err(err) => (-1, err.to_string(), json!(null)),
        };
        let msg = ServerMsg::result(self.id, code, message, data);
        if !self.window.send_ws_envelope(&msg.encode()) {
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
    active: Arc<Mutex<HashMap<(u8, u64), ActiveCall>>>,
}

impl ApiManager {
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
        }
    }

    /// Set once during startup; lock-free reads afterwards.
    pub fn bind_app(&self, app: Arc<NivaApp>) {
        let _ = self.app.set(app);
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
                    Err(err) => request.err(-1, err.to_string()),
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
        let handler: StreamHandler =
            Arc::new(move |ctx, request| Box::pin(f(ctx, request)) as StreamFuture);
        self.handlers.insert(
            name.into(),
            HandlerEntry {
                handler: HandlerKind::Stream(handler),
                timeout: None,
            },
        );
    }

    /// Entry for text frames from the transport (non-blocking).
    pub fn on_text(&self, window_id: u8, text: &str) {
        let msg: ClientMsg = match serde_json::from_str(text) {
            Ok(msg) => msg,
            Err(err) => {
                eprintln!("[niva] api bad message: {err}");
                return;
            }
        };
        match msg {
            ClientMsg::Hello { .. } => {}
            ClientMsg::Cancel { id } => self.cancel_call(window_id, id),
            ClientMsg::Call { id, method, args } => {
                self.dispatch(window_id, ApiRequest(id, method, ApiArguments(args)))
            }
        }
    }

    /// Entry for binary frames from the transport (non-blocking).
    pub fn on_binary(&self, window_id: u8, frame: &[u8]) {
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
        match active.get(&(window_id, header.id)) {
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
    pub fn cancel_call(&self, window_id: u8, id: u64) {
        let mut active = match lock!(self.active) {
            Ok(active) => active,
            Err(_) => return,
        };
        if let Some(call) = active.remove(&(window_id, id)) {
            call.cancel_tx.close();
        }
    }

    /// Abort all calls of a window (socket drop / window close).
    pub fn cancel_window(&self, window_id: u8) {
        let mut active = match lock!(self.active) {
            Ok(active) => active,
            Err(_) => return,
        };
        let ids: Vec<u64> = active
            .keys()
            .filter(|(wid, _)| *wid == window_id)
            .map(|(_, id)| *id)
            .collect();
        for id in ids {
            if let Some(call) = active.remove(&(window_id, id)) {
                call.cancel_tx.close();
            }
        }
    }

    pub(crate) fn dispatch(&self, window_id: u8, request: ApiRequest) {
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
                respond(&window, request.err(-1, "api not found".to_string()));
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
                (window_id, request.0),
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
                &job.ctx.window,
                job.request.err(-3, "server busy".to_string()),
            );
        }
    }
}

/// Deliver a terminal response over the window socket; log-and-drop when gone.
fn respond(window: &Arc<NivaWindow>, response: ApiResponse) {
    let msg = ServerMsg::result(response.0, response.1, response.2, response.3);
    if !window.send_ws_envelope(&msg.encode()) {
        eprintln!("[niva] api response dropped, window socket gone");
    }
}

#[derive(PartialEq)]
enum End {
    Done,
    Timeout,
    Cancelled,
}

async fn run_job(job: DispatchJob, active: Arc<Mutex<HashMap<(u8, u64), ActiveCall>>>) {
    let DispatchJob {
        ctx,
        request,
        handler,
        timeout,
    } = job;
    let wid = ctx.window.id;
    let id = ctx.id;
    let watch = ctx.clone();

    let reply = ctx.clone();
    let work = async {
        match handler {
            HandlerKind::Unary(f) => {
                let response = f(ctx.app.clone(), ctx.window.clone(), request.clone()).await;
                respond(&ctx.window, response);
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
        && let Some(call) = active.remove(&(wid, id))
    {
        call.cancel_tx.close();
    }
    if end == End::Timeout {
        respond(&reply.window, request.err(-2, "API timeout".to_string()));
    }
    // Cancelled: client went away, nothing to answer.
}
