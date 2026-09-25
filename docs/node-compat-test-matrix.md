# NodeCompat behavior test matrix

> 历史基线说明（2026-09-25）：本页保留统一 runtime 重构前的范围与验收记录。文中的“当前”、模块/API 数量、通过数、体积和旧配置仅适用于对应历史快照，不代表 `codex/architecture-implementation` 的验证结果。新实现进度及重新验收证据见[架构实施台账](architecture-implementation-plan.md)；原始 JSON 证据保持不变。

Scope: public surfaces described in `packages/node-compat/README.md` and the
package export map. Counts below are documented callable names/methods, not
assertion counts. Aliases and alternate entry points are called out separately
so the same underlying behavior is not counted twice. Test names are exact
names from `packages/node-compat/test`; only tests that assert behavior are
credited. `bridge.test.js`'s ESM-export test checks resolution/version only and
is not behavior coverage.

The suite uses JavaScript bridge/test doubles for Niva-facing calls. These tests
verify adapter behavior against simulated bridge responses; they do not
validate a native WebView, a real Niva binary, or real OS processes/networking.

On 2026-09-24, `examples/node-compat-macos-smoke/run.py` also passed in a real
macOS Niva WebView: 16 cases, 15 documented modules, and 185 distinct API/alias
checks. It used a disposable app/profile, actual native fs and child processes,
and the app's own loopback HTTP server. The package tests below remain the
separate mock-bridge layer. The Mac fixture checks HTTPS wrapper protocol
rejection only; it does not perform an outbound TLS request.

## Module behavior

