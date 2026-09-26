import "./setup-runtime.js";
import test from "node:test";
import assert from "node:assert/strict";
import processFacade, { version as facadeVersion, versions as facadeVersions } from "../dist/source/process.js";

const runtime = globalThis[Symbol.for("niva.node-compat.runtime")];

test("Niva process and ESM facade distinguish Node compatibility target from Niva product version", () => {
  const process = globalThis.Niva.process;
  assert.strictEqual(processFacade, process);
  assert.equal(process.version, "v22.14.0");
  assert.equal(process.versions.node, "22.14.0");
  assert.equal(process.versions.nodeCompat, "22.14.0");
  assert.equal(process.versions.niva, "0.9.9");
  assert.equal(facadeVersion, process.version);
  assert.strictEqual(facadeVersions, process.versions);
});

test("process factory does not expose the Native product version as process.version", () => {
  const process = runtime.createProcessModule({
    bootstrap: {
      process: {
        version: "v0.9.9",
        versions: { niva: "0.9.9", node: "0.9.9", nodeCompat: "0.9.9" },
        argv: [],
        env: {},
      },
    },
    bridge: {},
  });

  try {
    assert.equal(process.version, "v22.14.0");
    assert.equal(process.versions.node, "22.14.0");
    assert.equal(process.versions.nodeCompat, "22.14.0");
    assert.equal(process.versions.niva, "0.9.9");
  } finally {
    process.stdin?.__nivaResourceOwner?.release();
  }
});

test("process.chdir updates the Native cwd through the async IPC API", async () => {
  const calls = [];
  const process = runtime.createProcessModule({
    bootstrap: { process: { version: "v0.9.9", versions: { niva: "0.9.9" }, argv: [], env: {} } },
    bridge: {
      call(method, args) { calls.push([method, args]); return Promise.resolve(null); },
      callSync() { throw new Error("process.chdir must not use synchronous XHR"); },
    },
  });

  const change = process.chdir("/tmp");
  assert.ok(change instanceof Promise);
  assert.equal(await change, undefined);
  assert.deepEqual(calls, [["process.setCurrentDir", ["/tmp"]]]);
  process.stdin?.__nivaResourceOwner?.release();
});
