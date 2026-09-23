# Niva NodeCompat browser package

This optional package provides a bounded Node-shaped builtin subset for Niva
pages. It does not change the Niva binary or initialize script.

## ESM

```js
import path from "@niva/node-compat/path";
import { readFile, writeFile } from "@niva/node-compat/fs";
import { platform, homedir } from "@niva/node-compat/os";
import { exec, spawn } from "@niva/node-compat/child_process";

await writeFile(path.join("data", "hello.txt"), "hello");
console.log(await readFile(path.join("data", "hello.txt"), "utf8"));
console.log(await platform(), await homedir());
```

`path` keeps synchronous string operations. `path.resolve` needs the Niva
process working directory, which the bridge exposes asynchronously. Call
`await registerNodeCompat()` first, or `path.setCwd(value)` yourself, when that
directory matters. In browser-only use without Niva or Node, its initial cwd is
`/`.

Other ESM subpaths are `@niva/node-compat/events`, `/util`, `/querystring`,
`/buffer`, `/url`, `/crypto`, `/zlib`, `/http`, `/https`, `/assert`, and
`/stream` (with `/stream/promises`). Raw static imports such as
`import fs from "fs"` need a bundler alias or import map supplied by the
application packager; these package exports do not rewrite browser module
resolution.

All OS details that require native data are asynchronous (`platform`, `arch`,
`homedir`, and `tmpdir`). `EOL`, `sep`, and `delimiter` are synchronous constants
derived from the browser platform hint.

## Single-file Niva pages

The package build writes `dist/niva-node-compat.js`; Devtools stages that file
at the target URL `__niva_compat/node-compat.js`. Load that staged URL after
`initialize_script.js`. If the native bootstrap sets
`window.__niva_node_compat_modules` to an array, the script registers only
those modules. If it is absent, it registers the full supported set. Selected
modules get their bare and `node:` names; selecting `fs` also
registers `fs/promises` and `node:fs/promises`; selecting `assert` or `stream`
also registers their `/strict` or `/promises` aliases. The entry reads the
process cwd when `path` or `url` is selected and exposes that bootstrap promise as
`NivaNodeCompatReady`:

The supported module names are `path`, `os`, `fs`, `child_process`, `events`,
`util`, `querystring`, `buffer`, `url`, `crypto`, `zlib`, `http`, `https`,
`assert`, and `stream`.

```html
<script src="__niva_compat/node-compat.js"></script>
<script>
  NivaNodeCompatReady.then(function () {
    const path = require("path");
    const fs = require("fs");
    return fs.readFile(path.resolve("notes.txt"), "utf8");
  }).then(console.log);
</script>
```

## Supported signatures and limits

### `path`

Supports `join`, `resolve`, `basename`, `dirname`, `extname`, `normalize`,
`relative`, `parse`, `format`, `isAbsolute`, `toNamespacedPath`, `matchesGlob`
for common `*`, `?`, `[]`, and `**` patterns, `sep`, `delimiter`, and `posix` /
`win32` variants. The default variant follows the browser platform hint.
`path.resolve` uses a `/` initial cwd in a browser until
`registerNodeCompat()` finishes the async cwd lookup or `setCwd()` is called.

### `fs` and `fs/promises`

Async functions: `readFile`, `writeFile`, `appendFile`, `mkdir`, `readdir`,
`stat`, `access`, `rename`, `rm`, `cp`, and `copyFile`. Binary reads and process
output use the package `Buffer`; UTF-8 strings use the stream-backed Niva
`fs.read/write/append` overrides.
Permission-mode access checks, mode bits, inode/owner fields, bigint `Stats`,
and sync APIs are not supported. Stats sizes, timestamps, and type methods
reported by the bridge are available. `rm` removes directory trees only with
`recursive: true`; directory copies require `recursive: true`. Since native
remove is recursive, the
adapter checks for non-empty directories before using it when recursive is
false.
The ESM `fs/promises` subpath and classic `require("fs/promises")` use a
separate promises object as their default export.

### `os`

`platform()`, `arch()`, `homedir()`, and `tmpdir()` return promises backed by
`os.info` / `os.dirs`; `EOL`, `sep`, and `delimiter` are synchronous constants.
Other OS methods unavailable from the current bridge are not exposed.

### `child_process`

`spawn()` and callback-style `exec()` use `process.execStream`. `spawn` output
defaults to the package `Buffer` and can be decoded as UTF-8 with
`stdout.setEncoding("utf8")`; `exec` defaults to UTF-8 strings and accepts
`encoding: null` for `Buffer`. `stdin.write()` and `stdin.end()` send bytes
through the Niva stream.

The JS bridge groups binary chunks and invokes `onBlob` when each pipe ends, so
`spawn` data events are buffered per pipe rather than delivered live per chunk.
Attached process IDs are unavailable. Timeout, abort signals, non-piped stdio,
and `exec.maxBuffer` are unsupported. `child.kill()` returns `false` because
there is no per-process kill API; `child.cancel()` cancels the owning bridge call,
which terminates and reaps an attached native child. Detached processes
intentionally outlive that call. Streaming fs/process calls require a local Niva WebSocket
page; remote IPC pages do not support binary streams.

### `http` and `https`

