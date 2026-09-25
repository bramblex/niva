use std::{
    collections::BTreeMap,
    io::{self, Read, Write},
    net::{Shutdown, SocketAddr, TcpStream},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};

use anyhow::{Result, anyhow, bail};
use native_tls::{HandshakeError, TlsConnector as SystemTlsConnector};
use serde::Deserialize;
use serde_json::{Map, Value, json};
use ureq::{
    Agent, Error as UreqError,
    http::{self, HeaderName, HeaderValue, Method},
    tls::{RootCerts, TlsConfig, TlsProvider},
    unversioned::{
        resolver::DefaultResolver,
        transport::{
            Buffers, ConnectionDetails, Connector, Either, LazyBuffers, NextTimeout, Transport,
            TransportAdapter,
        },
    },
};

use crate::app::api_manager::{ApiManager, ApiRequest, CancellationContext};

const MAX_REQUEST_BODY_BYTES: usize = 256 * 1024;
const DEFAULT_RESPONSE_BODY_BYTES: usize = 1024 * 1024;
const MAX_RESPONSE_BODY_BYTES: usize = 1024 * 1024;
const MAX_RESPONSE_HEADER_BYTES: usize = 16 * 1024;
const MAX_RESPONSE_HEADER_COUNT: usize = 64;
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(10);
const MAX_TIMEOUT: Duration = Duration::from_secs(30);
const MAX_CONNECT_TIMEOUT: Duration = Duration::from_secs(3);

