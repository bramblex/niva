#[cfg(target_os = "macos")]
use std::net::IpAddr;
use std::{
    collections::HashMap,
    future::Future,
    io,
    net::{
        Ipv4Addr, Ipv6Addr, Shutdown, SocketAddr, TcpListener as StdTcpListener,
        TcpStream as StdTcpStream, ToSocketAddrs,
    },
    pin::Pin,
    sync::{
        Arc, Mutex, OnceLock,
        atomic::{AtomicU8, AtomicUsize, Ordering},
    },
    task::{Context, Poll},
    time::{Duration, Instant},
};

use anyhow::{Result, anyhow, bail};
use native_tls::{Certificate, HandshakeError, Identity, TlsAcceptor, TlsConnector, TlsStream};
#[cfg(target_os = "macos")]
use security_framework::{
    certificate::SecCertificate,
    policy::SecPolicy,
    secure_transport::{
        HandshakeError as SecureHandshakeError, SslConnectionType, SslContext, SslProtocol,
        SslProtocolSide, SslStream as SecureTlsStream,
    },
    trust::TrustOptions,
};
use serde::Deserialize;
use serde_json::{Value, json};
use smol::{
    Async,
    io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt},
};
#[cfg(target_os = "macos")]
use x509_parser::{
    extensions::GeneralName,
    oid_registry::{
        OID_X509_EXT_EXTENDED_KEY_USAGE, OID_X509_EXT_KEY_USAGE, OID_X509_EXT_SUBJECT_ALT_NAME,
    },
    prelude::{ParsedExtension, parse_x509_certificate},
};

use crate::app::api_manager::{ApiManager, ApiRequest, CallContext, InboundChunk};

const MAX_ACTIVE_PER_OWNER: usize = 128;
const MAX_PENDING_PER_LISTENER: usize = 32;
const PENDING_TTL: Duration = Duration::from_secs(30);
const MAX_UDP_DATAGRAM: usize = 65_507;
const IO_BUFFER_SIZE: usize = 16 * 1024;
const READ_CREDIT_WINDOW: usize = 256 * 1024;
const DEFAULT_CONNECT_TIMEOUT_MS: u64 = 30_000;
const MAX_CONNECT_TIMEOUT_MS: u64 = 300_000;
const IO_WAIT_ANY: u8 = 0;
const IO_WAIT_READ: u8 = 1;
const IO_WAIT_WRITE: u8 = 2;

type EventSink = Arc<dyn Fn(&str, Value) + Send + Sync>;
type ChunkSink = Arc<dyn Fn(&[u8], bool) + Send + Sync>;

