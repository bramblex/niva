# NodeCompat macOS WebView integration smoke

This fixture runs the generated `@niva/node-compat` classic bundle inside a
disposable Niva app, then calls the registered adapters through the real
macOS WebView and native bridge. It covers all 15 documented modules and the
`fs/promises`, `assert/strict`, and `stream/promises` aliases.

## Run

From the repository root:

```sh
cargo build --bin niva
python3 examples/node-compat-macos-smoke/run.py --binary target/debug/niva
```

The runner builds `packages/node-compat/dist/niva-node-compat.js`, stages the
classic bundle and package ESM sources under an isolated resource directory,
and starts the app with `nodeCompat: true`. It assigns the child process a
temporary `HOME` and `TMPDIR`, unique app UUID, data/cache profile, and resource
tree. All are removed after the process exits. Use `--keep-workdir` to retain
the generated files and stderr log after a failed run. `--timeout` changes the
overall page timeout.

The WebView is visible while the test runs. Keep macOS awake and unlocked. The
runner does not alter the user's shell environment or persistent app data.

## Static checks

These checks validate the runner and exact case catalog without launching Niva:

```sh
node --check examples/node-compat-macos-smoke/cases.js
python3 -m py_compile examples/node-compat-macos-smoke/run.py examples/node-compat-macos-smoke/test_runner.py
python3 -m unittest discover -s examples/node-compat-macos-smoke -p 'test_runner.py'
```

## Scope and limits

The page reports each API name only after its associated assertion passes. The
catalog includes:

- `path`: the documented string methods, POSIX and Windows variants, constants,
  and glob matching.
- `os`: platform, architecture, home/temp paths, and documented constants.
- `fs` and `fs/promises`: read/write/append, directory creation/listing, stats,
  access, rename, remove, recursive copy, file copy, and binary reads. Files are
  created under the per-run temporary directory and removed.
- `child_process`: `spawn`, `exec` callback results, stdin, stdout, and stderr;
  commands are limited to `cat`, `printf`, and a fixed shell command that writes
  a string to stderr.
- `events`, `util`, `querystring`, `buffer`, `url`, `crypto`, `zlib`, and
  `assert`, including their documented method groups and strict alias.
- `http`: `request`, `get`, and `post`, request/response properties and methods,
  response events, binary/text reads, and stream piping. Requests target only
  the Niva instance's own loopback static server; `post` checks its expected
  GET-only 405 response.
- `https`: `request`, `get`, and `post` reject an `http:` URL before networking.
  This keeps the default run deterministic and offline; it does not verify a
  successful TLS connection or certificate handling.
- `stream` and `stream/promises`: pipeline over child-process output, callback
  completion, and an HTTP response message.
- classic registration, `Niva.require`, `Niva.import`, bare `fs` import-map
  resolution, and the `node:` aliases for the documented modules.

The suite verifies the documented adapter surface used in real WebView calls;
it does not claim full Node.js compatibility, unsupported signatures, network
proxy behavior, remote IPC streaming, arbitrary HTTPS/TLS success, detached
process cleanup, or the exhaustive behavior of browser-native URL/Web Crypto /
compression implementations. HTTP body streaming is buffered by the native
bridge. Results are macOS-only and do not establish Windows behavior.
