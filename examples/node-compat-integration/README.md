# Unified runtime integration

Runs actual Node-style imports and Native bridge calls through a real Niva
WebView. The default run uses a disposable `--resource` tree; `--packaged`
builds a small temporary macOS app bundle and exercises the packaged resource
origin. Runtime modules are supplied by the Niva executable rather than copied
into the fixture.

```sh
cargo build --release -p niva
python3 examples/node-compat-integration/run.py target/release/niva --packaged
```

Requires Python 3 and OpenSSL. The runner creates a temporary two-day CA/leaf
certificate for the explicit TLS client test; it does not modify the system
trust store. It uses a child-only temporary `HOME`/`TMPDIR` and removes the
fixture resources, certificates, profile, and app data after exit.

The page and runner communicate through a fixture-owned JSON-line process
protocol. The page writes JSON events to Node `process.stdout` and reads
commands from `process.stdin`; no `Niva.api.host` or Native `--stdio` control
path is involved. The launcher uses `--config`/`--resource` for the unpackaged
run and the resource archive for the packaged run.

Default output is one required JSON result. `--trace` adds diagnostic progress
events without changing the assertions.

The fixture checks filesystem sync/callback/Promise/handle/stream/watch paths,
child processes, crypto, OS data, HTTP exchange, CA-verified HTTPS, UDP echo,
DNS, TCP half-close, ESM/CommonJS identity, and the pinned API inventory. These
are integration checks rather than complete upstream Node conformance.
