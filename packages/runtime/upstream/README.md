# Pinned Node.js upstream tests

This directory contains selected original test files and the license from the
Node.js `v22.14.0` source tree at commit
`5d2feb257bcee090e57900eb51720171a6aa92f3`. `manifest.json` fixes the selected
file set and each file's SHA-256. Files are stored under `node-v22.14.0/` using
their paths from the upstream repository; do not edit their contents.

Run from the repository root with Node.js 22.14.0, the version used by the
pinned fixture and CI. Build the unified runtime first, then the Native binary
for the real-WebView suite:

```sh
npm run build --workspace=packages/runtime
cargo build --release -p niva
npm run test:upstream:harness --workspace=packages/runtime
```

The two suites are separate. `js` runs the selected JavaScript contracts against
the built Niva adapters under the pinned Node test host; it does not route
subject modules to host Node implementations. Native calls are unavailable in
this suite and are reported unsupported when a selected case requires them:

```sh
npm run test:upstream --workspace=packages/runtime -- --suite js --report upstream-js-results.json
```

`native` launches the release binary in an isolated real WebView and exercises
the Native-backed adapters through the ordinary main-window process streams.
Set `NIVA_UPSTREAM_BINARY` to the binary just built:

```sh
NIVA_UPSTREAM_BINARY="$PWD/target/release/niva" npm run test:upstream --workspace=packages/runtime -- --suite native --report upstream-native-results.json
```

Each test uses a fresh subprocess and the same JavaScript realm as its imported
Niva modules. The `js` suite runs under host Node; host `assert`, `util`
(diagnostic formatting), `vm` (test realms), process and scheduling are test
infrastructure, not compatibility claims for the corresponding Niva modules.
The `native` suite covers only its selected calls through the real WebView path
of the supplied binary. Neither suite proves the full Node API surface,
complete platform behavior, packaged application behavior, or release
acceptance.

The harness must report unsupported tests and fail on them; unsupported,
skipped, missing, or failing selected cases are not passes. These checks run the
JavaScript adapter under host Node for fast contract feedback. They do not prove
behavior in a Niva WebView or native platform, and do not claim complete Node
module compatibility. See [the acceptance policy](../../../docs/node-upstream-conformance.md) for the acceptance
policy and evidence boundaries.