pub fn register_api_instances(api_manager: &mut ApiManager) {
    api_manager.register_stream_api_with("socket.tcpConnect", Some(None), tcp_connect);
    api_manager.register_stream_api_with("socket.tcpListen", Some(None), tcp_listen);
    api_manager.register_stream_api_with("socket.tcpAttach", Some(None), tcp_attach);
    api_manager.register_stream_api_with("socket.tlsConnect", Some(None), tls_connect);
    api_manager.register_stream_api_with("socket.tlsListen", Some(None), tls_listen);
    api_manager.register_stream_api_with("socket.tlsAttach", Some(None), tls_attach);
    api_manager.register_stream_api_with("socket.udpBind", Some(None), udp_bind);
    api_manager.register_stream_api("socket.udpSend", udp_send);
    api_manager.register_stream_api("socket.control", socket_control);
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Owner {
    window_id: u8,
    connection_id: u64,
}

impl From<&CallContext> for Owner {
    fn from(ctx: &CallContext) -> Self {
        Self {
            window_id: ctx.window.id,
            connection_id: ctx.connection_id,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum SocketKind {
    Tcp,
    Tls,
    Udp,
    TcpListener,
    TlsListener,
}

#[derive(Clone, Copy, Debug)]
enum SocketCommand {
    PauseRead,
    ResumeRead,
    ShutdownWrite,
    Close,
}

enum WriteShutdown {
    Tcp(StdTcpStream),
    // native-tls shutdown closes the TLS session on the current platform
    // backend; it cannot preserve reads after a TLS write-side close.
    Tls,
}

#[derive(Clone)]
struct ActiveHandle {
    owner: Owner,
    kind: SocketKind,
    parent_listener: Option<String>,
    read_control: Option<async_channel::Sender<SocketCommand>>,
    write_control: Option<async_channel::Sender<SocketCommand>>,
    udp: Option<Arc<smol::net::UdpSocket>>,
    read_credit: Option<Arc<ReadCredit>>,
    closed_tx: async_channel::Sender<()>,
    closed_rx: async_channel::Receiver<()>,
}

struct ReadCredit {
    available: AtomicUsize,
    wake_tx: async_channel::Sender<()>,
}

impl ReadCredit {
    fn new() -> (Arc<Self>, async_channel::Receiver<()>) {
        let (wake_tx, wake_rx) = async_channel::bounded(1);
        (
            Arc::new(Self {
                available: AtomicUsize::new(READ_CREDIT_WINDOW),
                wake_tx,
            }),
            wake_rx,
        )
    }

    fn grant(&self, bytes: usize) -> Result<()> {
        if bytes == 0 {
            bail!("read credit must be positive");
        }
        let _ = self
            .available
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |available| {
                Some(available.saturating_add(bytes).min(READ_CREDIT_WINDOW))
            });
        // A single queued wake coalesces an arbitrary burst of credits.
        let _ = self.wake_tx.try_send(());
        Ok(())
    }

    fn consume(&self, bytes: usize) {
        let _ = self
            .available
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |available| {
                Some(available.saturating_sub(bytes))
            });
    }

    fn consume_datagram(&self, size: usize) {
        // Empty UDP packets still consume one unit so they cannot bypass the
        // byte window and flood the WebSocket event queue.
        self.consume(size.max(1));
    }
}

enum PendingStream {
    Tcp(Async<StdTcpStream>),
    Tls(Async<StdTcpStream>, Arc<TlsAcceptor>),
}

struct PendingHandle {
    owner: Owner,
    listener_id: String,
    created: Instant,
    local_addr: SocketAddr,
    peer_addr: SocketAddr,
    listener_closed: async_channel::Receiver<()>,
    stream: PendingStream,
}

#[derive(Default)]
struct SocketRegistry {
    next_id: u64,
    active: HashMap<String, ActiveHandle>,
    pending: HashMap<String, PendingHandle>,
}

static SOCKETS: OnceLock<Mutex<SocketRegistry>> = OnceLock::new();

fn registry() -> &'static Mutex<SocketRegistry> {
    SOCKETS.get_or_init(|| Mutex::new(SocketRegistry::default()))
}

fn next_id(registry: &mut SocketRegistry, prefix: &str) -> String {
    registry.next_id = registry.next_id.wrapping_add(1).max(1);
    format!("{prefix}-{:016x}", registry.next_id)
}

fn register_active(
    owner: Owner,
    kind: SocketKind,
    parent_listener: Option<String>,
    read_control: Option<async_channel::Sender<SocketCommand>>,
    write_control: Option<async_channel::Sender<SocketCommand>>,
    udp: Option<Arc<smol::net::UdpSocket>>,
    read_credit: Option<Arc<ReadCredit>>,
) -> Result<(String, async_channel::Receiver<()>)> {
    let mut state = registry()
        .lock()
        .map_err(|_| anyhow!("socket registry poisoned"))?;
    let count = state
        .active
        .values()
        .filter(|entry| entry.owner == owner)
        .count();
    if count >= MAX_ACTIVE_PER_OWNER {
        bail!("socket handle limit reached for this WebSocket connection");
    }
    let id = next_id(&mut state, "socket");
    let (closed_tx, closed_rx) = async_channel::bounded(1);
    state.active.insert(
        id.clone(),
        ActiveHandle {
            owner,
            kind,
            parent_listener,
            read_control,
            write_control,
            udp,
            read_credit,
            closed_tx,
            closed_rx: closed_rx.clone(),
        },
    );
    Ok((id, closed_rx))
}

/// A dispatcher may drop the entire handler future when its owner disconnects.
/// Cleanup must therefore live in Drop, not only after an awaited pump loop.
struct ActiveRegistration {
    id: String,
    owner: Owner,
}

impl ActiveRegistration {
    fn new(id: &str, owner: Owner) -> Self {
        Self {
            id: id.to_owned(),
            owner,
        }
    }
}

impl Drop for ActiveRegistration {
    fn drop(&mut self) {
        remove_active(&self.id, self.owner);
    }
}

fn active_handle(id: &str, owner: Owner) -> Result<ActiveHandle> {
    let state = registry()
        .lock()
        .map_err(|_| anyhow!("socket registry poisoned"))?;
    let entry = state
        .active
        .get(id)
        .ok_or_else(|| anyhow!("unknown socket handle"))?;
    if entry.owner != owner {
        bail!("socket handle belongs to another window/frame connection");
    }
    Ok(entry.clone())
}

fn remove_active(id: &str, owner: Owner) {
    if let Ok(mut state) = registry().lock() {
        if state
            .active
            .get(id)
            .is_some_and(|entry| entry.owner == owner)
        {
            if let Some(parent) = state.active.remove(id) {
                parent.closed_tx.close();
            }
            let children: Vec<_> = state
                .active
                .iter()
                .filter(|(_, entry)| {
                    entry.parent_listener.as_deref() == Some(id) && entry.owner == owner
                })
                .map(|(child_id, _)| child_id.clone())
                .collect();
            for child_id in children {
                if let Some(child) = state.active.remove(&child_id) {
                    if let Some(tx) = child.read_control {
                        let _ = tx.try_send(SocketCommand::Close);
                    }
                    if let Some(tx) = child.write_control {
                        let _ = tx.try_send(SocketCommand::Close);
                    }
                    child.closed_tx.close();
                }
            }
        }
        state
            .pending
            .retain(|_, pending| pending.listener_id != id || pending.owner != owner);
    }
}

fn prune_pending(state: &mut SocketRegistry) {
    let now = Instant::now();
    state
        .pending
        .retain(|_, pending| now.duration_since(pending.created) < PENDING_TTL);
}

fn prune_expired_pending() {
    if let Ok(mut state) = registry().lock() {
        prune_pending(&mut state);
    }
}

fn add_pending(
    listener_id: &str,
    owner: Owner,
    local_addr: SocketAddr,
    peer_addr: SocketAddr,
    listener_closed: async_channel::Receiver<()>,
    stream: PendingStream,
) -> Result<Option<String>> {
    let mut state = registry()
        .lock()
        .map_err(|_| anyhow!("socket registry poisoned"))?;
    prune_pending(&mut state);
    let count = state
        .pending
        .values()
        .filter(|p| p.listener_id == listener_id)
        .count();
    if count >= MAX_PENDING_PER_LISTENER {
        return Ok(None);
    }
    let id = next_id(&mut state, "accepted");
    state.pending.insert(
        id.clone(),
        PendingHandle {
            owner,
            listener_id: listener_id.to_owned(),
            created: Instant::now(),
            local_addr,
            peer_addr,
            listener_closed,
            stream,
        },
    );
    Ok(Some(id))
}

fn take_pending(id: &str, owner: Owner) -> Result<PendingHandle> {
    let mut state = registry()
        .lock()
        .map_err(|_| anyhow!("socket registry poisoned"))?;
    prune_pending(&mut state);
    let pending_owner = state
        .pending
        .get(id)
        .map(|p| p.owner)
        .ok_or_else(|| anyhow!("unknown or expired accepted socket handle"))?;
    if pending_owner != owner {
        bail!("accepted socket handle belongs to another window/frame connection");
    }
    Ok(state.pending.remove(id).expect("checked pending handle"))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ConnectArgs {
    host: String,
    port: u16,
    timeout_ms: Option<u64>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ListenArgs {
    host: Option<String>,
    port: u16,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct AttachArgs {
    socket_id: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ControlArgs {
    socket_id: String,
    action: String,
    bytes: Option<usize>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct UdpSendArgs {
    socket_id: String,
    address: String,
    port: u16,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum TlsIdentityInput {
    Pem {
        key: String,
        cert: String,
    },
    Pkcs12 {
        #[serde(rename = "pfx", alias = "pfxDer")]
        pfx_der: Vec<u8>,
        passphrase: Option<String>,
    },
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct TlsListenArgs {
    host: Option<String>,
    port: u16,
    identity: TlsIdentityInput,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct TlsConnectArgs {
    host: String,
    port: u16,
    timeout_ms: Option<u64>,
    ca: Option<Vec<String>>,
}

async fn tcp_connect(ctx: CallContext, request: ApiRequest) -> Result<()> {
    let args = request.args().get::<(ConnectArgs,)>()?.0;
    let owner = Owner::from(&ctx);
    let addresses = resolve_host(&args.host, args.port).await?;
    let stream = connect_addresses(&ctx, &addresses, args.timeout_ms).await?;
    let (local, peer) = socket_addrs(&stream)?;
    let shutdown = tcp_write_shutdown(&stream)?;
    bridge_socket(
        ctx,
        owner,
        SocketKind::Tcp,
        stream,
        local,
        peer,
        shutdown,
        None,
    )
    .await
}

async fn tls_connect(ctx: CallContext, request: ApiRequest) -> Result<()> {
    let args = request.args().get::<(TlsConnectArgs,)>()?.0;
    let owner = Owner::from(&ctx);
    validate_host(&args.host)?;
    let tls_host = normalize_host(&args.host)?;
    let addresses = resolve_host(&args.host, args.port).await?;
    let mut last_error = None;
    for address in addresses {
        match connect_one(&ctx, address, args.timeout_ms).await {
            Ok(stream) => {
                let (local, peer) = socket_addrs(&stream)?;
                let tls = tls_client_handshake(
                    &tls_host,
                    stream,
                    &ctx,
                    args.timeout_ms,
                    args.ca.as_deref(),
                )
                .await?;
                return bridge_socket(
                    ctx,
                    owner,
                    SocketKind::Tls,
                    tls,
                    local,
                    peer,
                    WriteShutdown::Tls,
                    None,
                )
                .await;
            }
            Err(error) => last_error = Some(error),
        }
    }
    Err(last_error.unwrap_or_else(|| anyhow!("host resolved to no addresses")))
}

async fn tcp_listen(ctx: CallContext, request: ApiRequest) -> Result<()> {
    let args = request.args().get::<(ListenArgs,)>()?.0;
    listen_tcp(ctx, args, None).await
}

async fn tls_listen(ctx: CallContext, request: ApiRequest) -> Result<()> {
    let args = request.args().get::<(TlsListenArgs,)>()?.0;
    let identity = tls_identity(args.identity)?;
    let acceptor =
        Arc::new(TlsAcceptor::new(identity).map_err(|e| anyhow!("TLS identity rejected: {e}"))?);
    listen_tcp(
        ctx,
        ListenArgs {
            host: args.host,
            port: args.port,
        },
        Some(acceptor),
    )
    .await
}

async fn listen_tcp(
    ctx: CallContext,
    args: ListenArgs,
    tls: Option<Arc<TlsAcceptor>>,
) -> Result<()> {
    let host = args.host.unwrap_or_else(|| "127.0.0.1".to_owned());
    let addresses = resolve_host(&host, args.port).await?;
    let mut last_error = None;
    let mut listener = None;
    for addr in addresses {
        match Async::<StdTcpListener>::bind(addr) {
            Ok(value) => {
                listener = Some(value);
                break;
            }
            Err(error) => last_error = Some(error),
        }
    }
    let listener = listener.ok_or_else(|| {
        last_error
            .map(anyhow::Error::from)
            .unwrap_or_else(|| anyhow!("bind address resolved to none"))
    })?;
    let local_addr = listener.get_ref().local_addr()?;
    let owner = Owner::from(&ctx);
    let (read_control_tx, read_control_rx) = async_channel::bounded(8);
    let kind = if tls.is_some() {
        SocketKind::TlsListener
    } else {
        SocketKind::TcpListener
    };
    let (listener_id, listener_closed) =
        register_active(owner, kind, None, Some(read_control_tx), None, None, None)?;
    let registration = ActiveRegistration::new(&listener_id, owner);
    ctx.push("listening", json!({"socketId": listener_id, "address": local_addr.ip().to_string(), "port": local_addr.port()}));
    let result = accept_loop(
        ctx.clone(),
        owner,
        listener_id.clone(),
        local_addr,
        listener_closed,
        listener,
        tls,
        read_control_rx,
    )
    .await;
    drop(registration);
    match result {
        Ok(()) => {
            ctx.push("close", json!({"socketId":listener_id}));
            ctx.respond(Ok(json!({"socketId":listener_id,"closed":true})));
            Ok(())
        }
        Err(error) => Err(error),
    }
}

async fn accept_loop(
    ctx: CallContext,
    owner: Owner,
    listener_id: String,
    local_addr: SocketAddr,
    listener_closed: async_channel::Receiver<()>,
    listener: Async<StdTcpListener>,
    tls: Option<Arc<TlsAcceptor>>,
    controls: async_channel::Receiver<SocketCommand>,
) -> Result<()> {
    let mut paused = false;
    loop {
        if paused {
            match select3(
                controls.recv(),
                smol::Timer::after(Duration::from_millis(250)),
                ctx.cancelled(),
            )
            .await
            {
                Race3::First(Ok(SocketCommand::ResumeRead)) => paused = false,
                Race3::First(Ok(SocketCommand::Close)) | Race3::First(Err(_)) | Race3::Third(_) => {
                    return Ok(());
                }
                Race3::First(Ok(_)) => {}
                Race3::Second(_) => prune_expired_pending(),
            }
            continue;
        }
        match select4(
            controls.recv(),
            ctx.cancelled(),
            listener.accept(),
            smol::Timer::after(Duration::from_millis(250)),
        )
        .await
        {
            Race4::Third(Ok((stream, peer))) => {
                let pending_stream = if let Some(acceptor) = &tls {
                    PendingStream::Tls(stream, acceptor.clone())
                } else {
                    PendingStream::Tcp(stream)
                };
                match add_pending(&listener_id, owner, local_addr, peer, listener_closed.clone(), pending_stream)? {
                    Some(socket_id) => ctx.push("connection", json!({"socketId":socket_id,"address":peer.ip().to_string(),"port":peer.port()})),
                    None => ctx.push("connectionError", json!({"code":"ERR_SOCKET_ACCEPT_QUEUE_FULL"})),
                }
            }
            Race4::Third(Err(error)) => return Err(error.into()),
            Race4::First(Ok(SocketCommand::PauseRead)) => paused = true,
            Race4::First(Ok(SocketCommand::Close)) | Race4::Second(()) | Race4::First(Err(_)) => {
                return Ok(());
            }
            Race4::First(Ok(_)) | Race4::Fourth(_) => {
                prune_expired_pending();
            }
        }
    }
}

async fn tcp_attach(ctx: CallContext, request: ApiRequest) -> Result<()> {
    let args = request.args().get::<(AttachArgs,)>()?.0;
    let owner = Owner::from(&ctx);
    let pending = take_pending(&args.socket_id, owner)?;
    if pending.listener_closed.is_closed() {
        bail!("parent listener has closed");
    }
    match pending.stream {
        PendingStream::Tcp(stream) => {
            let shutdown = tcp_write_shutdown(&stream)?;
            bridge_socket(
                ctx,
                owner,
                SocketKind::Tcp,
                stream,
                pending.local_addr,
                pending.peer_addr,
                shutdown,
                Some(pending.listener_id),
            )
            .await
        }
        PendingStream::Tls(_, _) => bail!("accepted socket requires tlsAttach"),
    }
}

async fn tls_attach(ctx: CallContext, request: ApiRequest) -> Result<()> {
    let args = request.args().get::<(AttachArgs,)>()?.0;
    let owner = Owner::from(&ctx);
    let pending = take_pending(&args.socket_id, owner)?;
    if pending.listener_closed.is_closed() {
        bail!("parent listener has closed");
    }
    match pending.stream {
        PendingStream::Tls(stream, acceptor) => {
            let tls = tls_server_handshake(acceptor, stream, &ctx, pending.listener_closed.clone())
                .await?;
            bridge_socket(
                ctx,
                owner,
                SocketKind::Tls,
                tls,
                pending.local_addr,
                pending.peer_addr,
                WriteShutdown::Tls,
                Some(pending.listener_id),
            )
            .await
        }
        PendingStream::Tcp(_) => bail!("accepted socket requires tcpAttach"),
    }
}

async fn udp_bind(ctx: CallContext, request: ApiRequest) -> Result<()> {
    let args = request.args().get::<(ListenArgs,)>()?.0;
    let host = args.host.unwrap_or_else(|| "127.0.0.1".to_owned());
    let addresses = resolve_host(&host, args.port).await?;
    let mut last_error = None;
    let mut socket = None;
    for address in addresses {
        match smol::net::UdpSocket::bind(address).await {
            Ok(value) => {
                socket = Some(value);
                break;
            }
            Err(error) => last_error = Some(error),
        }
    }
    let socket = Arc::new(socket.ok_or_else(|| {
        last_error
            .map(anyhow::Error::from)
            .unwrap_or_else(|| anyhow!("bind address resolved to none"))
    })?);
    let local_addr = socket.local_addr()?;
    let owner = Owner::from(&ctx);
    let (control_tx, control_rx) = async_channel::bounded(8);
    let (read_credit, read_wake_rx) = ReadCredit::new();
    let (socket_id, _closed) = register_active(
        owner,
        SocketKind::Udp,
        None,
        Some(control_tx),
        None,
        Some(socket.clone()),
        Some(read_credit.clone()),
    )?;
    let registration = ActiveRegistration::new(&socket_id, owner);
    ctx.push("listening", json!({"socketId":socket_id,"address":local_addr.ip().to_string(),"port":local_addr.port()}));
    let mut buf = vec![0u8; MAX_UDP_DATAGRAM];
    let mut paused = false;
    let result = loop {
        if paused || read_credit.available.load(Ordering::Acquire) == 0 {
            match select3(control_rx.recv(), read_wake_rx.recv(), ctx.cancelled()).await {
                Race3::First(Ok(SocketCommand::ResumeRead)) => paused = false,
                Race3::First(Ok(SocketCommand::Close))
                | Race3::First(Err(_))
                | Race3::Third(()) => break Ok(()),
                Race3::Second(_) => {}
                Race3::First(Ok(_)) => {}
            }
            continue;
        }
        match select3(
            control_rx.recv(),
            ctx.cancelled(),
            socket.recv_from(&mut buf),
        )
        .await
        {
            Race3::Third(Ok((size, peer))) => {
                read_credit.consume_datagram(size);
                ctx.push("datagram", json!({"socketId":socket_id,"address":peer.ip().to_string(),"port":peer.port(),"size":size}));
                ctx.chunk(&buf[..size], true);
            }
            Race3::Third(Err(error)) => break Err(error.into()),
            Race3::First(Ok(SocketCommand::PauseRead)) => paused = true,
            Race3::First(Ok(SocketCommand::Close)) | Race3::First(Err(_)) | Race3::Second(_) => {
                break Ok(());
            }
            Race3::First(Ok(_)) => {}
        }
    };
    drop(registration);
    ctx.push("close", json!({"socketId":socket_id}));
    match result {
        Ok(()) => {
            ctx.respond(Ok(json!({"socketId":socket_id,"closed":true})));
            Ok(())
        }
        Err(error) => Err(error),
    }
}

async fn udp_send(ctx: CallContext, request: ApiRequest) -> Result<()> {
    let args = request.args().get::<(UdpSendArgs,)>()?.0;
    let owner = Owner::from(&ctx);
    let entry = active_handle(&args.socket_id, owner)?;
    if entry.kind != SocketKind::Udp {
        bail!("handle is not a UDP socket");
    }
    let socket = entry.udp.ok_or_else(|| anyhow!("UDP handle is closed"))?;
    let addresses = match select3(
        resolve_host(&args.address, args.port),
        ctx.cancelled(),
        entry.closed_rx.recv(),
    )
    .await
    {
        Race3::First(Ok(addresses)) => addresses,
        Race3::First(Err(error)) => return Err(error),
        Race3::Second(()) => bail!("UDP send cancelled"),
        Race3::Third(_) => bail!("UDP socket closed"),
    };
    let mut payload = Vec::new();
    loop {
        match select3(ctx.cancelled(), entry.closed_rx.recv(), ctx.next_chunk()).await {
            Race3::Third(Some(chunk)) => {
                if payload.len().saturating_add(chunk.data.len()) > MAX_UDP_DATAGRAM {
                    bail!("UDP datagram exceeds 65507 bytes");
                }
                payload.extend_from_slice(&chunk.data);
                if chunk.end {
                    break;
                }
            }
            Race3::Third(None) => bail!("UDP send stream ended before datagram END"),
            Race3::First(()) => bail!("UDP send stream cancelled"),
            Race3::Second(_) => bail!("UDP socket closed"),
        }
    }
    let mut last_error = None;
    for address in addresses {
        match select3(
            ctx.cancelled(),
            entry.closed_rx.recv(),
            socket.send_to(&payload, address),
        )
        .await
        {
            Race3::Third(Ok(size)) if size == payload.len() => {
                ctx.push("sent", json!({"socketId":args.socket_id,"bytes":size,"address":address.ip().to_string(),"port":address.port()}));
                ctx.respond(Ok(json!({"sent":size})));
                return Ok(());
            }
            Race3::Third(Ok(_)) => last_error = Some(anyhow!("partial UDP datagram send")),
            Race3::Third(Err(error)) => last_error = Some(error.into()),
            Race3::First(()) => bail!("UDP send cancelled"),
            Race3::Second(_) => bail!("UDP socket closed"),
        }
    }
    Err(last_error.unwrap_or_else(|| anyhow!("UDP destination resolved to no addresses")))
}

async fn socket_control(ctx: CallContext, request: ApiRequest) -> Result<()> {
    let args = request.args().get::<(ControlArgs,)>()?.0;
    let owner = Owner::from(&ctx);
    let entry = active_handle(&args.socket_id, owner)?;
    if args.action == "readAck" {
        let bytes = args
            .bytes
            .ok_or_else(|| anyhow!("readAck requires bytes"))?;
        let credit = entry
            .read_credit
            .as_ref()
            .ok_or_else(|| anyhow!("socket does not have a readable byte stream"))?;
        credit.grant(bytes)?;
        ctx.respond(Ok(
            json!({"socketId":args.socket_id,"action":args.action,"ok":true}),
        ));
        return Ok(());
    }
    let command = match args.action.as_str() {
        "pauseRead" => SocketCommand::PauseRead,
        "resumeRead" => SocketCommand::ResumeRead,
        "shutdownWrite" => SocketCommand::ShutdownWrite,
        "close" => SocketCommand::Close,
        _ => bail!("unsupported socket control action"),
    };
    let mut delivered = false;
    if matches!(
        command,
        SocketCommand::PauseRead | SocketCommand::ResumeRead | SocketCommand::Close
    ) {
        if let Some(tx) = &entry.read_control {
            try_send_control(tx, command)?;
            delivered = true;
        }
    }
    if matches!(command, SocketCommand::ShutdownWrite | SocketCommand::Close) {
        if let Some(tx) = &entry.write_control {
            try_send_control(tx, command)?;
            delivered = true;
        }
    }
    if !delivered {
        bail!("socket control action is not supported for this handle");
    }
    ctx.respond(Ok(
        json!({"socketId":args.socket_id,"action":args.action,"ok":true}),
    ));
    Ok(())
}

fn try_send_control(
    tx: &async_channel::Sender<SocketCommand>,
    command: SocketCommand,
) -> Result<()> {
    tx.try_send(command)
        .map_err(|error| anyhow!("socket control queue unavailable: {error}"))
}

async fn bridge_socket<I>(
    ctx: CallContext,
    owner: Owner,
    kind: SocketKind,
    io: I,
    local: SocketAddr,
    peer: SocketAddr,
    write_shutdown: WriteShutdown,
    parent_listener: Option<String>,
) -> Result<()>
where
    I: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    let (read_control_tx, read_control_rx) = async_channel::bounded(8);
    let (write_control_tx, write_control_rx) = async_channel::bounded(8);
    let (read_credit, read_wake_rx) = ReadCredit::new();
    let (socket_id, _closed) = register_active(
        owner,
        kind,
        parent_listener,
        Some(read_control_tx),
        Some(write_control_tx),
        None,
        Some(read_credit.clone()),
    )?;
    let registration = ActiveRegistration::new(&socket_id, owner);
    let event_name = if kind == SocketKind::Tls {
        "secureConnect"
    } else {
        "connect"
    };
    ctx.push(
        event_name,
        json!({
            "socketId":socket_id,
            "localAddress":local.ip().to_string(), "localPort":local.port(),
            "remoteAddress":peer.ip().to_string(), "remotePort":peer.port()
        }),
    );
    let (reader, writer) = smol::io::split(io);
    let read_event_ctx = ctx.clone();
    let read_events: EventSink = Arc::new(move |name, data| read_event_ctx.push(name, data));
    let read_chunk_ctx = ctx.clone();
    let read_chunks: ChunkSink = Arc::new(move |data, end| read_chunk_ctx.chunk(data, end));
    let write_event_ctx = ctx.clone();
    let write_events: EventSink = Arc::new(move |name, data| write_event_ctx.push(name, data));
    let read_loop = pump_socket_read(
        reader,
        read_control_rx,
        read_credit,
        read_wake_rx,
        ctx.cancel_rx.clone(),
        socket_id.clone(),
        read_events,
        read_chunks,
    );
    let write_loop = pump_socket_write(
        writer,
        write_control_rx,
        write_shutdown,
        ctx.inbound_rx.clone(),
        ctx.cancel_rx.clone(),
        write_events,
    );
    let outcome = join_socket_pumps_for_kind(kind, read_loop, write_loop).await;
    drop(registration);
    ctx.push(
        "close",
        json!({"socketId":socket_id,"hadError":outcome.is_err()}),
    );
    match outcome {
        Ok(()) => {
            ctx.respond(Ok(json!({"socketId":socket_id,"closed":true})));
            Ok(())
        }
        Err(error) => Err(error),
    }
}

async fn join_socket_pumps<R, W>(read_loop: R, write_loop: W) -> Result<()>
where
    R: Future<Output = Result<()>>,
    W: Future<Output = Result<()>>,
{
    smol::future::try_zip(read_loop, write_loop)
        .await
        .map(|_| ())
}

async fn join_socket_pumps_for_kind<R, W>(
    kind: SocketKind,
    read_loop: R,
    write_loop: W,
) -> Result<()>
where
    R: Future<Output = Result<()>>,
    W: Future<Output = Result<()>>,
{
    if kind == SocketKind::Tls {
        // native-tls shutdown closes the full TLS session on the current
        // Security Framework backend, so either completed direction ends both.
        smol::future::or(read_loop, write_loop).await
    } else {
        join_socket_pumps(read_loop, write_loop).await
    }
}

fn socket_addrs(stream: &Async<StdTcpStream>) -> Result<(SocketAddr, SocketAddr)> {
    Ok((
        stream.get_ref().local_addr()?,
        stream.get_ref().peer_addr()?,
    ))
}

fn tcp_write_shutdown(stream: &Async<StdTcpStream>) -> Result<WriteShutdown> {
    Ok(WriteShutdown::Tcp(stream.get_ref().try_clone()?))
}

async fn close_write_half<W>(writer: &mut W, shutdown: &WriteShutdown) -> Result<()>
where
    W: AsyncWrite + Unpin,
{
    match shutdown {
        WriteShutdown::Tcp(stream) => {
            writer.flush().await?;
            stream.shutdown(Shutdown::Write)?;
            Ok(())
        }
        WriteShutdown::Tls => {
            writer.close().await?;
            Ok(())
        }
    }
}

async fn pump_socket_read<R>(
    mut reader: R,
    controls: async_channel::Receiver<SocketCommand>,
    credit: Arc<ReadCredit>,
    credit_wake: async_channel::Receiver<()>,
    cancelled: async_channel::Receiver<()>,
    socket_id: String,
    events: EventSink,
    chunks: ChunkSink,
) -> Result<()>
where
    R: AsyncRead + Unpin,
{
    let mut buf = vec![0u8; IO_BUFFER_SIZE];
    let mut paused = false;
    loop {
        let available = credit.available.load(Ordering::Acquire);
        if paused || available == 0 {
            match select3(controls.recv(), credit_wake.recv(), cancelled.recv()).await {
                Race3::First(Ok(SocketCommand::ResumeRead)) => paused = false,
                Race3::First(Ok(SocketCommand::Close)) | Race3::First(Err(_)) | Race3::Third(_) => {
                    return Ok(());
                }
                Race3::First(Ok(SocketCommand::PauseRead)) => paused = true,
                Race3::First(Ok(_)) | Race3::Second(_) => {}
            }
            continue;
        }
        let limit = buf.len().min(available);
        match select3(
            controls.recv(),
            cancelled.recv(),
            reader.read(&mut buf[..limit]),
        )
        .await
        {
            Race3::Third(Ok(0)) => {
                events("end", json!({"socketId":socket_id}));
                chunks(&[], true);
                return Ok(());
            }
            Race3::Third(Ok(size)) => {
                credit.consume(size);
                chunks(&buf[..size], false);
            }
            Race3::Third(Err(error)) => return Err(error.into()),
            Race3::First(Ok(SocketCommand::PauseRead)) => paused = true,
            Race3::First(Ok(SocketCommand::Close)) | Race3::First(Err(_)) | Race3::Second(_) => {
                return Ok(());
            }
            Race3::First(Ok(_)) => {}
        }
    }
}

async fn pump_socket_write<W>(
    mut writer: W,
    controls: async_channel::Receiver<SocketCommand>,
    write_shutdown: WriteShutdown,
    inbound: async_channel::Receiver<InboundChunk>,
    cancelled: async_channel::Receiver<()>,
    events: EventSink,
) -> Result<()>
where
    W: AsyncWrite + Unpin,
{
    loop {
        match select3(controls.recv(), cancelled.recv(), inbound.recv()).await {
            Race3::Third(Ok(chunk)) => {
                writer.write_all(&chunk.data).await?;
                writer.flush().await?;
                events(
                    "writeAck",
                    json!({"seq":chunk.seq,"bytes":chunk.data.len()}),
                );
                if chunk.end {
                    close_write_half(&mut writer, &write_shutdown).await?;
                    return Ok(());
                }
            }
            Race3::Third(Err(_)) => {
                close_write_half(&mut writer, &write_shutdown).await?;
                return Ok(());
            }
            Race3::First(Ok(SocketCommand::ShutdownWrite)) => {
                close_write_half(&mut writer, &write_shutdown).await?;
                return Ok(());
            }
            Race3::First(Ok(SocketCommand::Close)) | Race3::First(Err(_)) | Race3::Second(_) => {
                return Ok(());
            }
            Race3::First(Ok(_)) => {}
        }
    }
}

async fn tls_client_handshake(
    host: &str,
    stream: Async<StdTcpStream>,
    ctx: &CallContext,
    timeout_ms: Option<u64>,
    ca: Option<&[String]>,
) -> Result<AsyncTlsIo> {
    let timeout = Duration::from_millis(
        timeout_ms
            .unwrap_or(DEFAULT_CONNECT_TIMEOUT_MS)
            .clamp(1, MAX_CONNECT_TIMEOUT_MS),
    );
    #[cfg(target_os = "macos")]
    let handshake = async move {
        match ca {
            Some(ca) => tls_client_handshake_explicit_ca_inner(host.to_owned(), stream, ca).await,
            None => tls_client_handshake_inner(tls_connector(None)?, host.to_owned(), stream).await,
        }
    };
    #[cfg(not(target_os = "macos"))]
    let handshake = async move {
        tls_client_handshake_inner(tls_connector(ca)?, host.to_owned(), stream).await
    };
    match select3(handshake, smol::Timer::after(timeout), ctx.cancelled()).await {
        Race3::First(result) => result,
        Race3::Second(_) => Err(anyhow!("TLS handshake timed out")),
        Race3::Third(()) => Err(anyhow!("TLS handshake cancelled")),
    }
}

async fn tls_client_handshake_inner(
    connector: TlsConnector,
    host: String,
    stream: Async<StdTcpStream>,
) -> Result<AsyncTlsIo> {
    let mut handshake = match connector.connect(&host, SocketIo::new(stream)) {
        Ok(tls) => return Ok(AsyncTlsIo::Native(tls)),
        Err(HandshakeError::Failure(error)) => {
            return Err(anyhow!("TLS handshake failed: {error}"));
        }
        Err(HandshakeError::WouldBlock(mid)) => mid,
    };
    loop {
        wait_socket_io(handshake.get_ref()).await?;
        handshake.get_ref().reset_wait();
        match handshake.handshake() {
            Ok(tls) => return Ok(AsyncTlsIo::Native(tls)),
            Err(HandshakeError::Failure(error)) => {
                return Err(anyhow!("TLS handshake failed: {error}"));
            }
            Err(HandshakeError::WouldBlock(mid)) => handshake = mid,
        }
    }
}

#[cfg(target_os = "macos")]
async fn tls_client_handshake_explicit_ca_inner(
    host: String,
    stream: Async<StdTcpStream>,
    ca: &[String],
) -> Result<AsyncTlsIo> {
    let anchors = macos_ca_certificates(ca)?;
    let mut context = SslContext::new(SslProtocolSide::CLIENT, SslConnectionType::STREAM)
        .map_err(|error| anyhow!("TLS context unavailable: {error}"))?;
    context
        .set_peer_domain_name(&host)
        .map_err(|error| anyhow!("TLS server name rejected: {error}"))?;
    context
        .set_protocol_version_min(SslProtocol::TLS12)
        .map_err(|error| anyhow!("TLS minimum protocol configuration failed: {error}"))?;
    context
        .set_break_on_server_auth(true)
        .map_err(|error| anyhow!("TLS verification gate unavailable: {error}"))?;

    let mut handshake = match context.handshake(SocketIo::new(stream)) {
        Ok(_) => bail!("TLS handshake completed without the peer verification gate"),
        Err(SecureHandshakeError::Failure(error)) => {
            return Err(anyhow!("TLS handshake failed: {error}"));
        }
        Err(SecureHandshakeError::Interrupted(handshake)) => handshake,
    };

    let mut peer_verified = false;
    loop {
        if handshake.server_auth_completed() {
            let mut trust = handshake
                .context()
                .peer_trust2()
                .map_err(|error| anyhow!("TLS peer trust unavailable: {error}"))?
                .ok_or_else(|| anyhow!("TLS peer certificate is unavailable"))?;
            #[allow(deprecated)]
            let peer_leaf = trust
                .certificate_at_index(0)
                .ok_or_else(|| anyhow!("TLS peer leaf certificate is unavailable"))?;
            let peer_der = peer_leaf.to_der();
            let explicitly_pinned_leaf = anchors.iter().any(|anchor| anchor.to_der() == peer_der);

            if explicitly_pinned_leaf {
                verify_macos_pinned_leaf_name_and_time(&peer_der, &host, macos_unix_timestamp()?)?;
                trust
                    .set_policy(&SecPolicy::create_x509())
                    .map_err(|error| anyhow!("TLS X.509 policy unavailable: {error}"))?;
                trust
                    .set_options(TrustOptions::LEAF_IS_CA)
                    .map_err(|error| anyhow!("TLS pinned-leaf policy unavailable: {error}"))?;
            }

            trust
                .set_anchor_certificates(&anchors)
                .map_err(|error| anyhow!("TLS CA configuration rejected: {error}"))?;
            trust
                .set_trust_anchor_certificates_only(true)
                .map_err(|error| anyhow!("TLS CA configuration rejected: {error}"))?;
            trust
                .evaluate_with_error()
                .map_err(|error| anyhow!("TLS certificate verification failed: {error}"))?;
            peer_verified = true;

            match handshake.handshake() {
                Ok(tls) if peer_verified => return Ok(AsyncTlsIo::SecureTransport(tls)),
                Ok(_) => bail!("TLS handshake completed before peer verification"),
                Err(SecureHandshakeError::Failure(error)) => {
                    return Err(anyhow!("TLS handshake failed: {error}"));
                }
                Err(SecureHandshakeError::Interrupted(next)) => handshake = next,
            }
            continue;
        }

        if !handshake.would_block() {
            return Err(anyhow!(
                "TLS handshake stopped before certificate verification"
            ));
        }
        wait_socket_io(handshake.get_ref()).await?;
        handshake.get_ref().reset_wait();
        match handshake.handshake() {
            Ok(tls) if peer_verified => return Ok(AsyncTlsIo::SecureTransport(tls)),
            Ok(_) => bail!("TLS handshake completed before peer verification"),
            Err(SecureHandshakeError::Failure(error)) => {
                return Err(anyhow!("TLS handshake failed: {error}"));
            }
            Err(SecureHandshakeError::Interrupted(next)) => handshake = next,
        }
    }
}

async fn tls_server_handshake(
    acceptor: Arc<TlsAcceptor>,
    stream: Async<StdTcpStream>,
    ctx: &CallContext,
    listener_closed: async_channel::Receiver<()>,
) -> Result<AsyncTlsIo> {
    let timeout = Duration::from_millis(DEFAULT_CONNECT_TIMEOUT_MS);
    match select4(
        tls_server_handshake_inner(acceptor, stream),
        smol::Timer::after(timeout),
        ctx.cancelled(),
        listener_closed.recv(),
    )
    .await
    {
        Race4::First(result) => result,
        Race4::Second(_) => Err(anyhow!("TLS client handshake timed out")),
        Race4::Third(()) => Err(anyhow!("TLS client handshake cancelled")),
        Race4::Fourth(_) => Err(anyhow!("TLS listener closed during handshake")),
    }
}

async fn tls_server_handshake_inner(
    acceptor: Arc<TlsAcceptor>,
    stream: Async<StdTcpStream>,
) -> Result<AsyncTlsIo> {
    let mut handshake = match acceptor.accept(SocketIo::new(stream)) {
        Ok(tls) => return Ok(AsyncTlsIo::Native(tls)),
        Err(HandshakeError::Failure(error)) => {
            return Err(anyhow!("TLS client handshake failed: {error}"));
        }
        Err(HandshakeError::WouldBlock(mid)) => mid,
    };
    loop {
        wait_socket_io(handshake.get_ref()).await?;
        handshake.get_ref().reset_wait();
        match handshake.handshake() {
            Ok(tls) => return Ok(AsyncTlsIo::Native(tls)),
            Err(HandshakeError::Failure(error)) => {
                return Err(anyhow!("TLS client handshake failed: {error}"));
            }
            Err(HandshakeError::WouldBlock(mid)) => handshake = mid,
        }
    }
}

async fn wait_socket_io(stream: &SocketIo) -> io::Result<()> {
    std::future::poll_fn(|cx| stream.poll_ready(cx)).await
}

/// `native-tls` exposes a synchronous Read/Write interface. This adapter
/// presents the already-nonblocking TCP handle to it; async callers wait on
/// the same handle's readiness notifications below.
struct SocketIo {
    stream: Async<StdTcpStream>,
    wait_for: AtomicU8,
}

impl SocketIo {
    fn new(stream: Async<StdTcpStream>) -> Self {
        Self {
            stream,
            wait_for: AtomicU8::new(IO_WAIT_ANY),
        }
    }

    fn poll_ready(&self, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        match self.wait_for.load(Ordering::Acquire) {
            IO_WAIT_READ => self.stream.poll_readable(cx),
            IO_WAIT_WRITE => self.stream.poll_writable(cx),
            _ => {
                let read = self.stream.poll_readable(cx);
                let write = self.stream.poll_writable(cx);
                match (read, write) {
                    (Poll::Ready(Err(error)), _) | (_, Poll::Ready(Err(error))) => {
                        Poll::Ready(Err(error))
                    }
                    (Poll::Ready(Ok(())), _) | (_, Poll::Ready(Ok(()))) => Poll::Ready(Ok(())),
                    (Poll::Pending, Poll::Pending) => Poll::Pending,
                }
            }
        }
    }

    fn record_result<T>(&self, direction: u8, result: io::Result<T>) -> io::Result<T> {
        self.wait_for.store(
            if result
                .as_ref()
                .is_err_and(|error| error.kind() == io::ErrorKind::WouldBlock)
            {
                direction
            } else {
                IO_WAIT_ANY
            },
            Ordering::Release,
        );
        result
    }

    fn reset_wait(&self) {
        self.wait_for.store(IO_WAIT_ANY, Ordering::Release);
    }
}

impl io::Read for SocketIo {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let mut stream = self.stream.get_ref();
        let result = io::Read::read(&mut stream, buf);
        self.record_result(IO_WAIT_READ, result)
    }
}

impl io::Write for SocketIo {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let mut stream = self.stream.get_ref();
        let result = io::Write::write(&mut stream, buf);
        self.record_result(IO_WAIT_WRITE, result)
    }
    fn flush(&mut self) -> io::Result<()> {
        let mut stream = self.stream.get_ref();
        let result = io::Write::flush(&mut stream);
        self.record_result(IO_WAIT_WRITE, result)
    }
}

enum AsyncTlsIo {
    Native(TlsStream<SocketIo>),
    #[cfg(target_os = "macos")]
    SecureTransport(SecureTlsStream<SocketIo>),
}

impl AsyncTlsIo {
    fn socket_io(&self) -> &SocketIo {
        match self {
            Self::Native(stream) => stream.get_ref(),
            #[cfg(target_os = "macos")]
            Self::SecureTransport(stream) => stream.get_ref(),
        }
    }

    fn close_tls(&mut self) -> io::Result<()> {
        match self {
            Self::Native(stream) => stream.shutdown(),
            #[cfg(target_os = "macos")]
            Self::SecureTransport(stream) => stream.close(),
        }
    }
}

impl io::Read for AsyncTlsIo {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        match self {
            Self::Native(stream) => io::Read::read(stream, buf),
            #[cfg(target_os = "macos")]
            Self::SecureTransport(stream) => io::Read::read(stream, buf),
        }
    }
}

impl io::Write for AsyncTlsIo {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        match self {
            Self::Native(stream) => io::Write::write(stream, buf),
            #[cfg(target_os = "macos")]
            Self::SecureTransport(stream) => io::Write::write(stream, buf),
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        match self {
            Self::Native(stream) => io::Write::flush(stream),
            #[cfg(target_os = "macos")]
            Self::SecureTransport(stream) => io::Write::flush(stream),
        }
    }
}

impl AsyncRead for AsyncTlsIo {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<io::Result<usize>> {
        let this = self.get_mut();
        loop {
            this.socket_io().reset_wait();
            match io::Read::read(this, buf) {
                Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                    match this.socket_io().poll_ready(cx) {
                        Poll::Ready(Ok(())) => continue,
                        Poll::Ready(Err(error)) => return Poll::Ready(Err(error)),
                        Poll::Pending => return Poll::Pending,
                    }
                }
                result => return Poll::Ready(result),
            }
        }
    }
}

impl AsyncWrite for AsyncTlsIo {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        let this = self.get_mut();
        loop {
            this.socket_io().reset_wait();
            match io::Write::write(this, buf) {
                Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                    match this.socket_io().poll_ready(cx) {
                        Poll::Ready(Ok(())) => continue,
                        Poll::Ready(Err(error)) => return Poll::Ready(Err(error)),
                        Poll::Pending => return Poll::Pending,
                    }
                }
                result => return Poll::Ready(result),
            }
        }
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        let this = self.get_mut();
        loop {
            this.socket_io().reset_wait();
            match io::Write::flush(this) {
                Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                    match this.socket_io().poll_ready(cx) {
                        Poll::Ready(Ok(())) => continue,
                        Poll::Ready(Err(error)) => return Poll::Ready(Err(error)),
                        Poll::Pending => return Poll::Pending,
                    }
                }
                result => return Poll::Ready(result),
            }
        }
    }

    fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        let this = self.get_mut();
        loop {
            this.socket_io().reset_wait();
            match this.close_tls() {
                Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                    match this.socket_io().poll_ready(cx) {
                        Poll::Ready(Ok(())) => continue,
                        Poll::Ready(Err(error)) => return Poll::Ready(Err(error)),
                        Poll::Pending => return Poll::Pending,
                    }
                }
                result => return Poll::Ready(result),
            }
        }
    }
}