| Module / export path(s) | README-documented surface (count) | Meaningful behavior tests |
| --- | --- | --- |
| `path` (`/path`) | 17: `join`, `resolve`, `basename`, `dirname`, `extname`, `normalize`, `relative`, `parse`, `format`, `isAbsolute`, `toNamespacedPath`, `matchesGlob`, `sep`, `delimiter`, `posix`, `win32`, and browser cwd behavior (`setCwd` / async registration) | `path.test.js`: “default path object has Node's path surface and host separator”; “POSIX operations match Node path semantics for common roots and segments”; “win32 operations match Node path semantics for drives and UNC paths”; “resolve uses the configured working directory and validates inputs”; “matchesGlob handles common path wildcards”. `public-api-gaps.test.js`: “path.toNamespacedPath resolves drive and UNC paths without double-prefixing”. Some individual `parse`/`format` fields and wildcard edge cases are not exhaustively covered. |
| `fs`, `fs/promises` (`/fs`, `/fs/promises`) | 11 async functions: `readFile`, `writeFile`, `appendFile`, `mkdir`, `readdir`, `stat`, `access`, `rename`, `rm`, `cp`, `copyFile`. `fs/promises` is a separate promises-object entry point. | `bridge.test.js`: “fs promise adapters bridge text, bytes, writes, stats, and directory entries”; “fs mutation adapters preserve bridge semantics and reject unsupported rm/cp cases”. Both exercise a mock Niva bridge. Explicit behavior coverage for every encoding/option/error branch and each alias is incomplete. |
| `os` (`/os`) | 7: async `platform`, `arch`, `homedir`, `tmpdir`; constants `EOL`, `sep`, `delimiter` | `bridge.test.js`: “os maps Niva native values to Node platform and architecture names”; `public-api-gaps.test.js`: “OS synchronous constants agree with the host path convention”. Explicit `homedir`/`tmpdir` branch assertions are not identified by these test names. |
| `child_process` (`/child_process`) | 2: `spawn`, callback-style `exec` | `bridge.test.js`: “spawn exposes event streams, stdin, status, and bridge limitations”; “exec runs through the shell and simulates callback errors with captured output”. `public-api-gaps.test.js`: “child output setEncoding incrementally decodes split UTF-8 and validates encodings”; “detached spawn passes its mode to the bridge and reports the detached pid”; “exec with encoding null returns binary Buffers”. Native process behavior is not tested. |
| `events` (`/events`) | 13 instance methods: `on`, `addListener`, `once`, `prependListener`, `prependOnceListener`, `off`, `removeListener`, `removeAllListeners`, `emit`, `listeners`, `rawListeners`, `listenerCount`, `eventNames`, plus `EventEmitter` constructor | `events-util.test.js`: “EventEmitter handles order, once, removal, symbols, and unhandled errors”. `public-api-gaps.test.js`: “EventEmitter aliases, once raw listeners, listener filtering, and errorMonitor work”. Several method combinations/options remain unenumerated. |
| `util` (`/util`) | 6: `format`, `inspect`, `promisify`, `callbackify`, `isDeepStrictEqual`, `deprecate` | `events-util.test.js`: “util.format covers common placeholders and extra inspected arguments”; “promisify and callbackify preserve receiver, errors, and async callback timing”; “deep equality checks cycles, prototypes, maps, sets, and typed arrays”; “deprecate warns once while preserving return value and receiver”. `documented-api-edges.test.js`: “util.inspect honors depth and ANSI color options”. |
| `querystring` (`/querystring`) | 4 canonical functions: `parse`, `stringify`, `escape`, `unescape`; `decode` aliases `parse`, `encode` aliases `stringify` in the ESM surface | `buffer-querystring-url.test.js`: “querystring parse and stringify match the legacy Node subset”; `public-api-gaps.test.js`: “querystring aliases and custom codec options preserve their documented behavior”. |
| `buffer` (`/buffer`) | 7 callable names: `Buffer.from`, `alloc`, `allocUnsafe`, `concat`, `byteLength`, `isBuffer`, `isEncoding`; plus documented instance operations `toString`, `write`, `fill`, `equals`, `compare`, `copy`, `indexOf`, `lastIndexOf`, `includes`, byte swaps, JSON conversion, and signed/unsigned 8/16/32-bit reads/writes | `buffer-querystring-url.test.js`: “Buffer subset agrees with Node for encodings and common byte operations”; `public-api-gaps.test.js`: “Buffer allocation, sizing, encoding checks, comparisons, and copies match Node”; “Buffer swaps and signed/unsigned integer accessors match Node byte-for-byte”. BigInt and floating-point accessors are explicitly unsupported. |
| `url` (`/url`) | 4: native `URL`, native `URLSearchParams`, `fileURLToPath`, `pathToFileURL` | `buffer-querystring-url.test.js`: “URL helpers preserve URLSearchParams and local file path semantics”; `public-api-gaps.test.js`: “Windows file URL helpers round-trip drive and UNC paths”. Native constructor behavior is provided by the runtime WebView, not an adapter test. |
| `crypto` (`/crypto`) | 3: `randomBytes`, `randomUUID`, `createHash`; hash algorithms SHA-1/256/384/512 and digest encodings are documented options | `crypto-zlib.test.js`: “async random APIs return correctly shaped secure values”; “Web Crypto hash supports chunked updates and Node encodings”. `public-api-gaps.test.js`: “crypto reports its supported hashes and hashes SHA-384/SHA-512 data”. `documented-api-edges.test.js`: “crypto and zlib reject clearly when browser primitives are unavailable”. Real WebView availability remains unverified. |
| `zlib` (`/zlib`) | 2: `gzip`, `gunzip` | `crypto-zlib.test.js`: “gzip and gunzip round-trip bytes compatible with Node zlib”. `documented-api-edges.test.js`: “crypto and zlib reject clearly when browser primitives are unavailable”. Real WebView compression support remains unverified. |
| `http` (`/http`) | 3: `request`, `get`, `post` | `http-assert.test.js`: “http get delivers response events and header/body promises”; “request buffers UTF-8 writes and maps Node request options to the Niva stream”; “HTTP bridge failures reject promises and emit a request error”; `public-api-gaps.test.js`: “http.post submits its body and IncomingMessage exposes buffer and response events”; “HTTP request accepts an options object followed by request overrides”. Tests use bridge doubles; no native HTTP service/WebView validation. |
| `https` (`/https`) | 3: `request`, `get`, `post` | `http-assert.test.js`: “https wrappers enforce scheme and post convenience submits the body”; `public-api-gaps.test.js`: “https.get and https.request use HTTPS and preserve request callbacks”. No native TLS/network validation. |
| `assert`, `assert/strict` (`/assert`, `/assert/strict`) | 20 named exports: callable `assert`, `AssertionError`, `strict`, and 17 assertion functions (`ok`, `fail`, `equal`, `notEqual`, `strictEqual`, `notStrictEqual`, `deepEqual`, `notDeepEqual`, `deepStrictEqual`, `notDeepStrictEqual`, `throws`, `doesNotThrow`, `rejects`, `doesNotReject`, `ifError`, `match`, `doesNotMatch`). The strict subpath and strict aliases are an additional entry form. | `http-assert.test.js`: “assert provides useful strict, deep, sync-error, and async-error checks”; `public-api-gaps.test.js`: “assert exports cover negative checks, regex checks, and sync/async no-throw helpers”. `documented-api-edges.test.js`: separate named cases call every documented assertion function on passing and failing inputs, plus callable assert, `AssertionError`, and strict aliases. |
| `stream`, `stream/promises` (`/stream`, `/stream/promises`) | 1 behavior: `pipeline(source, ..., destination, callback?)`; Promise entry exports the same operation | `http-assert.test.js`: “pipeline connects a buffered HTTP response adapter to an HTTP request adapter”; `public-api-gaps.test.js`: “stream/promises pipeline exports the Promise API and rejects stream errors”. Coverage is limited to supported adapters, not general Node streams. |

