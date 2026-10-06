# Niva page runtime

`@niva/runtime` is the TypeScript source for Niva's page-side Node-compatible
adapters and Native API facades. The Native application embeds the generated
bootstrap and serves the small ESM facade files from `/__niva_runtime/`.

## Page APIs

The Native bootstrap always creates `Niva`. Node-style APIs and Niva-specific
APIs are properties on that object; applications do not wait for a separate
runtime-ready promise.

```js
const text = await Niva.fs.promises.readFile("notes.txt", "utf8");
const response = await Niva.http.requestText({ url: "https://example.com/status" });

console.log(text, response.statusCode, response.body, Niva.os.info);
```

Filesystem, process, OS, path, URL, child-process, socket, TLS, DNS, HTTP,
streams, events, buffers, crypto, compression, timers and assertion adapters
are exposed as `Niva.fs`, `Niva.process`, `Niva.os`, `Niva.path`, `Niva.url`,
`Niva.child_process`, `Niva.net`, `Niva.tls`, `Niva.dns`, `Niva.http`,
`Niva.https`, `Niva.stream`, `Niva.events`, `Niva.buffer`, `Niva.crypto`,
`Niva.zlib`, `Niva.timers` and `Niva.assert`. Native-only APIs such as
`Niva.window`, `Niva.dialog`, `Niva.clipboard`, `Niva.tray`, `Niva.shortcut`,
`Niva.monitor`, `Niva.webview` and `Niva.resource` are also direct properties.
`Niva.os.info` is a read-only startup snapshot; dynamic OS methods query Native
when called. `Niva.os.dirs()` returns application data, cache and temporary
directories.

The current bridge contract is implemented in the runtime and Native source.
Async API creation/control uses IPC `t:"api_call"`; small Native results and
stream `channelOpened`/capability return through Wry `evaluate_script` invoking
`__niva_ipc_reply({sessionId,rid,sourceOrigin,response})`. WS v2 uses hello and
18-byte binary header version 2. The stream ticket is created and returned
before data transport attach. WebSocket carries only attach/data/ack/cancel; it
never dispatches API methods or carries method/args. Native resource ownership
stays with the creating IPC session, independent of the selected data transport.
Top-level documents and same-origin iframes share the Wry IPC/eval path. Remote
top-level origins may use only granted unary APIs; cross-origin iframes fail
closed even if granted, because Wry eval targets the main document and must
not expose child secrets to its parent. Synchronous XHR remains only for Node
compatibility methods that require synchronous results, with a first-use
warning per public method. Existing WK reply handlers and WebView2-specific
IPC/reply channels have been removed; use Wry IPC and `evaluate_script`.
State-changing operations such as changing cwd remain asynchronous. The actual
macOS two-lane WebView run and remaining platform limits are recorded in the
[Bridge v2 validation record](../../docs/bridge-v2-validation.md); source or
macOS results do not imply Windows validation. See the [Bridge contract](../../docs/bridge.md)
for security, flow-control and failure semantics.

The current source authorizes a restricted asynchronous unary subset for
remote top-level pages: UTF-8 text file operations and metadata, `requestText`,
`execText` and `execFileText`. Cross-origin iframes fail closed even if granted,
and remote pages cannot open streaming Channels.
`isTrustedLocal()` and `runtimeConfig.trustedLocal` describe the page's Native
permission context; transport choice stays inside the bridge and is not exposed
to Node adapters. Binary/default-Buffer file reads and synchronous operations
remain unavailable to remote pages.

For trusted local pages, Node-style async `readFile`, `writeFile` and
`appendFile` currently reuse the Native file-handle Channel. Large file
contents move in bounded binary chunks over the channel's selected data
transport; the API owner remains its creating IPC session.
Synchronous variants exist only for Node compatibility and use synchronous
XHR with a warning.

`Niva.http.requestText()` and `Niva.https.requestText()` are bounded text
helpers for IPC-capable pages. They do not replace the streaming Node-style
`http.request()` and `https.request()` contracts. Likewise,
`Niva.child_process.execText()` and `execFileText()` are bounded text helpers;
streaming child processes require a trusted local page. Their current
data transport is independent of the IPC API call that creates the operation.

## Optional Node globals

`injectCommonJs` and `injectEsm` are independent application options and both
default to `false`. The `Niva` object and its APIs exist regardless of these
flags.

- `injectCommonJs: true` installs `require`, `module`, `exports`, `global`,
  `Buffer`, the Node-shaped timer globals and the trusted main-window `process`
  global. CommonJS loading supports JavaScript and JSON files, caching and
  cycles. Native addons, CommonJS loading of ES modules and `require.extensions`
  are unsupported.
- `injectEsm: true` enables the ESM runtime mode. Trusted local pages receive
  their import map from Native HTML before page modules run. External pages
  must map/bundle Node imports themselves; Niva does not install relative
  `/__niva_runtime/` URLs without an asset-origin/CORS contract.

The ESM facades in `@niva/runtime/<module>` expose the same objects as
`Niva.<module>` after bootstrap. They are not a second runtime. `process.version`
and `process.versions.node` identify the compatibility target, Node 22.14.0;
`process.versions.nodeCompat` repeats that target and `process.versions.niva`
identifies the actual Niva product version. Niva does not impersonate a host
Node process or fall back to one for Native operations.

`Niva.crypto.timingSafeEqual()` is a JavaScript compatibility helper. It checks
equal-length inputs and visits every byte, but JavaScript execution is not
constant-time; it warns on every call. Do not use it to compare secrets where
timing resistance is required.

Type declarations are generated from
[`src/contracts.ts`](src/contracts.ts) and published through `@niva/types`.
That package has an optional `@types/node` 22.14.0 peer for the explicit native
Node tooling declaration mode. Browser consumers can use the default declaration
mode without installing Node globals. The runtime flags control JavaScript
global injection; see [`../types/README.md`](../types/README.md) for the
separate declaration modes.

## Build and verification

Run these commands from the repository root:

```sh
npm run build --workspace=packages/runtime
npm test --workspace=packages/runtime
npm run typecheck --workspace=packages/types
npm pack --dry-run --workspace=packages/types
cargo check -p niva
```

The runtime build runs `tsc`, emits the bootstrap, ESM facades and declarations,
and writes an input hash manifest. Rust rejects stale or incomplete generated
assets; run the runtime build after changing its source, scripts, manifests or
the root lockfile. The Native build does not run npm.

The pinned upstream runner targets Node 22.14.0:

```sh
npm run test:upstream:harness --workspace=packages/runtime
node packages/runtime/scripts/test-upstream.mjs --suite js --report /tmp/upstream-js.json
cargo build --release -p niva
NIVA_UPSTREAM_BINARY="$PWD/target/release/niva" node packages/runtime/scripts/test-upstream.mjs --suite all --report /tmp/upstream-all.json
```

The JS suite runs selected upstream sources against Niva adapters in a Node
host. The Native suite relays bridge calls into a fresh Niva WebView. Neither
suite alone establishes packaged-app or platform release acceptance; see the
[upstream policy](../../docs/node-upstream-conformance.md) and
[per-file results](../../docs/node-compat-upstream-results.json).