async fn resolve_host(host: &str, port: u16) -> Result<Vec<SocketAddr>> {
    validate_host(host)?;
    let host = normalize_host(host)?;
    let addrs = smol::unblock(move || {
        (host.as_str(), port)
            .to_socket_addrs()
            .map(|it| it.collect::<Vec<_>>())
    })
    .await?;
    let mut unique = Vec::new();
    for addr in addrs {
        if !unique.contains(&addr) {
            unique.push(addr);
        }
        if unique.len() >= 32 {
            break;
        }
    }
    if unique.is_empty() {
        bail!("host resolved to no addresses");
    }
    Ok(unique)
}

fn validate_host(host: &str) -> Result<()> {
    if host.is_empty()
        || host.contains('@')
        || host.contains('/')
        || host.contains('\\')
        || host.contains('\0')
    {
        bail!("invalid socket host");
    }
    Ok(())
}

fn normalize_host(host: &str) -> Result<String> {
    validate_host(host)?;
    if host.starts_with('[') && host.ends_with(']') {
        return Ok(host[1..host.len() - 1].to_owned());
    }
    Ok(host.to_owned())
}

async fn connect_addresses(
    ctx: &CallContext,
    addresses: &[SocketAddr],
    timeout_ms: Option<u64>,
) -> Result<Async<StdTcpStream>> {
    let timeout = Duration::from_millis(
        timeout_ms
            .unwrap_or(DEFAULT_CONNECT_TIMEOUT_MS)
            .clamp(1, MAX_CONNECT_TIMEOUT_MS),
    );
    let deadline = Instant::now() + timeout;
    let mut last_error = None;
    for address in addresses {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return Err(anyhow!("socket connect timed out"));
        }
        let connect = Async::<StdTcpStream>::connect(*address);
        match select3(connect, smol::Timer::after(remaining), ctx.cancelled()).await {
            Race3::First(Ok(stream)) => return Ok(stream),
            Race3::First(Err(error)) => last_error = Some(error),
            Race3::Second(_) => return Err(anyhow!("socket connect timed out")),
            Race3::Third(()) => return Err(anyhow!("socket connect cancelled")),
        }
    }
    Err(last_error
        .map(anyhow::Error::from)
        .unwrap_or_else(|| anyhow!("connect failed")))
}

