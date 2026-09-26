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

console.log(text, response.statusCode, Niva.os.info);
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

Native APIs are asynchronous by default. Their stable bridge calls Rust over
platform IPC; Native-to-JS frames and events use `evaluate_script`. Ordinary
async API calls use this IPC/evaluate_script route. For high-volume file or
network transfer, binary payloads, and streaming `child_process` stdio, the
runtime may use an authenticated WebSocket bridge as an optional performance
path. If that WebSocket cannot be established or is unavailable, the same
operation must remain available through the stable IPC/evaluate_script bridge.
The API and Rust handlers do not expose transport choice to callers. IPC binary
frames encode the complete wire frame with Base64 at the bridge boundary;
handlers continue to consume bytes. HTTP XHR is reserved for synchronous
Node-compatibility APIs and must emit a warning on first use of each method. State-changing
operations such as changing cwd should be asynchronous.

The runtime now implements this routing contract in source. macOS smoke covers
the stable IPC unary/channel path and basic WebSocket streams; pinned-resource
lifecycle and Windows WebView acceptance remain separate checks. The stable
asynchronous bridge is IPC/evaluate_script; WebSocket is an optional
optimization path.

Authorized remote IPC supports a restricted asynchronous unary subset: UTF-8
text file operations and metadata, `requestText`, `execText` and
`execFileText`. Remote pages cannot open streaming Channels.
`isTrustedLocal()` and `runtimeConfig.trustedLocal` describe the page's Native
permission context; transport choice stays inside the bridge and is not exposed
to Node adapters. Binary/default-Buffer file reads and synchronous operations
remain unavailable to remote pages.

For trusted local pages, Node-style async `readFile`, `writeFile` and
`appendFile` reuse the Native file-handle Channel. Large file contents therefore
move in bounded binary chunks, preferring WebSocket when available and using
IPC with Base64-encoded complete wire frames otherwise. Synchronous variants
exist only for Node compatibility and use synchronous XHR with a warning.

`Niva.http.requestText()` and `Niva.https.requestText()` are bounded text
helpers for IPC-capable pages. They do not replace the streaming Node-style
`http.request()` and `https.request()` contracts. Likewise,
`Niva.child_process.execText()` and `execFileText()` are bounded text helpers;
streaming child processes require a trusted local page and use the WebSocket
optimization when available or IPC Channel otherwise.

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
`Niva.<module>` after bootstrap. They are not a second runtime. The Node-shaped
`process.version` remains the Niva version; `process.versions.nodeCompat` names
the declared compatibility target, Node 22.14.0. Niva does not impersonate a
host Node process or fall back to one for Native operations.

`Niva.crypto.timingSafeEqual()` is a JavaScript compatibility helper. It checks
equal-length inputs and visits every byte, but JavaScript execution is not
constant-time; it warns on every call. Do not use it to compare secrets where
timing resistance is required.

Type declarations are generated from
[`src/contracts.ts`](src/contracts.ts) and published through `@niva/types`.
That package depends on `@types/node` 22.14.0 for the Node-shaped adapter types.
The runtime flags control JavaScript global injection; they do not isolate
TypeScript's ambient Node declarations. See [`../types/README.md`](../types/README.md).

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
