import assert from "node:assert/strict";
import test from "node:test";

import { resolveNodeCompatAssets } from "../src/build-scripts/node-compat-assets.mjs";

test("disabled config selects no NodeCompat files", () => {
  assert.deepEqual(resolveNodeCompatAssets(undefined), {
    enabled: false,
    modules: [],
    importmap: false,
    files: [],
  });
  assert.deepEqual(resolveNodeCompatAssets(false).files, []);
});

test("a module subset includes only its ESM import closure", () => {
  const selection = resolveNodeCompatAssets({ modules: ["path"], importmap: true });
  assert.deepEqual(selection.files, [
    "node-compat.js",
    "src/path.js",
    "src/runtime/bridge.js",
    "src/runtime/path.js",
  ]);
  assert.equal(selection.files.includes("src/fs.js"), false);
  assert.equal(selection.files.includes("src/runtime/fs.js"), false);
});

test("disabled static importmap still packages the selected ESM closure", () => {
  const selection = resolveNodeCompatAssets({ modules: ["fs"], importmap: false });
  assert.equal(selection.importmap, false);
  assert.deepEqual(selection.files, [
    "node-compat.js",
    "src/fs-promises.js",
    "src/fs.js",
    "src/runtime/bridge.js",
    "src/runtime/buffer.js",
    "src/runtime/fs.js",
    "src/runtime/path.js",
  ]);
});

test("subpath adapters are included with their owning module", () => {
  const selection = resolveNodeCompatAssets({ modules: ["assert", "stream"] });
  assert.ok(selection.files.includes("src/assert-strict.js"));
  assert.ok(selection.files.includes("src/stream-promises.js"));
  assert.ok(selection.files.includes("src/runtime/assert.js"));
  assert.ok(selection.files.includes("src/runtime/stream.js"));
  assert.equal(selection.files.includes("src/fs-promises.js"), false);
});

test("static importmap setting does not change the selected asset manifest", () => {
  const staticMap = resolveNodeCompatAssets({ modules: ["path", "fs"], importmap: true });
  const dynamicOnly = resolveNodeCompatAssets({ modules: ["path", "fs"], importmap: false });
  assert.deepEqual(dynamicOnly.files, staticMap.files);
});

test("boolean enablement uses the default module set and importmap", () => {
  const selection = resolveNodeCompatAssets(true);
  assert.equal(selection.enabled, true);
  assert.equal(selection.importmap, true);
  assert.ok(selection.files.includes("src/path.js"));
  assert.ok(selection.files.includes("src/zlib.js"));
});

test("unknown modules fail with a config error", () => {
  assert.throws(
    () => resolveNodeCompatAssets({ modules: ["not-a-builtin"] }),
    /Unknown nodeCompat module/,
  );
});