pub fn register_apis(api_manager: &mut ApiManager) {
    api_manager.register_cancellable_api("http.requestText", request_text);
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct RequestOptions {
    url: String,
    method: Option<String>,
    headers: Option<BTreeMap<String, String>>,
    body: Option<String>,
    timeout: Option<u64>,
    max_response_bytes: Option<usize>,
}

async fn request_text(
    ctx: CancellationContext,
    _app: Arc<crate::app::NivaApp>,
    _window: Arc<crate::app::window_manager::window::NivaWindow>,
    request: ApiRequest,
) -> Result<Value> {
    let (options,): (RequestOptions,) = request.args().get()?;
    let mut parsed_url = url::Url::parse(&options.url)?;
    anyhow::ensure!(
        matches!(parsed_url.scheme(), "http" | "https")
            && parsed_url.host_str().is_some()
            && parsed_url.username().is_empty()
            && parsed_url.password().is_none(),
        "EINVAL: http.requestText requires an absolute HTTP(S) URL without credentials"
    );
    parsed_url.set_fragment(None);
    let body = options.body.unwrap_or_default();
    anyhow::ensure!(
        body.len() <= MAX_REQUEST_BODY_BYTES,
        "EFBIG: HTTP request body exceeds 256 KiB"
    );
    let method = parse_method(options.method.as_deref())?;
    let timeout = options
        .timeout
        .map(Duration::from_millis)
        .unwrap_or(DEFAULT_TIMEOUT);
    anyhow::ensure!(
        !timeout.is_zero() && timeout <= MAX_TIMEOUT,
        "EINVAL: HTTP timeout must be between 1 and 30000 ms"
    );
    let max_response_bytes = options
        .max_response_bytes
        .unwrap_or(DEFAULT_RESPONSE_BODY_BYTES);
    anyhow::ensure!(
        max_response_bytes > 0 && max_response_bytes <= MAX_RESPONSE_BODY_BYTES,
        "EINVAL: HTTP response limit must be between 1 and 1048576 bytes"
    );
    let headers = validate_request_headers(options.headers.unwrap_or_default())?;

    let socket = Arc::new(SocketControl::default());
    let _socket_guard = SocketCancellationGuard(socket.clone());
    let worker_socket = socket.clone();
    let cancel_socket = socket.clone();
    let cancel_ctx = ctx.clone();
    let work = crate::blocking!({
        execute_request(
            parsed_url.as_str(),
            method,
            headers,
            body,
            timeout,
            max_response_bytes,
            worker_socket,
        )
    });
    let result = smol::future::or(work, async move {
        cancel_ctx.cancelled().await;
        cancel_socket.cancel();
        Err(anyhow!(
            "ECANCELED: HTTP request cancelled with its IPC session"
        ))
    })
    .await?;
    anyhow::ensure!(
        !ctx.is_cancelled(),
        "ECANCELED: HTTP request cancelled with its owner"
    );
    socket.clear();
    Ok(result)
}

fn parse_method(method: Option<&str>) -> Result<Method> {
    let method = method.unwrap_or("GET").to_ascii_uppercase();
    anyhow::ensure!(
        matches!(
            method.as_str(),
            "GET" | "HEAD" | "POST" | "PUT" | "PATCH" | "DELETE" | "OPTIONS"
        ),
        "EINVAL: unsupported HTTP method"
    );
    Method::from_bytes(method.as_bytes()).map_err(|error| anyhow!("EINVAL: {error}"))
}

fn validate_request_headers(
    headers: BTreeMap<String, String>,
) -> Result<Vec<(HeaderName, HeaderValue)>> {
    anyhow::ensure!(
        headers.len() <= MAX_RESPONSE_HEADER_COUNT,
        "E2BIG: too many HTTP headers"
    );
    let mut total_bytes = 0usize;
    let mut validated = Vec::with_capacity(headers.len());
    for (name, value) in headers {
        total_bytes = total_bytes
            .saturating_add(name.len())
            .saturating_add(value.len());
        anyhow::ensure!(
            total_bytes <= MAX_RESPONSE_HEADER_BYTES,
            "E2BIG: HTTP request headers exceed 16 KiB"
        );
        anyhow::ensure!(
            !matches!(
                name.to_ascii_lowercase().as_str(),
                "host"
                    | "content-length"
                    | "transfer-encoding"
                    | "connection"
                    | "proxy-authorization"
            ),
            "EINVAL: HTTP transport header is controlled by Niva"
        );
        validated.push((
            HeaderName::from_bytes(name.as_bytes())
                .map_err(|error| anyhow!("EINVAL: invalid HTTP header name: {error}"))?,
            HeaderValue::from_str(&value)
                .map_err(|error| anyhow!("EINVAL: invalid HTTP header value: {error}"))?,
        ));
    }
    Ok(validated)
}

fn execute_request(
    url: &str,
    method: Method,
    headers: Vec<(HeaderName, HeaderValue)>,
    body: String,
    timeout: Duration,
    max_response_bytes: usize,
    socket: Arc<SocketControl>,
) -> Result<Value> {
    if socket.is_cancelled() {
        bail!("ECANCELED: HTTP request cancelled with its IPC session");
    }
    let connect_timeout = timeout.min(MAX_CONNECT_TIMEOUT);
    let tls = TlsConfig::builder()
        .provider(TlsProvider::NativeTls)
        .root_certs(RootCerts::PlatformVerifier)
        .build();
    let config = Agent::config_builder()
        .tls_config(tls)
        .proxy(None)
        .http_status_as_error(false)
        .max_redirects(0)
        .max_response_header_size(MAX_RESPONSE_HEADER_BYTES)
        .timeout_global(Some(timeout))
        .timeout_resolve(Some(connect_timeout))
        .timeout_connect(Some(connect_timeout))
        .timeout_send_request(Some(connect_timeout))
        .timeout_send_body(Some(connect_timeout))
        .timeout_recv_response(Some(connect_timeout))
        .timeout_recv_body(Some(timeout))
        .build();
    let connector = ()
        .chain(AbortableTcpConnector {
            socket: socket.clone(),
        })
        .chain(PlatformTlsConnector);
    let agent = Agent::with_parts(config, connector, DefaultResolver::default());
    let uri = url.parse::<http::Uri>()?;
    let mut request = http::Request::builder().method(method).uri(uri);
    for (name, value) in headers {
        request = request.header(name, value);
    }
    // Do not delegate this one-shot operation to environment proxy settings,
    // and avoid a persistent pooled connection after the bounded text read.
    request = request.header(http::header::CONNECTION, "close");
    let request = request.body(body)?;
    let mut response = agent.run(request).map_err(map_ureq_error)?;

    let mut result_headers = Map::new();
    let mut header_bytes = 0usize;
    let mut header_count = 0usize;
    for (name, value) in response.headers().iter() {
        header_count += 1;
        header_bytes = header_bytes
            .saturating_add(name.as_str().len())
            .saturating_add(value.as_bytes().len());
        anyhow::ensure!(
            header_count <= MAX_RESPONSE_HEADER_COUNT && header_bytes <= MAX_RESPONSE_HEADER_BYTES,
            "E2BIG: HTTP response headers exceed their limit"
        );
        let value = value
            .to_str()
            .map_err(|_| anyhow!("ERR_INVALID_ENCODING: HTTP header is not ASCII"))?;
        let key = name.as_str().to_ascii_lowercase();
        if let Some(previous) = result_headers.get_mut(&key) {
            if key == "set-cookie" {
                if let Value::Array(values) = previous {
                    values.push(json!(value));
                } else {
                    let first = std::mem::replace(previous, Value::Null);
                    *previous = json!([first, value]);
                }
            } else if let Some(previous_text) = previous.as_str().map(str::to_owned) {
                *previous = json!(format!("{previous_text}, {value}"));
            }
        } else {
            result_headers.insert(key, json!(value));
        }
    }
    let status = response.status();
    let status_code = status.as_u16();
    let status_message = status.canonical_reason().unwrap_or("").to_owned();
    let response_bytes = response
        .body_mut()
        .with_config()
        .limit(max_response_bytes as u64)
        .read_to_vec()
        .map_err(map_ureq_error)?;
    let text = String::from_utf8(response_bytes)
        .map_err(|_| anyhow!("ERR_INVALID_ENCODING: HTTP response body is not valid UTF-8"))?;
    Ok(json!({
        "statusCode": status_code,
        "statusMessage": status_message,
        "headers": result_headers,
        "body": text,
    }))
}

fn map_ureq_error(error: UreqError) -> anyhow::Error {
    match error {
        UreqError::Timeout(_) => anyhow!("ETIMEDOUT: HTTP request timed out"),
        UreqError::HostNotFound => anyhow!("EHOSTUNREACH: HTTP host could not be resolved"),
        UreqError::BodyExceedsLimit(_) => {
            anyhow!("EFBIG: HTTP response body exceeds its byte limit")
        }
        UreqError::Io(error) if error.kind() == io::ErrorKind::TimedOut => {
            anyhow!("ETIMEDOUT: HTTP connection timed out")
        }
        UreqError::Io(error) if error.kind() == io::ErrorKind::Interrupted => {
            anyhow!("ECANCELED: HTTP request was cancelled")
        }
        other => anyhow!("HTTP request failed: {other}"),
    }
}

#[derive(Debug, Default)]
struct SocketControl {
    cancelled: AtomicBool,
    socket: Mutex<Option<TcpStream>>,
}

impl SocketControl {
    fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::Acquire)
    }

    fn register(&self, stream: &TcpStream) -> io::Result<()> {
        let clone = stream.try_clone()?;
        let mut socket = self
            .socket
            .lock()
            .map_err(|_| io::Error::other("HTTP socket registry poisoned"))?;
        if self.is_cancelled() {
            let _ = clone.shutdown(Shutdown::Both);
            return Err(io::Error::new(
                io::ErrorKind::Interrupted,
                "HTTP request cancelled",
            ));
        }
        *socket = Some(clone);
        Ok(())
    }

    fn cancel(&self) {
        self.cancelled.store(true, Ordering::Release);
        let socket = self
            .socket
            .lock()
            .ok()
            .and_then(|socket| socket.as_ref().and_then(|socket| socket.try_clone().ok()));
        if let Some(socket) = socket {
            let _ = socket.shutdown(Shutdown::Both);
        }
    }

    fn clear(&self) {
        if let Ok(mut socket) = self.socket.lock() {
            socket.take();
        }
    }
}

