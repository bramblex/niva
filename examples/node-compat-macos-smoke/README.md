# Node API runtime macOS WebView integration smoke

This fixture runs the unified runtime modules inside a disposable Niva app and
exercises them through a real macOS WebView and Native bridge. It covers the
same 15 module families and the `fs/promises`, `assert/strict`, and
`stream/promises` entry points as the preserved case catalog.

Run from the repository root:

```sh
cargo build --bin niva
python3 examples/node-compat-macos-smoke/run.py --binary target/debug/niva
```

The fixture copies only its HTML, JavaScript, and process-stream helper into a
temporary resource tree. Runtime code comes from the selected Niva binary.
The generated config opts into CommonJS globals and ESM import maps. The host
runner launches with `--config`/`--resource` and exchanges JSON lines through
the fixture's own `process.stdin`/`process.stdout` protocol. No Native host
message endpoint or `--stdio` switch is involved. A temporary `HOME`, `TMPDIR`,
app UUID, data/cache profile, and resource tree are removed after the run.

The WebView is visible while the test runs. Keep macOS awake and unlocked.
`--keep-workdir` retains generated files after a failed run, and `--timeout`
sets the overall page timeout.

Static checks do not launch Niva:

```sh
node --check examples/node-compat-macos-smoke/cases.js
python3 -m py_compile examples/node-compat-macos-smoke/run.py examples/node-compat-macos-smoke/test_runner.py
python3 -m unittest discover -s examples/node-compat-macos-smoke -p 'test_runner.py'
```

The catalog preserves the earlier API cases. Passing rows require their
associated assertion to complete. It does not establish full Node.js
compatibility, unsupported signatures, remote IPC streaming, arbitrary HTTPS
success, detached-process cleanup, or the exhaustive behavior of browser-native
URL/Web Crypto/compression implementations. HTTP response bodies use the
runtime's current transport limits. Results are macOS-only and do not establish
Windows behavior.

The migrated catalog uses CommonJS `require` and browser `import()` instead of the removed custom module registry. `os.sep` and `os.delimiter` were non-Node extensions and are retired; the equivalent `path.sep` and `path.delimiter` cases remain. These retired entries are not counted as current compatibility passes.

Legacy promise-returning fs/zlib overloads and HTTP post/response/result/body convenience methods are retired. Tests now use Node callbacks/promisify and actual response streams against an independent Python HTTP fixture. Node process warning events, querystring decoder percent encoding, and promise pipeline return values follow the upstream contracts.