Both modules expose `request(urlOrOptions, options?, callback?)`,
`get(urlOrOptions, callback?)`, and the package convenience
`post(urlOrOptions, body?, options?, callback?)`. `http` accepts `http:` URLs;
`https` accepts `https:` URLs. An options object can use `url`, or
`hostname`/`host`, `port`, `path`, `method`, and `headers`. Native requests do not use user or environment proxies.
Headers use a flat string map; duplicate names are collapsed by the native
bridge (input value arrays are joined with commas).
`request()` returns a `ClientRequest`; call `.end()` to send it. `.write()`
buffers UTF-8 text until `.end()`. `get()` and `post()` send automatically.
The native HTTP bridge rejects private, loopback, link-local, and other
special-use destinations (including DNS redirects to them). Its only loopback
exception is this Niva instance's exact HTTP service port.

The callback or `request.on("response", ...)` receives an `IncomingMessage`
when response headers arrive. The request also exposes `response`, a Promise
for headers, and `result`/`completion`, Promises that resolve after the body is
complete. The response exposes `statusCode`, lower-case `headers`, `text()`,
`json()`, `arrayBuffer()`, `buffer()`, and `data`/`end` events. HTTP status
codes, including errors such as 404, resolve normally when the native ureq
agent has `http_status_as_error(false)` enabled. The Niva HTTP integration must
keep that setting so 4xx/5xx status, headers, and body reach this adapter; if
the native API turns them into an error before its `head` event, JavaScript
cannot reconstruct the response. Redirect handling follows the native ureq
agent and may differ from Node's one-response behavior.

Niva's `http.requestStream` buffers each response pipe into one `Blob`, so the
response `data` event contains at most one complete `Buffer`; request writes
are buffered in memory and sent as one UTF-8 text body. Binary request bodies,
streaming upload, timeout, AbortSignal, agents, sockets, and remote IPC streams
are unsupported. Requests require a local Niva WebSocket page.

### `stream`

`pipeline(source, ..., destination, callback?)` connects streams that provide
`pipe()` and `on()`. With a callback it returns the destination and calls
`callback(error?)`; without a callback it returns a Promise that resolves to
the destination. It supports the package's child output streams, HTTP response
messages, and HTTP requests. The package does not provide Node's Readable,
Writable, Transform, backpressure, or general stream constructors.

### `assert`

Supports callable `assert(value, message?)`, `ok`, `fail`, `equal`, `notEqual`,
`strictEqual`, `notStrictEqual`, `deepEqual`, `notDeepEqual`,
`deepStrictEqual`, `notDeepStrictEqual`, `throws`, `doesNotThrow`, `rejects`,
`doesNotReject`, `ifError`, `match`, and `doesNotMatch`, plus `AssertionError`
and `strict`. The main `assert` entry keeps coercive `equal` / `deepEqual`
aliases; `assert/strict` exports a strict default and strict aliases.

### `events`

`EventEmitter` supports instance `on`, `addListener`, `once`, `prependListener`,
`prependOnceListener`, `off`, `removeListener`, `removeAllListeners`, `emit`,
`listeners`, `rawListeners`, `listenerCount`, and `eventNames`. It preserves
listener `this`, once listeners, and unhandled `error` behavior. Node async
iterator helpers and capture-rejection mode are not implemented.

### `util`

Supports `format`, `inspect`, `promisify`, `callbackify`, `isDeepStrictEqual`,
and `deprecate`. Inspect accepts `depth` and ANSI `colors`; format accepts
`%s`, `%d`, `%i`, `%f`, `%j`, `%o`, `%O`, `%c`, `%%`, and extra inspected arguments.
`promisify` resolves the callback's first success value. `deprecate` warns once
per wrapped function. This is not Node's full inspector or formatter.

### `querystring`

Supports `parse(value, separator?, equals?, { maxKeys?, decodeURIComponent? }?)`,
`stringify(value, separator?, equals?, { encodeURIComponent? }?)`, `escape`,
and `unescape`. Parsed objects have a null prototype.

### `buffer`

Supports `Buffer.from`, `alloc`, `allocUnsafe`, `concat`, `byteLength`,
`isBuffer`, and `isEncoding` with UTF-8, hex, base64/base64url, ASCII, Latin-1 /
binary, and UTF-16LE / UCS-2. Also supports `toString`, `write`, `fill`,
`equals`, `compare`, `copy`, `indexOf`, `lastIndexOf`, `includes`, byte swaps,
JSON conversion, and signed / unsigned 8-, 16-, and 32-bit integer reads and
writes. The result is a `Uint8Array` subclass; BigInt and floating point
accessors are not implemented. Classic registration sets global `Buffer` only
when `buffer` is selected and the page has no existing global.

### `url`

`URL` and `URLSearchParams` are native WebView constructors. `fileURLToPath(url)`
and `pathToFileURL(path)` support local POSIX paths and Windows drive / UNC
paths. POSIX rejects file URL hosts other than `localhost`; encoded path
separators are rejected.

### `crypto`

`randomBytes(size)` resolves to `Buffer`; `randomUUID()` resolves to a UUID
string. `createHash(algorithm)` supports Web Crypto SHA-1, SHA-256, SHA-384,
and SHA-512 with chained `update(data, inputEncoding)` and asynchronous
`digest(encoding?)`. `digest()` without an encoding resolves to `Buffer`; hex,
base64, base64url, and other Buffer-supported encodings resolve to strings.
MD5, HMAC, callback overloads, and synchronous crypto APIs are not provided.
Calls reject if Web Crypto is unavailable.

### `zlib`

`gzip(data)` and `gunzip(data)` resolve to `Buffer` using browser
`CompressionStream` / `DecompressionStream` with the `gzip` format. Sync,
callback, deflate, and streaming zlib APIs are not provided. The WebView must
implement those browser compression classes.

The classic script is generated from the runtime files with `npm run
build:classic`. Package-local tests run with `npm test`.
