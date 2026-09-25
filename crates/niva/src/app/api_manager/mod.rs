pub(crate) mod protocol;

use std::{
    collections::{HashMap, HashSet, hash_map::Entry},
    future::Future,
    io::{self, Write},
    pin::Pin,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering},
        mpsc,
    },
    thread,
    time::{Duration, Instant},
};

use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::{lock, unsafe_impl_sync_send};

use self::protocol::{ClientMsg, ServerMsg, WIRE_VERSION, decode_chunk, encode_chunk};
use super::{
    NivaApp,
    api::module::ModuleResolver,
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
pub type CancellableApiFuture = Pin<Box<dyn Future<Output = Result<Value>> + Send>>;
pub type CancellableApiHandler = Arc<
    dyn Fn(CancellationContext, Arc<NivaApp>, Arc<NivaWindow>, ApiRequest) -> CancellableApiFuture
        + Send
        + Sync,
>;

/// Native provenance supplied by a platform IPC callback. `source_url` is
/// never accepted from the JSON body; frame identity and generation are
/// transport-owned lifecycle keys, not permissions.
#[derive(Clone, Debug)]
pub struct IpcFrameSource {
    pub source_url: String,
    pub frame_id: u64,
    pub generation: u64,
    pub is_main_frame: bool,
}

/// Cancellation context for an explicitly registered bounded unary API.
/// It carries only cancellation ownership; authorization is performed by the
/// transport dispatcher, and it has no stream or binary transport.
#[derive(Clone)]
pub struct IpcCallContext {
    pub app: Arc<NivaApp>,
    pub window: Arc<NivaWindow>,
    pub id: u64,
    pub session_id: String,
    pub source_origin: String,
    pub frame_id: u64,
    pub generation: u64,
    pub is_main_frame: bool,
    cancel_rx: async_channel::Receiver<()>,
}

impl IpcCallContext {
    pub fn is_cancelled(&self) -> bool {
        self.cancel_rx.is_closed()
    }

    pub async fn cancelled(&self) {
        let _ = self.cancel_rx.recv().await;
    }

    pub fn cancellation(&self) -> CancellationContext {
        CancellationContext {
            window_id: self.window.id,
            call_id: self.id,
            owner: ApiCallOwner::Ipc {
                source_origin: self.source_origin.clone(),
                session_id: self.session_id.clone(),
                frame_id: self.frame_id,
                generation: self.generation,
            },
            cancel_rx: self.cancel_rx.clone(),
        }
    }
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub enum ApiCallOwner {
    WebSocket {
        connection_id: u64,
    },
    Synchronous {
        session_id: String,
    },
    Ipc {
        source_origin: String,
        session_id: String,
        frame_id: u64,
        generation: u64,
    },
}

/// Cancellation signal and transport owner for bounded, non-streaming APIs.
/// The owner is used only for cancellation and never for authorization.
#[derive(Clone)]
pub struct CancellationContext {
    pub window_id: u8,
    pub call_id: u64,
    pub owner: ApiCallOwner,
    cancel_rx: async_channel::Receiver<()>,
}

impl CancellationContext {
    pub fn is_cancelled(&self) -> bool {
        self.cancel_rx.is_closed()
    }

    pub async fn cancelled(&self) {
        let _ = self.cancel_rx.recv().await;
    }
}

type IpcApiFuture = Pin<Box<dyn Future<Output = Result<Value>> + Send>>;
type IpcApiHandler = Arc<dyn Fn(IpcCallContext, ApiRequest) -> IpcApiFuture + Send + Sync>;

#[derive(Deserialize)]
#[serde(
    tag = "t",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
enum IpcMessage {
    Call {
        id: u64,
        method: String,
        args: Value,
        session_id: String,
        token: Option<String>,
    },
    Heartbeat {
        session_id: String,
        token: Option<String>,
    },
}

enum HandlerKind {
    Unary(ApiHandler),
    CancellableUnary(CancellableApiHandler),
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
    lifecycle: Arc<CallLifecycle>,
}

impl std::ops::Deref for CallContext {
    type Target = CallInner;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl CallContext {
    pub fn cancellation(&self) -> CancellationContext {
        CancellationContext {
            window_id: self.window.id,
            call_id: self.id,
            owner: ApiCallOwner::WebSocket {
                connection_id: self.connection_id,
            },
            cancel_rx: self.cancel_rx.clone(),
        }
    }

    /// Best-effort stream push tied to this call (dropped when socket gone).
    pub fn push(&self, name: &str, data: Value) {
        if self.inner.lifecycle.is_terminal() {
            return;
        }
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
        if self.inner.lifecycle.is_terminal() {
            return;
        }
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
        self.respond_api_response(ApiResponse(self.id, code, message, data));
    }

    fn respond_api_response(&self, response: ApiResponse) {
        respond_once(&self.inner.ws_tx, &self.inner.lifecycle, response);
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
    ws_tx: mpsc::Sender<WsOut>,
    lifecycle: Arc<CallLifecycle>,
}

type CallKey = (u8, u64, u64);
type ActiveCalls = HashMap<CallKey, ActiveCall>;

#[derive(Clone, Copy, PartialEq, Eq)]
enum CallPhase {
    Queued,
    Running,
    Cancelled,
}

struct CallLifecycle {
    phase: Mutex<CallPhase>,
    terminal: AtomicBool,
}

impl CallLifecycle {
    fn new() -> Self {
        Self {
            phase: Mutex::new(CallPhase::Queued),
            terminal: AtomicBool::new(false),
        }
    }

    /// Linearize start against owner cancellation. Once this succeeds, the
    /// handler is considered started and cancellation cannot promise rollback.
    fn start(&self) -> bool {
        let Ok(mut phase) = self.phase.lock() else {
            self.cancel();
            return false;
        };
        if *phase != CallPhase::Queued {
            return false;
        }
        *phase = CallPhase::Running;
        true
    }

    fn cancel(&self) {
        if let Ok(mut phase) = self.phase.lock() {
            *phase = CallPhase::Cancelled;
            self.terminal.store(true, Ordering::Release);
        } else {
            self.terminal.store(true, Ordering::Release);
        }
    }

    fn stop_before_start(&self) {
        if let Ok(mut phase) = self.phase.lock() {
            *phase = CallPhase::Cancelled;
        } else {
            self.terminal.store(true, Ordering::Release);
        }
    }

    fn is_terminal(&self) -> bool {
        self.terminal.load(Ordering::Acquire)
    }

    fn claim_terminal(&self) -> bool {
        self.terminal
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_ok()
    }
}

struct DispatchJob {
    ctx: CallContext,
    request: ApiRequest,
    handler: HandlerKind,
    timeout: Option<Duration>,
}

type SyncCallKey = (u8, String, u64);
type SyncCalls = HashMap<SyncCallKey, async_channel::Sender<()>>;
type WsSessionKey = (u8, u64);
type WsSessions = HashMap<WsSessionKey, String>;

struct SyncCallRegistration {
    active: Arc<Mutex<SyncCalls>>,
    key: SyncCallKey,
    cancel_tx: async_channel::Sender<()>,
    completed: bool,
}

impl Drop for SyncCallRegistration {
    fn drop(&mut self) {
        crate::app::api::cancel_sync_process_call(self.key.0, &self.key.1, self.key.2);
        self.cancel_tx.close();
        if !self.completed {
            crate::app::api::cancel_sync_file_session(self.key.0, &self.key.1);
        }
        if let Ok(mut active) = self.active.lock() {
            active.remove(&self.key);
        }
    }
}

unsafe_impl_sync_send!(ApiManager);
pub struct ApiManager {
    app: std::sync::OnceLock<Arc<NivaApp>>,
    module_resolver: std::sync::OnceLock<Arc<ModuleResolver>>,
    handlers: HashMap<String, HandlerEntry>,
    ipc_handlers: HashMap<String, IpcApiHandler>,
    dispatch_tx: async_channel::Sender<DispatchJob>,
    default_timeout: Duration,
    active: Arc<Mutex<ActiveCalls>>,
    ipc_sessions: Arc<Mutex<IpcSessions>>,
    sync_active: Arc<Mutex<SyncCalls>>,
    ws_sessions: Mutex<WsSessions>,
    ipc_active: AtomicUsize,
    max_ipc_active: usize,
}

struct IpcPermit<'a>(&'a AtomicUsize);

impl Drop for IpcPermit<'_> {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::AcqRel);
    }
}

struct LimitedJsonWriter {
    bytes: Vec<u8>,
    limit: usize,
}

impl LimitedJsonWriter {
    fn new(limit: usize) -> Self {
        Self {
            bytes: Vec::with_capacity(limit.min(64 * 1024)),
            limit,
        }
    }
}

impl Write for LimitedJsonWriter {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        let remaining = self.limit.saturating_sub(self.bytes.len());
        let written = bytes.len().min(remaining);
        self.bytes.extend_from_slice(&bytes[..written]);
        Ok(written)
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

const IPC_SESSION_LEASE: Duration = Duration::from_secs(3);
const IPC_SESSION_CHECK_INTERVAL: Duration = Duration::from_millis(250);
const MAX_EXPIRED_IPC_SESSIONS_PER_WINDOW: usize = 4096;
const MAX_IPC_REQUEST_BYTES: usize = 256 * 1024;
const MAX_IPC_RESPONSE_BYTES: usize = 8 * 1024 * 1024;
const MAX_IPC_CALL_DURATION: Duration = Duration::from_secs(35);
const MAX_SYNC_RESPONSE_BYTES: usize = 100 * 1024 * 1024;

/// Explicit public IPC surface. Add methods here only after checking their
/// JSON/ownership behavior; this is intentionally not a namespace wildcard.
const IPC_METHOD_ALLOWLIST: &[&str] = &[
    // Bounded text/data fallback APIs.
    "fs.readText",
    "fs.writeText",
    "fs.appendText",
    "fs.node",
    "http.requestText",
    "process.execText",
    "process.execFileText",
    // Window state/control operations with no persistent child resource.
    "window.current",
    "window.close",
    "window.list",
    "window.scaleFactor",
    "window.innerPosition",
    "window.outerPosition",
    "window.setOuterPosition",
    "window.innerSize",
    "window.setInnerSize",
    "window.outerSize",
    "window.setMinInnerSize",
    "window.setMaxInnerSize",
    "window.setTheme",
    "window.setProgressBar",
    "window.requestRedraw",
    "window.setImePosition",
    "window.setBackgroundColor",
    "window.setFocusable",
    "window.title",
    "window.setTitle",
    "window.isVisible",
    "window.setVisible",
    "window.isFocused",
    "window.setFocus",
    "window.isResizable",
    "window.setResizable",
    "window.isMinimizable",
    "window.setMinimizable",
    "window.isMaximizable",
    "window.setMaximizable",
    "window.isClosable",
    "window.setClosable",
    "window.isMinimized",
    "window.setMinimized",
    "window.isMaximized",
    "window.setMaximized",
    "window.isDecorated",
    "window.setDecorated",
    "window.fullscreen",
    "window.setFullscreen",
    "window.setAlwaysOnTop",
    "window.setAlwaysOnBottom",
    "window.requestUserAttention",
    "window.setContentProtection",
    "window.setCursorIcon",
    "window.cursorPosition",
    "window.setCursorPosition",
    "window.setCursorGrab",
    "window.setCursorVisible",
    "window.setIgnoreCursorEvents",
    "window.theme",
    "window.isMenuVisible",
    // Simple host interaction and read-only display information.
    "clipboard.read",
    "clipboard.write",
    "dialog.showMessage",
    "dialog.pickFile",
    "dialog.pickFiles",
    "dialog.pickDir",
    "dialog.pickDirs",
    "dialog.saveFile",
    "monitor.list",
    "monitor.current",
    "monitor.primary",
    "monitor.fromPoint",
    "os.dirs",
    // Navigation/history and basic WebView status only.
    "webview.isDevtoolsOpen",
    "webview.url",
    "webview.reload",
    "webview.goBack",
    "webview.goForward",
    "webview.canGoBack",
    "webview.canGoForward",
];

const IPC_LEGACY_UNARY_METHODS: &[&str] = &[
    "window.current",
    "window.close",
    "window.list",
    "window.scaleFactor",
    "window.innerPosition",
    "window.outerPosition",
    "window.setOuterPosition",
    "window.innerSize",
    "window.setInnerSize",
    "window.outerSize",
    "window.setMinInnerSize",
    "window.setMaxInnerSize",
    "window.setTheme",
    "window.setProgressBar",
    "window.requestRedraw",
    "window.setImePosition",
    "window.setBackgroundColor",
    "window.setFocusable",
    "window.title",
    "window.setTitle",
    "window.isVisible",
    "window.setVisible",
    "window.isFocused",
    "window.setFocus",
    "window.isResizable",
    "window.setResizable",
    "window.isMinimizable",
    "window.setMinimizable",
    "window.isMaximizable",
    "window.setMaximizable",
    "window.isClosable",
    "window.setClosable",
    "window.isMinimized",
    "window.setMinimized",
    "window.isMaximized",
    "window.setMaximized",
    "window.isDecorated",
    "window.setDecorated",
    "window.fullscreen",
    "window.setFullscreen",
    "window.setAlwaysOnTop",
    "window.setAlwaysOnBottom",
    "window.requestUserAttention",
    "window.setContentProtection",
    "window.setCursorIcon",
    "window.cursorPosition",
    "window.setCursorPosition",
    "window.setCursorGrab",
    "window.setCursorVisible",
    "window.setIgnoreCursorEvents",
    "window.theme",
    "window.isMenuVisible",
    "clipboard.read",
    "clipboard.write",
    "dialog.showMessage",
    "dialog.pickFile",
    "dialog.pickFiles",
    "dialog.pickDir",
    "dialog.pickDirs",
    "dialog.saveFile",
    "monitor.list",
    "monitor.current",
    "monitor.primary",
    "monitor.fromPoint",
    "os.dirs",
    "webview.isDevtoolsOpen",
    "webview.url",
    "webview.reload",
    "webview.goBack",
    "webview.goForward",
    "webview.canGoBack",
    "webview.canGoForward",
];

enum IpcRunOutcome {
    Finished(Result<Value>),
    Cancelled,
    TimedOut,
}

fn validate_session_id(session_id: &str) -> Result<()> {
    anyhow::ensure!(
        session_id.len() == 32
            && session_id
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase()),
        "invalid IPC session id"
    );
    Ok(())
}

fn heartbeat_error(session_id: &str) -> Value {
    json!({
        "t": "heartbeatError",
        "sessionId": session_id,
        "code": "ERR_NIVA_SESSION_EXPIRED",
        "message": "IPC session expired; reload the page",
    })
}

fn heartbeat_failure(session_id: &str, code: &str, message: &str) -> String {
    serde_json::to_string(&json!({
        "t": "heartbeatError",
        "sessionId": session_id,
        "code": code,
        "message": message,
    }))
    .unwrap_or_else(|_| "{\"t\":\"heartbeatError\"}".into())
}

fn ipc_method_allowed(method: &str) -> bool {
    IPC_METHOD_ALLOWLIST.contains(&method)
}

fn legacy_unary_ipc_handler(handler: ApiHandler) -> IpcApiHandler {
    Arc::new(move |context, request| {
        let handler = handler.clone();
        Box::pin(async move {
            let response = handler(context.app.clone(), context.window.clone(), request).await;
            if response.1 == 0 {
                Ok(response.3)
            } else {
                Err(anyhow!(response.2))
            }
        }) as IpcApiFuture
    })
}

fn cancellable_ipc_handler(handler: CancellableApiHandler) -> IpcApiHandler {
    Arc::new(move |context, request| {
        let handler = handler.clone();
        Box::pin(async move {
            handler(
                context.cancellation(),
                context.app.clone(),
                context.window.clone(),
                request,
            )
            .await
        }) as IpcApiFuture
    })
}

fn authorize_ipc_source(
    window: &NivaWindow,
    source_url: &str,
    token: Option<&str>,
    method: Option<&str>,
) -> Result<String> {
    use subtle::ConstantTimeEq;

    let url = url::Url::parse(source_url).map_err(|_| anyhow!("invalid IPC source URL"))?;
    let origin = normalize_ipc_source_origin(&url)?;
    let decoded_path = super::resource_manager::percent_decode_path(url.path())
        .map_err(|_| anyhow!("invalid IPC source path"))?;
    anyhow::ensure!(
        !decoded_path
            .split('/')
            .any(|segment| segment.eq_ignore_ascii_case("__niva_fs")),
        "__niva_fs pages cannot use IPC"
    );

    if window.trusted_ws_origin.as_deref() == Some(origin.as_str()) {
        let supplied = token.unwrap_or_default().as_bytes();
        let expected = window.token.as_bytes();
        anyhow::ensure!(
            supplied.len() == expected.len() && bool::from(supplied.ct_eq(expected)),
            "trusted local IPC requires the window token"
        );
        return Ok(origin);
    }

    let allowed = match method {
        Some(method) => window.permissions.allows(source_url, method),
        None => window.permissions.allows_origin(source_url),
    };
    anyhow::ensure!(allowed, "permission denied");
    Ok(origin)
}

fn normalize_ipc_source_origin(url: &url::Url) -> Result<String> {
    anyhow::ensure!(
        url.username().is_empty() && url.password().is_none(),
        "IPC source URLs cannot contain credentials"
    );
    #[cfg(target_os = "macos")]
    if url.scheme() == "niva" {
        anyhow::ensure!(
            url.host_str() == Some("app") && url.port().is_none(),
            "only niva://app is a trusted custom-protocol source"
        );
        return Ok(super::custom_protocol::platform_origin().to_owned());
    }
    let origin = url.origin().ascii_serialization();
    anyhow::ensure!(origin != "null", "opaque IPC source is denied");
    Ok(origin)
}

async fn monitor_ipc_call(
    sessions: Arc<Mutex<IpcSessions>>,
    key: IpcSessionKey,
    call_id: u64,
    cancel_rx: async_channel::Receiver<()>,
) -> IpcRunOutcome {
    let started = Instant::now();
    loop {
        let event = smol::future::or(
            async {
                let _ = cancel_rx.recv().await;
                false
            },
            async {
                smol::Timer::after(IPC_SESSION_CHECK_INTERVAL).await;
                true
            },
        )
        .await;
        if !event {
            return IpcRunOutcome::Cancelled;
        }

        if started.elapsed() >= MAX_IPC_CALL_DURATION {
            crate::app::api::cancel_ipc_process_call(
                key.window_id,
                &key.source_origin,
                &key.session_id,
                key.frame_id,
                key.generation,
                call_id,
            );
            if let Ok(mut sessions) = sessions.lock()
                && let Some(sender) = sessions.cancel_call(&key, call_id)
            {
                sender.close();
            }
            return IpcRunOutcome::TimedOut;
        }

        let result = match sessions.lock() {
            Ok(mut sessions) => sessions.expire_if_stale(&key, Instant::now()),
            Err(_) => return IpcRunOutcome::Cancelled,
        };
        match result {
            Ok(false) => {}
            Ok(true) => return IpcRunOutcome::Cancelled,
            Err(senders) => {
                crate::app::api::cancel_ipc_process_session(
                    key.window_id,
                    &key.source_origin,
                    &key.session_id,
                    key.frame_id,
                    key.generation,
                );
                for sender in senders {
                    sender.close();
                }
                return IpcRunOutcome::Cancelled;
            }
        }
    }
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
struct IpcSessionKey {
    window_id: u8,
    source_origin: String,
    session_id: String,
    frame_id: u64,
    generation: u64,
    is_main_frame: bool,
}

struct ActiveIpcSession {
    last_heartbeat: Instant,
    calls: HashMap<u64, async_channel::Sender<()>>,
}

#[derive(Default)]
struct IpcSessions {
    active: HashMap<IpcSessionKey, ActiveIpcSession>,
    expired: HashSet<IpcSessionKey>,
    expired_per_window: HashMap<u8, usize>,
    blocked_windows: HashSet<u8>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum IpcSessionError {
    Expired,
    DuplicateCall,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum IpcHeartbeat {
    Active,
    Idle,
    Expired,
}

impl IpcSessions {
    fn reserve(
        &mut self,
        key: IpcSessionKey,
        call_id: u64,
        cancel_tx: async_channel::Sender<()>,
        now: Instant,
    ) -> std::result::Result<(), IpcSessionError> {
        if self.blocked_windows.contains(&key.window_id) || self.expired.contains(&key) {
            return Err(IpcSessionError::Expired);
        }
        let session = self.active.entry(key).or_insert_with(|| ActiveIpcSession {
            last_heartbeat: now,
            calls: HashMap::new(),
        });
        if session.calls.contains_key(&call_id) {
            return Err(IpcSessionError::DuplicateCall);
        }
        session.last_heartbeat = now;
        session.calls.insert(call_id, cancel_tx);
        Ok(())
    }

    fn heartbeat(&mut self, key: &IpcSessionKey, now: Instant) -> IpcHeartbeat {
        if self.blocked_windows.contains(&key.window_id) || self.expired.contains(key) {
            return IpcHeartbeat::Expired;
        }
        if let Some(session) = self.active.get_mut(key) {
            session.last_heartbeat = now;
            IpcHeartbeat::Active
        } else {
            // An ack can race with the final result. Idle sessions own no
            // cancellable resource, so acknowledging them cannot renew work.
            IpcHeartbeat::Idle
        }
    }

    fn complete(&mut self, key: &IpcSessionKey, call_id: u64) {
        let remove_session = if let Some(session) = self.active.get_mut(key) {
            if let Some(sender) = session.calls.remove(&call_id) {
                sender.close();
            }
            session.calls.is_empty()
        } else {
            false
        };
        if remove_session {
            self.active.remove(key);
        }
    }

    fn cancel_call(
        &mut self,
        key: &IpcSessionKey,
        call_id: u64,
    ) -> Option<async_channel::Sender<()>> {
        let session = self.active.get_mut(key)?;
        let sender = session.calls.remove(&call_id);
        if session.calls.is_empty() {
            self.active.remove(key);
        }
        sender
    }

    fn expire_if_stale(
        &mut self,
        key: &IpcSessionKey,
        now: Instant,
    ) -> std::result::Result<bool, Vec<async_channel::Sender<()>>> {
        let Some(session) = self.active.get(key) else {
            return Ok(self.expired.contains(key) || self.blocked_windows.contains(&key.window_id));
        };
        if now.saturating_duration_since(session.last_heartbeat) < IPC_SESSION_LEASE {
            return Ok(false);
        }
        let session = self.active.remove(key).expect("session checked above");
        self.remember_expired(key.clone());
        Err(session.calls.into_values().collect())
    }

    fn cancel_matching(
        &mut self,
        mut matches: impl FnMut(&IpcSessionKey) -> bool,
        tombstone: bool,
    ) -> Vec<async_channel::Sender<()>> {
        let keys = self
            .active
            .keys()
            .filter(|key| matches(key))
            .cloned()
            .collect::<Vec<_>>();
        let mut cancellations = Vec::new();
        for key in keys {
            if let Some(session) = self.active.remove(&key) {
                cancellations.extend(session.calls.into_values());
            }
            if tombstone {
                self.remember_expired(key);
            }
        }
        cancellations
    }

    fn remember_expired(&mut self, key: IpcSessionKey) {
        if self.expired.contains(&key) {
            return;
        }
        let count = self.expired_per_window.entry(key.window_id).or_default();
        if *count >= MAX_EXPIRED_IPC_SESSIONS_PER_WINDOW {
            // Preserve the no-revival guarantee without unbounded tombstones.
            // This window stays fail-closed until it is closed.
            self.blocked_windows.insert(key.window_id);
            return;
        }
        *count += 1;
        self.expired.insert(key);
    }

    fn close_window(&mut self, window_id: u8) -> Vec<async_channel::Sender<()>> {
        let cancellations = self.cancel_matching(|key| key.window_id == window_id, false);
        self.expired.retain(|key| key.window_id != window_id);
        self.expired_per_window.remove(&window_id);
        self.blocked_windows.remove(&window_id);
        cancellations
    }
}

fn claim_terminal(lifecycle: &CallLifecycle) -> bool {
    lifecycle.claim_terminal()
}

fn respond_once(
    tx: &mpsc::Sender<WsOut>,
    lifecycle: &CallLifecycle,
    response: ApiResponse,
) -> bool {
    if !claim_terminal(lifecycle) {
        return false;
    }
    respond(tx, response);
    true
}

fn notify_duplicate_active_id(tx: &mpsc::Sender<WsOut>, seq: u64, request_id: u64) {
    let protocol_error = ServerMsg::protocol_error(
        seq,
        request_id,
        "ERR_DUPLICATE_ACTIVE_REQUEST_ID",
        "request id is already active on this connection",
    );
    let _ = tx.send(WsOut::Text(protocol_error.encode()));
}

fn cancel_active_call(call: ActiveCall) {
    call.lifecycle.cancel();
    call.cancel_tx.close();
}

fn reserve_active_call(active: &mut ActiveCalls, key: CallKey, call: ActiveCall) -> bool {
    match active.entry(key) {
        Entry::Vacant(entry) => {
            entry.insert(call);
            true
        }
        Entry::Occupied(_) => {
            cancel_active_call(call);
            false
        }
    }
}

fn take_active_calls(
    active: &mut ActiveCalls,
    mut should_remove: impl FnMut(&CallKey) -> bool,
) -> Vec<ActiveCall> {
    let keys = active
        .keys()
        .copied()
        .filter(|key| should_remove(key))
        .collect::<Vec<_>>();
    keys.into_iter()
        .filter_map(|key| {
            let call = active.remove(&key)?;
            call.lifecycle.cancel();
            Some(call)
        })
        .collect()
}

fn remove_active_if_same(
    active: &Mutex<ActiveCalls>,
    key: CallKey,
    lifecycle: &Arc<CallLifecycle>,
) -> Option<ActiveCall> {
    let mut active = match active.lock() {
        Ok(active) => active,
        Err(err) => {
            crate::niva_log!(
                crate::app::logging::Level::Warn,
                "[niva] api active table poisoned during rollback: {err}"
            );
            return None;
        }
    };
    if active
        .get(&key)
        .is_some_and(|call| Arc::ptr_eq(&call.lifecycle, lifecycle))
    {
        let call = active.remove(&key);
        if let Some(call) = &call {
            call.lifecycle.stop_before_start();
        }
        call
    } else {
        None
    }
}

fn enqueue_or_reject<T>(
    dispatch_tx: &async_channel::Sender<T>,
    job: T,
    active: &Mutex<ActiveCalls>,
    key: CallKey,
    lifecycle: Arc<CallLifecycle>,
    cancel_tx: async_channel::Sender<()>,
    on_reject: impl FnOnce(T, bool),
) -> bool {
    match dispatch_tx.try_send(job) {
        Ok(()) => true,
        Err(err) => {
            let job = err.into_inner();
            remove_active_if_same(active, key, &lifecycle);
            let should_respond = claim_terminal(&lifecycle);
            lifecycle.cancel();
            cancel_tx.close();
            on_reject(job, should_respond);
            false
        }
    }
}

fn reject_reserved_call(
    active: &Mutex<ActiveCalls>,
    key: CallKey,
    lifecycle: Arc<CallLifecycle>,
    cancel_tx: async_channel::Sender<()>,
    tx: &mpsc::Sender<WsOut>,
    response: ApiResponse,
) {
    remove_active_if_same(active, key, &lifecycle);
    let should_respond = claim_terminal(&lifecycle);
    lifecycle.cancel();
    cancel_tx.close();
    if should_respond {
        respond(tx, response);
    }
}

fn route_inbound_chunk(
    active: &Mutex<ActiveCalls>,
    window_id: u8,
    connection_id: u64,
    header: self::protocol::ChunkHeader,
    payload: &[u8],
) {
    let key = (window_id, connection_id, header.id);
    let failure = {
        let mut calls = match active.lock() {
            Ok(calls) => calls,
            Err(err) => {
                crate::niva_log!(
                    crate::app::logging::Level::Warn,
                    "[niva] api active table poisoned: {err}"
                );
                return;
            }
        };
        let Some(call) = calls.get_mut(&key) else {
            crate::niva_log!(
                crate::app::logging::Level::Warn,
                "[niva] api chunk for unknown call {}",
                header.id
            );
            return;
        };
        if call.lifecycle.is_terminal() {
            return;
        }
        let chunk = InboundChunk {
            seq: header.seq,
            data: payload.to_vec(),
            end: header.end,
        };
        let message = match call.inbound_tx.try_send(chunk) {
            Ok(()) => return,
            Err(async_channel::TrySendError::Full(_)) => "inbound stream queue full",
            Err(async_channel::TrySendError::Closed(_)) => "inbound stream consumer closed",
        };
        let Some(call) = calls.remove(&key) else {
            return;
        };
        let should_respond = claim_terminal(&call.lifecycle);
        call.lifecycle.cancel();
        Some((call, message, should_respond))
    };

    if let Some((call, message, should_respond)) = failure {
        call.cancel_tx.close();
        if should_respond {
            respond(
                &call.ws_tx,
                ApiResponse(header.id, -3, message.to_string(), json!(null)),
            );
        }
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
            module_resolver: std::sync::OnceLock::new(),
            handlers: HashMap::new(),
            ipc_handlers: HashMap::new(),
            dispatch_tx,
            default_timeout,
            active,
            ipc_sessions: Arc::new(Mutex::new(IpcSessions::default())),
            sync_active: Arc::new(Mutex::new(HashMap::new())),
            ws_sessions: Mutex::new(HashMap::new()),
            ipc_active: AtomicUsize::new(0),
            max_ipc_active: max_queue.max(1),
        }
    }

    /// Set once during startup; lock-free reads afterwards.
    pub fn bind_app(&self, app: Arc<NivaApp>) {
        let _ = self.module_resolver.set(Arc::new(ModuleResolver::new(
            app.resource(),
            app.launch_info.uuid.clone(),
        )));
        let _ = self.app.set(app);
    }

    pub fn module_resolver(&self) -> Result<Arc<ModuleResolver>> {
        self.module_resolver
            .get()
            .cloned()
            .ok_or_else(|| anyhow!("module resolver not initialized"))
    }

    /// Handle one platform-provided WebView IPC message. Source URL and frame
    /// identity are supplied by the native callback, never by the JSON body.
    /// IPC exposes an exact JSON-only allowlist; no stream or binary handler
    /// can be reached through this method.
    pub async fn ipc_message(
        &self,
        window_id: u8,
        frame: IpcFrameSource,
        raw_body: &str,
    ) -> Result<String> {
        if raw_body.len() > MAX_IPC_REQUEST_BYTES {
            return Err(anyhow!("IPC request exceeds 256 KiB"));
        }
        let message: IpcMessage = serde_json::from_str(raw_body)?;
        match message {
            IpcMessage::Heartbeat { session_id, token } => {
                self.ipc_heartbeat(window_id, frame, &session_id, token.as_deref())
            }
            IpcMessage::Call {
                id,
                method,
                args,
                session_id,
                token,
            } => {
                self.ipc_call(
                    window_id,
                    frame,
                    id,
                    method,
                    args,
                    session_id,
                    token.as_deref(),
                )
                .await
            }
        }
    }

    fn ipc_heartbeat(
        &self,
        window_id: u8,
        frame: IpcFrameSource,
        session_id: &str,
        token: Option<&str>,
    ) -> Result<String> {
        let app = self
            .app
            .get()
            .cloned()
            .ok_or_else(|| anyhow!("API manager not ready"))?;
        let window = match app
            .window()
            .and_then(|manager| manager.get_window(window_id))
        {
            Ok(window) => window,
            Err(_) => {
                return Ok(heartbeat_failure(
                    session_id,
                    "ERR_NIVA_IPC_DENIED",
                    "IPC window is unavailable",
                ));
            }
        };
        let source_origin = match authorize_ipc_source(&window, &frame.source_url, token, None) {
            Ok(origin) => origin,
            Err(_) => {
                return Ok(heartbeat_failure(
                    session_id,
                    "ERR_NIVA_IPC_DENIED",
                    "permission denied",
                ));
            }
        };
        if validate_session_id(session_id).is_err() {
            return Ok(heartbeat_failure(
                session_id,
                "ERR_NIVA_IPC_DENIED",
                "invalid IPC session id",
            ));
        }
        let key = IpcSessionKey {
            window_id,
            source_origin,
            session_id: session_id.to_owned(),
            frame_id: frame.frame_id,
            generation: frame.generation,
            is_main_frame: frame.is_main_frame,
        };
        let status = self
            .ipc_sessions
            .lock()
            .map_err(|_| anyhow!("IPC session table poisoned"))?
            .heartbeat(&key, Instant::now());
        let response = match status {
            IpcHeartbeat::Active | IpcHeartbeat::Idle => json!({
                "t": "heartbeatAck",
                "sessionId": session_id,
                "expiresInMs": IPC_SESSION_LEASE.as_millis(),
            }),
            IpcHeartbeat::Expired => heartbeat_error(session_id),
        };
        Ok(serde_json::to_string(&response)?)
    }

    async fn ipc_call(
        &self,
        window_id: u8,
        frame: IpcFrameSource,
        id: u64,
        method: String,
        args: Value,
        session_id: String,
        token: Option<&str>,
    ) -> Result<String> {
        let request = ApiRequest(id, method, ApiArguments(args));
        let denied = |code, message: &str, data| {
            ServerMsg::result(id, code, message.to_owned(), data).encode()
        };
        validate_session_id(&session_id).map_err(|_| anyhow!("invalid IPC session id"))?;

        let previous = self.ipc_active.fetch_add(1, Ordering::AcqRel);
        if previous >= self.max_ipc_active {
            self.ipc_active.fetch_sub(1, Ordering::AcqRel);
            return Ok(denied(-3, "IPC server busy", json!(null)));
        }
        let _permit = IpcPermit(&self.ipc_active);

        let app = self
            .app
            .get()
            .cloned()
            .ok_or_else(|| anyhow!("API manager not ready"))?;
        let window = app.window()?.get_window(window_id)?;
        let source_origin =
            match authorize_ipc_source(&window, &frame.source_url, token, Some(&request.1)) {
                Ok(origin) => origin,
                Err(_) => return Ok(denied(-4, "permission denied", json!(null))),
            };
        if matches!(
            request.1.as_str(),
            "window.open" | "webview.baseFileSystemUrl"
        ) || !ipc_method_allowed(&request.1)
        {
            return Ok(denied(-4, "permission denied", json!(null)));
        }
        let handler = self.ipc_handlers.get(&request.1).cloned().or_else(|| {
            match self.handlers.get(&request.1).map(|entry| &entry.handler) {
                Some(HandlerKind::CancellableUnary(handler)) => {
                    Some(cancellable_ipc_handler(handler.clone()))
                }
                Some(HandlerKind::Unary(handler))
                    if IPC_LEGACY_UNARY_METHODS.contains(&request.1.as_str()) =>
                {
                    Some(legacy_unary_ipc_handler(handler.clone()))
                }
                Some(HandlerKind::Unary(_)) | Some(HandlerKind::Stream(_)) | None => None,
            }
        });
        let Some(handler) = handler else {
            return Ok(denied(-4, "API unavailable over IPC", json!(null)));
        };
        let key = IpcSessionKey {
            window_id,
            source_origin,
            session_id: session_id.clone(),
            frame_id: frame.frame_id,
            generation: frame.generation,
            is_main_frame: frame.is_main_frame,
        };
        let (cancel_tx, cancel_rx) = async_channel::bounded(1);
        match self
            .ipc_sessions
            .lock()
            .map_err(|_| anyhow!("IPC session table poisoned"))?
            .reserve(key.clone(), id, cancel_tx.clone(), Instant::now())
        {
            Ok(()) => {}
            Err(IpcSessionError::Expired) => {
                return Ok(denied(
                    -1,
                    "ERR_NIVA_SESSION_EXPIRED: reload the page before retrying",
                    json!({"code":"ERR_NIVA_SESSION_EXPIRED"}),
                ));
            }
            Err(IpcSessionError::DuplicateCall) => {
                return Ok(denied(-1, "duplicate IPC request id", json!(null)));
            }
        }

        let context = IpcCallContext {
            app,
            window,
            id,
            session_id,
            source_origin: key.source_origin.clone(),
            frame_id: key.frame_id,
            generation: key.generation,
            is_main_frame: frame.is_main_frame,
            cancel_rx: cancel_rx.clone(),
        };
        let work = handler(context, request.clone());
        let monitor = monitor_ipc_call(self.ipc_sessions.clone(), key.clone(), id, cancel_rx);
        let outcome =
            smol::future::or(async move { IpcRunOutcome::Finished(work.await) }, monitor).await;

        let response = match outcome {
            IpcRunOutcome::Finished(Ok(value)) => ServerMsg::result(id, 0, "ok".into(), value),
            IpcRunOutcome::Finished(Err(error)) => {
                ServerMsg::result(id, -1, error.to_string(), native_error_data(&error))
            }
            IpcRunOutcome::Cancelled => ServerMsg::result(
                id,
                -1,
                "ERR_NIVA_SESSION_EXPIRED: IPC task was cancelled".into(),
                json!({"code":"ERR_NIVA_SESSION_EXPIRED"}),
            ),
            IpcRunOutcome::TimedOut => ServerMsg::result(
                id,
                -2,
                "ETIMEDOUT: IPC request exceeded its maximum duration".into(),
                json!({"code":"ETIMEDOUT"}),
            ),
        };
        if let Ok(mut sessions) = self.ipc_sessions.lock() {
            sessions.complete(&key, id);
        }
        cancel_tx.close();
        let encoded = response.encode();
        if encoded.len() > MAX_IPC_RESPONSE_BYTES {
            return Ok(denied(
                -1,
                "IPC response exceeds 8 MiB",
                json!({"code":"ERR_IPC_RESPONSE_TOO_LARGE"}),
            ));
        }
        Ok(encoded)
    }

    /// Cancel active IPC work from one native WebView2 frame generation.
    pub fn cancel_ipc_frame(&self, window_id: u8, frame_id: u64, generation: u64) {
        crate::app::api::cancel_ipc_process_frame(window_id, frame_id, generation);
        let senders = match self.ipc_sessions.lock() {
            Ok(mut sessions) => sessions.cancel_matching(
                |key| {
                    key.window_id == window_id
                        && key.frame_id == frame_id
                        && key.generation == generation
                },
                true,
            ),
            Err(_) => return,
        };
        for sender in senders {
            sender.close();
        }
    }

    /// Cancel all IPC sessions when the top-level document navigates. The
    /// replacement document receives a new opaque session id.
    pub fn cancel_ipc_window_for_navigation(&self, window_id: u8) {
        crate::app::api::cancel_ipc_process_window(window_id);
        let senders = match self.ipc_sessions.lock() {
            Ok(mut sessions) => sessions.cancel_matching(|key| key.window_id == window_id, true),
            Err(_) => return,
        };
        for sender in senders {
            sender.close();
        }
    }

    fn cancel_ipc_window(&self, window_id: u8) {
        crate::app::api::cancel_ipc_process_window(window_id);
        let senders = match self.ipc_sessions.lock() {
            Ok(mut sessions) => sessions.close_window(window_id),
            Err(_) => return,
        };
        for sender in senders {
            sender.close();
        }
    }

    fn cancel_sync_window(&self, window_id: u8) {
        let senders = self
            .sync_active
            .lock()
            .map(|mut active| {
                let keys = active
                    .keys()
                    .filter(|key| key.0 == window_id)
                    .cloned()
                    .collect::<Vec<_>>();
                keys.into_iter()
                    .filter_map(|key| active.remove(&key))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        for sender in senders {
            sender.close();
        }
        crate::app::api::cancel_sync_file_window(window_id);
    }

    fn cancel_sync_session(&self, window_id: u8, session_id: &str) {
        crate::app::api::cancel_sync_process_session(window_id, session_id);
        let senders = self
            .sync_active
            .lock()
            .map(|mut active| {
                let keys = active
                    .keys()
                    .filter(|key| key.0 == window_id && key.1 == session_id)
                    .cloned()
                    .collect::<Vec<_>>();
                keys.into_iter()
                    .filter_map(|key| active.remove(&key))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        for sender in senders {
            sender.close();
        }
        crate::app::api::cancel_sync_file_session(window_id, session_id);
    }

    /// Called only after the loopback HTTP endpoint authenticates window token
    /// and exact origin. UI/main-thread handlers must never run while XHR blocks
    /// that thread; socket streams likewise require their WS connection identity.
    pub async fn sync_call(
        &self,
        window_id: u8,
        session_id: &str,
        request: ApiRequest,
    ) -> Result<String> {
        let app = self
            .app
            .get()
            .cloned()
            .ok_or_else(|| anyhow!("API manager not ready"))?;
        let window = app.window()?.get_window(window_id)?;
        let response = if !sync_method_allowed(&request.1) {
            request.err(-4, "API unavailable over synchronous XHR")
        } else {
            let previous = self.ipc_active.fetch_add(1, Ordering::AcqRel);
            if previous >= self.max_ipc_active {
                self.ipc_active.fetch_sub(1, Ordering::AcqRel);
                request.err(-3, "API server busy")
            } else {
                let _permit = IpcPermit(&self.ipc_active);
                match self.handlers.get(&request.1).map(|entry| &entry.handler) {
                    Some(HandlerKind::Unary(handler)) => {
                        // Blocking OS calls run on the existing blocking pool.
                        // A timeout is not cancellation and could leave a
                        // mutation running, so wait for a definitive result.
                        handler(app, window, request.clone()).await
                    }
                    Some(HandlerKind::CancellableUnary(handler)) => {
                        let handler = handler.clone();
                        if validate_session_id(session_id).is_err() {
                            request.err(-4, "invalid synchronous session id")
                        } else {
                            let key = (window_id, session_id.to_owned(), request.0);
                            let (cancel_tx, cancel_rx) = async_channel::bounded(1);
                            let inserted = match self.sync_active.lock() {
                                Ok(mut active) if !active.contains_key(&key) => {
                                    active.insert(key.clone(), cancel_tx.clone());
                                    true
                                }
                                _ => false,
                            };
                            if !inserted {
                                request.err(-1, "duplicate active synchronous request id")
                            } else {
                                let mut registration = SyncCallRegistration {
                                    active: self.sync_active.clone(),
                                    key,
                                    cancel_tx,
                                    completed: false,
                                };
                                let still_current = app
                                    .window()
                                    .and_then(|manager| manager.get_window(window_id))
                                    .is_ok_and(|current| Arc::ptr_eq(&current, &window));
                                if !still_current {
                                    request.err(-1, "synchronous request owner window closed")
                                } else {
                                    let context = CancellationContext {
                                        window_id,
                                        call_id: request.0,
                                        owner: ApiCallOwner::Synchronous {
                                            session_id: session_id.to_owned(),
                                        },
                                        cancel_rx,
                                    };
                                    let result =
                                        handler(context, app, window, request.clone()).await;
                                    registration.completed = true;
                                    match result {
                                        Ok(value) => request.ok(value),
                                        Err(error) => ApiResponse(
                                            request.0,
                                            -1,
                                            error.to_string(),
                                            native_error_data(&error),
                                        ),
                                    }
                                }
                            }
                        }
                    }
                    Some(HandlerKind::Stream(_)) | None => {
                        request.err(-4, "API is not a synchronous unary handler")
                    }
                }
            }
        };
        let mut writer = LimitedJsonWriter::new(MAX_SYNC_RESPONSE_BYTES);
        if serde_json::to_writer(&mut writer, &response).is_ok() {
            // The serializer only writes valid UTF-8 JSON bytes.
            return String::from_utf8(writer.bytes)
                .map_err(|error| anyhow!("serialize sync response as UTF-8: {error}"));
        }
        serde_json::to_string(&request.err(
            -1,
            "synchronous API response exceeds the bounded 100 MiB limit",
        ))
        .map_err(Into::into)
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

    /// Register a Native operation that exists only on the explicitly
    /// allowlisted JSON IPC transport. Unlike unary/stream registrations,
    /// this handler receives a per-page cancellation context and cannot be
    /// reached through a stream or synchronous XHR.
    pub fn register_ipc_api<S, F, Fut, T>(&mut self, name: S, f: F)
    where
        S: Into<String>,
        F: Fn(IpcCallContext, ApiRequest) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<T>> + Send + 'static,
        T: Serialize + Send + 'static,
    {
        let f = Arc::new(f);
        let handler: IpcApiHandler = Arc::new(move |ctx, request| {
            let f = f.clone();
            Box::pin(async move {
                let data = f(ctx, request).await?;
                Ok(serde_json::to_value(data)?)
            }) as IpcApiFuture
        });
        self.ipc_handlers.insert(name.into(), handler);
    }

    /// Register a bounded unary API that uses the same Native operation on
    /// WebSocket and IPC transports. The explicit transport owner lets work
    /// such as child processes and HTTP sockets close synchronously on owner
    /// cancellation; its IPC reachability still comes only from the exact
    /// `IPC_METHOD_ALLOWLIST` below.
    pub fn register_cancellable_api<S, F, Fut, T>(&mut self, name: S, f: F)
    where
        S: Into<String>,
        F: Fn(CancellationContext, Arc<NivaApp>, Arc<NivaWindow>, ApiRequest) -> Fut
            + Send
            + Sync
            + 'static,
        Fut: Future<Output = Result<T>> + Send + 'static,
        T: Serialize + Send + 'static,
    {
        let f = Arc::new(f);
        let handler: CancellableApiHandler = Arc::new(move |context, app, window, request| {
            let f = f.clone();
            Box::pin(async move {
                serde_json::to_value(f(context, app, window, request).await?).map_err(Into::into)
            }) as CancellableApiFuture
        });
        self.handlers.insert(
            name.into(),
            HandlerEntry {
                handler: HandlerKind::CancellableUnary(handler),
                timeout: None,
            },
        );
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
                crate::niva_log!(
                    crate::app::logging::Level::Warn,
                    "[niva] api bad message: {err}"
                );
                return;
            }
        };
        match msg {
            ClientMsg::Hello { wid, v, session_id } => {
                if wid != window_id
                    || v != WIRE_VERSION
                    || !self.bind_ws_session(window_id, connection_id, &session_id)
                {
                    crate::niva_log!(
                        crate::app::logging::Level::Warn,
                        "[niva] rejected WebSocket session binding"
                    );
                    self.cancel_connection(window_id, connection_id);
                }
            }
            ClientMsg::Cancel { id, session_id } => {
                if self.ws_session_matches(window_id, connection_id, &session_id) {
                    self.cancel_call(window_id, connection_id, id);
                }
            }
            ClientMsg::Call {
                id,
                method,
                args,
                session_id,
            } => {
                let request = ApiRequest(id, method, ApiArguments(args));
                if self.ws_session_matches(window_id, connection_id, &session_id) {
                    self.dispatch(window_id, connection_id, tx.clone(), request);
                } else {
                    respond(tx, request.err(-4, "invalid WebSocket session id"));
                }
            }
        }
    }

    pub fn bind_ws_session(&self, window_id: u8, connection_id: u64, session_id: &str) -> bool {
        if validate_session_id(session_id).is_err() {
            return false;
        }
        let Ok(mut sessions) = self.ws_sessions.lock() else {
            return false;
        };
        let key = (window_id, connection_id);
        match sessions.get(&key) {
            Some(existing) => existing == session_id,
            None => {
                sessions.insert(key, session_id.to_owned());
                true
            }
        }
    }

    pub(crate) fn ws_session_matches(
        &self,
        window_id: u8,
        connection_id: u64,
        session_id: &str,
    ) -> bool {
        self.ws_sessions
            .lock()
            .ok()
            .and_then(|sessions| sessions.get(&(window_id, connection_id)).cloned())
            .is_some_and(|expected| expected == session_id)
    }

    /// Entry for binary frames from the transport (non-blocking).
    pub fn on_binary(&self, window_id: u8, connection_id: u64, frame: &[u8]) {
        let (header, payload) = match decode_chunk(frame) {
            Ok(pair) => pair,
            Err(err) => {
                crate::niva_log!(
                    crate::app::logging::Level::Warn,
                    "[niva] api bad chunk: {err}"
                );
                return;
            }
        };
        route_inbound_chunk(&self.active, window_id, connection_id, header, payload);
    }

    /// Abort one call (client cancel). Silent when unknown.
    pub fn cancel_call(&self, window_id: u8, connection_id: u64, id: u64) {
        crate::app::api::cancel_process_call(window_id, connection_id, id);
        let call = {
            let mut active = match lock!(self.active) {
                Ok(active) => active,
                Err(_) => return,
            };
            let call = active.remove(&(window_id, connection_id, id));
            if let Some(call) = &call {
                call.lifecycle.cancel();
            }
            call
        };
        if let Some(call) = call {
            cancel_active_call(call);
        }
    }

    /// Disconnecting one frame must not cancel calls from sibling frames.
    pub fn cancel_connection(&self, window_id: u8, connection_id: u64) {
        crate::app::api::cancel_process_connection(window_id, connection_id);
        let session_id = self
            .ws_sessions
            .lock()
            .ok()
            .and_then(|mut sessions| sessions.remove(&(window_id, connection_id)));
        if let Some(session_id) = session_id {
            self.cancel_sync_session(window_id, &session_id);
        }
        let calls = {
            let mut active = match lock!(self.active) {
                Ok(active) => active,
                Err(_) => return,
            };
            take_active_calls(&mut active, |(wid, cid, _)| {
                *wid == window_id && *cid == connection_id
            })
        };
        for call in calls {
            cancel_active_call(call);
        }
    }

    /// Abort all calls of a window (socket drop / window close).
    pub fn cancel_window(&self, window_id: u8) {
        crate::app::api::cancel_process_window(window_id);
        if let Ok(mut sessions) = self.ws_sessions.lock() {
            sessions.retain(|(owner, _), _| *owner != window_id);
        }
        self.cancel_sync_window(window_id);
        let calls = {
            let mut active = match lock!(self.active) {
                Ok(active) => active,
                Err(_) => return,
            };
            take_active_calls(&mut active, |(wid, _, _)| *wid == window_id)
        };
        for call in calls {
            cancel_active_call(call);
        }
        self.cancel_ipc_window(window_id);
        if self.app.get().is_some_and(|app| {
            app.window()
                .is_ok_and(|manager| manager.list_windows().is_empty())
        }) && let Some(resolver) = self.module_resolver.get()
        {
            resolver.cleanup();
        }
    }

    pub(crate) fn dispatch(
        &self,
        window_id: u8,
        connection_id: u64,
        tx: mpsc::Sender<WsOut>,
        request: ApiRequest,
    ) {
        let (cancel_tx, cancel_rx) = async_channel::bounded::<()>(1);
        let (inbound_tx, inbound_rx) = async_channel::bounded::<InboundChunk>(64);
        let lifecycle = Arc::new(CallLifecycle::new());
        let key = (window_id, connection_id, request.0);
        let reservation = match lock!(self.active) {
            Ok(mut active) => reserve_active_call(
                &mut active,
                key,
                ActiveCall {
                    cancel_tx: cancel_tx.clone(),
                    inbound_tx,
                    ws_tx: tx.clone(),
                    lifecycle: lifecycle.clone(),
                },
            ),
            Err(err) => {
                crate::niva_log!(
                    crate::app::logging::Level::Warn,
                    "[niva] api active table poisoned: {err}"
                );
                respond(&tx, request.err(-1, "active call table unavailable"));
                return;
            }
        };
        if !reservation {
            // A result carrying this id would terminate the original pending
            // call at the peer. Reject the duplicate with a connection event
            // that has no call id and leave the original call untouched.
            lifecycle.cancel();
            cancel_tx.close();
            let event_seq = self
                .app
                .get()
                .and_then(|app| {
                    app.window()
                        .ok()
                        .and_then(|manager| manager.get_window(window_id).ok())
                })
                .map(|window| window.next_event_seq())
                .unwrap_or(request.0);
            notify_duplicate_active_id(&tx, event_seq, request.0);
            return;
        }
        let app_result = self
            .app
            .get()
            .cloned()
            .ok_or_else(|| anyhow!("API manager not ready"));
        let window_result = match &app_result {
            Ok(app) => app
                .window()
                .and_then(|manager| manager.get_window(window_id)),
            Err(error) => Err(anyhow!("{error:#}")),
        };
        let app = match app_result {
            Ok(app) => app,
            Err(error) => {
                reject_reserved_call(
                    &self.active,
                    key,
                    lifecycle,
                    cancel_tx,
                    &tx,
                    request.err(-1, error.to_string()),
                );
                return;
            }
        };
        let window = match window_result {
            Ok(window) => window,
            Err(error) => {
                reject_reserved_call(
                    &self.active,
                    key,
                    lifecycle,
                    cancel_tx,
                    &tx,
                    request.err(-1, error.to_string()),
                );
                return;
            }
        };
        let entry = match self.handlers.get(&request.1) {
            Some(entry) => entry,
            None => {
                reject_reserved_call(
                    &self.active,
                    key,
                    lifecycle,
                    cancel_tx,
                    &tx,
                    request.err(-1, "api not found"),
                );
                return;
            }
        };
        let timeout = match entry.timeout {
            Some(custom) => custom,
            None => Some(self.default_timeout),
        };
        let handler = match &entry.handler {
            HandlerKind::Unary(h) => HandlerKind::Unary(h.clone()),
            HandlerKind::CancellableUnary(h) => HandlerKind::CancellableUnary(h.clone()),
            HandlerKind::Stream(h) => HandlerKind::Stream(h.clone()),
        };
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
                lifecycle: lifecycle.clone(),
            }),
        };
        let job = DispatchJob {
            ctx,
            request,
            handler,
            timeout,
        };
        enqueue_or_reject(
            &self.dispatch_tx,
            job,
            &self.active,
            key,
            lifecycle,
            cancel_tx,
            |job, should_respond| {
                crate::niva_log!(
                    crate::app::logging::Level::Warn,
                    "[niva] api dispatch queue full, rejecting"
                );
                if should_respond {
                    respond(&job.ctx.ws_tx, job.request.err(-3, "server busy"));
                }
            },
        );
    }
}

/// Deliver a terminal response over the window socket; log-and-drop when gone.
fn respond(tx: &mpsc::Sender<WsOut>, response: ApiResponse) {
    let msg = ServerMsg::result(response.0, response.1, response.2, response.3);
    if tx.send(WsOut::Text(msg.encode())).is_err() {
        crate::niva_log!(
            crate::app::logging::Level::Warn,
            "[niva] api response dropped, window socket gone"
        );
    }
}

#[derive(PartialEq)]
enum End {
    Done,
    Timeout,
    Cancelled,
}

async fn run_job(job: DispatchJob, active: Arc<Mutex<ActiveCalls>>) {
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
        if !ctx.lifecycle.start() {
            return End::Cancelled;
        }
        match handler {
            HandlerKind::Unary(f) => {
                let response = f(ctx.app.clone(), ctx.window.clone(), request.clone()).await;
                ctx.respond_api_response(response);
            }
            HandlerKind::CancellableUnary(f) => {
                let cancellation = ctx.cancellation();
                match f(
                    cancellation,
                    ctx.app.clone(),
                    ctx.window.clone(),
                    request.clone(),
                )
                .await
                {
                    Ok(value) => ctx.respond(Ok(value)),
                    Err(error) => ctx.respond::<Value>(Err(error)),
                }
            }
            HandlerKind::Stream(f) => match f(ctx, request.clone()).await {
                Ok(()) => reply.respond(Ok(Value::Null)),
                Err(err) => reply.respond::<Value>(Err(err)),
            },
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

    // Claim the call's only terminal result before owner cleanup. Blocking
    // work may outlive the dropped handler future, but it cannot send a late
    // result after timeout or cancellation.
    match end {
        End::Timeout => {
            crate::app::api::cancel_process_call(wid, connection_id, id);
            reply.respond_api_response(request.err(-2, "API timeout"));
        }
        End::Cancelled => {
            crate::app::api::cancel_process_call(wid, connection_id, id);
            reply.inner.lifecycle.cancel();
        }
        End::Done => {}
    }

    // Always close the cancel channel on task end: unblocks ctx waiters
    // (collect loops, stdin pumps) even when nobody cancelled.
    if let Some(call) = remove_active_if_same(&active, (wid, connection_id, id), &reply.lifecycle) {
        call.cancel_tx.close();
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
    } else if let Some(code) = structured_error_code(&error.to_string()) {
        json!({ "code": code })
    } else {
        json!(null)
    }
}

fn structured_error_code(message: &str) -> Option<&str> {
    let code = message.split_once(':')?.0;
    if code == "MODULE_NOT_FOUND" {
        return Some(code);
    }
    if let Some(suffix) = code.strip_prefix("ERR_")
        && !suffix.is_empty()
        && suffix
            .bytes()
            .all(|byte| byte.is_ascii_uppercase() || byte.is_ascii_digit() || byte == b'_')
    {
        return Some(code);
    }
    if let Some(suffix) = code.strip_prefix('E')
        && !suffix.is_empty()
        && suffix
            .bytes()
            .all(|byte| byte.is_ascii_uppercase() || byte.is_ascii_digit() || byte == b'_')
    {
        return Some(code);
    }
    None
}

fn sync_method_allowed(method: &str) -> bool {
    method.starts_with("fs.")
        || matches!(
            method,
            "module.resolve"
                | "module.load"
                | "os.cpus"
                | "os.freemem"
                | "os.networkInterfaces"
                | "os.uptime"
                | "os.dnsLookup"
                | "os.dnsServers"
                | "process.currentDir"
                | "process.setCurrentDir"
                | "process.spawnSync"
        )
}

#[cfg(test)]
mod lifecycle_tests {
    use super::*;

    fn take_text(rx: &mpsc::Receiver<WsOut>) -> String {
        match rx.try_recv().unwrap() {
            WsOut::Text(text) => text,
            WsOut::Binary(_) => panic!("expected a text frame"),
        }
    }

    fn active_call(
        lifecycle: Arc<CallLifecycle>,
        ws_tx: mpsc::Sender<WsOut>,
        cancel_tx: async_channel::Sender<()>,
        inbound_tx: async_channel::Sender<InboundChunk>,
    ) -> ActiveCall {
        ActiveCall {
            lifecycle,
            ws_tx,
            cancel_tx,
            inbound_tx,
        }
    }

    #[test]
    fn duplicate_active_id_notifies_without_finishing_original_call() {
        let key = (3, 9, 42);
        let mut active = ActiveCalls::new();
        let (ws_tx, ws_rx) = mpsc::channel();
        let (original_cancel_tx, original_cancel_rx) = async_channel::bounded(1);
        let (original_inbound_tx, _original_inbound_rx) = async_channel::bounded(1);
        let original_lifecycle = Arc::new(CallLifecycle::new());
        assert!(reserve_active_call(
            &mut active,
            key,
            active_call(
                original_lifecycle.clone(),
                ws_tx.clone(),
                original_cancel_tx,
                original_inbound_tx,
            ),
        ));

        let (duplicate_cancel_tx, duplicate_cancel_rx) = async_channel::bounded(1);
        let (duplicate_inbound_tx, _duplicate_inbound_rx) = async_channel::bounded(1);
        assert!(!reserve_active_call(
            &mut active,
            key,
            active_call(
                Arc::new(CallLifecycle::new()),
                ws_tx.clone(),
                duplicate_cancel_tx,
                duplicate_inbound_tx,
            ),
        ));
        assert_eq!(active.len(), 1);
        assert!(Arc::ptr_eq(
            &active.get(&key).unwrap().lifecycle,
            &original_lifecycle
        ));
        assert!(!original_lifecycle.is_terminal());
        assert!(!original_cancel_rx.is_closed());
        assert!(duplicate_cancel_rx.is_closed());

        notify_duplicate_active_id(&ws_tx, 11, key.2);
        assert!(original_lifecycle.start());
        assert!(respond_once(
            &ws_tx,
            &original_lifecycle,
            ApiResponse(key.2, 0, "ok".into(), json!({"done":true})),
        ));
        assert!(!respond_once(
            &ws_tx,
            &original_lifecycle,
            ApiResponse(key.2, -1, "late error".into(), Value::Null),
        ));

        let event = take_text(&ws_rx);
        let event: Value = serde_json::from_str(&event).unwrap();
        assert_eq!(event["t"], "event");
        assert!(event.get("id").is_none());
        assert_eq!(event["name"], "bridge.protocolError");
        assert_eq!(event["data"]["requestId"], key.2);

        let result = take_text(&ws_rx);
        let result: Value = serde_json::from_str(&result).unwrap();
        assert_eq!(result["t"], "result");
        assert_eq!(result["id"], key.2);
        assert_eq!(result["code"], 0);
        assert!(ws_rx.try_recv().is_err());
    }

    #[test]
    fn rejected_dispatch_rolls_back_only_its_active_entry_and_answers_once() {
        let key = (2, 6, 17);
        let (dispatch_tx, dispatch_rx) = async_channel::bounded::<u8>(1);
        dispatch_tx.try_send(1).unwrap();
        let (ws_tx, ws_rx) = mpsc::channel();
        let (cancel_tx, cancel_rx) = async_channel::bounded(1);
        let (inbound_tx, _inbound_rx) = async_channel::bounded(1);
        let lifecycle = Arc::new(CallLifecycle::new());
        let mut active = ActiveCalls::new();
        assert!(reserve_active_call(
            &mut active,
            key,
            active_call(
                lifecycle.clone(),
                ws_tx.clone(),
                cancel_tx.clone(),
                inbound_tx,
            ),
        ));
        let active = Mutex::new(active);

        let accepted = enqueue_or_reject(
            &dispatch_tx,
            7,
            &active,
            key,
            lifecycle.clone(),
            cancel_tx,
            |job, should_respond| {
                assert_eq!(job, 7);
                if should_respond {
                    respond(
                        &ws_tx,
                        ApiResponse(key.2, -3, "server busy".into(), Value::Null),
                    );
                }
            },
        );

        assert!(!accepted);
        assert!(active.lock().unwrap().is_empty());
        assert!(lifecycle.is_terminal());
        assert!(!lifecycle.start());
        assert!(cancel_rx.is_closed());
        assert_eq!(dispatch_rx.try_recv().unwrap(), 1);
        let result = take_text(&ws_rx);
        let result: Value = serde_json::from_str(&result).unwrap();
        assert_eq!(result["id"], key.2);
        assert_eq!(result["code"], -3);
        assert!(ws_rx.try_recv().is_err());
    }

    #[test]
    fn full_inbound_queue_terminates_the_call_with_one_failure() {
        let key = (4, 8, 23);
        let (ws_tx, ws_rx) = mpsc::channel();
        let (cancel_tx, cancel_rx) = async_channel::bounded(1);
        let (inbound_tx, inbound_rx) = async_channel::bounded(1);
        inbound_tx
            .try_send(InboundChunk {
                seq: 1,
                data: vec![1],
                end: false,
            })
            .unwrap();
        let lifecycle = Arc::new(CallLifecycle::new());
        let mut active = ActiveCalls::new();
        assert!(reserve_active_call(
            &mut active,
            key,
            active_call(lifecycle.clone(), ws_tx.clone(), cancel_tx, inbound_tx,),
        ));
        let active = Mutex::new(active);

        route_inbound_chunk(
            &active,
            key.0,
            key.1,
            self::protocol::ChunkHeader {
                id: key.2,
                seq: 2,
                start: false,
                end: true,
                stderr: false,
            },
            &[2],
        );

        assert!(lifecycle.is_terminal());
        assert!(!lifecycle.start());
        assert!(cancel_rx.is_closed());
        assert!(active.lock().unwrap().is_empty());
        assert_eq!(inbound_rx.try_recv().unwrap().data, vec![1]);
        assert!(inbound_rx.try_recv().is_err());
        let result = take_text(&ws_rx);
        let result: Value = serde_json::from_str(&result).unwrap();
        assert_eq!(result["id"], key.2);
        assert_eq!(result["code"], -3);
        assert_eq!(result["message"], "inbound stream queue full");
        assert!(ws_rx.try_recv().is_err());
    }

    #[test]
    fn disconnect_removes_only_calls_owned_by_that_window_connection() {
        let owned = (1, 10, 1);
        let sibling = (1, 11, 1);
        let (ws_tx, _ws_rx) = mpsc::channel();
        let (owned_cancel_tx, owned_cancel_rx) = async_channel::bounded(1);
        let (owned_in_tx, _owned_in_rx) = async_channel::bounded(1);
        let owned_lifecycle = Arc::new(CallLifecycle::new());
        let (sibling_cancel_tx, sibling_cancel_rx) = async_channel::bounded(1);
        let (sibling_in_tx, _sibling_in_rx) = async_channel::bounded(1);
        let sibling_lifecycle = Arc::new(CallLifecycle::new());
        let mut active = ActiveCalls::new();
        assert!(reserve_active_call(
            &mut active,
            owned,
            active_call(
                owned_lifecycle.clone(),
                ws_tx.clone(),
                owned_cancel_tx,
                owned_in_tx,
            ),
        ));
        assert!(reserve_active_call(
            &mut active,
            sibling,
            active_call(
                sibling_lifecycle.clone(),
                ws_tx,
                sibling_cancel_tx,
                sibling_in_tx,
            ),
        ));

        let calls = take_active_calls(&mut active, |(wid, cid, _)| *wid == 1 && *cid == 10);
        for call in calls {
            cancel_active_call(call);
        }
        assert_eq!(active.len(), 1);
        assert!(active.contains_key(&sibling));
        assert!(owned_lifecycle.is_terminal());
        assert!(!owned_lifecycle.start());
        assert!(owned_cancel_rx.is_closed());
        assert!(!sibling_lifecycle.is_terminal());
        assert!(sibling_lifecycle.start());
        assert!(!sibling_cancel_rx.is_closed());
    }
}

#[cfg(test)]
mod ipc_tests {
    use super::*;

    #[cfg(target_os = "macos")]
    #[test]
    fn macos_ipc_normalizes_only_the_native_niva_app_origin() {
        assert_eq!(
            normalize_ipc_source_origin(&url::Url::parse("niva://app/index.html").unwrap())
                .unwrap(),
            super::super::custom_protocol::platform_origin()
        );
        for source in [
            "niva://other/index.html",
            "niva://user@app/index.html",
            "niva://app:9000/index.html",
            "file:///tmp/page.html",
        ] {
            assert!(
                normalize_ipc_source_origin(&url::Url::parse(source).unwrap()).is_err(),
                "{source}"
            );
        }
    }

    fn key(session_id: &str, frame_id: u64) -> IpcSessionKey {
        IpcSessionKey {
            window_id: 7,
            source_origin: "http://127.0.0.1:3000".into(),
            session_id: session_id.into(),
            frame_id,
            generation: 2,
            is_main_frame: frame_id == 0,
        }
    }

    #[test]
    fn ipc_manifest_is_explicit_and_excludes_persistent_or_binary_routes() {
        for method in [
            "fs.readText",
            "fs.writeText",
            "fs.appendText",
            "fs.node",
            "http.requestText",
            "process.execText",
            "process.execFileText",
            "window.title",
            "window.close",
            "clipboard.read",
            "dialog.showMessage",
            "monitor.primary",
            "os.dirs",
            "webview.reload",
        ] {
            assert!(ipc_method_allowed(method), "{method}");
        }
        for method in [
            "window.open",
            "webview.baseFileSystemUrl",
            "webview.eval",
            "menu.create",
            "tray.create",
            "shortcut.register",
            "process.execStream",
            "process.spawnSync",
            "process.execSync",
            "fs.watch",
            "fs.readFile",
            "module.resolve",
            "module.load",
            "os.info",
            "os.timingSafeEqual",
        ] {
            assert!(!ipc_method_allowed(method), "{method}");
        }
    }

    #[test]
    fn ipc_messages_accept_the_bootstrap_javascript_camel_case_envelopes() {
        let call: IpcMessage = serde_json::from_str(
            r#"{"t":"call","id":7,"method":"http.requestText","args":[{}],"sessionId":"0123456789abcdef0123456789abcdef","token":"window-token"}"#,
        )
        .unwrap();
        assert!(
            matches!(call, IpcMessage::Call { id: 7, session_id, .. } if session_id == "0123456789abcdef0123456789abcdef")
        );

        let heartbeat: IpcMessage = serde_json::from_str(
            r#"{"t":"heartbeat","sessionId":"0123456789abcdef0123456789abcdef","token":"window-token"}"#,
        )
        .unwrap();
        assert!(
            matches!(heartbeat, IpcMessage::Heartbeat { session_id, .. } if session_id == "0123456789abcdef0123456789abcdef")
        );
    }

    #[test]
    fn native_bridge_contract_methods_are_registered_on_the_intended_transports() {
        let options: NivaOptions = serde_json::from_value(json!({
            "name": "registration-test",
            "uuid": "a51c1728-d174-42d4-8f57-7d296c966b51"
        }))
        .unwrap();
        let mut manager = ApiManager::new(&options);
        crate::app::api::register_api_instances(&mut manager);

        for method in [
            "module.resolve",
            "module.load",
            "fs.readText",
            "fs.writeText",
            "fs.appendText",
            "fs.node",
            "http.requestText",
            "process.execText",
            "process.execFileText",
            "process.spawnSync",
        ] {
            assert!(
                manager.handlers.contains_key(method) || manager.ipc_handlers.contains_key(method),
                "missing native bridge handler {method}"
            );
        }

        for method in ["module.resolve", "module.load"] {
            assert!(matches!(
                manager.handlers.get(method).map(|entry| &entry.handler),
                Some(HandlerKind::Unary(_))
            ));
            assert!(sync_method_allowed(method));
            assert!(!ipc_method_allowed(method));
        }
        for method in ["fs.readText", "fs.writeText", "fs.appendText"] {
            assert!(manager.ipc_handlers.contains_key(method));
            assert!(ipc_method_allowed(method));
        }
        assert!(manager.handlers.contains_key("fs.node"));
        assert!(manager.ipc_handlers.contains_key("fs.node"));
        assert!(matches!(
            manager.handlers.get("fs.node").map(|entry| &entry.handler),
            Some(HandlerKind::CancellableUnary(_))
        ));
        assert!(ipc_method_allowed("fs.node"));
        assert!(sync_method_allowed("fs.node"));
        for method in [
            "http.requestText",
            "process.execText",
            "process.execFileText",
        ] {
            assert!(matches!(
                manager.handlers.get(method).map(|entry| &entry.handler),
                Some(HandlerKind::CancellableUnary(_))
            ));
            assert!(ipc_method_allowed(method));
        }
        assert!(matches!(
            manager
                .handlers
                .get("process.spawnSync")
                .map(|entry| &entry.handler),
            Some(HandlerKind::CancellableUnary(_))
        ));
        assert!(sync_method_allowed("process.spawnSync"));
        assert!(!ipc_method_allowed("process.spawnSync"));
    }

    #[test]
    fn websocket_session_binding_is_exact_and_disconnect_scoped() {
        let options: NivaOptions = serde_json::from_value(json!({
            "name": "session-test",
            "uuid": "a51c1728-d174-42d4-8f57-7d296c966b51"
        }))
        .unwrap();
        let manager = ApiManager::new(&options);
        let session = "0123456789abcdef0123456789abcdef";
        assert!(manager.bind_ws_session(9, 44, session));
        assert!(manager.ws_session_matches(9, 44, session));
        assert!(!manager.ws_session_matches(9, 44, "abcdef0123456789abcdef0123456789"));
        manager.cancel_connection(9, 44);
        assert!(!manager.ws_session_matches(9, 44, session));
    }

    #[test]
    fn heartbeat_renews_only_active_work_and_expiration_cannot_be_revived() {
        let now = Instant::now();
        let mut sessions = IpcSessions::default();
        let owner = key("0123456789abcdef0123456789abcdef", 0);
        let sibling = key("abcdef0123456789abcdef0123456789", 0);
        let (owner_tx, owner_rx) = async_channel::bounded(1);
        let (sibling_tx, sibling_rx) = async_channel::bounded(1);
        sessions.reserve(owner.clone(), 1, owner_tx, now).unwrap();
        sessions
            .reserve(sibling.clone(), 2, sibling_tx, now)
            .unwrap();

        assert_eq!(
            sessions.heartbeat(&owner, now + Duration::from_secs(2)),
            IpcHeartbeat::Active
        );
        assert_eq!(
            sessions.heartbeat(&key("fedcba9876543210fedcba9876543210", 0), now),
            IpcHeartbeat::Idle
        );
        assert!(matches!(
            sessions.expire_if_stale(&owner, now + Duration::from_secs(4)),
            Ok(false)
        ));

        let expired = sessions
            .expire_if_stale(&owner, now + Duration::from_secs(6))
            .unwrap_err();
        assert_eq!(expired.len(), 1);
        expired[0].close();
        assert!(owner_rx.is_closed());
        assert_eq!(
            sessions.heartbeat(&owner, now + Duration::from_secs(7)),
            IpcHeartbeat::Expired
        );
        let (late_tx, _) = async_channel::bounded(1);
        assert_eq!(
            sessions.reserve(owner.clone(), 3, late_tx, now + Duration::from_secs(7)),
            Err(IpcSessionError::Expired)
        );
        assert!(!sibling_rx.is_closed());
    }

    #[test]
    fn ipc_session_cancellation_is_frame_and_generation_scoped() {
        let now = Instant::now();
        let mut sessions = IpcSessions::default();
        let selected = key("0123456789abcdef0123456789abcdef", 11);
        let sibling = key("0123456789abcdef0123456789abcdef", 12);
        let newer_generation = IpcSessionKey {
            generation: 3,
            ..selected.clone()
        };
        let (selected_tx, selected_rx) = async_channel::bounded(1);
        let (sibling_tx, sibling_rx) = async_channel::bounded(1);
        let (newer_tx, newer_rx) = async_channel::bounded(1);
        sessions
            .reserve(selected.clone(), 1, selected_tx, now)
            .unwrap();
        sessions
            .reserve(sibling.clone(), 2, sibling_tx, now)
            .unwrap();
        sessions
            .reserve(newer_generation, 3, newer_tx, now)
            .unwrap();

        let cancelled = sessions.cancel_matching(
            |key| key.window_id == 7 && key.frame_id == 11 && key.generation == 2,
            true,
        );
        assert_eq!(cancelled.len(), 1);
        cancelled[0].close();
        assert!(selected_rx.is_closed());
        assert!(!sibling_rx.is_closed());
        assert!(!newer_rx.is_closed());
    }

    #[test]
    fn synchronous_transport_excludes_ui_and_connection_owned_apis() {
        for method in [
            "window.open",
            "process.exit",
            "socket.control",
            "socket.tcpConnect",
            "webview.eval",
            "process.env",
            "os.info",
            "os.timingSafeEqual",
            "os.dirs",
            "process.uptime",
            "process.memoryUsage",
            "process.cpuUsage",
            "process.execSync",
            "process.execFileSync",
        ] {
            assert!(!sync_method_allowed(method), "{method}");
        }
        for method in [
            "fs.stat",
            "fs.readFile",
            "module.resolve",
            "module.load",
            "os.cpus",
            "os.freemem",
            "os.networkInterfaces",
            "os.uptime",
            "os.dnsLookup",
            "os.dnsServers",
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

    #[test]
    fn structured_native_errors_preserve_node_and_system_error_codes() {
        for (message, expected) in [
            ("MODULE_NOT_FOUND: Cannot find module", "MODULE_NOT_FOUND"),
            ("ERR_REQUIRE_ESM: cannot require module", "ERR_REQUIRE_ESM"),
            ("EACCES: permission denied", "EACCES"),
            ("ETIMEDOUT: operation timed out", "ETIMEDOUT"),
        ] {
            let data = native_error_data(&anyhow!("{message}"));
            assert_eq!(data["code"], expected);
        }
        assert_eq!(
            native_error_data(&anyhow!("unstructured error")),
            Value::Null
        );
    }
}