/// Cancellation still reaches the blocking transport if the async handler is
/// dropped by its dispatcher, timeout, or IPC lease monitor.
struct SocketCancellationGuard(Arc<SocketControl>);

impl Drop for SocketCancellationGuard {
    fn drop(&mut self) {
        self.0.cancel();
    }
}

#[derive(Debug)]
struct AbortableTcpConnector {
    socket: Arc<SocketControl>,
}

impl<In: Transport> Connector<In> for AbortableTcpConnector {
    type Out = Either<In, AbortableTcpTransport>;

    fn connect(
        &self,
        details: &ConnectionDetails,
        chained: Option<In>,
    ) -> std::result::Result<Option<Self::Out>, UreqError> {
        if let Some(chained) = chained {
            return Ok(Some(Either::A(chained)));
        }
        if self.socket.is_cancelled() {
            return Err(
                io::Error::new(io::ErrorKind::Interrupted, "HTTP request cancelled").into(),
            );
        }
        let start = Instant::now();
        let total_timeout = details
            .timeout
            .not_zero()
            .map(|duration| *duration)
            .unwrap_or(MAX_CONNECT_TIMEOUT);
        let mut last_error = None;
        for address in &details.addrs {
            if self.socket.is_cancelled() {
                return Err(
                    io::Error::new(io::ErrorKind::Interrupted, "HTTP request cancelled").into(),
                );
            }
            let remaining = total_timeout.saturating_sub(start.elapsed());
            if remaining.is_zero() {
                return Err(UreqError::Timeout(details.timeout.reason));
            }
            match connect_cancellable(*address, remaining, &self.socket) {
                Ok(stream) => {
                    stream.set_nodelay(true)?;
                    self.socket.register(&stream)?;
                    let buffers = LazyBuffers::new(
                        details.config.input_buffer_size(),
                        details.config.output_buffer_size(),
                    );
                    return Ok(Some(Either::B(AbortableTcpTransport {
                        stream,
                        buffers,
                        socket: self.socket.clone(),
                        previous_read_timeout: None,
                        previous_write_timeout: None,
                    })));
                }
                Err(error)
                    if error.kind() == io::ErrorKind::Interrupted && self.socket.is_cancelled() =>
                {
                    return Err(error.into());
                }
                Err(error) => last_error = Some(error),
            }
        }
        Err(last_error
            .unwrap_or_else(|| {
                io::Error::new(io::ErrorKind::NotFound, "HTTP host has no addresses")
            })
            .into())
    }
}

