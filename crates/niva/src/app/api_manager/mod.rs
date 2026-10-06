pub(crate) mod protocol;

use std::{
    collections::{HashMap, HashSet, hash_map::Entry},
    future::Future,
    hash::{Hash, Hasher},
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
    /// URL of the current top-level document, captured by the native WebView
    /// callback. Cross-origin frames are rejected before dispatch; same-origin
    /// child frames may relay through their parent using browser same-origin
    /// access, but never supply this value themselves.
    pub top_url: String,
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
    BridgeSession {
        owner_id: u64,
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

/// Cancellation signal and stable page-session owner for bounded, non-streaming APIs.
/// The owner is used only for resource cancellation and never for authorization.
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

    /// Stable owner id for resources created by stream APIs. IPC unary
    /// controls share the originating page session's owner with its streams.
    pub fn resource_owner_id(&self) -> Option<u64> {
        match &self.owner {
            ApiCallOwner::BridgeSession { owner_id } => Some(*owner_id),
            ApiCallOwner::Ipc { session_id, .. } => {
                Some(session_owner_id(self.window_id, session_id))
            }
            ApiCallOwner::Synchronous { .. } => None,
        }
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
    #[serde(rename = "session_close")]
    SessionClose {
        session_id: String,
        token: Option<String>,
    },
    #[serde(rename = "api_call")]
    ApiCall {
        rid: u64,
        id: u64,
        method: String,
        args: Value,
        session_id: String,
        token: Option<String>,
    },
    ChannelAttach {
        rid: u64,
        id: u64,
        session_id: String,
        token: Option<String>,
        capability: String,
    },
    ChannelSend {
        rid: u64,
        id: u64,
        session_id: String,
        token: Option<String>,
        capability: String,
        frame: String,
    },
    ChannelAck {
        rid: u64,
        id: u64,
        session_id: String,
        token: Option<String>,
        capability: String,
        seq: u64,
    },
    ChannelCancel {
        rid: u64,
        id: u64,
        session_id: String,
        token: Option<String>,
        capability: String,
    },
    Heartbeat {
        rid: Option<u64>,
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
    outbound: CallOutput,
    pub seq: AtomicU64,
    pub cancel_rx: async_channel::Receiver<()>,
    pub inbound_rx: async_channel::Receiver<InboundChunk>,
    http_response_ack_rx: async_channel::Receiver<u64>,
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
            owner: ApiCallOwner::BridgeSession {
                owner_id: self.connection_id,
            },
            cancel_rx: self.cancel_rx.clone(),
        }
    }

    /// Best-effort stream push tied to this call (dropped when its bridge closes).
    pub fn push(&self, name: &str, data: Value) {
        if self.inner.lifecycle.is_terminal() {
            return;
        }
        let seq = self.seq.fetch_add(1, Ordering::Relaxed);
        let msg = ServerMsg::event(Some(self.id), seq, name.to_string(), data);
        let text = msg.encode();
        if matches!(&self.inner.outbound, CallOutput::Channel(_))
            && text.len() > MAX_CHANNEL_TEXT_FRAME_BYTES
        {
            self.respond_api_response(ApiResponse(
                self.id,
                -1,
                "ERR_NIVA_CHANNEL_FRAME_TOO_LARGE".into(),
                json!({"code":"ERR_NIVA_CHANNEL_FRAME_TOO_LARGE"}),
            ));
            return;
        }
        let _ = self.inner.outbound.send(WsOut::Text(text));
    }

    /// Best-effort binary chunk tied to this call (dropped when its bridge closes).
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
        if matches!(&self.inner.outbound, CallOutput::Channel(_)) {
            if data.is_empty() {
                let seq = self.seq.fetch_add(1, Ordering::Relaxed);
                let frame = encode_chunk(self.id, seq, seq == 1, end, stderr, data);
                let _ = self.inner.outbound.send(WsOut::Binary(frame));
                return;
            }
            for (index, chunk) in data.chunks(MAX_CHANNEL_BINARY_PAYLOAD).enumerate() {
                let seq = self.seq.fetch_add(1, Ordering::Relaxed);
                let is_last = (index + 1) * MAX_CHANNEL_BINARY_PAYLOAD >= data.len();
                let frame = encode_chunk(self.id, seq, seq == 1, end && is_last, stderr, chunk);
                if !self.inner.outbound.send(WsOut::Binary(frame)) {
                    return;
                }
            }
            return;
        }
        let seq = self.seq.fetch_add(1, Ordering::Relaxed);
        // First chunk of a call opens with START (seq starts at 1).
        let frame = encode_chunk(self.id, seq, seq == 1, end, stderr, data);
        let _ = self.inner.outbound.send(WsOut::Binary(frame));
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
        if let CallOutput::WebSocket(sender) = &self.inner.outbound {
            respond_once(sender, &self.inner.lifecycle, response);
            return;
        }
        if !claim_terminal(&self.inner.lifecycle) {
            return;
        }
        let mut message = ServerMsg::result(response.0, response.1, response.2, response.3);
        let mut text = message.encode();
        if matches!(&self.inner.outbound, CallOutput::Channel(_))
            && text.len() > MAX_CHANNEL_TEXT_FRAME_BYTES
        {
            message = ServerMsg::result(
                self.id,
                -1,
                "ERR_NIVA_CHANNEL_FRAME_TOO_LARGE".into(),
                json!({"code":"ERR_NIVA_CHANNEL_FRAME_TOO_LARGE"}),
            );
            text = message.encode();
        }
        let _ = self.inner.outbound.send(WsOut::Text(text));
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

    /// Blocking receive that also wakes when this call is cancelled. Use this
    /// from blocking stream handlers that must hold a resource lock while
    /// consuming the upload lane.
    pub fn next_chunk_blocking_cancellable(&self) -> Option<InboundChunk> {
        smol::block_on(smol::future::or(
            async {
                let _ = self.cancel_rx.recv().await;
                None
            },
            async { self.inbound_rx.recv().await.ok() },
        ))
    }

    /// Wait for the next HTTP response-body credit, waking when this stream is
    /// cancelled. Response credits use a separate IPC control lane so request
    /// upload END remains final.
    pub fn next_http_response_ack_blocking(&self) -> Option<u64> {
        smol::block_on(smol::future::or(
            async {
                let _ = self.cancel_rx.recv().await;
                None
            },
            async { self.inner.http_response_ack_rx.recv().await.ok() },
        ))
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
    outbound: CallOutput,
    lifecycle: Arc<CallLifecycle>,
    ipc_session: Option<IpcSessionKey>,
}

type CallKey = (u8, u64, u64);
type ActiveCalls = HashMap<CallKey, ActiveCall>;

#[derive(Clone)]
enum CallOutput {
    WebSocket(mpsc::Sender<WsOut>),
    Channel(mpsc::SyncSender<WsOut>),
}

impl CallOutput {
    fn send(&self, message: WsOut) -> bool {
        match self {
            Self::WebSocket(sender) => sender.send(message).is_ok(),
            Self::Channel(sender) => sender.send(message).is_ok(),
        }
    }
}

fn is_terminal_output(output: &WsOut) -> bool {
    let WsOut::Text(text) = output else {
        return false;
    };
    serde_json::from_str::<Value>(text)
        .ok()
        .is_some_and(|value| value.get("t").and_then(Value::as_str) == Some("result"))
}

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

struct DispatchChannels {
    cancel_tx: async_channel::Sender<()>,
    cancel_rx: async_channel::Receiver<()>,
    inbound_tx: async_channel::Sender<InboundChunk>,
    inbound_rx: async_channel::Receiver<InboundChunk>,
    http_response_ack_rx: async_channel::Receiver<u64>,
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
    ipc_channels: Arc<Mutex<HashMap<IpcChannelKey, IpcChannelCall>>>,
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
    // Bounded text and metadata APIs explicitly allowed over unary IPC.
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

fn session_owner_id(window_id: u8, session_id: &str) -> u64 {
    // The high bit separates stable page-session owners from per-WebView
    // WebSocket connection ids. IPC calls and IPC channels use this owner;
    // WebSocket work is owned by its concrete connection id.
    const OWNER_BIT: u64 = 1 << 63;
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    window_id.hash(&mut hasher);
    session_id.hash(&mut hasher);
    OWNER_BIT | (hasher.finish() & !OWNER_BIT)
}

fn new_channel_capability() -> Result<String> {
    let mut bytes = [0u8; 24];
    getrandom::fill(&mut bytes)
        .map_err(|error| anyhow!("channel capability entropy unavailable: {error}"))?;
    Ok(bytes.iter().map(|byte| format!("{byte:02x}")).collect())
}

fn constant_time_string_eq(left: &str, right: &str) -> bool {
    use subtle::ConstantTimeEq;
    let left = left.as_bytes();
    let right = right.as_bytes();
    left.len() == right.len() && bool::from(left.ct_eq(right))
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

fn with_rid(raw: String, rid: u64) -> String {
    let Ok(mut value) = serde_json::from_str::<Value>(&raw) else {
        return raw;
    };
    let Some(object) = value.as_object_mut() else {
        return raw;
    };
    object.insert("rid".into(), json!(rid));
    serde_json::to_string(&value).unwrap_or(raw)
}

fn with_optional_rid(raw: String, rid: Option<u64>) -> String {
    rid.map_or(raw.clone(), |rid| with_rid(raw, rid))
}

fn channel_error(rid: u64, id: u64, code: &str, message: &str, retryable: bool) -> String {
    serde_json::to_string(&json!({
        "t": "channelError",
        "rid": rid,
        "id": id,
        "code": code,
        "message": message,
        "retryable": retryable,
    }))
    .unwrap_or_else(|_| "{\"t\":\"channelError\"}".into())
}

fn ipc_method_allowed(method: &str) -> bool {
    IPC_METHOD_ALLOWLIST.contains(&method)
}

fn ipc_unary_method_allowed(trusted_local: bool, method: &str) -> bool {
    trusted_local || ipc_method_allowed(method)
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

    let trusted_origin = window.trusted_ws_origin.as_deref();
    if trusted_local_origin_matches(trusted_origin, &origin) {
        authorize_ipc_source_origin(trusted_origin, &window.token, &origin, token, false)?;
        return Ok(origin);
    }
    let remote_grant = match method {
        Some(method) => window.permissions.allows(source_url, method),
        None => window.permissions.allows_origin(source_url),
    };
    authorize_ipc_source_origin(trusted_origin, &window.token, &origin, token, remote_grant)?;
    Ok(origin)
}

fn authorize_ipc_source_origin(
    trusted_origin: Option<&str>,
    expected_token: &str,
    source_origin: &str,
    token: Option<&str>,
    remote_grant: bool,
) -> Result<bool> {
    if trusted_local_origin_matches(trusted_origin, source_origin) {
        anyhow::ensure!(
            trusted_local_ipc_auth_matches(trusted_origin, expected_token, source_origin, token),
            "trusted local origin requires the window token"
        );
        return Ok(true);
    }
    anyhow::ensure!(remote_grant, "permission denied");
    Ok(false)
}

fn trusted_local_ipc_source(window: &NivaWindow, source_origin: &str, token: Option<&str>) -> bool {
    trusted_local_ipc_auth_matches(
        window.trusted_ws_origin.as_deref(),
        &window.token,
        source_origin,
        token,
    )
}

fn trusted_local_origin_matches(trusted_origin: Option<&str>, source_origin: &str) -> bool {
    trusted_origin == Some(source_origin)
}

fn trusted_local_ipc_auth_matches(
    trusted_origin: Option<&str>,
    expected_token: &str,
    source_origin: &str,
    token: Option<&str>,
) -> bool {
    use subtle::ConstantTimeEq;

    if !trusted_local_origin_matches(trusted_origin, source_origin) {
        return false;
    }
    let supplied = token.unwrap_or_default().as_bytes();
    let expected = expected_token.as_bytes();
    supplied.len() == expected.len() && bool::from(supplied.ct_eq(expected))
}

fn may_open_channel_stream(trusted_local: bool) -> bool {
    trusted_local
}

fn normalize_ipc_source_origin(url: &url::Url) -> Result<String> {
    anyhow::ensure!(
        url.username().is_empty() && url.password().is_none(),
        "IPC source URLs cannot contain credentials"
    );
    if let Some(origin) = super::custom_protocol::origin_from_custom_uri(url) {
        return Ok(origin);
    }
    let origin = url.origin().ascii_serialization();
    anyhow::ensure!(origin != "null", "opaque IPC source is denied");
    Ok(origin)
}

fn same_origin_ipc_source(frame: &IpcFrameSource) -> Result<String> {
    let source_url =
        url::Url::parse(&frame.source_url).map_err(|_| anyhow!("IPC source URL is unavailable"))?;
    let top_url =
        url::Url::parse(&frame.top_url).map_err(|_| anyhow!("IPC top-level URL is unavailable"))?;
    let source_origin = normalize_ipc_source_origin(&source_url)?;
    let top_origin = normalize_ipc_source_origin(&top_url)?;
    anyhow::ensure!(
        source_origin == top_origin,
        "cross-origin frame IPC is denied"
    );
    Ok(source_origin)
}

async fn monitor_ipc_call(
    sessions: Arc<Mutex<IpcSessions>>,
    active: Arc<Mutex<ActiveCalls>>,
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
                // A heartbeat monitor can expire the whole session while a
                // stream call (including process.stdin) is still active.
                // Remove exact-key active calls and run their native cancel
                // hooks before closing any session sender.
                cancel_ipc_active_session_calls(&active, &key);
                for sender in senders {
                    sender.close();
                }
                return IpcRunOutcome::Cancelled;
            }
        }
    }
}

async fn monitor_ipc_channel_session(
    sessions: Arc<Mutex<IpcSessions>>,
    active: Arc<Mutex<ActiveCalls>>,
    channels: Arc<Mutex<HashMap<IpcChannelKey, IpcChannelCall>>>,
    key: IpcSessionKey,
    call_id: u64,
) {
    loop {
        smol::Timer::after(IPC_SESSION_CHECK_INTERVAL).await;
        let mut expired_senders = Vec::new();
        let expired = match sessions.lock() {
            Ok(sessions) if !sessions.active_call(&key, call_id) => false,
            Ok(mut sessions) => match sessions.expire_if_stale(&key, Instant::now()) {
                Ok(false) => continue,
                Ok(true) => true,
                Err(senders) => {
                    expired_senders = senders;
                    true
                }
            },
            Err(_) => true,
        };
        if !expired {
            cleanup_ipc_channel_resources(
                &channels,
                &IpcChannelKey {
                    session: key.clone(),
                    call_id,
                },
            );
            return;
        }

        crate::app::api::cancel_ipc_process_session(
            key.window_id,
            &key.source_origin,
            &key.session_id,
            key.frame_id,
            key.generation,
        );
        cancel_ipc_active_session_calls(&active, &key);
        for sender in expired_senders {
            sender.close();
        }
        cleanup_ipc_channel_session_resources(&channels, &key);
        return;
    }
}

fn cleanup_ipc_channel_resources(
    channels: &Mutex<HashMap<IpcChannelKey, IpcChannelCall>>,
    key: &IpcChannelKey,
) {
    if let Ok(mut channels) = channels.lock() {
        channels.remove(key);
    }
}

fn cancel_ipc_channel_call(
    api: &ApiManager,
    sessions: &Mutex<IpcSessions>,
    channels: &Mutex<HashMap<IpcChannelKey, IpcChannelCall>>,
    key: &IpcChannelKey,
) {
    let sender = sessions
        .lock()
        .ok()
        .and_then(|mut sessions| sessions.cancel_call(&key.session, key.call_id));
    if !api.cancel_ipc_active_call(&key.session, key.call_id)
        && let Some(sender) = sender
    {
        sender.close();
    }
    cleanup_ipc_channel_resources(channels, key);
}

fn cleanup_ipc_channel_session_resources(
    channels: &Mutex<HashMap<IpcChannelKey, IpcChannelCall>>,
    key: &IpcSessionKey,
) {
    if let Ok(mut channels) = channels.lock() {
        let ids = channels
            .keys()
            .filter(|channel_key| &channel_key.session == key)
            .cloned()
            .collect::<Vec<_>>();
        for id in ids {
            channels.remove(&id);
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
    connection_id: u64,
    calls: HashMap<u64, async_channel::Sender<()>>,
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
struct IpcChannelKey {
    session: IpcSessionKey,
    call_id: u64,
}

struct IpcChannelCall {
    method: String,
    capability: String,
    connection_id: u64,
    transport: Option<IpcChannelTransport>,
    attach_tx: async_channel::Sender<IpcChannelTransport>,
    next_upload_seq: u64,
    upload_ended: bool,
    next_http_response_ack_seq: u64,
    http_response_ack_tx: async_channel::Sender<u64>,
    awaiting_output_seq: Option<u64>,
    output_ack_tx: async_channel::Sender<u64>,
}

#[derive(Clone)]
enum IpcChannelTransport {
    Ipc,
    WebSocket {
        connection_id: u64,
        sender: mpsc::Sender<WsOut>,
    },
}

impl IpcChannelTransport {
    fn same_route(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Ipc, Self::Ipc) => true,
            (
                Self::WebSocket {
                    connection_id: left,
                    ..
                },
                Self::WebSocket {
                    connection_id: right,
                    ..
                },
            ) => left == right,
            _ => false,
        }
    }
}

#[derive(Debug, Eq, PartialEq)]
enum WsAttachDeliveryError {
    PumpUnavailable,
    ConfirmationUnavailable(String),
}

fn attach_ws_transport_then_confirm(
    channel: &mut IpcChannelCall,
    transport: IpcChannelTransport,
    confirm: impl FnOnce() -> std::result::Result<(), String>,
) -> std::result::Result<(), WsAttachDeliveryError> {
    channel.transport = Some(transport.clone());
    if channel.attach_tx.try_send(transport).is_err() {
        channel.transport = None;
        return Err(WsAttachDeliveryError::PumpUnavailable);
    }
    confirm().map_err(WsAttachDeliveryError::ConfirmationUnavailable)
}

fn find_ws_channel_key(
    channels: &HashMap<IpcChannelKey, IpcChannelCall>,
    window_id: u8,
    trusted_origin: &str,
    session_id: &str,
    call_id: u64,
    capability: &str,
) -> Option<IpcChannelKey> {
    channels
        .iter()
        .find(|(key, channel)| {
            key.session.window_id == window_id
                && key.session.source_origin == trusted_origin
                && key.session.session_id == session_id
                && key.call_id == call_id
                && constant_time_string_eq(&channel.capability, capability)
        })
        .map(|(key, _)| key.clone())
}

const MAX_CHANNEL_BINARY_PAYLOAD: usize = 16 * 1024;
const MAX_CHANNEL_TEXT_FRAME_BYTES: usize = 64 * 1024;
const CHANNEL_OUTBOUND_QUEUE_FRAMES: usize = 8;
const CHANNEL_FRAME_ACK_TIMEOUT: Duration = Duration::from_secs(3);

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
        connection_id: u64,
        now: Instant,
    ) -> std::result::Result<u64, IpcSessionError> {
        if self.blocked_windows.contains(&key.window_id) || self.expired.contains(&key) {
            return Err(IpcSessionError::Expired);
        }
        let session = self.active.entry(key).or_insert_with(|| ActiveIpcSession {
            last_heartbeat: now,
            connection_id,
            calls: HashMap::new(),
        });
        if session.calls.contains_key(&call_id) {
            return Err(IpcSessionError::DuplicateCall);
        }
        session.last_heartbeat = now;
        session.calls.insert(call_id, cancel_tx);
        Ok(session.connection_id)
    }

    fn touch_call(&mut self, key: &IpcSessionKey, call_id: u64, now: Instant) -> bool {
        let Some(session) = self.active.get_mut(key) else {
            return false;
        };
        if !session.calls.contains_key(&call_id) {
            return false;
        }
        session.last_heartbeat = now;
        true
    }

    fn active_call(&self, key: &IpcSessionKey, call_id: u64) -> bool {
        self.active
            .get(key)
            .is_some_and(|session| session.calls.contains_key(&call_id))
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

    fn retire(&mut self, key: &IpcSessionKey) -> Vec<(u64, async_channel::Sender<()>)> {
        let calls = self
            .active
            .remove(key)
            .map(|session| session.calls.into_iter().collect())
            .unwrap_or_default();
        // Tombstone even an idle key so a delayed api_call cannot revive a
        // session after pagehide. Repeated closes are idempotent; new unknown
        // ids consume the existing bounded per-window tombstone budget.
        self.remember_expired(key.clone());
        calls
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

fn cancel_ipc_active_call(
    active: &Mutex<ActiveCalls>,
    session: &IpcSessionKey,
    call_id: u64,
) -> bool {
    let owner_id = session_owner_id(session.window_id, &session.session_id);
    let active_key = (session.window_id, owner_id, call_id);
    let call = {
        let mut active = match active.lock() {
            Ok(active) => active,
            Err(_) => return false,
        };
        if !active
            .get(&active_key)
            .is_some_and(|call| call.ipc_session.as_ref() == Some(session))
        {
            return false;
        }
        let call = active.remove(&active_key);
        if let Some(call) = &call {
            call.lifecycle.cancel();
        }
        call
    };
    let Some(call) = call else {
        return false;
    };

    crate::app::api::cancel_ipc_process_call(
        session.window_id,
        &session.source_origin,
        &session.session_id,
        session.frame_id,
        session.generation,
        call_id,
    );
    // This wakes resources such as process.stdin registered under the stable
    // owner/call pair. It is safe only after the full-key check and atomic
    // active-call removal above.
    crate::app::api::cancel_process_call(session.window_id, owner_id, call_id);
    cancel_active_call(call);
    true
}

fn cancel_ipc_active_session_calls(active: &Mutex<ActiveCalls>, session: &IpcSessionKey) {
    let owner_id = session_owner_id(session.window_id, &session.session_id);
    let call_ids = active
        .lock()
        .map(|active| {
            active
                .iter()
                .filter_map(|(key, call)| {
                    (key.0 == session.window_id
                        && key.1 == owner_id
                        && call.ipc_session.as_ref() == Some(session))
                    .then_some(key.2)
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    for call_id in call_ids {
        cancel_ipc_active_call(active, session, call_id);
    }
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
    output: &CallOutput,
    response: ApiResponse,
) {
    remove_active_if_same(active, key, &lifecycle);
    let should_respond = claim_terminal(&lifecycle);
    lifecycle.cancel();
    cancel_tx.close();
    if should_respond {
        respond_output(output, response);
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
            respond_output(
                &call.outbound,
                ApiResponse(header.id, -3, message.to_string(), json!(null)),
            );
        }
    }
}

fn validate_ipc_channel_upload_frame(
    raw_frame: &[u8],
    call_id: u64,
    expected_seq: u64,
) -> Result<(protocol::ChunkHeader, &[u8])> {
    anyhow::ensure!(
        raw_frame.len() <= protocol::BIN_HEADER_LEN + MAX_CHANNEL_BINARY_PAYLOAD,
        "ERR_NIVA_CHANNEL_FRAME: binary frame exceeds the 16 KiB payload limit"
    );
    let (header, payload) =
        decode_chunk(raw_frame).map_err(|error| anyhow!("ERR_NIVA_CHANNEL_FRAME: {error}"))?;
    let valid_flags = protocol::FLAG_START | protocol::FLAG_END | protocol::FLAG_STDERR;
    anyhow::ensure!(
        raw_frame[1] & !valid_flags == 0,
        "ERR_NIVA_CHANNEL_FRAME: unknown upload flags"
    );
    anyhow::ensure!(
        header.id == call_id && header.start == (header.seq == 1) && !header.stderr,
        "ERR_NIVA_CHANNEL_FRAME: invalid call id or upload flags"
    );
    anyhow::ensure!(
        header.seq == expected_seq,
        "ERR_NIVA_CHANNEL_SEQUENCE: expected upload sequence {expected_seq}"
    );
    anyhow::ensure!(
        payload.len() <= MAX_CHANNEL_BINARY_PAYLOAD,
        "ERR_NIVA_CHANNEL_FRAME: payload exceeds the 16 KiB limit"
    );
    Ok((header, payload))
}

fn validate_open_channel_upload_frame<'a>(
    channel: &IpcChannelCall,
    raw_frame: &'a [u8],
    call_id: u64,
) -> Result<(protocol::ChunkHeader, &'a [u8])> {
    anyhow::ensure!(
        !channel.upload_ended,
        "ERR_NIVA_CHANNEL_CLOSED: upload stream already ended"
    );
    validate_ipc_channel_upload_frame(raw_frame, call_id, channel.next_upload_seq)
}

fn grant_http_response_credit(channel: &mut IpcChannelCall, seq: u64) -> Result<()> {
    anyhow::ensure!(
        channel.method == "http.requestStream",
        "ERR_NIVA_CHANNEL_DENIED: response credit requires an HTTP request stream"
    );
    anyhow::ensure!(
        channel.transport.is_some(),
        "ERR_NIVA_CHANNEL_CLOSED: HTTP request stream is not attached"
    );
    anyhow::ensure!(
        channel.upload_ended,
        "ERR_NIVA_CHANNEL_STATE: HTTP response credit is unavailable before upload END"
    );
    anyhow::ensure!(
        seq == channel.next_http_response_ack_seq,
        "ERR_NIVA_CHANNEL_SEQUENCE: expected HTTP response credit {}",
        channel.next_http_response_ack_seq
    );
    let next_seq = seq
        .checked_add(1)
        .ok_or_else(|| anyhow!("ERR_NIVA_CHANNEL_SEQUENCE: HTTP response sequence exhausted"))?;
    channel
        .http_response_ack_tx
        .try_send(seq)
        .map_err(|error| match error {
            async_channel::TrySendError::Full(_) => {
                anyhow!("ERR_NIVA_CHANNEL_BACKPRESSURE: HTTP response credit is already pending")
            }
            async_channel::TrySendError::Closed(_) => {
                anyhow!("ERR_NIVA_CHANNEL_CLOSED: HTTP request stream is closed")
            }
        })?;
    channel.next_http_response_ack_seq = next_seq;
    Ok(())
}

fn grant_http_response_credit_for_session(
    channels: &Mutex<HashMap<IpcChannelKey, IpcChannelCall>>,
    active: &Mutex<ActiveCalls>,
    session: &IpcSessionKey,
    stream_id: u64,
    seq: u64,
) -> Result<()> {
    let key = IpcChannelKey {
        session: session.clone(),
        call_id: stream_id,
    };
    let mut channels = channels
        .lock()
        .map_err(|_| anyhow!("IPC channel table poisoned"))?;
    let channel = channels
        .get_mut(&key)
        .ok_or_else(|| anyhow!("ERR_NIVA_CHANNEL_DENIED: unknown HTTP request stream"))?;
    anyhow::ensure!(
        channel.connection_id == session_owner_id(session.window_id, &session.session_id),
        "ERR_NIVA_CHANNEL_DENIED: HTTP request stream owner mismatch"
    );
    let active_key = (session.window_id, channel.connection_id, stream_id);
    let active = active
        .lock()
        .map_err(|_| anyhow!("active call table poisoned"))?;
    anyhow::ensure!(
        active
            .get(&active_key)
            .is_some_and(|call| !call.lifecycle.is_terminal()),
        "ERR_NIVA_CHANNEL_CLOSED: HTTP request stream is not active"
    );
    grant_http_response_credit(channel, seq)
}

fn acknowledge_ipc_channel_output(channel: &mut IpcChannelCall, seq: u64) -> Result<()> {
    anyhow::ensure!(
        channel.awaiting_output_seq == Some(seq),
        "ERR_NIVA_CHANNEL_SEQUENCE: ACK does not match the pending output frame"
    );
    channel.awaiting_output_seq = None;
    channel
        .output_ack_tx
        .try_send(seq)
        .map_err(|error| anyhow!("ERR_NIVA_CHANNEL_ACK: {error}"))
}

fn send_ws_channel_frame(
    sender: &mpsc::Sender<WsOut>,
    session_id: &str,
    call_id: u64,
    channel_seq: u64,
    frame: WsOut,
) -> bool {
    match frame {
        WsOut::Text(data) => serde_json::to_string(&json!({
            "t":"channelData",
            "sessionId":session_id,
            "id":call_id,
            "seq":channel_seq,
            "frame":{"t":"text","data":data},
        }))
        .map(|text| sender.send(WsOut::Text(text)).is_ok())
        .unwrap_or(false),
        WsOut::Binary(mut bytes) => {
            let Ok((header, _)) = decode_chunk(&bytes) else {
                return false;
            };
            if header.id != call_id {
                return false;
            }
            bytes[10..18].copy_from_slice(&channel_seq.to_be_bytes());
            sender.send(WsOut::Binary(bytes)).is_ok()
        }
    }
}

fn enqueue_ipc_channel_chunk(
    active: &Mutex<ActiveCalls>,
    key: CallKey,
    chunk: InboundChunk,
) -> Result<usize> {
    let byte_length = chunk.data.len();
    let active = active
        .lock()
        .map_err(|_| anyhow!("active call table poisoned"))?;
    let active_call = active
        .get(&key)
        .ok_or_else(|| anyhow!("ERR_NIVA_CHANNEL_CLOSED: stream consumer is closed"))?;
    anyhow::ensure!(
        !active_call.lifecycle.is_terminal(),
        "ERR_NIVA_CHANNEL_CLOSED: stream consumer is closed"
    );
    match active_call.inbound_tx.try_send(chunk) {
        Ok(()) => Ok(byte_length),
        Err(async_channel::TrySendError::Full(_)) => anyhow::bail!(
            "ERR_NIVA_CHANNEL_BACKPRESSURE: upload queue is full; retry this sequence"
        ),
        Err(async_channel::TrySendError::Closed(_)) => {
            anyhow::bail!("ERR_NIVA_CHANNEL_CLOSED: stream consumer is closed")
        }
    }
}

impl ApiManager {
    pub fn ipc_error_envelope(
        raw_body: &str,
        error: &str,
        frame: &IpcFrameSource,
    ) -> Option<String> {
        if raw_body.len() > MAX_IPC_REQUEST_BYTES {
            return None;
        }
        let request: Value = serde_json::from_str(raw_body).ok()?;
        let session_id = request.get("sessionId")?.as_str()?;
        let rid = request.get("rid")?.as_u64()?;
        let id = request.get("id").and_then(Value::as_u64).unwrap_or(0);
        let source_origin = same_origin_ipc_source(frame).ok()?;
        let response = match request.get("t").and_then(Value::as_str) {
            Some("channelAttach" | "channelSend" | "channelAck" | "channelCancel") => {
                json!({
                    "t":"channelError", "id":id,
                    "code":"ERR_NIVA_IPC", "message":error, "retryable":false,
                })
            }
            Some("heartbeat") => json!({
                "t":"heartbeatError", "sessionId":session_id,
                "code":"ERR_NIVA_IPC", "message":error,
            }),
            _ => json!({
                "t":"result", "id":id, "code":-1,
                "message":error, "data":null,
            }),
        };
        serde_json::to_string(&json!({
            "sessionId":session_id,
            "rid":rid,
            "sourceOrigin":source_origin,
            "response":response,
        }))
        .ok()
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
                        if matches!(&job.ctx.inner.outbound, CallOutput::Channel(_)) {
                            let active = loop_active.clone();
                            let key = (job.ctx.window.id, job.ctx.connection_id, job.ctx.id);
                            let lifecycle = job.ctx.inner.lifecycle.clone();
                            let output = job.ctx.inner.outbound.clone();
                            let request = job.request.clone();
                            let worker = thread::Builder::new()
                                .name("niva-channel-call".into())
                                .spawn(move || smol::block_on(run_job(job, active)));
                            if let Err(error) = worker {
                                if let Some(call) =
                                    remove_active_if_same(&loop_active, key, &lifecycle)
                                {
                                    call.cancel_tx.close();
                                }
                                if claim_terminal(&lifecycle) {
                                    respond_output(
                                        &output,
                                        request.err(
                                            -1,
                                            format!(
                                                "unable to start Native channel worker: {error}"
                                            ),
                                        ),
                                    );
                                }
                            }
                        } else {
                            smol::spawn(run_job(job, loop_active.clone())).detach();
                        }
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
            ipc_channels: Arc::new(Mutex::new(HashMap::new())),
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

    /// Grant one application-level response-body credit for an HTTP request
    /// stream. The control API has already authenticated this exact IPC frame;
    /// the ticket lookup binds the credit to that frame's complete session and
    /// to the live `http.requestStream` resource.
    pub fn acknowledge_http_response(
        &self,
        context: &IpcCallContext,
        stream_id: u64,
        seq: u64,
    ) -> Result<()> {
        anyhow::ensure!(
            !context.is_cancelled(),
            "ECANCELED: HTTP response acknowledgement was cancelled"
        );
        let session = IpcSessionKey {
            window_id: context.window.id,
            source_origin: context.source_origin.clone(),
            session_id: context.session_id.clone(),
            frame_id: context.frame_id,
            generation: context.generation,
            is_main_frame: context.is_main_frame,
        };
        grant_http_response_credit_for_session(
            &self.ipc_channels,
            &self.active,
            &session,
            stream_id,
            seq,
        )
    }

    pub fn module_resolver(&self) -> Result<Arc<ModuleResolver>> {
        self.module_resolver
            .get()
            .cloned()
            .ok_or_else(|| anyhow!("module resolver not initialized"))
    }

    /// Handle one platform-provided WebView IPC message. Source URL and frame
    /// identity are supplied by the native callback, never by the JSON body.
    /// Trusted local frames may open registered streams; remote frames retain
    /// the explicit unary allowlist and cannot reach stream handlers.
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
        let (session_id, rid) = match &message {
            IpcMessage::SessionClose { session_id, .. } => (session_id.clone(), None),
            IpcMessage::ApiCall {
                session_id, rid, ..
            }
            | IpcMessage::ChannelAttach {
                session_id, rid, ..
            }
            | IpcMessage::ChannelSend {
                session_id, rid, ..
            }
            | IpcMessage::ChannelAck {
                session_id, rid, ..
            }
            | IpcMessage::ChannelCancel {
                session_id, rid, ..
            } => (session_id.clone(), Some(*rid)),
            IpcMessage::Heartbeat {
                session_id, rid, ..
            } => (session_id.clone(), *rid),
        };
        let source_origin = same_origin_ipc_source(&frame)?;
        let response = match message {
            IpcMessage::SessionClose { session_id, token } => self
                .ipc_session_close(window_id, frame, &session_id, token.as_deref())
                .map(|accepted| {
                    json!({
                        "t":"sessionClosed",
                        "sessionId":session_id,
                        "accepted":accepted,
                    })
                    .to_string()
                }),
            IpcMessage::Heartbeat {
                rid,
                session_id,
                token,
            } => self.ipc_heartbeat(window_id, frame, &session_id, token.as_deref(), rid),
            IpcMessage::ChannelAttach {
                rid,
                id,
                session_id,
                token,
                capability,
            } => self.ipc_channel_attach(
                window_id,
                frame,
                rid,
                id,
                &session_id,
                token.as_deref(),
                &capability,
            ),
            IpcMessage::ChannelSend {
                rid,
                id,
                session_id,
                token,
                capability,
                frame: encoded_frame,
            } => self.ipc_channel_send(
                window_id,
                frame,
                rid,
                id,
                &session_id,
                token.as_deref(),
                &capability,
                &encoded_frame,
            ),
            IpcMessage::ChannelAck {
                rid,
                id,
                session_id,
                token,
                capability,
                seq,
            } => self.ipc_channel_ack(
                window_id,
                frame,
                rid,
                id,
                &session_id,
                token.as_deref(),
                &capability,
                seq,
            ),
            IpcMessage::ChannelCancel {
                rid,
                id,
                session_id,
                token,
                capability,
            } => self.ipc_channel_cancel(
                window_id,
                frame,
                rid,
                id,
                &session_id,
                token.as_deref(),
                &capability,
            ),
            IpcMessage::ApiCall {
                rid,
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
                    rid,
                )
                .await
            }
        }?;
        let mut response: Value = serde_json::from_str(&response)?;
        if let Some(response) = response.as_object_mut() {
            // `rid`, session identity and provenance are carried by the
            // evaluate-script envelope, outside the API response payload.
            response.remove("rid");
        }
        Ok(serde_json::to_string(&json!({
            "sessionId": session_id,
            "rid": rid,
            "sourceOrigin": source_origin,
            "response": response,
        }))?)
    }

    fn ipc_heartbeat(
        &self,
        window_id: u8,
        frame: IpcFrameSource,
        session_id: &str,
        token: Option<&str>,
        rid: Option<u64>,
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
                return Ok(with_optional_rid(
                    heartbeat_failure(
                        session_id,
                        "ERR_NIVA_IPC_DENIED",
                        "IPC window is unavailable",
                    ),
                    rid,
                ));
            }
        };
        let source_origin = match authorize_ipc_source(&window, &frame.source_url, token, None) {
            Ok(origin) => origin,
            Err(_) => {
                return Ok(with_optional_rid(
                    heartbeat_failure(session_id, "ERR_NIVA_IPC_DENIED", "permission denied"),
                    rid,
                ));
            }
        };
        if validate_session_id(session_id).is_err() {
            return Ok(with_optional_rid(
                heartbeat_failure(session_id, "ERR_NIVA_IPC_DENIED", "invalid IPC session id"),
                rid,
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
        let mut response = response;
        if let Some(rid) = rid {
            response["rid"] = json!(rid);
        }
        Ok(serde_json::to_string(&response)?)
    }

    fn ipc_session_close(
        &self,
        window_id: u8,
        frame: IpcFrameSource,
        session_id: &str,
        token: Option<&str>,
    ) -> Result<bool> {
        validate_session_id(session_id)?;
        let source_origin = same_origin_ipc_source(&frame)?;
        let app = self
            .app
            .get()
            .cloned()
            .ok_or_else(|| anyhow!("API manager not ready"))?;
        let window = app.window()?.get_window(window_id)?;
        let authorized_origin = authorize_ipc_source(&window, &frame.source_url, token, None)?;
        anyhow::ensure!(
            authorized_origin == source_origin,
            "IPC source origin changed during session close"
        );
        let key = IpcSessionKey {
            window_id,
            source_origin: source_origin.clone(),
            session_id: session_id.to_owned(),
            frame_id: frame.frame_id,
            generation: frame.generation,
            is_main_frame: frame.is_main_frame,
        };
        let calls = self
            .ipc_sessions
            .lock()
            .map_err(|_| anyhow!("IPC session table poisoned"))?
            .retire(&key);
        for (call_id, sender) in calls {
            if !self.cancel_ipc_active_call(&key, call_id) {
                sender.close();
            }
        }
        self.cancel_ipc_channel_sessions(|channel| channel == &key);
        crate::app::api::cancel_ipc_process_session(
            window_id,
            &source_origin,
            session_id,
            frame.frame_id,
            frame.generation,
        );
        if trusted_local_ipc_source(&window, &source_origin, token) {
            self.cancel_sync_session(window_id, session_id);
        }
        Ok(true)
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
        rid: u64,
    ) -> Result<String> {
        let request = ApiRequest(id, method, ApiArguments(args));
        let denied = |code, message: &str, data| {
            with_rid(
                ServerMsg::result(id, code, message.to_owned(), data).encode(),
                rid,
            )
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
        let trusted_local = trusted_local_ipc_source(&window, &source_origin, token);
        if !trusted_local
            && matches!(
                request.1.as_str(),
                "window.open" | "webview.baseFileSystemUrl"
            )
        {
            return Ok(denied(-4, "permission denied", json!(null)));
        }
        if let Some(HandlerEntry {
            handler: HandlerKind::Stream(_),
            ..
        }) = self.handlers.get(&request.1)
        {
            if !may_open_channel_stream(trusted_local) {
                return Ok(denied(
                    -4,
                    "stream APIs require trusted local origin",
                    json!(null),
                ));
            }
            let key = IpcSessionKey {
                window_id,
                source_origin,
                session_id: session_id.clone(),
                frame_id: frame.frame_id,
                generation: frame.generation,
                is_main_frame: frame.is_main_frame,
            };
            return self.ipc_stream_open(key, id, request, rid);
        }
        if !ipc_unary_method_allowed(trusted_local, &request.1) {
            return Ok(denied(-4, "permission denied", json!(null)));
        }
        let handler = self.ipc_handlers.get(&request.1).cloned().or_else(|| {
            match self.handlers.get(&request.1).map(|entry| &entry.handler) {
                Some(HandlerKind::CancellableUnary(handler)) => {
                    Some(cancellable_ipc_handler(handler.clone()))
                }
                Some(HandlerKind::Unary(handler))
                    if trusted_local || IPC_LEGACY_UNARY_METHODS.contains(&request.1.as_str()) =>
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
        let connection_id = session_owner_id(key.window_id, &key.session_id);
        match self
            .ipc_sessions
            .lock()
            .map_err(|_| anyhow!("IPC session table poisoned"))?
            .reserve(
                key.clone(),
                id,
                cancel_tx.clone(),
                connection_id,
                Instant::now(),
            ) {
            Ok(_) => {}
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
        let monitor = monitor_ipc_call(
            self.ipc_sessions.clone(),
            self.active.clone(),
            key.clone(),
            id,
            cancel_rx,
        );
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
        Ok(with_rid(encoded, rid))
    }

    fn ipc_stream_open(
        &self,
        key: IpcSessionKey,
        id: u64,
        request: ApiRequest,
        rid: u64,
    ) -> Result<String> {
        let app = self
            .app
            .get()
            .cloned()
            .ok_or_else(|| anyhow!("API manager not ready"))?;
        let window = app.window()?.get_window(key.window_id)?;
        let (cancel_tx, cancel_rx) = async_channel::bounded(1);
        let (inbound_tx, inbound_rx) = async_channel::bounded::<InboundChunk>(8);
        let (http_response_ack_tx, http_response_ack_rx) = async_channel::bounded::<u64>(1);
        let (outbound_tx, outbound_rx) = mpsc::sync_channel::<WsOut>(CHANNEL_OUTBOUND_QUEUE_FRAMES);
        let (output_ack_tx, output_ack_rx) = async_channel::bounded::<u64>(1);
        let (attach_tx, attach_rx) = async_channel::bounded::<IpcChannelTransport>(1);
        let connection_id = session_owner_id(key.window_id, &key.session_id);
        let session_connection_id = match self
            .ipc_sessions
            .lock()
            .map_err(|_| anyhow!("IPC session table poisoned"))?
            .reserve(
                key.clone(),
                id,
                cancel_tx.clone(),
                connection_id,
                Instant::now(),
            ) {
            Ok(connection_id) => connection_id,
            Err(IpcSessionError::Expired) => {
                return Ok(with_rid(
                    ServerMsg::result(
                        id,
                        -1,
                        "ERR_NIVA_SESSION_EXPIRED: reload the page before retrying".into(),
                        json!({"code":"ERR_NIVA_SESSION_EXPIRED"}),
                    )
                    .encode(),
                    rid,
                ));
            }
            Err(IpcSessionError::DuplicateCall) => {
                return Ok(with_rid(
                    ServerMsg::result(id, -1, "duplicate IPC request id".into(), json!(null))
                        .encode(),
                    rid,
                ));
            }
        };
        let capability = new_channel_capability()?;
        let channel_key = IpcChannelKey {
            session: key.clone(),
            call_id: id,
        };
        {
            let mut channels = self
                .ipc_channels
                .lock()
                .map_err(|_| anyhow!("IPC channel table poisoned"))?;
            if channels.len() >= self.max_ipc_active {
                drop(channels);
                if let Ok(mut sessions) = self.ipc_sessions.lock()
                    && let Some(sender) = sessions.cancel_call(&key, id)
                {
                    sender.close();
                }
                return Ok(with_rid(
                    ServerMsg::result(
                        id,
                        -3,
                        "IPC channel limit reached".into(),
                        json!({"code":"ERR_NIVA_CHANNEL_BUSY"}),
                    )
                    .encode(),
                    rid,
                ));
            }
            if channels.contains_key(&channel_key) {
                cancel_tx.close();
                if let Ok(mut sessions) = self.ipc_sessions.lock()
                    && let Some(sender) = sessions.cancel_call(&key, id)
                {
                    sender.close();
                }
                return Ok(with_rid(
                    ServerMsg::result(id, -1, "duplicate active IPC channel".into(), json!(null))
                        .encode(),
                    rid,
                ));
            }
            channels.insert(
                channel_key.clone(),
                IpcChannelCall {
                    method: request.1.clone(),
                    capability: capability.clone(),
                    connection_id: session_connection_id,
                    transport: None,
                    attach_tx,
                    next_upload_seq: 1,
                    upload_ended: false,
                    next_http_response_ack_seq: 1,
                    http_response_ack_tx,
                    awaiting_output_seq: None,
                    output_ack_tx,
                },
            );
        }

        let channels = DispatchChannels {
            cancel_tx: cancel_tx.clone(),
            cancel_rx,
            inbound_tx,
            inbound_rx,
            http_response_ack_rx,
        };
        let queued = self.dispatch_with_output(
            key.window_id,
            session_connection_id,
            CallOutput::Channel(outbound_tx),
            request,
            channels,
            Some(key.clone()),
        );
        if !queued {
            self.remove_ipc_channel(&channel_key);
            if let Ok(mut sessions) = self.ipc_sessions.lock()
                && let Some(sender) = sessions.cancel_call(&key, id)
            {
                sender.close();
            }
            return Ok(with_rid(
                ServerMsg::result(
                    id,
                    -3,
                    "IPC stream dispatch queue is full".into(),
                    json!({"code":"ERR_NIVA_CHANNEL_BUSY"}),
                )
                .encode(),
                rid,
            ));
        }

        let monitor_sessions = self.ipc_sessions.clone();
        let monitor_active = self.active.clone();
        let monitor_channels = self.ipc_channels.clone();
        let monitor_key = key.clone();
        smol::spawn(async move {
            monitor_ipc_channel_session(
                monitor_sessions,
                monitor_active,
                monitor_channels,
                monitor_key,
                id,
            )
            .await;
        })
        .detach();

        let push_app = app.clone();
        let push_window = window;
        let push_key = channel_key.clone();
        let push_channels = self.ipc_channels.clone();
        let push_sessions = self.ipc_sessions.clone();
        let push_output = outbound_rx;
        let push_ack_rx = output_ack_rx;
        let push_attach_rx = attach_rx;
        let pump_result = thread::Builder::new()
            .name("niva-ipc-channel".into())
            .spawn(move || {
                let transport = match smol::block_on(push_attach_rx.recv()) {
                    Ok(transport) => transport,
                    Err(_) => {
                        cancel_ipc_channel_call(
                            &push_app.api(),
                            &push_sessions,
                            &push_channels,
                            &push_key,
                        );
                        return;
                    }
                };
                let mut channel_seq = 1u64;
                while let Ok(frame) = push_output.recv() {
                    let pending = push_channels.lock().ok().and_then(|mut channels| {
                        let channel = channels.get_mut(&push_key)?;
                        if channel.awaiting_output_seq.is_some() {
                            return None;
                        }
                        channel.awaiting_output_seq = Some(channel_seq);
                        Some(())
                    });
                    let Some(()) = pending else {
                        break;
                    };
                    let terminal = is_terminal_output(&frame);
                    let sent = match transport.clone() {
                        IpcChannelTransport::Ipc => push_window.send_ipc_frame(
                            &push_key.session.session_id,
                            id,
                            channel_seq,
                            frame,
                        ),
                        IpcChannelTransport::WebSocket { sender, .. } => send_ws_channel_frame(
                            &sender,
                            &push_key.session.session_id,
                            id,
                            channel_seq,
                            frame,
                        ),
                    };
                    if !sent {
                        cancel_ipc_channel_call(
                            &push_app.api(),
                            &push_sessions,
                            &push_channels,
                            &push_key,
                        );
                        return;
                    }
                    let ack = smol::block_on(smol::future::or(
                        async { push_ack_rx.recv().await.ok() },
                        async {
                            smol::Timer::after(CHANNEL_FRAME_ACK_TIMEOUT).await;
                            None
                        },
                    ));
                    if ack != Some(channel_seq) {
                        cancel_ipc_channel_call(
                            &push_app.api(),
                            &push_sessions,
                            &push_channels,
                            &push_key,
                        );
                        return;
                    }
                    if terminal {
                        if let Ok(mut sessions) = push_sessions.lock() {
                            sessions.complete(&push_key.session, id);
                        }
                        cleanup_ipc_channel_resources(&push_channels, &push_key);
                        return;
                    }
                    channel_seq = channel_seq.saturating_add(1);
                }
                cancel_ipc_channel_call(&push_app.api(), &push_sessions, &push_channels, &push_key);
            });
        if let Err(error) = pump_result {
            cancel_ipc_channel_call(
                &app.api(),
                &self.ipc_sessions,
                &self.ipc_channels,
                &channel_key,
            );
            return Ok(with_rid(
                ServerMsg::result(
                    id,
                    -1,
                    format!("unable to start IPC channel pump: {error}"),
                    json!({"code":"ERR_NIVA_CHANNEL_UNAVAILABLE"}),
                )
                .encode(),
                rid,
            ));
        }

        Ok(serde_json::to_string(&json!({
            "t": "channelOpened",
            "rid": rid,
            "id": id,
            "capability": capability,
        }))?)
    }

    fn ipc_channel_attach(
        &self,
        window_id: u8,
        frame: IpcFrameSource,
        rid: u64,
        call_id: u64,
        session_id: &str,
        token: Option<&str>,
        capability: &str,
    ) -> Result<String> {
        let Some(token) = token else {
            return Ok(channel_error(
                rid,
                call_id,
                "ERR_NIVA_CHANNEL_DENIED",
                "trusted local page token is required",
                false,
            ));
        };
        let key = match self.authorize_ipc_channel_route(
            window_id,
            &frame.source_url,
            token,
            session_id,
            call_id,
            capability,
            Some(&frame),
        ) {
            Ok(key) => key,
            Err(error) => {
                return Ok(channel_error(
                    rid,
                    call_id,
                    "ERR_NIVA_CHANNEL_DENIED",
                    &error.to_string(),
                    false,
                ));
            }
        };
        match self.bind_channel_transport(&key, IpcChannelTransport::Ipc) {
            Ok(()) => Ok(serde_json::to_string(&json!({
                "t":"channelAttached",
                "rid":rid,
                "id":call_id,
                "accepted":true,
            }))?),
            Err(error) => Ok(channel_error(
                rid,
                call_id,
                "ERR_NIVA_CHANNEL_ATTACH",
                &error.to_string(),
                false,
            )),
        }
    }

    fn bind_channel_transport(
        &self,
        key: &IpcChannelKey,
        transport: IpcChannelTransport,
    ) -> Result<()> {
        let mut channels = self
            .ipc_channels
            .lock()
            .map_err(|_| anyhow!("IPC channel table poisoned"))?;
        let channel = channels
            .get_mut(key)
            .ok_or_else(|| anyhow!("ERR_NIVA_CHANNEL_CLOSED: channel is no longer active"))?;
        if let Some(existing) = &channel.transport {
            anyhow::ensure!(
                existing.same_route(&transport),
                "ERR_NIVA_CHANNEL_ATTACHED: ticket is already pinned to another transport"
            );
            return Ok(());
        }
        channel.transport = Some(transport.clone());
        if channel.attach_tx.try_send(transport).is_err() {
            channel.transport = None;
            anyhow::bail!("ERR_NIVA_CHANNEL_CLOSED: channel pump is unavailable");
        }
        Ok(())
    }

    fn remove_ipc_channel(&self, key: &IpcChannelKey) {
        if let Ok(mut channels) = self.ipc_channels.lock() {
            channels.remove(key);
        }
    }

    fn authorize_ipc_channel_route(
        &self,
        window_id: u8,
        source_url: &str,
        token: &str,
        session_id: &str,
        call_id: u64,
        capability: &str,
        frame: Option<&IpcFrameSource>,
    ) -> Result<IpcChannelKey> {
        validate_session_id(session_id)?;
        let app = self
            .app
            .get()
            .cloned()
            .ok_or_else(|| anyhow!("API manager not ready"))?;
        let window = app.window()?.get_window(window_id)?;
        let source_origin = authorize_ipc_source(&window, source_url, Some(token), None)?;
        anyhow::ensure!(
            trusted_local_ipc_source(&window, &source_origin, Some(token)),
            "ERR_NIVA_CHANNEL_DENIED: channel routes require a trusted local page"
        );

        let channels = self
            .ipc_channels
            .lock()
            .map_err(|_| anyhow!("IPC channel table poisoned"))?;
        let match_key = channels.keys().find(|key| {
            key.session.window_id == window_id
                && key.session.source_origin == source_origin
                && key.session.session_id == session_id
                && key.call_id == call_id
                && frame.is_none_or(|frame| {
                    key.session.frame_id == frame.frame_id
                        && key.session.generation == frame.generation
                        && key.session.is_main_frame == frame.is_main_frame
                })
                && channels
                    .get(*key)
                    .is_some_and(|channel| constant_time_string_eq(&channel.capability, capability))
        });
        let Some(key) = match_key.cloned() else {
            anyhow::bail!("ERR_NIVA_CHANNEL_DENIED: unknown channel capability");
        };
        drop(channels);

        let active = self
            .ipc_sessions
            .lock()
            .map_err(|_| anyhow!("IPC session table poisoned"))?
            .touch_call(&key.session, call_id, Instant::now());
        anyhow::ensure!(active, "ERR_NIVA_SESSION_EXPIRED: inactive IPC channel");
        Ok(key)
    }

    fn ipc_channel_send(
        &self,
        window_id: u8,
        frame: IpcFrameSource,
        rid: u64,
        call_id: u64,
        session_id: &str,
        token: Option<&str>,
        capability: &str,
        encoded_frame: &str,
    ) -> Result<String> {
        let Some(token) = token else {
            return Ok(channel_error(
                rid,
                call_id,
                "ERR_NIVA_CHANNEL_DENIED",
                "trusted local page token is required",
                false,
            ));
        };
        let key = match self.authorize_ipc_channel_route(
            window_id,
            &frame.source_url,
            token,
            session_id,
            call_id,
            capability,
            Some(&frame),
        ) {
            Ok(key) => key,
            Err(error) => {
                return Ok(channel_error(
                    rid,
                    call_id,
                    "ERR_NIVA_CHANNEL_DENIED",
                    &error.to_string(),
                    false,
                ));
            }
        };
        use base64::Engine;
        let raw_frame = match base64::engine::general_purpose::STANDARD.decode(encoded_frame) {
            Ok(frame) => frame,
            Err(_) => {
                return Ok(channel_error(
                    rid,
                    call_id,
                    "ERR_NIVA_CHANNEL_FRAME",
                    "channelSend frame is not valid Base64",
                    false,
                ));
            }
        };
        let mut channels = self
            .ipc_channels
            .lock()
            .map_err(|_| anyhow!("IPC channel table poisoned"))?;
        let channel = channels
            .get_mut(&key)
            .ok_or_else(|| anyhow!("ERR_NIVA_CHANNEL_CLOSED: channel is no longer active"))?;
        if !matches!(channel.transport.as_ref(), Some(IpcChannelTransport::Ipc)) {
            return Ok(channel_error(
                rid,
                call_id,
                "ERR_NIVA_CHANNEL_TRANSPORT",
                "channel is not attached to IPC",
                false,
            ));
        }
        let (header, payload) =
            match validate_open_channel_upload_frame(channel, &raw_frame, call_id) {
                Ok(frame) => frame,
                Err(error) => {
                    return Ok(channel_error(
                        rid,
                        call_id,
                        if error.to_string().contains("BACKPRESSURE") {
                            "ERR_NIVA_CHANNEL_BACKPRESSURE"
                        } else if error.to_string().contains("SEQUENCE") {
                            "ERR_NIVA_CHANNEL_SEQUENCE"
                        } else {
                            "ERR_NIVA_CHANNEL_FRAME"
                        },
                        &error.to_string(),
                        error.to_string().contains("BACKPRESSURE"),
                    ));
                }
            };
        let active_key = (window_id, channel.connection_id, call_id);
        let bytes = match enqueue_ipc_channel_chunk(
            &self.active,
            active_key,
            InboundChunk {
                seq: header.seq,
                data: payload.to_vec(),
                end: header.end,
            },
        ) {
            Ok(bytes) => bytes,
            Err(error) => {
                let retryable = error.to_string().contains("BACKPRESSURE");
                return Ok(channel_error(
                    rid,
                    call_id,
                    if retryable {
                        "ERR_NIVA_CHANNEL_BACKPRESSURE"
                    } else {
                        "ERR_NIVA_CHANNEL_CLOSED"
                    },
                    &error.to_string(),
                    retryable,
                ));
            }
        };
        channel.next_upload_seq = channel.next_upload_seq.saturating_add(1);
        channel.upload_ended = header.end;
        Ok(serde_json::to_string(&json!({
            "t":"channelAck",
            "rid":rid,
            "id":call_id,
            "accepted":true,
            "seq":header.seq,
            "bytes":bytes,
        }))?)
    }

    fn ipc_channel_ack(
        &self,
        window_id: u8,
        frame: IpcFrameSource,
        rid: u64,
        call_id: u64,
        session_id: &str,
        token: Option<&str>,
        capability: &str,
        seq: u64,
    ) -> Result<String> {
        let Some(token) = token else {
            return Ok(channel_error(
                rid,
                call_id,
                "ERR_NIVA_CHANNEL_DENIED",
                "trusted local page token is required",
                false,
            ));
        };
        let key = match self.authorize_ipc_channel_route(
            window_id,
            &frame.source_url,
            token,
            session_id,
            call_id,
            capability,
            Some(&frame),
        ) {
            Ok(key) => key,
            Err(error) => {
                return Ok(channel_error(
                    rid,
                    call_id,
                    "ERR_NIVA_CHANNEL_DENIED",
                    &error.to_string(),
                    false,
                ));
            }
        };
        let mut channels = self
            .ipc_channels
            .lock()
            .map_err(|_| anyhow!("IPC channel table poisoned"))?;
        let Some(channel) = channels.get_mut(&key) else {
            return Ok(channel_error(
                rid,
                call_id,
                "ERR_NIVA_CHANNEL_CLOSED",
                "channel is no longer active",
                false,
            ));
        };
        if !matches!(channel.transport.as_ref(), Some(IpcChannelTransport::Ipc)) {
            return Ok(channel_error(
                rid,
                call_id,
                "ERR_NIVA_CHANNEL_TRANSPORT",
                "channel is not attached to IPC",
                false,
            ));
        }
        if let Err(error) = acknowledge_ipc_channel_output(channel, seq) {
            let error_text = error.to_string();
            return Ok(channel_error(
                rid,
                call_id,
                if error_text.contains("SEQUENCE") {
                    "ERR_NIVA_CHANNEL_SEQUENCE"
                } else {
                    "ERR_NIVA_CHANNEL_ACK"
                },
                &error_text,
                false,
            ));
        }
        Ok(serde_json::to_string(&json!({
            "t":"channelAckReceived","rid":rid,"id":call_id,"accepted":true,"seq":seq
        }))?)
    }

    fn ipc_channel_cancel(
        &self,
        window_id: u8,
        frame: IpcFrameSource,
        rid: u64,
        call_id: u64,
        session_id: &str,
        token: Option<&str>,
        capability: &str,
    ) -> Result<String> {
        let Some(token) = token else {
            return Ok(channel_error(
                rid,
                call_id,
                "ERR_NIVA_CHANNEL_DENIED",
                "trusted local page token is required",
                false,
            ));
        };
        let key = match self.authorize_ipc_channel_route(
            window_id,
            &frame.source_url,
            token,
            session_id,
            call_id,
            capability,
            Some(&frame),
        ) {
            Ok(key) => key,
            Err(error) => {
                return Ok(channel_error(
                    rid,
                    call_id,
                    "ERR_NIVA_CHANNEL_DENIED",
                    &error.to_string(),
                    false,
                ));
            }
        };
        let is_active = self
            .ipc_channels
            .lock()
            .map_err(|_| anyhow!("IPC channel table poisoned"))?
            .contains_key(&key);
        if !is_active {
            return Ok(channel_error(
                rid,
                call_id,
                "ERR_NIVA_CHANNEL_CLOSED",
                "channel is no longer active",
                false,
            ));
        };
        let sender = self
            .ipc_sessions
            .lock()
            .ok()
            .and_then(|mut sessions| sessions.cancel_call(&key.session, call_id));
        if !self.cancel_ipc_active_call(&key.session, call_id)
            && let Some(sender) = sender
        {
            sender.close();
        }
        self.remove_ipc_channel(&key);
        Ok(serde_json::to_string(&json!({
            "t":"channelCancelled","rid":rid,"id":call_id,"accepted":true
        }))?)
    }

    fn cancel_ipc_channel_sessions(&self, mut matches: impl FnMut(&IpcSessionKey) -> bool) {
        let selected = self
            .ipc_channels
            .lock()
            .map(|channels| {
                channels
                    .iter()
                    .filter(|(key, _)| matches(&key.session))
                    .map(|(key, _)| key.clone())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        for key in selected {
            let sender = self
                .ipc_sessions
                .lock()
                .ok()
                .and_then(|mut sessions| sessions.cancel_call(&key.session, key.call_id));
            if !self.cancel_ipc_active_call(&key.session, key.call_id)
                && let Some(sender) = sender
            {
                sender.close();
            }
            self.remove_ipc_channel(&key);
        }
    }

    /// Cancel active IPC work from one native WebView2 frame generation.
    pub fn cancel_ipc_frame(&self, window_id: u8, frame_id: u64, generation: u64) {
        crate::app::api::cancel_ipc_process_frame(window_id, frame_id, generation);
        let affected_sessions = self
            .ipc_sessions
            .lock()
            .map(|sessions| {
                sessions
                    .active
                    .keys()
                    .filter(|key| {
                        key.window_id == window_id
                            && key.frame_id == frame_id
                            && key.generation == generation
                    })
                    .map(|key| key.session_id.clone())
                    .collect::<HashSet<_>>()
            })
            .unwrap_or_default();
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
        // Channel cancellation invokes the full-key process.stdin hook while
        // each stream sender is still open. Only then close remaining unary
        // senders retired with the frame.
        self.cancel_ipc_channel_sessions(|key| {
            key.window_id == window_id && key.frame_id == frame_id && key.generation == generation
        });
        for sender in senders {
            sender.close();
        }
        for session_id in &affected_sessions {
            self.cancel_sync_session(window_id, session_id);
        }
        let ws_owners = self
            .ws_sessions
            .lock()
            .map(|mut sessions| {
                let matches = sessions
                    .iter()
                    .filter(|((owner, _), session)| {
                        *owner == window_id && affected_sessions.contains(*session)
                    })
                    .map(|(key, _)| *key)
                    .collect::<Vec<_>>();
                for key in &matches {
                    sessions.remove(key);
                }
                matches
                    .into_iter()
                    .map(|(_, connection_id)| connection_id)
                    .collect::<HashSet<_>>()
            })
            .unwrap_or_default();
        for connection_id in ws_owners {
            self.cancel_owner_connection(window_id, connection_id);
        }
    }

    /// Cancel all IPC sessions when the top-level document navigates. The
    /// replacement document receives a new opaque session id.
    pub fn cancel_ipc_window_for_navigation(&self, window_id: u8) {
        crate::app::api::cancel_ipc_process_window(window_id);
        let senders = self
            .ipc_sessions
            .lock()
            .map(|mut sessions| sessions.cancel_matching(|key| key.window_id == window_id, true))
            .unwrap_or_default();
        self.cancel_ipc_channel_sessions(|key| key.window_id == window_id);
        for sender in senders {
            sender.close();
        }
        let ws_owners = self
            .ws_sessions
            .lock()
            .map(|mut sessions| {
                let connection_ids = sessions
                    .iter()
                    .filter(|((owner, _), _)| *owner == window_id)
                    .map(|((_, connection_id), _)| *connection_id)
                    .collect::<HashSet<_>>();
                sessions.retain(|(owner, _), _| *owner != window_id);
                connection_ids
            })
            .unwrap_or_default();
        for connection_id in ws_owners {
            self.cancel_owner_connection(window_id, connection_id);
        }
        self.cancel_sync_window(window_id);
    }

    fn cancel_ipc_window(&self, window_id: u8) {
        crate::app::api::cancel_ipc_process_window(window_id);
        let senders = match self.ipc_sessions.lock() {
            Ok(mut sessions) => sessions.close_window(window_id),
            Err(_) => return,
        };
        self.cancel_ipc_channel_sessions(|key| key.window_id == window_id);
        for sender in senders {
            sender.close();
        }
    }

    fn cancel_sync_window(&self, window_id: u8) {
        let session_ids = self
            .sync_active
            .lock()
            .map(|active| {
                active
                    .keys()
                    .filter(|key| key.0 == window_id)
                    .map(|key| key.1.clone())
                    .collect::<HashSet<_>>()
            })
            .unwrap_or_default();
        for session_id in session_ids {
            self.cancel_sync_session(window_id, &session_id);
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
    /// that thread; long-lived stream resources use their session owner ID.
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

    /// Register a bounded unary API that uses the same Native operation across
    /// bridge transports. The stable page-session owner lets work such as child
    /// processes close on session cancellation; remote IPC reachability still
    /// comes only from the exact `IPC_METHOD_ALLOWLIST` below.
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
            ClientMsg::Attach {
                id,
                session_id,
                capability,
            } => {
                if let Err((code, message)) = self.ws_attach_channel(
                    window_id,
                    connection_id,
                    tx,
                    id,
                    &session_id,
                    &capability,
                ) {
                    let _ = tx.send(WsOut::Text(
                        json!({
                            "t":"attachError", "id":id, "sessionId":session_id,
                            "code":code, "message":message,
                        })
                        .to_string(),
                    ));
                }
            }
            ClientMsg::Ack {
                id,
                session_id,
                seq,
            } => {
                if let Err((code, message)) =
                    self.ws_ack_channel(window_id, connection_id, &session_id, id, seq)
                {
                    let _ = tx.send(WsOut::Text(
                        json!({
                            "t":"channelError", "id":id, "sessionId":session_id,
                            "code":code, "message":message, "retryable":false,
                        })
                        .to_string(),
                    ));
                }
            }
            ClientMsg::Cancel { id, session_id } => {
                self.ws_cancel_channel(window_id, connection_id, &session_id, id);
            }
        }
    }

    fn ws_attach_channel(
        &self,
        window_id: u8,
        connection_id: u64,
        tx: &mpsc::Sender<WsOut>,
        call_id: u64,
        session_id: &str,
        capability: &str,
    ) -> std::result::Result<(), (&'static str, String)> {
        if !self.ws_session_matches(window_id, connection_id, session_id) {
            return Err((
                "ERR_NIVA_CHANNEL_DENIED",
                "invalid WebSocket session".into(),
            ));
        }
        let app = self.app.get().cloned().ok_or((
            "ERR_NIVA_CHANNEL_CLOSED",
            "API manager is shutting down".into(),
        ))?;
        let window = app
            .window()
            .and_then(|windows| windows.get_window(window_id))
            .map_err(|error| ("ERR_NIVA_CHANNEL_CLOSED", error.to_string()))?;
        let trusted_origin = window.trusted_ws_origin.as_deref().ok_or((
            "ERR_NIVA_CHANNEL_DENIED",
            "WebSocket streams require a trusted local page".into(),
        ))?;
        let mut channels = self
            .ipc_channels
            .lock()
            .map_err(|error| ("ERR_NIVA_CHANNEL_CLOSED", error.to_string()))?;
        let key = find_ws_channel_key(
            &channels,
            window_id,
            trusted_origin,
            session_id,
            call_id,
            capability,
        )
        .ok_or((
            "ERR_NIVA_CHANNEL_DENIED",
            "invalid or expired stream ticket".into(),
        ))?;
        let channel = channels
            .get_mut(&key)
            .ok_or(("ERR_NIVA_CHANNEL_CLOSED", "stream ticket expired".into()))?;
        let transport = IpcChannelTransport::WebSocket {
            connection_id,
            sender: tx.clone(),
        };
        if let Some(existing) = &channel.transport {
            if !existing.same_route(&transport) {
                return Err((
                    "ERR_NIVA_CHANNEL_ATTACHED",
                    "stream ticket is already pinned to another transport".into(),
                ));
            }
            let response = json!({"t":"attached","id":call_id,"sessionId":session_id});
            tx.send(WsOut::Text(response.to_string()))
                .map_err(|error| ("ERR_NIVA_CHANNEL_CLOSED", error.to_string()))?;
            return Ok(());
        }

        let response = json!({"t":"attached","id":call_id,"sessionId":session_id});
        match attach_ws_transport_then_confirm(channel, transport, || {
            tx.send(WsOut::Text(response.to_string()))
                .map_err(|error| error.to_string())
        }) {
            Ok(()) => {}
            Err(WsAttachDeliveryError::PumpUnavailable) => {
                return Err((
                    "ERR_NIVA_CHANNEL_CLOSED",
                    "stream channel pump is unavailable".into(),
                ));
            }
            Err(WsAttachDeliveryError::ConfirmationUnavailable(error)) => {
                // The pump has been released and the ticket is pinned. If the
                // success reply cannot be queued, consume the ticket and cancel
                // the call after dropping the channel lock; it must not be
                // rebound to IPC on a later attach attempt.
                channels.remove(&key);
                drop(channels);
                self.cancel_ipc_active_call(&key.session, call_id);
                return Err(("ERR_NIVA_CHANNEL_CLOSED", error));
            }
        }
        Ok(())
    }

    fn ws_channel_key(
        &self,
        window_id: u8,
        connection_id: u64,
        session_id: &str,
        call_id: u64,
    ) -> Option<IpcChannelKey> {
        if !self.ws_session_matches(window_id, connection_id, session_id) {
            return None;
        }
        self.ipc_channels.lock().ok().and_then(|channels| {
            channels
                .iter()
                .find(|(key, channel)| {
                    key.session.window_id == window_id
                        && key.session.session_id == session_id
                        && key.call_id == call_id
                        && matches!(
                            channel.transport.as_ref(),
                            Some(IpcChannelTransport::WebSocket {
                                connection_id: owner,
                                ..
                            }) if *owner == connection_id
                        )
                })
                .map(|(key, _)| key.clone())
        })
    }

    fn ws_ack_channel(
        &self,
        window_id: u8,
        connection_id: u64,
        session_id: &str,
        call_id: u64,
        seq: u64,
    ) -> std::result::Result<(), (&'static str, String)> {
        let key = self
            .ws_channel_key(window_id, connection_id, session_id, call_id)
            .ok_or((
                "ERR_NIVA_CHANNEL_DENIED",
                "stream is not attached to this socket".into(),
            ))?;
        let mut channels = self
            .ipc_channels
            .lock()
            .map_err(|error| ("ERR_NIVA_CHANNEL_CLOSED", error.to_string()))?;
        let channel = channels
            .get_mut(&key)
            .ok_or(("ERR_NIVA_CHANNEL_CLOSED", "stream is closed".into()))?;
        acknowledge_ipc_channel_output(channel, seq)
            .map_err(|error| ("ERR_NIVA_CHANNEL_ACK", error.to_string()))
    }

    fn ws_cancel_channel(&self, window_id: u8, connection_id: u64, session_id: &str, call_id: u64) {
        let Some(key) = self.ws_channel_key(window_id, connection_id, session_id, call_id) else {
            return;
        };
        let sender = self
            .ipc_sessions
            .lock()
            .ok()
            .and_then(|mut sessions| sessions.cancel_call(&key.session, call_id));
        if !self.cancel_ipc_active_call(&key.session, call_id)
            && let Some(sender) = sender
        {
            sender.close();
        }
        self.remove_ipc_channel(&key);
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
        let Some(session_id) = self
            .ws_sessions
            .lock()
            .ok()
            .and_then(|sessions| sessions.get(&(window_id, connection_id)).cloned())
        else {
            return;
        };
        let (header, _payload) = match decode_chunk(frame) {
            Ok(pair) => pair,
            Err(err) => {
                crate::niva_log!(
                    crate::app::logging::Level::Warn,
                    "[niva] api bad chunk: {err}"
                );
                return;
            }
        };
        let Some(key) = self.ws_channel_key(window_id, connection_id, &session_id, header.id)
        else {
            crate::niva_log!(
                crate::app::logging::Level::Warn,
                "[niva] dropped WebSocket data for an unattached channel"
            );
            return;
        };
        let (owner_id, upload_header, upload_payload) = {
            let mut channels = match self.ipc_channels.lock() {
                Ok(channels) => channels,
                Err(_) => return,
            };
            let Some(channel) = channels.get_mut(&key) else {
                return;
            };
            let (upload_header, upload_payload) =
                match validate_open_channel_upload_frame(channel, frame, header.id) {
                    Ok(upload) => upload,
                    Err(error) => {
                        crate::niva_log!(
                            crate::app::logging::Level::Warn,
                            "[niva] rejected WebSocket channel upload: {error}"
                        );
                        return;
                    }
                };
            channel.next_upload_seq = channel.next_upload_seq.saturating_add(1);
            channel.upload_ended = upload_header.end;
            (
                channel.connection_id,
                upload_header,
                upload_payload.to_vec(),
            )
        };
        route_inbound_chunk(
            &self.active,
            window_id,
            owner_id,
            upload_header,
            &upload_payload,
        );
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

    /// Cancel an IPC-owned call only when its complete native session key
    /// matches. IPC owner ids intentionally remain stable per session id, so
    /// they cannot authorize cross-origin cancellation on their own.
    fn cancel_ipc_active_call(&self, session: &IpcSessionKey, call_id: u64) -> bool {
        cancel_ipc_active_call(&self.active, session, call_id)
    }

    fn cancel_owner_connection(&self, window_id: u8, owner_id: u64) {
        crate::app::api::cancel_process_connection(window_id, owner_id);
        let calls = {
            let mut active = match lock!(self.active) {
                Ok(active) => active,
                Err(_) => return,
            };
            take_active_calls(&mut active, |(wid, owner, _)| {
                *wid == window_id && *owner == owner_id
            })
        };
        for call in calls {
            cancel_active_call(call);
        }
    }

    /// Disconnecting one frame must not cancel calls from sibling frames.
    pub fn cancel_connection(&self, window_id: u8, connection_id: u64) {
        if let Ok(mut sessions) = self.ws_sessions.lock() {
            sessions.remove(&(window_id, connection_id));
        }
        let attached = self
            .ipc_channels
            .lock()
            .map(|channels| {
                channels
                    .iter()
                    .filter_map(|(key, channel)| {
                        matches!(
                            channel.transport.as_ref(),
                            Some(IpcChannelTransport::WebSocket {
                                connection_id: attached_connection,
                                ..
                            }) if *attached_connection == connection_id
                        )
                        .then_some((key.clone(), channel.connection_id))
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        for (key, _owner_id) in attached {
            let sender = self
                .ipc_sessions
                .lock()
                .ok()
                .and_then(|mut sessions| sessions.cancel_call(&key.session, key.call_id));
            if !self.cancel_ipc_active_call(&key.session, key.call_id)
                && let Some(sender) = sender
            {
                sender.close();
            }
            self.remove_ipc_channel(&key);
        }
        self.cancel_owner_connection(window_id, connection_id);
    }

    /// Abort all calls of a window (window close or page lifecycle cleanup).
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

    fn dispatch_with_output(
        &self,
        window_id: u8,
        connection_id: u64,
        outbound: CallOutput,
        request: ApiRequest,
        channels: DispatchChannels,
        ipc_session: Option<IpcSessionKey>,
    ) -> bool {
        let DispatchChannels {
            cancel_tx,
            cancel_rx,
            inbound_tx,
            inbound_rx,
            http_response_ack_rx,
        } = channels;
        let lifecycle = Arc::new(CallLifecycle::new());
        let key = (window_id, connection_id, request.0);
        let reservation = match lock!(self.active) {
            Ok(mut active) => reserve_active_call(
                &mut active,
                key,
                ActiveCall {
                    cancel_tx: cancel_tx.clone(),
                    inbound_tx,
                    outbound: outbound.clone(),
                    lifecycle: lifecycle.clone(),
                    ipc_session,
                },
            ),
            Err(err) => {
                crate::niva_log!(
                    crate::app::logging::Level::Warn,
                    "[niva] api active table poisoned: {err}"
                );
                respond_output(&outbound, request.err(-1, "active call table unavailable"));
                return false;
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
            match &outbound {
                CallOutput::WebSocket(tx) => notify_duplicate_active_id(tx, event_seq, request.0),
                CallOutput::Channel(tx) => {
                    let msg = ServerMsg::protocol_error(
                        event_seq,
                        request.0,
                        "ERR_DUPLICATE_ACTIVE_REQUEST_ID",
                        "request id is already active on this channel",
                    );
                    let _ = tx.send(WsOut::Text(msg.encode()));
                }
            }
            return false;
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
                    &outbound,
                    request.err(-1, error.to_string()),
                );
                return false;
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
                    &outbound,
                    request.err(-1, error.to_string()),
                );
                return false;
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
                    &outbound,
                    request.err(-1, "api not found"),
                );
                return false;
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
                outbound: outbound.clone(),
                seq: AtomicU64::new(1),
                cancel_rx,
                inbound_rx,
                http_response_ack_rx,
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
                    respond_output(&job.ctx.inner.outbound, job.request.err(-3, "server busy"));
                }
            },
        )
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

fn respond_output(output: &CallOutput, response: ApiResponse) {
    let msg = ServerMsg::result(response.0, response.1, response.2, response.3);
    if !output.send(WsOut::Text(msg.encode())) {
        crate::niva_log!(
            crate::app::logging::Level::Warn,
            "[niva] API response dropped because its transport is closed"
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
            outbound: CallOutput::WebSocket(ws_tx),
            cancel_tx,
            inbound_tx,
            ipc_session: None,
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

    #[test]
    fn ipc_normalizes_uuid_bound_custom_protocol_origins() {
        let scheme = "niva-a51c1728d17442d48f577d296c966b51";
        let origin = super::super::custom_protocol::origin_for_scheme(scheme);
        assert_eq!(
            normalize_ipc_source_origin(
                &url::Url::parse(&format!("{scheme}://app/index.html")).unwrap()
            )
            .unwrap(),
            origin
        );
        let other_scheme = "niva-b61c1728d17442d48f577d296c966b51";
        let other_origin = normalize_ipc_source_origin(
            &url::Url::parse(&format!("{other_scheme}://app/index.html")).unwrap(),
        )
        .unwrap();
        assert_ne!(other_origin, origin);

        #[cfg(any(target_os = "windows", target_os = "android"))]
        assert_eq!(
            normalize_ipc_source_origin(
                &url::Url::parse(&format!("http://{scheme}.app/index.html")).unwrap()
            )
            .unwrap(),
            origin
        );

        for source in [
            "niva://app/index.html",
            "niva-a51c1728d17442d48f577d296c966b51://other/index.html",
            "niva-a51c1728d17442d48f577d296c966b51://user@app/index.html",
            "niva-a51c1728d17442d48f577d296c966b51://app:9000/index.html",
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
    fn ipc_messages_accept_api_calls_and_channel_controls() {
        let close: IpcMessage = serde_json::from_str(
            r#"{"t":"session_close","sessionId":"0123456789abcdef0123456789abcdef","token":"window-token"}"#,
        )
        .unwrap();
        assert!(matches!(
            close,
            IpcMessage::SessionClose { session_id, .. }
                if session_id == "0123456789abcdef0123456789abcdef"
        ));
        assert!(
            serde_json::from_str::<IpcMessage>(
                r#"{"t":"sessionClose","sessionId":"0123456789abcdef0123456789abcdef"}"#
            )
            .is_err()
        );
        assert!(
            serde_json::from_str::<IpcMessage>(
                r#"{"t":"session_close","rid":1,"sessionId":"0123456789abcdef0123456789abcdef"}"#
            )
            .is_err()
        );

        let call: IpcMessage = serde_json::from_str(
            r#"{"t":"api_call","rid":70,"id":7,"method":"http.requestText","args":[{}],"sessionId":"0123456789abcdef0123456789abcdef","token":"window-token"}"#,
        )
        .unwrap();
        assert!(
            matches!(call, IpcMessage::ApiCall { rid: 70, id: 7, session_id, .. } if session_id == "0123456789abcdef0123456789abcdef")
        );
        assert!(serde_json::from_str::<IpcMessage>(
            r#"{"t":"call","rid":70,"id":7,"method":"http.requestText","args":[],"sessionId":"0123456789abcdef0123456789abcdef"}"#
        )
        .is_err());

        let heartbeat: IpcMessage = serde_json::from_str(
            r#"{"t":"heartbeat","rid":71,"sessionId":"0123456789abcdef0123456789abcdef","token":"window-token"}"#,
        )
        .unwrap();
        assert!(
            matches!(heartbeat, IpcMessage::Heartbeat { session_id, .. } if session_id == "0123456789abcdef0123456789abcdef")
        );

        let send: IpcMessage = serde_json::from_str(
            r#"{"t":"channelSend","rid":72,"id":7,"sessionId":"0123456789abcdef0123456789abcdef","capability":"0123456789abcdef","frame":"AQEAAAAAAAAABwAAAAAAAAAB"}"#,
        )
        .unwrap();
        assert!(
            matches!(send, IpcMessage::ChannelSend { rid: 72, id: 7, capability, .. } if capability == "0123456789abcdef")
        );

        let ack: IpcMessage = serde_json::from_str(
            r#"{"t":"channelAck","rid":74,"id":7,"sessionId":"0123456789abcdef0123456789abcdef","capability":"0123456789abcdef","seq":3}"#,
        )
        .unwrap();
        assert!(matches!(
            ack,
            IpcMessage::ChannelAck {
                rid: 74,
                id: 7,
                seq: 3,
                ..
            }
        ));

        let cancel: IpcMessage = serde_json::from_str(
            r#"{"t":"channelCancel","rid":73,"id":7,"sessionId":"0123456789abcdef0123456789abcdef","capability":"0123456789abcdef"}"#,
        )
        .unwrap();
        assert!(matches!(
            cancel,
            IpcMessage::ChannelCancel { rid: 73, id: 7, .. }
        ));

        let attach: IpcMessage = serde_json::from_str(
            r#"{"t":"channelAttach","rid":75,"id":7,"sessionId":"0123456789abcdef0123456789abcdef","capability":"0123456789abcdef"}"#,
        )
        .unwrap();
        assert!(matches!(
            attach,
            IpcMessage::ChannelAttach { rid: 75, id: 7, .. }
        ));
    }

    #[test]
    fn ipc_source_must_match_native_top_origin_including_custom_scheme_normalization() {
        let local = IpcFrameSource {
            source_url: "niva-a51c1728d17442d48f577d296c966b51://app/child".into(),
            top_url: "niva-a51c1728d17442d48f577d296c966b51://app/index.html".into(),
            frame_id: 1,
            generation: 0,
            is_main_frame: false,
        };
        assert_eq!(
            same_origin_ipc_source(&local).unwrap(),
            "niva-a51c1728d17442d48f577d296c966b51://app"
        );
        let cross_origin = IpcFrameSource {
            source_url: "https://remote.example/frame".into(),
            top_url: "https://local.example/index".into(),
            frame_id: 2,
            generation: 0,
            is_main_frame: false,
        };
        assert!(same_origin_ipc_source(&cross_origin).is_err());
    }

    #[test]
    fn ws_ticket_requires_matching_window_session_origin_and_capability() {
        let session_id = "0123456789abcdef0123456789abcdef";
        let key = IpcChannelKey {
            session: IpcSessionKey {
                window_id: 7,
                source_origin: "niva-a51c1728d17442d48f577d296c966b51://app".into(),
                session_id: session_id.into(),
                frame_id: 0,
                generation: 0,
                is_main_frame: true,
            },
            call_id: 42,
        };
        let (attach_tx, _attach_rx) = async_channel::bounded(1);
        let (ack_tx, _ack_rx) = async_channel::bounded(1);
        let channel = IpcChannelCall {
            method: "http.requestStream".into(),
            capability: "0123456789abcdef0123456789abcdef".into(),
            connection_id: session_owner_id(7, session_id),
            transport: None,
            attach_tx,
            next_upload_seq: 1,
            upload_ended: false,
            next_http_response_ack_seq: 1,
            http_response_ack_tx: async_channel::bounded::<u64>(1).0,
            awaiting_output_seq: None,
            output_ack_tx: ack_tx,
        };
        let mut channels = HashMap::new();
        channels.insert(key.clone(), channel);
        assert_eq!(
            find_ws_channel_key(
                &channels,
                7,
                &key.session.source_origin,
                session_id,
                42,
                "0123456789abcdef0123456789abcdef",
            ),
            Some(key.clone())
        );
        assert!(
            find_ws_channel_key(
                &channels,
                7,
                &key.session.source_origin,
                "abcdef0123456789abcdef0123456789",
                42,
                "0123456789abcdef0123456789abcdef",
            )
            .is_none()
        );
        assert!(
            find_ws_channel_key(
                &channels,
                8,
                &key.session.source_origin,
                session_id,
                42,
                "0123456789abcdef0123456789abcdef",
            )
            .is_none()
        );
        assert!(
            find_ws_channel_key(
                &channels,
                7,
                "https://remote.example",
                session_id,
                42,
                "0123456789abcdef0123456789abcdef",
            )
            .is_none()
        );
        assert!(
            find_ws_channel_key(
                &channels,
                7,
                &key.session.source_origin,
                session_id,
                42,
                "wrong-capability",
            )
            .is_none()
        );
    }

    #[test]
    fn channel_ticket_transport_cannot_be_rebound() {
        let options: NivaOptions = serde_json::from_value(json!({
            "name": "ticket-pin-test",
            "uuid": "a51c1728-d174-42d4-8f57-7d296c966b51"
        }))
        .unwrap();
        let manager = ApiManager::new(&options);
        let session_id = "0123456789abcdef0123456789abcdef";
        let key = IpcChannelKey {
            session: IpcSessionKey {
                window_id: 7,
                source_origin: "niva-a51c1728d17442d48f577d296c966b51://app".into(),
                session_id: session_id.into(),
                frame_id: 0,
                generation: 0,
                is_main_frame: true,
            },
            call_id: 42,
        };
        let (attach_tx, attach_rx) = async_channel::bounded(1);
        let (ack_tx, _ack_rx) = async_channel::bounded(1);
        manager.ipc_channels.lock().unwrap().insert(
            key.clone(),
            IpcChannelCall {
                method: "http.requestStream".into(),
                capability: "capability".into(),
                connection_id: session_owner_id(7, session_id),
                transport: None,
                attach_tx,
                next_upload_seq: 1,
                upload_ended: false,
                next_http_response_ack_seq: 1,
                http_response_ack_tx: async_channel::bounded::<u64>(1).0,
                awaiting_output_seq: None,
                output_ack_tx: ack_tx,
            },
        );
        assert!(
            attach_rx.try_recv().is_err(),
            "ticket is pending before attach"
        );
        assert!(
            manager.ipc_channels.lock().unwrap()[&key]
                .transport
                .is_none()
        );
        let ws_sender = mpsc::channel().0;
        manager
            .bind_channel_transport(
                &key,
                IpcChannelTransport::WebSocket {
                    connection_id: 99,
                    sender: ws_sender.clone(),
                },
            )
            .unwrap();
        assert!(matches!(
            attach_rx.try_recv(),
            Ok(IpcChannelTransport::WebSocket {
                connection_id: 99,
                ..
            })
        ));
        assert_eq!(
            manager.ipc_channels.lock().unwrap()[&key].connection_id,
            session_owner_id(7, session_id),
            "data transport must not replace the IPC resource owner"
        );
        assert!(
            manager
                .bind_channel_transport(
                    &key,
                    IpcChannelTransport::WebSocket {
                        connection_id: 100,
                        sender: ws_sender
                    },
                )
                .is_err()
        );
        manager
            .bind_channel_transport(
                &key,
                IpcChannelTransport::WebSocket {
                    connection_id: 99,
                    sender: mpsc::channel().0,
                },
            )
            .unwrap();
    }

    #[test]
    fn websocket_attach_queues_pump_before_success_and_never_confirms_unavailable_pump() {
        let (attach_tx, attach_rx) = async_channel::bounded(1);
        let (ack_tx, _ack_rx) = async_channel::bounded(1);
        let mut channel = IpcChannelCall {
            method: "http.requestStream".into(),
            capability: "capability".into(),
            connection_id: 7,
            transport: None,
            attach_tx,
            next_upload_seq: 1,
            upload_ended: false,
            next_http_response_ack_seq: 1,
            http_response_ack_tx: async_channel::bounded::<u64>(1).0,
            awaiting_output_seq: None,
            output_ack_tx: ack_tx,
        };
        let transport = IpcChannelTransport::WebSocket {
            connection_id: 99,
            sender: mpsc::channel().0,
        };
        let mut pump_was_queued_before_confirmation = false;

        attach_ws_transport_then_confirm(&mut channel, transport, || {
            pump_was_queued_before_confirmation = matches!(
                attach_rx.try_recv(),
                Ok(IpcChannelTransport::WebSocket {
                    connection_id: 99,
                    ..
                })
            );
            Ok(())
        })
        .unwrap();

        assert!(pump_was_queued_before_confirmation);
        assert!(matches!(
            channel.transport,
            Some(IpcChannelTransport::WebSocket {
                connection_id: 99,
                ..
            })
        ));

        let (attach_tx, attach_rx) = async_channel::bounded(1);
        drop(attach_rx);
        let (ack_tx, _ack_rx) = async_channel::bounded(1);
        let mut unavailable_channel = IpcChannelCall {
            method: "http.requestStream".into(),
            capability: "capability".into(),
            connection_id: 7,
            transport: None,
            attach_tx,
            next_upload_seq: 1,
            upload_ended: false,
            next_http_response_ack_seq: 1,
            http_response_ack_tx: async_channel::bounded::<u64>(1).0,
            awaiting_output_seq: None,
            output_ack_tx: ack_tx,
        };
        let mut confirmation_called = false;
        assert_eq!(
            attach_ws_transport_then_confirm(
                &mut unavailable_channel,
                IpcChannelTransport::WebSocket {
                    connection_id: 100,
                    sender: mpsc::channel().0,
                },
                || {
                    confirmation_called = true;
                    Ok(())
                },
            ),
            Err(WsAttachDeliveryError::PumpUnavailable)
        );
        assert!(!confirmation_called);
        assert!(unavailable_channel.transport.is_none());
    }

    #[test]
    fn websocket_attach_confirmation_failure_keeps_ticket_pinned_until_cleanup() {
        let (attach_tx, attach_rx) = async_channel::bounded(1);
        let (ack_tx, _ack_rx) = async_channel::bounded(1);
        let mut channel = IpcChannelCall {
            method: "http.requestStream".into(),
            capability: "capability".into(),
            connection_id: 7,
            transport: None,
            attach_tx,
            next_upload_seq: 1,
            upload_ended: false,
            next_http_response_ack_seq: 1,
            http_response_ack_tx: async_channel::bounded::<u64>(1).0,
            awaiting_output_seq: None,
            output_ack_tx: ack_tx,
        };
        let transport = IpcChannelTransport::WebSocket {
            connection_id: 99,
            sender: mpsc::channel().0,
        };

        assert_eq!(
            attach_ws_transport_then_confirm(&mut channel, transport, || {
                Err("confirmation queue closed".into())
            }),
            Err(WsAttachDeliveryError::ConfirmationUnavailable(
                "confirmation queue closed".into()
            ))
        );
        assert!(matches!(
            attach_rx.try_recv(),
            Ok(IpcChannelTransport::WebSocket {
                connection_id: 99,
                ..
            })
        ));
        assert!(matches!(
            channel.transport,
            Some(IpcChannelTransport::WebSocket {
                connection_id: 99,
                ..
            })
        ));
    }

    #[test]
    fn websocket_pump_uses_one_sequence_for_binary_data_and_ack() {
        let (sender, receiver) = mpsc::channel();
        let original = encode_chunk(42, 7, false, false, true, b"stderr");
        assert!(send_ws_channel_frame(
            &sender,
            "0123456789abcdef0123456789abcdef",
            42,
            3,
            WsOut::Binary(original),
        ));
        let WsOut::Binary(frame) = receiver.recv().unwrap() else {
            panic!("binary frame expected");
        };
        let (header, payload) = decode_chunk(&frame).unwrap();
        assert_eq!(header.id, 42);
        assert_eq!(header.seq, 3);
        assert!(header.stderr);
        assert_eq!(payload, b"stderr");

        let (sender, receiver) = mpsc::channel();
        assert!(send_ws_channel_frame(
            &sender,
            "0123456789abcdef0123456789abcdef",
            42,
            4,
            WsOut::Text("{\"t\":\"result\",\"id\":42}".into()),
        ));
        let WsOut::Text(frame) = receiver.recv().unwrap() else {
            panic!("text frame expected");
        };
        let value: Value = serde_json::from_str(&frame).unwrap();
        assert_eq!(value["t"], "channelData");
        assert_eq!(value["seq"], 4);
        assert_eq!(value["frame"]["data"], "{\"t\":\"result\",\"id\":42}");
    }

    #[test]
    fn websocket_api_method_dispatch_is_rejected() {
        let options: NivaOptions = serde_json::from_value(json!({
            "name": "ws-api-rejected-test",
            "uuid": "a51c1728-d174-42d4-8f57-7d296c966b51"
        }))
        .unwrap();
        let mut manager = ApiManager::new(&options);
        let calls = Arc::new(AtomicUsize::new(0));
        let calls_in_handler = calls.clone();
        manager.register_api("test.api", move |_, _, _| {
            let calls = calls_in_handler.clone();
            async move {
                calls.fetch_add(1, Ordering::SeqCst);
                Ok(Value::Null)
            }
        });
        let manager = manager;
        let session_id = "0123456789abcdef0123456789abcdef";
        assert!(manager.bind_ws_session(3, 4, session_id));
        let (tx, _rx) = mpsc::channel();
        for text in [
            r#"{"t":"call","id":5,"method":"test.api","args":[],"sessionId":"0123456789abcdef0123456789abcdef"}"#,
            r#"{"t":"api_call","id":5,"method":"test.api","args":[],"sessionId":"0123456789abcdef0123456789abcdef"}"#,
        ] {
            manager.on_text(3, 4, &tx, text);
        }
        assert_eq!(calls.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn only_exact_trusted_local_origin_and_token_can_open_streams() {
        let trusted_origin = "niva-a51c1728d17442d48f577d296c966b51://app";
        assert!(trusted_local_ipc_auth_matches(
            Some(trusted_origin),
            "window-token",
            trusted_origin,
            Some("window-token"),
        ));
        assert!(trusted_local_origin_matches(
            Some(trusted_origin),
            trusted_origin
        ));
        assert!(!trusted_local_ipc_auth_matches(
            Some(trusted_origin),
            "window-token",
            trusted_origin,
            None,
        ));
        assert!(!trusted_local_ipc_auth_matches(
            Some(trusted_origin),
            "window-token",
            "https://remote.example",
            Some("window-token"),
        ));
        assert!(!trusted_local_ipc_auth_matches(
            Some(trusted_origin),
            "window-token",
            trusted_origin,
            Some("wrong-token"),
        ));
        assert!(
            authorize_ipc_source_origin(
                Some(trusted_origin),
                "window-token",
                trusted_origin,
                None,
                true,
            )
            .is_err()
        );
        assert!(
            !authorize_ipc_source_origin(
                Some(trusted_origin),
                "window-token",
                "https://remote.example",
                None,
                true,
            )
            .unwrap()
        );
        assert!(
            authorize_ipc_source_origin(
                Some(trusted_origin),
                "window-token",
                "https://remote.example",
                None,
                false,
            )
            .is_err()
        );
        assert!(may_open_channel_stream(true));
        assert!(!may_open_channel_stream(false));
        assert!(ipc_unary_method_allowed(true, "module.resolve"));
        assert!(ipc_unary_method_allowed(false, "http.requestText"));
        assert!(!ipc_unary_method_allowed(false, "module.resolve"));
        assert!(ipc_method_allowed("http.requestText"));
        assert!(!ipc_method_allowed("socket.tcpConnect"));
    }

    #[test]
    fn ipc_session_owner_is_stable_per_window_and_session() {
        let session_id = "0123456789abcdef0123456789abcdef";
        let owner_id = session_owner_id(5, session_id);
        assert_ne!(owner_id, 0);
        assert_eq!(owner_id & (1 << 63), 1 << 63);
        assert_eq!(session_owner_id(5, session_id), owner_id);
        assert_ne!(session_owner_id(6, session_id), owner_id);
        assert_ne!(
            session_owner_id(5, "abcdef0123456789abcdef0123456789"),
            owner_id
        );

        let mut sessions = IpcSessions::default();
        let key = IpcSessionKey {
            window_id: 5,
            source_origin: "niva-a51c1728d17442d48f577d296c966b51://app".into(),
            session_id: session_id.into(),
            frame_id: 0,
            generation: 0,
            is_main_frame: true,
        };
        let (first_tx, _) = async_channel::bounded(1);
        let (second_tx, _) = async_channel::bounded(1);
        assert_eq!(
            sessions
                .reserve(key.clone(), 1, first_tx, owner_id, Instant::now())
                .unwrap(),
            owner_id
        );
        assert_eq!(
            sessions
                .reserve(
                    key,
                    2,
                    second_tx,
                    session_owner_id(5, session_id),
                    Instant::now()
                )
                .unwrap(),
            owner_id
        );
    }

    #[test]
    fn closing_colliding_remote_ipc_key_does_not_cancel_trusted_call_or_channel() {
        let options: NivaOptions = serde_json::from_value(json!({
            "name": "session-close-isolation-test",
            "uuid": "a51c1728-d174-42d4-8f57-7d296c966b51"
        }))
        .unwrap();
        let manager = ApiManager::new(&options);
        let window_id = 5;
        let session_id = "0123456789abcdef0123456789abcdef";
        let call_id = 31;
        let owner_id = session_owner_id(window_id, session_id);
        let trusted = IpcSessionKey {
            window_id,
            source_origin: "niva-a51c1728d17442d48f577d296c966b51://app".into(),
            session_id: session_id.into(),
            frame_id: 0,
            generation: 0,
            is_main_frame: true,
        };
        let remote = IpcSessionKey {
            source_origin: "https://granted.example".into(),
            ..trusted.clone()
        };

        let (trusted_cancel_tx, trusted_cancel_rx) = async_channel::bounded(1);
        let (remote_cancel_tx, remote_cancel_rx) = async_channel::bounded(1);
        {
            let mut sessions = manager.ipc_sessions.lock().unwrap();
            sessions
                .reserve(
                    trusted.clone(),
                    call_id,
                    trusted_cancel_tx,
                    owner_id,
                    Instant::now(),
                )
                .unwrap();
            sessions
                .reserve(
                    remote.clone(),
                    call_id,
                    remote_cancel_tx,
                    owner_id,
                    Instant::now(),
                )
                .unwrap();
        }

        // The active-call key deliberately collides: both origins chose the
        // same session and call ids. Its full IPC key identifies the trusted
        // owner and must be checked before the stable owner id is cancelled.
        let (inbound_tx, _) = async_channel::bounded(1);
        let (outbound_tx, _) = mpsc::sync_channel(1);
        let trusted_lifecycle = Arc::new(CallLifecycle::new());
        manager.active.lock().unwrap().insert(
            (window_id, owner_id, call_id),
            ActiveCall {
                cancel_tx: async_channel::bounded(1).0,
                inbound_tx,
                outbound: CallOutput::Channel(outbound_tx),
                lifecycle: trusted_lifecycle.clone(),
                ipc_session: Some(trusted.clone()),
            },
        );

        let channel = || {
            let (attach_tx, _) = async_channel::bounded(1);
            let (http_response_ack_tx, _) = async_channel::bounded(1);
            let (output_ack_tx, _) = async_channel::bounded(1);
            IpcChannelCall {
                method: "process.stdin".into(),
                capability: "test-capability".into(),
                connection_id: owner_id,
                transport: None,
                attach_tx,
                next_upload_seq: 1,
                upload_ended: false,
                next_http_response_ack_seq: 1,
                http_response_ack_tx,
                awaiting_output_seq: None,
                output_ack_tx,
            }
        };
        let trusted_channel = IpcChannelKey {
            session: trusted.clone(),
            call_id,
        };
        let remote_channel = IpcChannelKey {
            session: remote.clone(),
            call_id,
        };
        manager.ipc_channels.lock().unwrap().extend([
            (trusted_channel.clone(), channel()),
            (remote_channel.clone(), channel()),
        ]);

        let retired = manager.ipc_sessions.lock().unwrap().retire(&remote);
        assert_eq!(retired.len(), 1);
        for (retired_call_id, sender) in retired {
            if !manager.cancel_ipc_active_call(&remote, retired_call_id) {
                sender.close();
            }
        }
        let expired_count = manager.ipc_sessions.lock().unwrap().expired_per_window[&window_id];
        assert!(
            manager
                .ipc_sessions
                .lock()
                .unwrap()
                .retire(&remote)
                .is_empty()
        );
        assert_eq!(
            manager.ipc_sessions.lock().unwrap().expired_per_window[&window_id],
            expired_count
        );
        let (late_tx, _) = async_channel::bounded(1);
        assert_eq!(
            manager.ipc_sessions.lock().unwrap().reserve(
                remote.clone(),
                call_id + 1,
                late_tx,
                owner_id,
                Instant::now(),
            ),
            Err(IpcSessionError::Expired)
        );
        manager.cancel_ipc_channel_sessions(|key| key == &remote);

        assert!(remote_cancel_rx.is_closed());
        assert!(!trusted_cancel_rx.is_closed());
        assert!(!trusted_lifecycle.is_terminal());
        assert!(
            manager
                .active
                .lock()
                .unwrap()
                .contains_key(&(window_id, owner_id, call_id))
        );
        let channels = manager.ipc_channels.lock().unwrap();
        assert!(channels.contains_key(&trusted_channel));
        assert!(!channels.contains_key(&remote_channel));

        // A fresh runtime session from the same authorized origin can start
        // after the retired key is tombstoned.
        let fresh = IpcSessionKey {
            session_id: "abcdef0123456789abcdef0123456789".into(),
            ..remote.clone()
        };
        let (fresh_tx, _) = async_channel::bounded(1);
        assert!(
            manager
                .ipc_sessions
                .lock()
                .unwrap()
                .reserve(fresh, call_id, fresh_tx, owner_id, Instant::now())
                .is_ok()
        );
    }

    #[test]
    fn ipc_channel_upload_checks_frame_identity_order_and_payload_limit() {
        let frame = encode_chunk(7, 1, true, false, false, b"raw bytes");
        let (header, payload) = validate_ipc_channel_upload_frame(&frame, 7, 1).unwrap();
        assert_eq!(header.id, 7);
        assert_eq!(payload, b"raw bytes");

        assert!(
            validate_ipc_channel_upload_frame(&frame, 8, 1)
                .unwrap_err()
                .to_string()
                .contains("CHANNEL_FRAME")
        );
        assert!(
            validate_ipc_channel_upload_frame(&frame, 7, 2)
                .unwrap_err()
                .to_string()
                .contains("CHANNEL_SEQUENCE")
        );
        let ended = encode_chunk(7, 1, true, true, false, b"raw bytes");
        assert!(
            validate_ipc_channel_upload_frame(&ended, 7, 1)
                .unwrap()
                .0
                .end
        );
        let stderr = encode_chunk(7, 1, true, false, true, b"raw bytes");
        assert!(validate_ipc_channel_upload_frame(&stderr, 7, 1).is_err());
        let mut unknown_flags = frame.clone();
        unknown_flags[1] |= 0x80;
        assert!(validate_ipc_channel_upload_frame(&unknown_flags, 7, 1).is_err());

        let oversized = encode_chunk(
            7,
            1,
            true,
            false,
            false,
            &vec![0; MAX_CHANNEL_BINARY_PAYLOAD + 1],
        );
        assert!(validate_ipc_channel_upload_frame(&oversized, 7, 1).is_err());
    }

    #[test]
    fn websocket_upload_rejects_frames_after_end() {
        let options: NivaOptions = serde_json::from_value(json!({
            "name": "ws-upload-end-test",
            "uuid": "a51c1728-d174-42d4-8f57-7d296c966b51"
        }))
        .unwrap();
        let manager = ApiManager::new(&options);
        let window_id = 7;
        let connection_id = 44;
        let session_id = "0123456789abcdef0123456789abcdef";
        let call_id = 42;
        let owner_id = session_owner_id(window_id, session_id);
        assert!(manager.bind_ws_session(window_id, connection_id, session_id));

        let key = IpcChannelKey {
            session: IpcSessionKey {
                window_id,
                source_origin: "niva-a51c1728d17442d48f577d296c966b51://app".into(),
                session_id: session_id.into(),
                frame_id: 0,
                generation: 0,
                is_main_frame: true,
            },
            call_id,
        };
        let (cancel_tx, _) = async_channel::bounded(1);
        let (inbound_tx, inbound_rx) = async_channel::bounded(2);
        let (outbound_tx, _) = mpsc::channel();
        manager.active.lock().unwrap().insert(
            (window_id, owner_id, call_id),
            ActiveCall {
                cancel_tx,
                inbound_tx,
                outbound: CallOutput::WebSocket(outbound_tx),
                lifecycle: Arc::new(CallLifecycle::new()),
                ipc_session: Some(key.session.clone()),
            },
        );
        let (ack_tx, _ack_rx) = async_channel::bounded(1);
        let (attach_tx, _attach_rx) = async_channel::bounded(1);
        manager.ipc_channels.lock().unwrap().insert(
            key,
            IpcChannelCall {
                method: "http.requestStream".into(),
                capability: "capability".into(),
                connection_id: owner_id,
                transport: Some(IpcChannelTransport::WebSocket {
                    connection_id,
                    sender: mpsc::channel().0,
                }),
                attach_tx,
                next_upload_seq: 1,
                upload_ended: false,
                next_http_response_ack_seq: 1,
                http_response_ack_tx: async_channel::bounded::<u64>(1).0,
                awaiting_output_seq: None,
                output_ack_tx: ack_tx,
            },
        );

        manager.on_binary(
            window_id,
            connection_id,
            &encode_chunk(call_id, 1, true, true, false, b"final"),
        );
        let final_chunk = inbound_rx.try_recv().unwrap();
        assert_eq!(final_chunk.seq, 1);
        assert_eq!(final_chunk.data, b"final");
        assert!(final_chunk.end);

        manager.on_binary(
            window_id,
            connection_id,
            &encode_chunk(call_id, 2, false, false, false, b"after-end"),
        );
        assert!(inbound_rx.try_recv().is_err());
        let channels = manager.ipc_channels.lock().unwrap();
        let channel = channels.values().next().unwrap();
        assert_eq!(channel.next_upload_seq, 2);
        assert!(channel.upload_ended);
    }

    #[test]
    fn channel_input_queue_returns_retryable_backpressure_without_dropping_sequence() {
        let (out_tx, _out_rx) = mpsc::channel();
        let (cancel_tx, _) = async_channel::bounded(1);
        let (inbound_tx, inbound_rx) = async_channel::bounded(1);
        let lifecycle = Arc::new(CallLifecycle::new());
        let key = (3, 9, 7);
        let active = Mutex::new(HashMap::from([(
            key,
            ActiveCall {
                lifecycle,
                outbound: CallOutput::WebSocket(out_tx),
                cancel_tx,
                inbound_tx: inbound_tx.clone(),
                ipc_session: None,
            },
        )]));

        assert_eq!(
            enqueue_ipc_channel_chunk(
                &active,
                key,
                InboundChunk {
                    seq: 1,
                    data: b"one".to_vec(),
                    end: false,
                },
            )
            .unwrap(),
            3
        );
        let error = enqueue_ipc_channel_chunk(
            &active,
            key,
            InboundChunk {
                seq: 2,
                data: b"two".to_vec(),
                end: true,
            },
        )
        .unwrap_err();
        assert!(error.to_string().contains("ERR_NIVA_CHANNEL_BACKPRESSURE"));
        assert_eq!(inbound_rx.try_recv().unwrap().seq, 1);
        enqueue_ipc_channel_chunk(
            &active,
            key,
            InboundChunk {
                seq: 2,
                data: b"two".to_vec(),
                end: true,
            },
        )
        .unwrap();
        let second = inbound_rx.try_recv().unwrap();
        assert_eq!(second.seq, 2);
        assert!(second.end);
    }

    #[test]
    fn output_ack_requires_the_exact_pending_sequence_and_is_one_shot() {
        let (ack_tx, ack_rx) = async_channel::bounded(1);
        let (attach_tx, _attach_rx) = async_channel::bounded(1);
        let mut channel = IpcChannelCall {
            method: "http.requestStream".into(),
            capability: "capability".into(),
            connection_id: 1,
            transport: None,
            attach_tx,
            next_upload_seq: 1,
            upload_ended: false,
            next_http_response_ack_seq: 1,
            http_response_ack_tx: async_channel::bounded::<u64>(1).0,
            awaiting_output_seq: Some(8),
            output_ack_tx: ack_tx,
        };
        assert!(acknowledge_ipc_channel_output(&mut channel, 7).is_err());
        acknowledge_ipc_channel_output(&mut channel, 8).unwrap();
        assert_eq!(ack_rx.try_recv().unwrap(), 8);
        assert!(acknowledge_ipc_channel_output(&mut channel, 8).is_err());
    }

    #[test]
    fn http_response_credit_is_session_pinned_ordered_and_bounded_after_upload_end() {
        let session_id = "0123456789abcdef0123456789abcdef";
        let session = IpcSessionKey {
            window_id: 7,
            source_origin: "niva-a51c1728d17442d48f577d296c966b51://app".into(),
            session_id: session_id.into(),
            frame_id: 3,
            generation: 4,
            is_main_frame: false,
        };
        let channel_key = IpcChannelKey {
            session: session.clone(),
            call_id: 42,
        };

        let make_channel = |method: &str, upload_ended: bool| {
            let (http_response_ack_tx, http_response_ack_rx) = async_channel::bounded(1);
            let (attach_tx, _attach_rx) = async_channel::bounded(1);
            let (output_ack_tx, _output_ack_rx) = async_channel::bounded(1);
            (
                IpcChannelCall {
                    method: method.into(),
                    capability: "capability".into(),
                    connection_id: session_owner_id(session.window_id, &session.session_id),
                    transport: Some(IpcChannelTransport::Ipc),
                    attach_tx,
                    next_upload_seq: 3,
                    upload_ended,
                    next_http_response_ack_seq: 1,
                    http_response_ack_tx,
                    awaiting_output_seq: None,
                    output_ack_tx,
                },
                http_response_ack_rx,
            )
        };
        let (channel, response_acks) = make_channel("http.requestStream", true);
        let channel_table = Mutex::new(HashMap::from([(channel_key.clone(), channel)]));
        let lifecycle = Arc::new(CallLifecycle::new());
        let (cancel_tx, _cancel_rx) = async_channel::bounded(1);
        let (inbound_tx, _inbound_rx) = async_channel::bounded(1);
        let (outbound_tx, _outbound_rx) = mpsc::sync_channel(1);
        let active_key = (
            session.window_id,
            session_owner_id(session.window_id, &session.session_id),
            channel_key.call_id,
        );
        let active_table = Mutex::new(HashMap::from([(
            active_key,
            ActiveCall {
                cancel_tx,
                inbound_tx,
                outbound: CallOutput::Channel(outbound_tx),
                lifecycle: lifecycle.clone(),
                ipc_session: Some(session.clone()),
            },
        )]));

        let mut wrong_frame = session.clone();
        wrong_frame.frame_id += 1;
        assert!(
            grant_http_response_credit_for_session(
                &channel_table,
                &active_table,
                &wrong_frame,
                channel_key.call_id,
                1,
            )
            .is_err()
        );

        assert!(
            grant_http_response_credit_for_session(
                &channel_table,
                &active_table,
                &session,
                channel_key.call_id,
                2,
            )
            .is_err()
        );
        assert!(matches!(
            response_acks.try_recv(),
            Err(async_channel::TryRecvError::Empty)
        ));

        grant_http_response_credit_for_session(
            &channel_table,
            &active_table,
            &session,
            channel_key.call_id,
            1,
        )
        .unwrap();
        assert!(
            grant_http_response_credit_for_session(
                &channel_table,
                &active_table,
                &session,
                channel_key.call_id,
                2,
            )
            .unwrap_err()
            .to_string()
            .contains("BACKPRESSURE")
        );
        assert_eq!(response_acks.try_recv().unwrap(), 1);
        grant_http_response_credit_for_session(
            &channel_table,
            &active_table,
            &session,
            channel_key.call_id,
            2,
        )
        .unwrap();
        assert_eq!(response_acks.try_recv().unwrap(), 2);
        assert!(
            grant_http_response_credit_for_session(
                &channel_table,
                &active_table,
                &session,
                channel_key.call_id,
                2,
            )
            .is_err()
        );

        lifecycle.cancel();
        assert!(
            grant_http_response_credit_for_session(
                &channel_table,
                &active_table,
                &session,
                channel_key.call_id,
                3,
            )
            .is_err()
        );

        cleanup_ipc_channel_resources(&channel_table, &channel_key);
        assert!(matches!(
            response_acks.try_recv(),
            Err(async_channel::TryRecvError::Closed)
        ));

        let (mut wrong_method, _) = make_channel("fs.handle", true);
        assert!(grant_http_response_credit(&mut wrong_method, 1).is_err());
        let (mut before_end, _) = make_channel("http.requestStream", false);
        assert!(grant_http_response_credit(&mut before_end, 1).is_err());
        let (detached, _) = make_channel("http.requestStream", true);
        let mut detached = detached;
        detached.transport = None;
        assert!(grant_http_response_credit(&mut detached, 1).is_err());
    }

    #[test]
    fn channel_output_sink_is_bounded_and_reports_a_closed_receiver() {
        let (sender, receiver) = mpsc::sync_channel(1);
        let output = CallOutput::Channel(sender.clone());
        assert!(output.send(WsOut::Text("first".into())));
        assert!(matches!(
            sender.try_send(WsOut::Text("overflow".into())),
            Err(mpsc::TrySendError::Full(_))
        ));
        assert!(matches!(receiver.try_recv(), Ok(WsOut::Text(text)) if text == "first"));
        drop(receiver);
        assert!(!output.send(WsOut::Text("closed".into())));
    }

    #[test]
    fn blocked_channel_producer_resumes_after_ack_and_receiver_close() {
        let (out_tx, out_rx) = mpsc::sync_channel(1);
        let output = CallOutput::Channel(out_tx);
        let (ack_tx, ack_rx) = async_channel::bounded(1);
        let (pump_ready_tx, pump_ready_rx) = mpsc::channel();
        let pump = std::thread::Builder::new()
            .name("test-ipc-channel-pump".into())
            .spawn(move || {
                assert!(matches!(out_rx.recv().unwrap(), WsOut::Text(ref value) if value == "one"));
                pump_ready_tx.send(()).unwrap();
                assert_eq!(smol::block_on(ack_rx.recv()).unwrap(), 1);
                assert!(matches!(out_rx.recv().unwrap(), WsOut::Text(ref value) if value == "two"));
                assert!(
                    matches!(out_rx.recv().unwrap(), WsOut::Text(ref value) if value == "three")
                );
            })
            .unwrap();

        assert!(output.send(WsOut::Text("one".into())));
        pump_ready_rx.recv().unwrap();
        assert!(output.send(WsOut::Text("two".into())));
        let sender = output.clone();
        let third = std::thread::spawn(move || sender.send(WsOut::Text("three".into())));
        ack_tx.try_send(1).unwrap();
        assert!(third.join().unwrap());
        pump.join().unwrap();
        assert!(!output.send(WsOut::Text("after-close".into())));
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
            "fs.handle",
            "fs.handleControl",
            "http.requestText",
            "http.responseAck",
            "process.execText",
            "process.execFileText",
            "process.signal",
            "process.spawnSync",
            "socket.control",
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
        assert!(manager.ipc_handlers.contains_key("http.responseAck"));
        assert!(!ipc_method_allowed("http.responseAck"));
        assert!(matches!(
            manager
                .handlers
                .get("process.spawnSync")
                .map(|entry| &entry.handler),
            Some(HandlerKind::CancellableUnary(_))
        ));
        assert!(sync_method_allowed("process.spawnSync"));
        assert!(!ipc_method_allowed("process.spawnSync"));

        for method in ["fs.handleControl", "process.signal", "socket.control"] {
            assert!(matches!(
                manager.handlers.get(method).map(|entry| &entry.handler),
                Some(HandlerKind::CancellableUnary(_))
            ));
            assert!(
                !ipc_method_allowed(method),
                "resource controls remain unavailable to remote IPC callers"
            );
        }
        assert!(matches!(
            manager
                .handlers
                .get("fs.handle")
                .map(|entry| &entry.handler),
            Some(HandlerKind::Stream(_))
        ));
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
        assert_ne!(44, session_owner_id(9, session));
        assert!(!manager.ws_session_matches(9, 44, "abcdef0123456789abcdef0123456789"));
        manager.cancel_connection(9, 44);
        assert!(!manager.ws_session_matches(9, 44, session));
    }

    #[test]
    fn websocket_disconnect_cancels_attached_stream_by_ipc_session_owner() {
        let options: NivaOptions = serde_json::from_value(json!({
            "name": "disconnect-scope-test",
            "uuid": "a51c1728-d174-42d4-8f57-7d296c966b51"
        }))
        .unwrap();
        let manager = ApiManager::new(&options);
        let window_id = 247;
        let connection_id = 44;
        let session_id = "0123456789abcdef0123456789abcdef";
        let ipc_owner_id = session_owner_id(window_id, session_id);
        assert!(manager.bind_ws_session(window_id, connection_id, session_id));
        let ipc_key = IpcSessionKey {
            window_id,
            source_origin: "niva-a51c1728d17442d48f577d296c966b51://app".into(),
            session_id: session_id.into(),
            frame_id: 0,
            generation: 0,
            is_main_frame: true,
        };
        let attached_call_id = 11;
        let fallback_call_id = 12;
        let (attached_cancel_tx, attached_cancel_rx) = async_channel::bounded(1);
        let (fallback_cancel_tx, fallback_cancel_rx) = async_channel::bounded(1);
        let (attached_inbound_tx, _) = async_channel::bounded(1);
        let (fallback_inbound_tx, _) = async_channel::bounded(1);
        let (attached_out_tx, _) = mpsc::sync_channel(1);
        let (fallback_out_tx, _) = mpsc::sync_channel(1);
        let attached_lifecycle = Arc::new(CallLifecycle::new());
        let fallback_lifecycle = Arc::new(CallLifecycle::new());
        manager.active.lock().unwrap().insert(
            (window_id, ipc_owner_id, attached_call_id),
            ActiveCall {
                lifecycle: attached_lifecycle.clone(),
                outbound: CallOutput::Channel(attached_out_tx),
                cancel_tx: attached_cancel_tx.clone(),
                inbound_tx: attached_inbound_tx,
                ipc_session: Some(ipc_key.clone()),
            },
        );
        manager.active.lock().unwrap().insert(
            (window_id, ipc_owner_id, fallback_call_id),
            ActiveCall {
                lifecycle: fallback_lifecycle.clone(),
                outbound: CallOutput::Channel(fallback_out_tx),
                cancel_tx: fallback_cancel_tx.clone(),
                inbound_tx: fallback_inbound_tx,
                ipc_session: Some(ipc_key.clone()),
            },
        );
        manager
            .ipc_sessions
            .lock()
            .unwrap()
            .reserve(
                ipc_key.clone(),
                attached_call_id,
                attached_cancel_tx,
                ipc_owner_id,
                Instant::now(),
            )
            .unwrap();
        manager
            .ipc_sessions
            .lock()
            .unwrap()
            .reserve(
                ipc_key.clone(),
                fallback_call_id,
                fallback_cancel_tx,
                ipc_owner_id,
                Instant::now(),
            )
            .unwrap();
        let (attached_ack_tx, _attached_ack_rx) = async_channel::bounded(1);
        let (attached_route_tx, _attached_route_rx) = async_channel::bounded(1);
        let attached_key = IpcChannelKey {
            session: ipc_key,
            call_id: attached_call_id,
        };
        let (fallback_ack_tx, _fallback_ack_rx) = async_channel::bounded(1);
        let (fallback_route_tx, _fallback_route_rx) = async_channel::bounded(1);
        let fallback_key = IpcChannelKey {
            session: attached_key.session.clone(),
            call_id: fallback_call_id,
        };
        manager.ipc_channels.lock().unwrap().insert(
            attached_key.clone(),
            IpcChannelCall {
                method: "http.requestStream".into(),
                capability: "attached-capability".into(),
                connection_id: ipc_owner_id,
                transport: Some(IpcChannelTransport::WebSocket {
                    connection_id,
                    sender: mpsc::channel().0,
                }),
                attach_tx: attached_route_tx,
                next_upload_seq: 1,
                upload_ended: false,
                next_http_response_ack_seq: 1,
                http_response_ack_tx: async_channel::bounded::<u64>(1).0,
                awaiting_output_seq: None,
                output_ack_tx: attached_ack_tx,
            },
        );
        manager.ipc_channels.lock().unwrap().insert(
            fallback_key.clone(),
            IpcChannelCall {
                method: "http.requestStream".into(),
                capability: "fallback-capability".into(),
                connection_id: ipc_owner_id,
                transport: Some(IpcChannelTransport::Ipc),
                attach_tx: fallback_route_tx,
                next_upload_seq: 1,
                upload_ended: false,
                next_http_response_ack_seq: 1,
                http_response_ack_tx: async_channel::bounded::<u64>(1).0,
                awaiting_output_seq: None,
                output_ack_tx: fallback_ack_tx,
            },
        );

        manager.cancel_connection(window_id, connection_id);

        assert!(!manager.ws_session_matches(window_id, connection_id, session_id));
        assert!(matches!(
            *attached_lifecycle.phase.lock().unwrap(),
            CallPhase::Cancelled
        ));
        assert!(attached_cancel_rx.is_closed());
        assert!(manager.active.lock().unwrap().contains_key(&(
            window_id,
            ipc_owner_id,
            fallback_call_id
        )));
        assert!(matches!(
            *fallback_lifecycle.phase.lock().unwrap(),
            CallPhase::Queued
        ));
        assert!(fallback_cancel_rx.try_recv().is_err());
        assert!(
            !manager
                .ipc_channels
                .lock()
                .unwrap()
                .contains_key(&attached_key)
        );
        let channels = manager.ipc_channels.lock().unwrap();
        let fallback_channel = channels.get(&fallback_key).unwrap();
        assert_eq!(fallback_channel.connection_id, ipc_owner_id);
        assert!(!matches!(
            fallback_channel.transport.as_ref(),
            Some(IpcChannelTransport::WebSocket { .. })
        ));
        assert!(
            manager
                .ipc_sessions
                .lock()
                .unwrap()
                .active_call(&fallback_key.session, fallback_call_id)
        );
    }

    #[test]
    fn heartbeat_renews_only_active_work_and_expiration_cannot_be_revived() {
        let now = Instant::now();
        let mut sessions = IpcSessions::default();
        let owner = key("0123456789abcdef0123456789abcdef", 0);
        let sibling = key("abcdef0123456789abcdef0123456789", 0);
        let (owner_tx, owner_rx) = async_channel::bounded(1);
        let (sibling_tx, sibling_rx) = async_channel::bounded(1);
        sessions
            .reserve(owner.clone(), 1, owner_tx, 1, now)
            .unwrap();
        sessions
            .reserve(sibling.clone(), 2, sibling_tx, 2, now)
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
            sessions.reserve(owner.clone(), 3, late_tx, 1, now + Duration::from_secs(7)),
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
            .reserve(selected.clone(), 1, selected_tx, 1, now)
            .unwrap();
        sessions
            .reserve(sibling.clone(), 2, sibling_tx, 2, now)
            .unwrap();
        sessions
            .reserve(newer_generation, 3, newer_tx, 3, now)
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
    fn error_reply_envelope_keeps_request_identity_and_native_origin() {
        let source = IpcFrameSource {
            source_url: "https://example.com/page".into(),
            top_url: "https://example.com/other".into(),
            frame_id: 0,
            generation: 0,
            is_main_frame: true,
        };
        let reply = ApiManager::ipc_error_envelope(
            r#"{"t":"api_call","rid":41,"id":42,"method":"os.info","args":[],"sessionId":"0123456789abcdef0123456789abcdef"}"#,
            "bad request",
            &source,
        )
        .unwrap();
        let decoded: Value = serde_json::from_str(&reply).unwrap();
        assert_eq!(decoded["sessionId"], "0123456789abcdef0123456789abcdef");
        assert_eq!(decoded["rid"], 41);
        assert_eq!(decoded["sourceOrigin"], "https://example.com");
        assert_eq!(decoded["response"]["t"], "result");
        assert_eq!(decoded["response"]["id"], 42);
        assert_eq!(decoded["response"]["code"], -1);
        assert!(ApiManager::ipc_error_envelope(
            r#"{"t":"api_call","rid":41,"id":42,"sessionId":"0123456789abcdef0123456789abcdef"}"#,
            "cross origin",
            &IpcFrameSource {
                source_url: "https://evil.example/page".into(),
                top_url: "https://example.com/page".into(),
                frame_id: 1,
                generation: 0,
                is_main_frame: false,
            },
        )
        .is_none());
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
