# Devtools CLI on a real Native Niva binary

This smoke test launches the built Devtools distribution inside Niva's real
WebView and exercises the `--build` CLI entry. It checks both an invalid project
and a valid project with no selected packaging kit. Each case must exit with
code 1, emit one structured failure record to stderr, create no build output,
and exit before the timeout. Each case gets a fresh Devtools UUID; failure
fixtures also clear the saved kit key in a copied resource tree so a user's
earlier kit choice cannot change the missing-kit result.

Build Devtools first, then run with a newly supplied Native binary:

```sh
npm run build --workspace=packages/devtools
python3 examples/devtools-cli-smoke/run.py \
  --binary /path/to/new/target/release/niva \
  --devtools-dist packages/devtools/build \
  --output /tmp/niva-devtools-cli-check
```

The success case uses the main-provided macOS arm64 test kit. It contains one
hash-pinned real runtime and the Rust/JavaScript license materials; it is not a
three-platform release kit. The actual `target/release/niva-packager` was copied
into that fixture and its version/hash are reported by the driver. Run it with:

```sh
cp -p target/release/niva-packager /tmp/niva-resource-layout-v4/single-host-test-kit/niva-packager
python3 examples/devtools-cli-smoke/run.py \
  --binary /path/to/latest/niva \
  --devtools-dist packages/devtools/build \
  --kit-directory /tmp/niva-resource-layout-v4/single-host-test-kit \
  --output /tmp/niva-devtools-cli-kit-check
```

The success case copies the actual Devtools distribution and seeds only the
selected kit path in the isolated page's local storage; the Native bridge and
the kit's real `niva-packager` perform the build. It does not use a fake
packager.

The driver saves each project's config, exact command, stdout, stderr and a
`result.json` report containing Native and Devtools distribution hashes. A
timeout is a failure and kills only the launched Native process group. Use the
latest Native binary provided for acceptance; older builds with mismatched
session/bootstrap fields do not count as final evidence.