/// Connect using the project's async socket driver while ureq calls its
/// synchronous Connector hook. Dropping the pending connect future closes its
/// in-progress socket immediately when the owner cancels; DNS resolution is
/// completed by ureq before this hook and remains bounded by its resolver
/// timeout.
fn connect_cancellable(
    address: SocketAddr,
    timeout: Duration,
    socket: &SocketControl,
) -> io::Result<TcpStream> {
    smol::block_on(async {
        let connect = async {
            let stream = smol::net::TcpStream::connect(address).await?;
            let shared: Arc<smol::Async<TcpStream>> = stream.into();
            let cloned = shared.as_ref().get_ref().try_clone()?;
            cloned.set_nonblocking(false)?;
            Ok(cloned)
        };
        let cancelled = async {
            loop {
                if socket.is_cancelled() {
                    return Err(io::Error::new(
                        io::ErrorKind::Interrupted,
                        "HTTP request cancelled",
                    ));
                }
                smol::Timer::after(Duration::from_millis(20)).await;
            }
        };
        let timed_out = async {
            smol::Timer::after(timeout).await;
            Err(io::Error::new(
                io::ErrorKind::TimedOut,
                "HTTP connection timed out",
            ))
        };
        smol::future::or(connect, smol::future::or(cancelled, timed_out)).await
    })
}

struct AbortableTcpTransport {
    stream: TcpStream,
    buffers: LazyBuffers,
    socket: Arc<SocketControl>,
    previous_read_timeout: Option<Duration>,
    previous_write_timeout: Option<Duration>,
}

impl std::fmt::Debug for AbortableTcpTransport {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.debug_struct("AbortableTcpTransport").finish()
    }
}

impl Transport for AbortableTcpTransport {
    fn buffers(&mut self) -> &mut dyn Buffers {
        &mut self.buffers
    }