async fn connect_one(
    ctx: &CallContext,
    address: SocketAddr,
    timeout_ms: Option<u64>,
) -> Result<Async<StdTcpStream>> {
    connect_addresses(ctx, &[address], timeout_ms).await
}

fn tls_identity(input: TlsIdentityInput) -> Result<Identity> {
    match input {
        TlsIdentityInput::Pem { key, cert } => {
            Identity::from_pkcs8(cert.as_bytes(), key.as_bytes())
                .map_err(|e| anyhow!("TLS PEM identity rejected: {e}"))
        }
        TlsIdentityInput::Pkcs12 {
            pfx_der,
            passphrase,
        } => Identity::from_pkcs12(&pfx_der, passphrase.as_deref().unwrap_or(""))
            .map_err(|e| anyhow!("TLS PKCS#12 identity rejected: {e}")),
    }
}

fn tls_connector(ca: Option<&[String]>) -> Result<TlsConnector> {
    let mut builder = TlsConnector::builder();
    if let Some(cas) = ca {
        builder.disable_built_in_roots(true);
        for pem in cas {
            let certificates = Certificate::stack_from_pem(pem.as_bytes())
                .map_err(|e| anyhow!("invalid CA certificate: {e}"))?;
            if certificates.is_empty() {
                bail!("invalid CA certificate: PEM contains no certificates");
            }
            for certificate in certificates {
                builder.add_root_certificate(certificate);
            }
        }
    }
    builder
        .build()
        .map_err(|e| anyhow!("TLS connector unavailable: {e}"))
}

