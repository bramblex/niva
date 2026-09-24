# Niva NodeCompat browser package

Compatibility acceptance is defined by the
[Node upstream conformance policy](../../docs/node-upstream-conformance.md).
The target is a strict subset of Node contracts, verified with pinned upstream
tests; existing adapter tests and the smoke fixture do not certify that subset.
The implementation guide below is not an upstream-test acceptance report.

Niva embeds this browser package and enables it by default. No Node runtime or
separate adapter resource staging is needed by a packaged application. Set
`nodeCompat: false` to disable it; an object can select modules and import maps.

## Application usage

```js
await NivaNodeCompatReady;
const fs = require("fs");
const path = require("path");

fs.writeFileSync(path.resolve("notes.txt"), "hello");
const text = await require("fs/promises").readFile("notes.txt", "utf8");
fs.stat("notes.txt", (error, stats) => {
  if (error) throw error;
  console.log(text, stats.size);
});
```

ESM wrappers are available under `@niva/node-compat/<module>`; local HTML also
receives mappings for selected bare and `node:` names. `fs` callback methods,
`fs/promises`, and `fs.*Sync` are distinct interfaces.

The 22 module families are `path`, `os`, `fs`, `child_process`, `events`, `util`,
`querystring`, `buffer`, `url`, `crypto`, `zlib`, `http`, `https`, `assert`,
`stream`, `process`, `net`, `dgram`, `tls`, `dns`, `string_decoder`, and `timers`.
Selected modules expose `fs/promises`, `stream/promises`, `dns/promises`,
`timers/promises`, and `assert/strict` where applicable. The real host `process`
object is only installed in the trusted main window's top-level page.

## Layers

- Pure JS libraries provide Buffer, streams, common hashes/KDFs, compression,
  and HTTP/DNS codecs. See the generated vendor license notices.
- Native provides filesystem/process operations, TCP/UDP sockets, TLS, and
  actual OS observations. `dns.lookup` uses the system resolver.
- JS builds HTTP/HTTPS clients and application servers on those sockets, and
  DNS record queries on UDP/TCP. The internal Niva resource/bridge HTTP server
  remains separate.
- Static process/OS data is injected at startup. Dynamic values such as cwd,
  free memory, CPU data, interfaces and uptime are queried at call time.
- Local synchronous APIs use authenticated synchronous XHR. Streams use WS.
  External IPC supports a restricted, explicitly authorized unary subset.

The old `Niva.api.http` client has been removed. Applications use `http` or
`https`. `child_process.fork` is excluded. In `--stdio` mode stdin/stdout are
reserved for the host NDJSON protocol. Browser timer `ref`/`unref` methods do
not implement Node event-loop liveness.

## Build and verify

```sh
npm run build:vendor --workspace=packages/node-compat
npm run test --workspace=packages/node-compat
cargo build -p niva
python3 examples/node-compat-integration/run.py --packaged
```

The integration runner requires OpenSSL for temporary local TLS certificates.
It exercises the embedded adapter in a real WebView, with disposable files,
configuration and a child-only HOME/TMPDIR. Mock tests, real bridge smoke,
upstream contract tests, and each release platform's size gate are recorded
separately in [implementation status](../../docs/node-compat-implementation.md).

`runtime-files.json` defines the embedded classic order and ESM asset closure.
Rust builds use checked-in sources and vendor output; missing declared assets
fail the build. Cargo does not run npm. After dependency changes regenerate
`src/runtime/vendor.js` and its license notices before building Native.

## Pinned official contracts

The frozen selection contains 58 original Node v22.14.0 test files across 22
module families. The first 30 originals are retained with their original hashes.
Adding a file does not mean its contracts pass; see the per-file report in
[`docs/node-compat-upstream-results.json`](../../docs/node-compat-upstream-results.json).

Use Node **22.14.0** for these commands, from the repository root:

```sh
node packages/node-compat/scripts/test-upstream.mjs --self-test
node packages/node-compat/scripts/test-upstream.mjs --suite js --report /tmp/upstream-js.json
cargo build --release -p niva
NIVA_UPSTREAM_BINARY="$PWD/target/release/niva" node packages/node-compat/scripts/test-upstream.mjs --suite all --report /tmp/upstream-all.json
```

The JS suite runs unchanged upstream sources against Niva adapters in a Node
host. The Native suite relays bridge operations into a fresh hidden Niva WebView
for each file: synchronous XHR, asynchronous calls, and binary WS streams all
reach the real native implementation. Node filesystem/network APIs are not
substituted. This is distinct from running the upstream CommonJS source itself
inside WebView; the packaged WebView smoke remains a separate gate.

Native relay runs are currently verified on macOS only. The environment variable
above applies only to that command. HTTPS and DNS selected files exercise option
validation, not network exchanges. Unknown helpers, missing callbacks, exceptions,
timeouts and skipped assertions fail the gate. The URL test's explicit opposite-
platform branch is reported separately as `platform-excluded`.

Two Node-specific checks are explicitly excluded by user-approved scope in
`upstream/environment-exclusions.json`. The runner pins the policy and original
source hashes, replaces only those exact statement sites in memory, and requires
each exclusion marker to execute once. All other checks still run; arbitrary skips
remain failures. Reports count the two exclusion sites separately from passed files.
Use `--include-environment-specific` for the preserved full unfiltered diagnostic.