    fn transmit_output(
        &mut self,
        amount: usize,
        timeout: NextTimeout,
    ) -> std::result::Result<(), UreqError> {
        update_socket_timeout(
            &self.stream,
            timeout,
            &mut self.previous_write_timeout,
            TcpStream::set_write_timeout,
        )?;
        let output = self.buffers.output()[..amount].to_vec();
        match self.stream.write_all(&output) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == io::ErrorKind::TimedOut => {
                Err(UreqError::Timeout(timeout.reason))
            }
            Err(error) => Err(error.into()),
        }
    }

    fn await_input(&mut self, timeout: NextTimeout) -> std::result::Result<bool, UreqError> {
        update_socket_timeout(
            &self.stream,
            timeout,
            &mut self.previous_read_timeout,
            TcpStream::set_read_timeout,
        )?;
        let result = {
            let target = self.buffers.input_append_buf();
            self.stream.read(target)
        };
        let amount = match result {
            Ok(amount) => amount,
            Err(error) if error.kind() == io::ErrorKind::TimedOut => {
                return Err(UreqError::Timeout(timeout.reason));
            }
            Err(error) => return Err(error.into()),
        };
        self.buffers.input_appended(amount);
        Ok(amount > 0)
    }

    fn is_open(&mut self) -> bool {
        !self.socket.is_cancelled() && self.stream.peer_addr().is_ok()
    }
}

/// Adds platform-verified TLS to ureq's existing cancellable transport.
///
/// ureq's `native-tls-no-default` feature keeps its HTTP implementation while
/// omitting its embedded Mozilla root bundle and built-in TLS connector. The
/// HTTPS endpoint still gets normal SNI and hostname verification from
/// `native-tls`; its default trust roots are the host platform's roots.
#[derive(Debug, Default)]
struct PlatformTlsConnector;

impl<In: Transport> Connector<In> for PlatformTlsConnector {
    type Out = Either<In, PlatformTlsTransport>;

    fn connect(
        &self,
        details: &ConnectionDetails,
        chained: Option<In>,
    ) -> std::result::Result<Option<Self::Out>, UreqError> {
        let Some(transport) = chained else {
            panic!("PlatformTlsConnector requires a chained transport");
        };
        let tls_config = details.config.tls_config();
        if !details.needs_tls()
            || transport.is_tls()
            || tls_config.provider() != TlsProvider::NativeTls
        {
            return Ok(Some(Either::A(transport)));
        }

        // This client is intentionally pinned to platform trust. Do not silently
        // reinterpret a disabled or alternate-root config as system verification.
        if tls_config.disable_verification()
            || !matches!(tls_config.root_certs(), RootCerts::PlatformVerifier)
        {
            return Err(UreqError::Tls(
                "Niva HTTPS requires verified platform root certificates",
            ));
        }

        let authority = details
            .uri
            .authority()
            .expect("HTTPS URI must have authority");
        let authority_host = authority.host();
        let domain = authority_host
            .strip_prefix('[')
            .and_then(|host| host.strip_suffix(']'))
            .unwrap_or(authority_host);

        let mut builder = SystemTlsConnector::builder();
        builder
            .disable_built_in_roots(false)
            .use_sni(tls_config.use_sni());
        let connector = builder
            .build()
            .map_err(|error| UreqError::Other(Box::new(error)))?;

        // The adapter applies ureq's current connect/handshake deadline to every
        // underlying read and write. Preserve the original transport error so
        // timeouts and cancellation aren't collapsed into a generic TLS error.
        let mut adapter = TlsIoErrorCapture::wrap(TransportAdapter::new(transport.boxed()));
        adapter.get_mut().set_timeout(details.timeout);
        let captured = adapter.captured_errors();
        let stream = match connector.connect(domain, adapter) {
            Ok(stream) => stream,
            Err(HandshakeError::Failure(error)) => {
                if let Some(transport_error) = take_captured_tls_error(&captured)? {
                    return Err(transport_error);
                }
                return Err(UreqError::Other(Box::new(error)));
            }
            Err(HandshakeError::WouldBlock(_)) => {
                unreachable!("Niva's TLS transport is blocking")
            }
        };

        let buffers = LazyBuffers::new(
            details.config.input_buffer_size(),
            details.config.output_buffer_size(),
        );
        Ok(Some(Either::B(PlatformTlsTransport { buffers, stream })))
    }
}