#[cfg(target_os = "macos")]
fn macos_ca_certificates(ca: &[String]) -> Result<Vec<SecCertificate>> {
    let mut certificates = Vec::new();
    for pem in ca {
        for block in x509_parser::pem::Pem::iter_from_buffer(pem.as_bytes()) {
            let block = block.map_err(|error| anyhow!("invalid CA certificate PEM: {error}"))?;
            if block.label != "CERTIFICATE" {
                bail!("invalid CA certificate PEM block: {}", block.label);
            }
            let (trailing, _) = parse_x509_certificate(&block.contents)
                .map_err(|error| anyhow!("invalid CA certificate DER: {error}"))?;
            if !trailing.is_empty() {
                bail!("invalid CA certificate DER: trailing data");
            }
            certificates.push(
                SecCertificate::from_der(&block.contents)
                    .map_err(|error| anyhow!("invalid CA certificate: {error}"))?,
            );
        }
    }
    Ok(certificates)
}

#[cfg(target_os = "macos")]
fn macos_tls_dns_name_matches(hostname: &str, pattern: &str) -> bool {
    fn strip_one_trailing_dot(value: &str) -> &str {
        value.strip_suffix('.').unwrap_or(value)
    }

    fn host_parts(value: &str) -> Vec<String> {
        strip_one_trailing_dot(value)
            .split('.')
            .map(str::to_ascii_lowercase)
            .collect()
    }

    let hostname = host_parts(hostname);
    let pattern = host_parts(pattern);
    if hostname.len() != pattern.len()
        || pattern.iter().any(String::is_empty)
        || pattern
            .iter()
            .any(|part| part.bytes().any(|byte| !(0x21..=0x7e).contains(&byte)))
    {
        return false;
    }
    if hostname[1..] != pattern[1..] {
        return false;
    }

    let left_pattern = &pattern[0];
    if !left_pattern.contains('*') || left_pattern.contains("xn--") {
        return hostname[0] == *left_pattern;
    }
    if left_pattern.matches('*').count() != 1 || pattern.len() <= 2 {
        return false;
    }

    let (prefix, suffix) = left_pattern
        .split_once('*')
        .expect("one wildcard was checked");
    let label = &hostname[0];
    prefix.len() + suffix.len() <= label.len()
        && label.starts_with(prefix)
        && label.ends_with(suffix)
}

