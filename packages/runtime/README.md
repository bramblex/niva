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

Native transport is grouped under `Niva.bridge`: `call`, `callSync`, `stream`,
`streamSend` and `isIpcOnly`. Synchronous XHR is available only to trusted local
pages. Binary streams, Node HTTP request streams, socket APIs and file handles
use the local WebSocket transport. Authorized remote IPC supports a restricted
asynchronous subset: UTF-8 text file operations and metadata, `requestText`,
`execText` and `execFileText`. Binary/default-Buffer file reads, synchronous
operations, handles and streams do not silently fall back to IPC.

`Niva.http.requestText()` and `Niva.https.requestText()` are bounded text
helpers for IPC-capable pages. They do not replace the streaming Node-style
`http.request()` and `https.request()` contracts. Likewise,
`Niva.child_process.execText()` and `execFileText()` are bounded text helpers;
streaming child processes require the local WebSocket transport.

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