struct TlsIoErrorCapture<S> {
    stream: S,
    errors: Arc<Mutex<Option<UreqError>>>,
}

impl<S> TlsIoErrorCapture<S> {
    fn wrap(stream: S) -> Self {
        Self {
            stream,
            errors: Arc::new(Mutex::new(None)),
        }
    }

    fn get_mut(&mut self) -> &mut S {
        &mut self.stream
    }

    fn captured_errors(&self) -> Arc<Mutex<Option<UreqError>>> {
        self.errors.clone()
    }
}

impl<S: Read> Read for TlsIoErrorCapture<S> {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        self.stream
            .read(buffer)
            .map_err(|error| capture_tls_io_error(error, &self.errors))
    }
}

impl<S: Write> Write for TlsIoErrorCapture<S> {
    fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
        self.stream
            .write(buffer)
            .map_err(|error| capture_tls_io_error(error, &self.errors))
    }

    fn flush(&mut self) -> io::Result<()> {
        self.stream
            .flush()
            .map_err(|error| capture_tls_io_error(error, &self.errors))
    }
}

fn capture_tls_io_error(error: io::Error, captured: &Mutex<Option<UreqError>>) -> io::Error {
    let error = UreqError::from(error);
    if let Ok(mut captured) = captured.lock() {
        if captured.is_none() {
            *captured = Some(error);
        }
    }
    io::Error::other("underlying HTTP transport error was captured")
}

fn take_captured_tls_error(
    captured: &Mutex<Option<UreqError>>,
) -> std::result::Result<Option<UreqError>, UreqError> {
    let mut captured = captured.lock().map_err(|_| {
        UreqError::Other(Box::new(io::Error::other("TLS error capture was poisoned")))
    })?;
    Ok(captured.take())
}

struct PlatformTlsTransport {
    buffers: LazyBuffers,
    stream: native_tls::TlsStream<TlsIoErrorCapture<TransportAdapter>>,
}

impl std::fmt::Debug for PlatformTlsTransport {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.debug_struct("PlatformTlsTransport").finish()
    }
}

impl Transport for PlatformTlsTransport {
    fn buffers(&mut self) -> &mut dyn Buffers {
        &mut self.buffers
    }

    fn transmit_output(
        &mut self,
        amount: usize,
        timeout: NextTimeout,
    ) -> std::result::Result<(), UreqError> {
        let stream = &mut self.stream;
        stream.get_mut().get_mut().set_timeout(timeout);
        let output = self.buffers.output()[..amount].to_vec();
        let result = stream.write_all(&output);
        take_captured_tls_error(&stream.get_mut().errors)?.map_or(Ok(()), Err)?;
        result?;
        Ok(())
    }

    fn await_input(&mut self, timeout: NextTimeout) -> std::result::Result<bool, UreqError> {
        let stream = &mut self.stream;
        stream.get_mut().get_mut().set_timeout(timeout);
        let input = self.buffers.input_append_buf();
        let result = stream.read(input);
        let amount = match result {
            Ok(amount) => {
                if amount == 0 {
                    take_captured_tls_error(&stream.get_mut().errors)?.map_or(Ok(()), Err)?;
                }
                amount
            }
            Err(error) => {
                take_captured_tls_error(&stream.get_mut().errors)?.map_or(Ok(()), Err)?;
                return Err(error.into());
            }
        };
        self.buffers.input_appended(amount);
        Ok(amount > 0)
    }

    fn is_open(&mut self) -> bool {
        self.stream.get_mut().get_mut().get_mut().is_open()
    }

    fn is_tls(&self) -> bool {
        true
    }
}