## Package exports and coverage gaps

The export map contains 20 paths: `.`, 18 module/subpath entries (`./path`,
`./os`, `./fs`, `./fs/promises`, `./child_process`, `./events`, `./util`,
`./querystring`, `./buffer`, `./url`, `./crypto`, `./zlib`, `./http`,
`./https`, `./assert`, `./assert/strict`, `./stream`, `./stream/promises`),
and `./classic`. `./` root re-exports the modules and
`registerNodeCompat`; classic registration/allowlist behavior has behavior
tests in `bridge.test.js` (“classic entry registers the four modules and async
cwd bootstrap”; “classic registration honors the selected-module allowlist”).
`bridge.test.js` also has “package ESM exports resolve and registration enforces
the bridge version”; that test proves import resolution and version rejection,
not behavior for each export path.

Known coverage boundaries from the documented surface:

- The package unit tests use bridge doubles. The separate macOS fixture gives
  real WebView/native evidence for the 185 listed checks. Windows, Linux, and
  outbound TLS remain unverified by that fixture.
- The README documents branch/option limits that are not shown as exhaustively
  covered by named tests: fs option/encoding/error combinations; all OS
  directory values; process cancellation/deadline and unsupported stdio paths;
  individual EventEmitter edge combinations; URL malformed/encoded separator
  cases; and HTTP redirect/private-address/status edge cases. Missing browser
  crypto/compression APIs now have explicit rejection tests, but platform
  availability still requires real WebView evidence.
- Exported names not documented in the README include several wrapper-level
  constants and aliases (for example `os.devNull`, `os.info`/`dirs`,
  `events.defaultMaxListeners`/`errorMonitor`, `querystring.decode`/`encode`,
  `buffer.SlowBuffer` and metadata, and `crypto.getHashes`). Their presence is
  not proof of documented or complete behavior coverage. `path.setCwd` is
  mentioned in README prose; `path.getCwd` is exported but not documented.
- The current tests exercise the principal documented operations, but neither
  this matrix nor the test names establish 100% method, branch, alias, or
  platform coverage.
