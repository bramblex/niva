# Pinned Node.js upstream tests

This directory contains selected original test files and the license from the
Node.js `v22.14.0` source tree at commit
`5d2feb257bcee090e57900eb51720171a6aa92f3`. `manifest.json` fixes the selected
file set and each file's SHA-256. Files are stored under `node-v22.14.0/` using
their paths from the upstream repository; do not edit their contents.

From the repository root, run the harness checks and selected upstream tests:

```sh
npm run build:vendor --workspace=packages/node-compat
npm run test:upstream:harness --workspace=packages/node-compat
npm run test:upstream --workspace=packages/node-compat
```

Use Node 22.14.0 to reproduce the pinned CI environment. Add
`-- --report upstream-results.json` to the last command to save the machine
report in the package directory. Each test uses a fresh subprocess and the same
JavaScript realm as its imported Niva modules. Host `assert`, `util` (diagnostic
formatting), `vm` (test realms), process and scheduling are test infrastructure;
they are not compatibility claims for the corresponding Niva modules.

The harness must report unsupported tests and fail on them; unsupported,
skipped, missing, or failing selected cases are not passes. These checks run the
JavaScript adapter under host Node for fast contract feedback. They do not prove
behavior in a Niva WebView or native platform, and do not claim complete Node
module compatibility. See [the acceptance policy](../../../docs/node-upstream-conformance.md) for the acceptance
policy and evidence boundaries.