fn update_socket_timeout(
    stream: &TcpStream,
    timeout: NextTimeout,
    previous: &mut Option<Duration>,
    set_timeout: impl FnOnce(&TcpStream, Option<Duration>) -> io::Result<()>,
) -> std::result::Result<(), UreqError> {
    let current = timeout.not_zero().map(|duration| *duration);
    if current != *previous {
        set_timeout(stream, current)?;
        *previous = current;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        net::TcpListener,
        process::Command,
        sync::atomic::AtomicUsize,
        time::{SystemTime, UNIX_EPOCH},
    };

    static NEXT_TEST_CERT_DIR: AtomicUsize = AtomicUsize::new(0);

    struct TestTlsMaterial(std::path::PathBuf);

    impl Drop for TestTlsMaterial {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn untrusted_tls_server() -> Option<(String, std::thread::JoinHandle<()>, TestTlsMaterial)> {
        if Command::new("openssl").arg("version").output().is_err() {
            return None;
        }
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .ok()?
            .as_nanos();
        let sequence = NEXT_TEST_CERT_DIR.fetch_add(1, Ordering::Relaxed);
        let material = TestTlsMaterial(std::env::temp_dir().join(format!(
            "niva-http-test-tls-{}-{nonce}-{sequence}",
            std::process::id(),
        )));
        std::fs::create_dir(&material.0).ok()?;
        let generated = Command::new("openssl")
            .args([
                "req",
                "-x509",
                "-newkey",
                "rsa:2048",
                "-nodes",
                "-keyout",
                "key.pem",
                "-out",
                "cert.pem",
                "-days",
                "2",
                "-subj",
                "/CN=localhost",
                "-addext",
                "subjectAltName=DNS:localhost",
            ])
            .current_dir(&material.0)
            .output()
            .ok()?;
        if !generated.status.success() {
            return None;
        }
        let converted = Command::new("openssl")
            .args(["pkey", "-in", "key.pem", "-out", "key-pkcs8.pem"])
            .current_dir(&material.0)
            .output()
            .ok()?;
        if !converted.status.success() {
            return None;
        }
        let certificate = std::fs::read(material.0.join("cert.pem")).ok()?;
        let key = std::fs::read(material.0.join("key-pkcs8.pem")).ok()?;
        let identity = native_tls::Identity::from_pkcs8(&certificate, &key).ok()?;
        let acceptor = native_tls::TlsAcceptor::new(identity).ok()?;
        let listener = TcpListener::bind(("127.0.0.1", 0)).ok()?;
        let address = listener.local_addr().ok()?;
        let server = std::thread::spawn(move || {
            let (stream, _) = listener.accept().unwrap();
            assert!(acceptor.accept(stream).is_err());
        });
        Some((
            format!("https://localhost:{}/resource", address.port()),
            server,
            material,
        ))
    }

    fn stalled_tls_server() -> Option<(
        String,
        std::thread::JoinHandle<()>,
        async_channel::Receiver<()>,
    )> {
        let listener = TcpListener::bind(("127.0.0.1", 0)).ok()?;
        let address = listener.local_addr().ok()?;
        let (accepted_tx, accepted_rx) = async_channel::bounded(1);
        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let _ = accepted_tx.send_blocking(());
            let mut bytes = Vec::new();
            let _ = stream.read_to_end(&mut bytes);
        });
        Some((
            format!("https://127.0.0.1:{}/resource", address.port()),
            server,
            accepted_rx,
        ))
    }

    fn serve_once(response: &'static [u8]) -> (String, std::thread::JoinHandle<()>) {
        let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let address = listener.local_addr().unwrap();
        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(2)))
                .unwrap();
            let mut request = [0u8; 4096];
            let _ = stream.read(&mut request);
            stream.write_all(response).unwrap();
            let _ = stream.flush();
        });
        (format!("http://{address}/resource"), server)
    }

    fn run(url: &str, max_response_bytes: usize) -> Result<Value> {
        execute_request(
            url,
            parse_method(Some("get"))?,
            Vec::new(),
            String::new(),
            Duration::from_secs(2),
            max_response_bytes,
            Arc::new(SocketControl::default()),
        )
    }

    #[test]
    fn request_text_preserves_404_text_and_normalizes_method_case() {
        let (url, server) = serve_once(
            b"HTTP/1.1 404 Not Found\r\nContent-Length: 7\r\nConnection: close\r\n\r\nmissing",
        );
        let result = run(&url, 32).unwrap();
        server.join().unwrap();
        assert_eq!(result["statusCode"], 404);
        assert_eq!(result["body"], "missing");
        assert_eq!(parse_method(Some("get")).unwrap(), Method::GET);
    }

    #[test]
    fn request_text_rejects_responses_over_the_body_limit() {
        let (url, server) =
            serve_once(b"HTTP/1.1 200 OK\r\nContent-Length: 5\r\nConnection: close\r\n\r\n12345");
        let error = run(&url, 4).unwrap_err();
        server.join().unwrap();
        assert!(error.to_string().contains("EFBIG"));
    }

    #[test]
    fn request_text_rejects_invalid_utf8_response_bodies() {
        let (url, server) =
            serve_once(b"HTTP/1.1 200 OK\r\nContent-Length: 1\r\nConnection: close\r\n\r\n\xff");
        let error = run(&url, 8).unwrap_err();
        server.join().unwrap();
        assert!(error.to_string().contains("ERR_INVALID_ENCODING"));
    }

    #[test]
    fn request_text_cancellation_closes_an_active_socket() {
        let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let address = listener.local_addr().unwrap();
        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut request = [0u8; 4096];
            let _ = stream.read(&mut request);
            std::thread::sleep(Duration::from_secs(2));
        });
        let socket = Arc::new(SocketControl::default());
        let worker_socket = socket.clone();
        let started = Instant::now();
        let worker = std::thread::spawn(move || {
            execute_request(
                &format!("http://{address}/slow"),
                Method::GET,
                Vec::new(),
                String::new(),
                Duration::from_secs(10),
                32,
                worker_socket,
            )
        });
        std::thread::sleep(Duration::from_millis(100));
        socket.cancel();
        let result = worker.join().unwrap();
        let elapsed = started.elapsed();
        let _ = server.join();
        assert!(result.is_err());
        assert!(elapsed < Duration::from_secs(1));
    }

    #[test]
    fn request_text_rejects_an_untrusted_tls_certificate() {
        let Some((url, server, _material)) = untrusted_tls_server() else {
            panic!("OpenSSL is required to create the local test certificate");
        };
        let error = execute_request(
            &url,
            Method::GET,
            Vec::new(),
            String::new(),
            Duration::from_secs(2),
            32,
            Arc::new(SocketControl::default()),
        )
        .unwrap_err();
        server.join().unwrap();
        let message = error.to_string().to_ascii_lowercase();
        assert!(
            message.contains("certificate") || message.contains("trust"),
            "{message}"
        );
    }

    #[test]
    #[ignore = "requires outbound HTTPS with a platform-trusted certificate"]
    fn request_text_accepts_a_platform_trusted_tls_certificate() {
        let result = execute_request(
            "https://example.com/",
            Method::GET,
            Vec::new(),
            String::new(),
            Duration::from_secs(10),
            16 * 1024,
            Arc::new(SocketControl::default()),
        )
        .unwrap();
        assert_eq!(result["statusCode"], 200);
        assert!(result["body"].as_str().unwrap().contains("Example Domain"));
    }

    #[test]
    fn request_text_cancels_during_tls_handshake() {
        let (url, server, accepted) = stalled_tls_server().unwrap();
        let socket = Arc::new(SocketControl::default());
        let worker_socket = socket.clone();
        let started = Instant::now();
        let worker = std::thread::spawn(move || {
            execute_request(
                &url,
                Method::GET,
                Vec::new(),
                String::new(),
                Duration::from_secs(10),
                32,
                worker_socket,
            )
        });
        accepted
            .recv_blocking()
            .expect("client TCP connection should reach the local TLS peer");
        socket.cancel();
        let result = worker.join().unwrap();
        let elapsed = started.elapsed();
        server.join().unwrap();
        assert!(result.is_err());
        assert!(elapsed < Duration::from_secs(1));
    }

    #[test]
    fn request_text_applies_timeout_during_tls_handshake() {
        let (url, server, accepted) = stalled_tls_server().unwrap();
        let started = Instant::now();
        let result = execute_request(
            &url,
            Method::GET,
            Vec::new(),
            String::new(),
            Duration::from_millis(250),
            32,
            Arc::new(SocketControl::default()),
        );
        accepted
            .recv_blocking()
            .expect("client TCP connection should reach the local TLS peer");
        let elapsed = started.elapsed();
        let _ = server.join();
        assert!(result.is_err());
        assert!(elapsed < Duration::from_secs(2));
    }
}