#[cfg(target_os = "macos")]
fn verify_macos_pinned_leaf_name_and_time(der: &[u8], hostname: &str, now: i64) -> Result<()> {
    let (trailing, certificate) = parse_x509_certificate(der)
        .map_err(|error| anyhow!("invalid TLS server certificate: {error}"))?;
    if !trailing.is_empty() {
        bail!("invalid TLS server certificate: trailing data");
    }
    let validity = certificate.validity();
    if now < validity.not_before.timestamp() || now > validity.not_after.timestamp() {
        bail!("TLS server certificate is outside its validity period");
    }

    let mut san_seen = false;
    let mut dns_names = Vec::new();
    let mut ip_addresses = Vec::new();
    let mut key_usage_seen = false;
    let mut key_usage_allows_tls_server = true;
    let mut extended_key_usage_seen = false;
    let mut extended_key_usage_allows_tls_server = true;

    for extension in certificate.extensions() {
        let parsed = extension.parsed_extension();
        if parsed.error().is_some() {
            bail!("malformed TLS server certificate extension");
        }
        if extension.critical
            && matches!(
                parsed,
                ParsedExtension::UnsupportedExtension { .. } | ParsedExtension::Unparsed
            )
        {
            bail!("unsupported critical TLS server certificate extension");
        }

        if extension.oid == OID_X509_EXT_SUBJECT_ALT_NAME {
            if std::mem::replace(&mut san_seen, true) {
                bail!("duplicate TLS server subjectAltName extension");
            }
            let ParsedExtension::SubjectAlternativeName(names) = parsed else {
                bail!("malformed TLS server subjectAltName extension");
            };
            for name in &names.general_names {
                match name {
                    GeneralName::DNSName(name) => dns_names.push(*name),
                    GeneralName::IPAddress(address) if matches!(address.len(), 4 | 16) => {
                        ip_addresses.push(*address);
                    }
                    GeneralName::IPAddress(_) | GeneralName::Invalid(_, _) => {
                        bail!("malformed TLS server subjectAltName entry");
                    }
                    _ => {}
                }
            }
        } else if extension.oid == OID_X509_EXT_KEY_USAGE {
            if std::mem::replace(&mut key_usage_seen, true) {
                bail!("duplicate TLS server keyUsage extension");
            }
            let ParsedExtension::KeyUsage(usage) = parsed else {
                bail!("malformed TLS server keyUsage extension");
            };
            key_usage_allows_tls_server =
                usage.digital_signature() || usage.key_encipherment() || usage.key_agreement();
        } else if extension.oid == OID_X509_EXT_EXTENDED_KEY_USAGE {
            if std::mem::replace(&mut extended_key_usage_seen, true) {
                bail!("duplicate TLS server extendedKeyUsage extension");
            }
            let ParsedExtension::ExtendedKeyUsage(usage) = parsed else {
                bail!("malformed TLS server extendedKeyUsage extension");
            };
            extended_key_usage_allows_tls_server = usage.server_auth || usage.any;
        }
    }

    if !key_usage_allows_tls_server {
        bail!("TLS server certificate keyUsage does not permit server authentication");
    }
    if !extended_key_usage_allows_tls_server {
        bail!("TLS server certificate extendedKeyUsage does not permit server authentication");
    }

    let hostname = hostname.strip_suffix('.').unwrap_or(hostname);
    if let Ok(host_ip) = hostname.parse::<IpAddr>() {
        let matches = ip_addresses.iter().any(|address| match address.len() {
            4 => IpAddr::V4(Ipv4Addr::from(<[u8; 4]>::try_from(*address).unwrap())) == host_ip,
            16 => IpAddr::V6(Ipv6Addr::from(<[u8; 16]>::try_from(*address).unwrap())) == host_ip,
            _ => false,
        });
        if !matches {
            bail!("TLS server certificate IP subjectAltName mismatch");
        }
        return Ok(());
    }

    if !dns_names.is_empty() {
        if !dns_names
            .iter()
            .any(|pattern| macos_tls_dns_name_matches(hostname, pattern))
        {
            bail!("TLS server certificate DNS subjectAltName mismatch");
        }
        return Ok(());
    }

    if !certificate
        .subject()
        .iter_common_name()
        .filter_map(|name| name.as_str().ok())
        .any(|common_name| macos_tls_dns_name_matches(hostname, common_name))
    {
        bail!("TLS server certificate common name mismatch");
    }
    Ok(())
}

#[cfg(target_os = "macos")]
fn macos_unix_timestamp() -> Result<i64> {
    let seconds = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|error| anyhow!("system time is before the Unix epoch: {error}"))?
        .as_secs();
    i64::try_from(seconds).map_err(|_| anyhow!("system time is out of range"))
}

