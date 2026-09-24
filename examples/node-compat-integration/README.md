# Embedded NodeCompat integration

Runs actual NodeCompat calls through a real Niva WebView and Native bridge.
The default run uses a disposable debug resource tree; `--packaged` builds a
small temporary macOS app bundle and uses its `niva://` resource origin.
NodeCompat assets are supplied by the executable, not copied into the app.

```sh
cargo build --release -p niva
python3 examples/node-compat-integration/run.py target/release/niva --packaged
```

Requires Python 3 and OpenSSL. A temporary two-day CA/leaf certificate is passed
explicitly to the TLS client; no system trust store is changed. The runner uses
a temporary HOME/TMPDIR only for its child process and removes its resource,
certificate, profile and data directory after exit.

Default output is one required JSON result. `--trace` enables diagnostic
progress output. Assertions remain active in both modes.

The fixture checks filesystem sync/callback/Promise/handle/stream/watch paths,
child processes, crypto, OS data, a 600 KB HTTP exchange, CA-verified HTTPS and
close, UDP echo, DNS A/TXT and truncated-UDP-to-TCP fallback, TCP half-close,
ESM/CommonJS identity, and presence of the fixed 179 API entries. Presence and
smoke success are separate from upstream Node behavioral conformance; see
[implementation evidence](../../docs/node-compat-implementation.md).