enum Race3<A, B, C> {
    First(A),
    Second(B),
    Third(C),
}
enum Race4<A, B, C, D> {
    First(A),
    Second(B),
    Third(C),
    Fourth(D),
}
async fn select3<F1, F2, F3>(a: F1, b: F2, c: F3) -> Race3<F1::Output, F2::Output, F3::Output>
where
    F1: Future,
    F2: Future,
    F3: Future,
{
    smol::future::race(
        async { Race3::First(a.await) },
        smol::future::race(async { Race3::Second(b.await) }, async {
            Race3::Third(c.await)
        }),
    )
    .await
}
async fn select4<F1, F2, F3, F4>(
    a: F1,
    b: F2,
    c: F3,
    d: F4,
) -> Race4<F1::Output, F2::Output, F3::Output, F4::Output>
where
    F1: Future,
    F2: Future,
    F3: Future,
    F4: Future,
{
    smol::future::race(
        async { Race4::First(a.await) },
        smol::future::race(
            async { Race4::Second(b.await) },
            smol::future::race(async { Race4::Third(c.await) }, async {
                Race4::Fourth(d.await)
            }),
        ),
    )
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

    struct TestCertDir(std::path::PathBuf);

    static NEXT_TLS_CERT_DIR: AtomicUsize = AtomicUsize::new(0);

    impl Drop for TestCertDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn test_tls_material() -> Option<(TestCertDir, String, String, String)> {
        use std::process::Command;
        use std::time::{SystemTime, UNIX_EPOCH};

        if Command::new("openssl").arg("version").output().is_err() {
            return None;
        }
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .ok()?
            .as_nanos();
        let sequence = NEXT_TLS_CERT_DIR.fetch_add(1, Ordering::Relaxed);
        let directory = TestCertDir(std::env::temp_dir().join(format!(
            "niva-socket-tls-{}-{nonce}-{sequence}",
            std::process::id(),
        )));
        std::fs::create_dir(&directory.0).ok()?;
        let run = |args: &[&str]| {
            let output = Command::new("openssl")
                .args(args)
                .current_dir(&directory.0)
                .output()
                .expect("openssl was detected above");
            assert!(
                output.status.success(),
                "openssl {:?} failed: {}",
                args,
                String::from_utf8_lossy(&output.stderr),
            );
        };
        run(&[
            "req",
            "-x509",
            "-newkey",
            "rsa:2048",
            "-nodes",
            "-keyout",
            "ca.key",
            "-out",
            "ca.crt",
            "-days",
            "2",
            "-subj",
            "/CN=NivaSocketTestCA",
            "-addext",
            "basicConstraints=critical,CA:TRUE",
            "-addext",
            "keyUsage=critical,keyCertSign,cRLSign",
        ]);
        run(&[
            "req",
            "-newkey",
            "rsa:2048",
            "-nodes",
            "-keyout",
            "server.key",
            "-out",
            "server.csr",
            "-subj",
            "/CN=localhost",
        ]);
        std::fs::write(
            directory.0.join("server.ext"),
            "basicConstraints=critical,CA:FALSE\nkeyUsage=critical,digitalSignature,keyEncipherment\nextendedKeyUsage=serverAuth\nsubjectAltName=DNS:localhost\n",
        ).ok()?;
        run(&[
            "x509",
            "-req",
            "-in",
            "server.csr",
            "-CA",
            "ca.crt",
            "-CAkey",
            "ca.key",
            "-CAcreateserial",
            "-out",
            "server.crt",
            "-days",
            "2",
            "-extfile",
            "server.ext",
        ]);
        let ca = std::fs::read_to_string(directory.0.join("ca.crt")).ok()?;
        let certificate = std::fs::read_to_string(directory.0.join("server.crt")).ok()?;
        let key = std::fs::read_to_string(directory.0.join("server.key")).ok()?;
        Some((directory, ca, certificate, key))
    }

    #[cfg(target_os = "macos")]
    fn test_pinned_leaf_material(
        subject_alt_name: Option<&str>,
        extended_key_usage: &str,
        key_usage: &str,
        extra_extension: Option<&str>,
    ) -> Option<(TestCertDir, String, String, Vec<u8>)> {
        use std::process::Command;
        use std::time::{SystemTime, UNIX_EPOCH};

        if Command::new("openssl").arg("version").output().is_err() {
            return None;
        }
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .ok()?
            .as_nanos();
        let sequence = NEXT_TLS_CERT_DIR.fetch_add(1, Ordering::Relaxed);
        let directory = TestCertDir(std::env::temp_dir().join(format!(
            "niva-socket-pinned-tls-{}-{nonce}-{sequence}",
            std::process::id(),
        )));
        std::fs::create_dir(&directory.0).ok()?;

        let mut request = [
            "req",
            "-x509",
            "-newkey",
            "rsa:2048",
            "-nodes",
            "-keyout",
            "leaf.rsa",
            "-out",
            "leaf.crt",
            "-days",
            "2",
            "-subj",
            "/CN=localhost",
            "-addext",
            "basicConstraints=critical,CA:TRUE",
        ]
        .into_iter()
        .map(str::to_owned)
        .collect::<Vec<_>>();
        request.extend([
            "-addext".to_owned(),
            format!("keyUsage=critical,{key_usage}"),
            "-addext".to_owned(),
            format!("extendedKeyUsage={extended_key_usage}"),
        ]);
        if let Some(subject_alt_name) = subject_alt_name {
            request.extend([
                "-addext".to_owned(),
                format!("subjectAltName={subject_alt_name}"),
            ]);
        }
        if let Some(extra_extension) = extra_extension {
            request.extend(["-addext".to_owned(), extra_extension.to_owned()]);
        }

        let run = |args: &[&str]| {
            let output = Command::new("openssl")
                .args(args)
                .current_dir(&directory.0)
                .output()
                .expect("openssl was detected above");
            assert!(
                output.status.success(),
                "openssl {:?} failed: {}",
                args,
                String::from_utf8_lossy(&output.stderr),
            );
        };
        let output = Command::new("openssl")
            .args(&request)
            .current_dir(&directory.0)
            .output()
            .expect("openssl was detected above");
        assert!(
            output.status.success(),
            "openssl {:?} failed: {}",
            request,
            String::from_utf8_lossy(&output.stderr),
        );
        run(&[
            "pkcs8", "-topk8", "-nocrypt", "-in", "leaf.rsa", "-out", "leaf.pk8",
        ]);
        run(&[
            "x509", "-in", "leaf.crt", "-outform", "DER", "-out", "leaf.der",
        ]);
        let cert = std::fs::read_to_string(directory.0.join("leaf.crt")).ok()?;
        let key = std::fs::read_to_string(directory.0.join("leaf.pk8")).ok()?;
        let der = std::fs::read(directory.0.join("leaf.der")).ok()?;
        Some((directory, cert, key, der))
    }

    #[cfg(target_os = "macos")]
    fn certificate_validity_midpoint(der: &[u8]) -> i64 {
        let (_, certificate) = x509_parser::parse_x509_certificate(der).unwrap();
        let validity = certificate.validity();
        validity.not_before.timestamp()
            + (validity.not_after.timestamp() - validity.not_before.timestamp()) / 2
    }

    #[test]
    fn read_credit_caps_window_and_charges_empty_udp_datagrams() {
        let (credit, wake) = ReadCredit::new();
        assert_eq!(credit.available.load(Ordering::Acquire), READ_CREDIT_WINDOW);
        credit.consume(READ_CREDIT_WINDOW - 2);
        assert_eq!(credit.available.load(Ordering::Acquire), 2);
        credit.grant(1).unwrap();
        assert_eq!(credit.available.load(Ordering::Acquire), 3);
        assert!(wake.try_recv().is_ok());

        credit.consume_datagram(0);
        assert_eq!(credit.available.load(Ordering::Acquire), 2);
        credit.consume_datagram(10);
        assert_eq!(credit.available.load(Ordering::Acquire), 0);
        credit.grant(usize::MAX).unwrap();
        assert_eq!(credit.available.load(Ordering::Acquire), READ_CREDIT_WINDOW);
        assert!(credit.grant(0).is_err());
    }

    #[test]
    fn closing_listener_revokes_attached_handles_and_wakes_both_pumps() {
        let owner = Owner {
            window_id: 1,
            connection_id: 10,
        };
        let (listener_control, _) = async_channel::bounded(1);
        let (listener_id, listener_closed) = register_active(
            owner,
            SocketKind::TcpListener,
            None,
            Some(listener_control),
            None,
            None,
            None,
        )
        .unwrap();
        assert!(!listener_closed.is_closed());

        let (read_tx, read_rx) = async_channel::bounded(1);
        let (write_tx, write_rx) = async_channel::bounded(1);
        let (child_id, _) = register_active(
            owner,
            SocketKind::Tcp,
            Some(listener_id.clone()),
            Some(read_tx),
            Some(write_tx),
            None,
            None,
        )
        .unwrap();
        assert!(active_handle(&child_id, owner).is_ok());

        remove_active(&listener_id, owner);
        assert!(listener_closed.is_closed());
        assert!(active_handle(&child_id, owner).is_err());
        assert!(matches!(read_rx.try_recv(), Ok(SocketCommand::Close)));
        assert!(matches!(write_rx.try_recv(), Ok(SocketCommand::Close)));
    }

    #[test]
    fn dropping_handler_future_releases_udp_registry_and_port() {
        smol::block_on(async {
            let owner = Owner {
                window_id: 239,
                connection_id: 991_001,
            };
            let sibling = Owner {
                window_id: 239,
                connection_id: 991_002,
            };
            let (sibling_id, _) =
                register_active(sibling, SocketKind::Tcp, None, None, None, None, None).unwrap();
            let sibling_guard = ActiveRegistration::new(&sibling_id, sibling);
            let socket = Arc::new(smol::net::UdpSocket::bind("127.0.0.1:0").await.unwrap());
            let address = socket.local_addr().unwrap();
            let (id, closed) =
                register_active(owner, SocketKind::Udp, None, None, None, Some(socket), None)
                    .unwrap();
            let (started_tx, started_rx) = async_channel::bounded(1);
            let captured_id = id.clone();
            smol::future::or(
                async move {
                    let _registration = ActiveRegistration::new(&captured_id, owner);
                    started_tx.send(()).await.unwrap();
                    std::future::pending::<()>().await;
                },
                async {
                    started_rx.recv().await.unwrap();
                },
            )
            .await;
            assert!(closed.is_closed());
            assert!(active_handle(&id, owner).is_err());
            assert!(active_handle(&sibling_id, sibling).is_ok());
            let rebound =
                std::net::UdpSocket::bind(address).expect("cancelled UDP port remains occupied");
            drop(rebound);
            drop(sibling_guard);
        });
    }

    #[test]
    fn loopback_tcp_stream_uses_existing_smol_primitives() {
        smol::block_on(async {
            let listener =
                Async::<StdTcpListener>::bind(SocketAddr::from(([127, 0, 0, 1], 0))).unwrap();
            let addr = listener.get_ref().local_addr().unwrap();
            let server = smol::spawn(async move {
                let (mut s, _) = listener.accept().await.unwrap();
                let mut b = [0; 4];
                s.read_exact(&mut b).await.unwrap();
                s.write_all(&b).await.unwrap();
            });
            let mut client = Async::<StdTcpStream>::connect(addr).await.unwrap();
            client.write_all(b"niva").await.unwrap();
            let mut reply = [0; 4];
            client.read_exact(&mut reply).await.unwrap();
            assert_eq!(&reply, b"niva");
            server.await;
        });
    }

    #[test]
    fn tls_native_session_close_cancels_the_other_pump() {
        smol::block_on(async {
            let pending_read = std::future::pending::<Result<()>>();
            let closed_write = async { Ok(()) };
            join_socket_pumps_for_kind(SocketKind::Tls, pending_read, closed_write)
                .await
                .unwrap();
        });
    }

    #[test]
    fn tcp_fin_emits_end_and_server_can_reply_before_full_close() {
        smol::block_on(async {
            let listener =
                Async::<StdTcpListener>::bind(SocketAddr::from(([127, 0, 0, 1], 0))).unwrap();
            let address = listener.get_ref().local_addr().unwrap();
            let received = Arc::new(Mutex::new(Vec::new()));
            let end_seen = Arc::new(AtomicBool::new(false));
            let server_received = received.clone();
            let server_end_seen = end_seen.clone();
            let server = smol::spawn(async move {
                let (stream, _) = listener.accept().await.unwrap();
                let write_shutdown = tcp_write_shutdown(&stream).unwrap();
                let (reader, writer) = smol::io::split(stream);
                let (_read_control_tx, read_controls) = async_channel::bounded(8);
                let (_write_control_tx, write_controls) = async_channel::bounded(8);
                let (_cancel_tx, cancelled) = async_channel::bounded(1);
                let (credit, credit_wake) = ReadCredit::new();
                let (reply_tx, replies) = async_channel::bounded(1);

                let received_bytes = server_received.clone();
                let reply_on_end: ChunkSink = Arc::new(move |data, end| {
                    if end {
                        reply_tx
                            .try_send(InboundChunk {
                                seq: 1,
                                data: b"reply".to_vec(),
                                end: true,
                            })
                            .expect("server reply should fit the bounded test channel");
                    } else {
                        received_bytes.lock().unwrap().extend_from_slice(data);
                    }
                });
                let event_end_seen = server_end_seen.clone();
                let events: EventSink = Arc::new(move |name, _data| {
                    if name == "end" {
                        event_end_seen.store(true, Ordering::Release);
                    }
                });
                let read_loop = pump_socket_read(
                    reader,
                    read_controls,
                    credit,
                    credit_wake,
                    cancelled.clone(),
                    "test-server-socket".to_owned(),
                    events,
                    reply_on_end,
                );
                let write_events: EventSink = Arc::new(|_name, _data| {});
                let write_loop = pump_socket_write(
                    writer,
                    write_controls,
                    write_shutdown,
                    replies,
                    cancelled,
                    write_events,
                );
                join_socket_pumps_for_kind(SocketKind::Tcp, read_loop, write_loop)
                    .await
                    .unwrap();
            });

            let client = Async::<StdTcpStream>::connect(address).await.unwrap();
            let write_shutdown = tcp_write_shutdown(&client).unwrap();
            let (reader, writer) = smol::io::split(client);
            let (_read_control_tx, read_controls) = async_channel::bounded(8);
            let (_write_control_tx, write_controls) = async_channel::bounded(8);
            let (_cancel_tx, cancelled) = async_channel::bounded(1);
            let (credit, credit_wake) = ReadCredit::new();
            let (request_tx, requests) = async_channel::bounded(1);
            request_tx
                .try_send(InboundChunk {
                    seq: 1,
                    data: b"request".to_vec(),
                    end: true,
                })
                .unwrap();
            let client_reply = Arc::new(Mutex::new(Vec::new()));
            let client_end_seen = Arc::new(AtomicBool::new(false));
            let end_flag = client_end_seen.clone();
            let client_events: EventSink = Arc::new(move |name, _data| {
                if name == "end" {
                    end_flag.store(true, Ordering::Release);
                }
            });
            let reply_bytes = client_reply.clone();
            let client_chunks: ChunkSink = Arc::new(move |data, _end| {
                reply_bytes.lock().unwrap().extend_from_slice(data);
            });
            let client_read_loop = pump_socket_read(
                reader,
                read_controls,
                credit,
                credit_wake,
                cancelled.clone(),
                "test-client-socket".to_owned(),
                client_events,
                client_chunks,
            );
            let client_write_events: EventSink = Arc::new(|_name, _data| {});
            let client_write_loop = pump_socket_write(
                writer,
                write_controls,
                write_shutdown,
                requests,
                cancelled,
                client_write_events,
            );
            join_socket_pumps_for_kind(SocketKind::Tcp, client_read_loop, client_write_loop)
                .await
                .unwrap();
            server.await;

            assert_eq!(&*received.lock().unwrap(), b"request");
            assert!(end_seen.load(Ordering::Acquire));
            assert_eq!(&*client_reply.lock().unwrap(), b"reply");
            assert!(client_end_seen.load(Ordering::Acquire));
        });
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn pinned_leaf_verification_checks_time_server_usage_and_node_name_rules() {
        let Some((_directory, _cert, _key, der)) =
            test_pinned_leaf_material(None, "serverAuth", "digitalSignature", None)
        else {
            eprintln!("skipping pinned-leaf TLS validation test: openssl CLI is unavailable");
            return;
        };
        let now = certificate_validity_midpoint(&der);
        verify_macos_pinned_leaf_name_and_time(&der, "localhost", now).unwrap();
        assert!(verify_macos_pinned_leaf_name_and_time(&der, "wronghost", now).is_err());

        let (_, certificate) = x509_parser::parse_x509_certificate(&der).unwrap();
        assert!(
            verify_macos_pinned_leaf_name_and_time(
                &der,
                "localhost",
                certificate.validity().not_before.timestamp() - 1,
            )
            .is_err()
        );
        assert!(
            verify_macos_pinned_leaf_name_and_time(
                &der,
                "localhost",
                certificate.validity().not_after.timestamp() + 1,
            )
            .is_err()
        );

        let Some((_directory, _cert, _key, dns_san_der)) = test_pinned_leaf_material(
            Some("DNS:other.example"),
            "serverAuth",
            "digitalSignature",
            None,
        ) else {
            unreachable!("openssl availability was checked above");
        };
        let dns_now = certificate_validity_midpoint(&dns_san_der);
        verify_macos_pinned_leaf_name_and_time(&dns_san_der, "other.example", dns_now).unwrap();
        assert!(
            verify_macos_pinned_leaf_name_and_time(&dns_san_der, "localhost", dns_now).is_err()
        );

        let Some((_directory, _cert, _key, ip_san_der)) =
            test_pinned_leaf_material(Some("IP:127.0.0.1"), "serverAuth", "digitalSignature", None)
        else {
            unreachable!("openssl availability was checked above");
        };
        let ip_now = certificate_validity_midpoint(&ip_san_der);
        verify_macos_pinned_leaf_name_and_time(&ip_san_der, "127.0.0.1", ip_now).unwrap();
        verify_macos_pinned_leaf_name_and_time(&ip_san_der, "localhost", ip_now).unwrap();
        assert!(verify_macos_pinned_leaf_name_and_time(&ip_san_der, "127.0.0.2", ip_now).is_err());

        let Some((_directory, _cert, _key, wildcard_der)) = test_pinned_leaf_material(
            Some("DNS:*.example.test"),
            "serverAuth",
            "digitalSignature",
            None,
        ) else {
            unreachable!("openssl availability was checked above");
        };
        let wildcard_now = certificate_validity_midpoint(&wildcard_der);
        verify_macos_pinned_leaf_name_and_time(&wildcard_der, "api.example.test", wildcard_now)
            .unwrap();
        assert!(
            verify_macos_pinned_leaf_name_and_time(
                &wildcard_der,
                "deep.api.example.test",
                wildcard_now,
            )
            .is_err()
        );
        assert!(macos_tls_dns_name_matches(
            "api.example.test",
            "a*.example.test"
        ));
        assert!(!macos_tls_dns_name_matches(
            "deep.api.example.test",
            "*.example.test"
        ));
        assert!(!macos_tls_dns_name_matches(
            "xn--user.example.test",
            "xn--*.example.test"
        ));

        let Some((_directory, _cert, _key, wrong_eku_der)) =
            test_pinned_leaf_material(None, "clientAuth", "digitalSignature", None)
        else {
            unreachable!("openssl availability was checked above");
        };
        let wrong_eku_now = certificate_validity_midpoint(&wrong_eku_der);
        assert!(
            verify_macos_pinned_leaf_name_and_time(&wrong_eku_der, "localhost", wrong_eku_now)
                .is_err()
        );

        let Some((_directory, _cert, _key, wrong_key_usage_der)) =
            test_pinned_leaf_material(None, "serverAuth", "keyCertSign,cRLSign", None)
        else {
            unreachable!("openssl availability was checked above");
        };
        let wrong_key_usage_now = certificate_validity_midpoint(&wrong_key_usage_der);
        assert!(
            verify_macos_pinned_leaf_name_and_time(
                &wrong_key_usage_der,
                "localhost",
                wrong_key_usage_now,
            )
            .is_err()
        );

        let Some((_directory, _cert, _key, unknown_critical_der)) = test_pinned_leaf_material(
            None,
            "serverAuth",
            "digitalSignature",
            Some("1.2.3.4=critical,DER:01:01:FF"),
        ) else {
            unreachable!("openssl availability was checked above");
        };
        let unknown_critical_now = certificate_validity_midpoint(&unknown_critical_der);
        assert!(
            verify_macos_pinned_leaf_name_and_time(
                &unknown_critical_der,
                "localhost",
                unknown_critical_now,
            )
            .is_err()
        );

        let Some((_directory, _cert, _key, malformed_san_der)) = test_pinned_leaf_material(
            None,
            "serverAuth",
            "digitalSignature",
            Some("subjectAltName=critical,DER:01:01:FF"),
        ) else {
            unreachable!("openssl availability was checked above");
        };
        let malformed_san_now = certificate_validity_midpoint(&malformed_san_der);
        assert!(
            verify_macos_pinned_leaf_name_and_time(
                &malformed_san_der,
                "localhost",
                malformed_san_now,
            )
            .is_err()
        );
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn pinned_leaf_tls_handshake_verifies_before_returning_the_socket() {
        smol::block_on(async {
            let Some((_directory, certificate, key, _der)) = test_pinned_leaf_material(
                None,
                "serverAuth",
                "digitalSignature,keyCertSign,cRLSign",
                None,
            ) else {
                eprintln!("skipping pinned-leaf handshake test: openssl CLI is unavailable");
                return;
            };
            let identity = tls_identity(TlsIdentityInput::Pem {
                key,
                cert: certificate.clone(),
            })
            .unwrap();
            let ca_bundle = format!("{certificate}\n{certificate}");
            assert_eq!(
                macos_ca_certificates(std::slice::from_ref(&ca_bundle))
                    .unwrap()
                    .len(),
                2
            );
            let acceptor = Arc::new(TlsAcceptor::new(identity).unwrap());
            let listener =
                Async::<StdTcpListener>::bind(SocketAddr::from(([127, 0, 0, 1], 0))).unwrap();
            let address = listener.get_ref().local_addr().unwrap();

            let server = smol::spawn(async move {
                for index in 0..2 {
                    let (stream, _) = listener.accept().await.unwrap();
                    let result = tls_server_handshake_inner(acceptor.clone(), stream).await;
                    if index == 0 {
                        let mut tls =
                            result.expect("valid TLS client should complete verification");
                        let mut request = [0u8; 4];
                        tls.read_exact(&mut request).await.unwrap();
                        assert_eq!(&request, b"ping");
                        tls.write_all(b"pong").await.unwrap();
                        tls.flush().await.unwrap();
                    } else {
                        assert!(result.is_err(), "wrong-host client must not complete TLS");
                    }
                }
            });

            let stream = Async::<StdTcpStream>::connect(address).await.unwrap();
            let mut tls = tls_client_handshake_explicit_ca_inner(
                "localhost".to_owned(),
                stream,
                std::slice::from_ref(&ca_bundle),
            )
            .await
            .unwrap();
            tls.write_all(b"ping").await.unwrap();
            tls.flush().await.unwrap();
            let mut response = [0u8; 4];
            tls.read_exact(&mut response).await.unwrap();
            assert_eq!(&response, b"pong");
            drop(tls);

            let stream = Async::<StdTcpStream>::connect(address).await.unwrap();
            let wrong_host = tls_client_handshake_explicit_ca_inner(
                "wronghost".to_owned(),
                stream,
                std::slice::from_ref(&ca_bundle),
            )
            .await;
            assert!(
                wrong_host.is_err(),
                "hostname mismatch must fail before returning a TLS socket"
            );
            server.await;
        });
    }

    #[test]
    fn tls_loopback_uses_default_validation_for_explicit_ca_and_hostname() {
        smol::block_on(async {
            let Some((_directory, ca, certificate, key)) = test_tls_material() else {
                eprintln!("skipping TLS validation test: openssl CLI is unavailable");
                return;
            };
            let identity = tls_identity(TlsIdentityInput::Pem {
                key,
                cert: certificate,
            })
            .unwrap();
            let acceptor = Arc::new(TlsAcceptor::new(identity).unwrap());
            #[cfg(not(target_os = "macos"))]
            let make_connector = || tls_connector(Some(&[ca.clone()])).unwrap();
            let listener =
                Async::<StdTcpListener>::bind(SocketAddr::from(([127, 0, 0, 1], 0))).unwrap();
            let address = listener.get_ref().local_addr().unwrap();

            let server = smol::spawn(async move {
                let (stream, _) = listener.accept().await.unwrap();
                let mut tls = tls_server_handshake_inner(acceptor.clone(), stream)
                    .await
                    .unwrap();
                let mut request = [0u8; 4];
                tls.read_exact(&mut request).await.unwrap();
                tls.write_all(&request).await.unwrap();
                tls.flush().await.unwrap();
                drop(tls);
                let (stream, _) = listener.accept().await.unwrap();
                let _ = tls_server_handshake_inner(acceptor, stream).await;
            });

            let stream = Async::<StdTcpStream>::connect(address).await.unwrap();
            #[cfg(target_os = "macos")]
            let mut tls = tls_client_handshake_explicit_ca_inner(
                "localhost".to_owned(),
                stream,
                &[ca.clone()],
            )
            .await
            .unwrap();
            #[cfg(not(target_os = "macos"))]
            let mut tls =
                tls_client_handshake_inner(make_connector(), "localhost".to_owned(), stream)
                    .await
                    .unwrap();
            tls.write_all(b"niva").await.unwrap();
            tls.flush().await.unwrap();
            let mut reply = [0u8; 4];
            tls.read_exact(&mut reply).await.unwrap();
            assert_eq!(&reply, b"niva");
            drop(tls);

            let stream = Async::<StdTcpStream>::connect(address).await.unwrap();
            #[cfg(target_os = "macos")]
            let wrong_host = tls_client_handshake_explicit_ca_inner(
                "wronghost".to_owned(),
                stream,
                &[ca.clone()],
            )
            .await;
            #[cfg(not(target_os = "macos"))]
            let wrong_host =
                tls_client_handshake_inner(make_connector(), "wronghost".to_owned(), stream).await;
            assert!(wrong_host.is_err(), "TLS must reject a hostname mismatch");
            server.await;
        });
    }

    #[test]
    fn tls_connector_accepts_a_bundle_of_ca_certificates() {
        let Some((_directory, ca, _certificate, _key)) = test_tls_material() else {
            eprintln!("skipping TLS CA bundle test: openssl CLI is unavailable");
            return;
        };
        let bundle = format!("{ca}\n{ca}");
        assert!(tls_connector(Some(&[bundle])).is_ok());
    }

    #[test]
    fn loopback_udp_preserves_datagram_and_peer() {
        smol::block_on(async {
            let recv = Arc::new(
                smol::net::UdpSocket::bind(SocketAddr::from(([127, 0, 0, 1], 0)))
                    .await
                    .unwrap(),
            );
            let send = smol::net::UdpSocket::bind(SocketAddr::from(([127, 0, 0, 1], 0)))
                .await
                .unwrap();
            let target = recv.local_addr().unwrap();
            send.send_to(b"one datagram", target).await.unwrap();
            let mut buf = [0u8; 64];
            let (n, peer) = recv.recv_from(&mut buf).await.unwrap();
            assert_eq!(&buf[..n], b"one datagram");
            assert_eq!(peer, send.local_addr().unwrap());
        });
    }
}
