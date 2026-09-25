import "./setup-runtime.js";
import test from "node:test";
import assert from "node:assert/strict";
import "../dist/source/runtime/bridge.js";

const runtime = globalThis[Symbol.for("niva.node-compat.runtime")];

test("sync bridge errors preserve native filesystem codes and bridge metadata", () => {
  const native = Object.assign(new Error("missing file"), {
    code: "NIVA_BRIDGE_ERROR",
    bridgeCode: -1,
    data: { code: "ENOENT", errno: 2, path: "/absent", syscall: "open" },
  });
  const niva = { bridge: { callSync() { throw native; } } };

  assert.throws(() => runtime.callSync(niva, "fs.node", []), (error) => {
    assert.equal(error, native);
    assert.equal(error.code, "ENOENT");
    assert.equal(error.bridgeCode, -1);
    assert.equal(error.errno, 2);
    assert.equal(error.path, "/absent");
    assert.equal(error.syscall, "open");
    return true;
  });
});

test("async bridge calls reject Error instances with decoded native codes", async () => {
  const niva = {
    bridge: {
      call() {
        return Promise.reject({ code: -1, message: "MODULE_NOT_FOUND: Cannot find module 'optional'", data: { code: "MODULE_NOT_FOUND" } });
      },
    },
  };

  await assert.rejects(runtime.call(niva, "module.resolve", ["optional", ""]), (error) => {
    assert.ok(error instanceof Error);
    assert.equal(error.code, "MODULE_NOT_FOUND");
    assert.equal(error.bridgeCode, -1);
    return true;
  });
});

test("nativeError recognizes explicit non-errno error prefixes", () => {
  const error = runtime.nativeError(Object.assign(new Error("MODULE_NOT_FOUND: missing"), { code: "NIVA_BRIDGE_ERROR" }));
  assert.equal(error.code, "MODULE_NOT_FOUND");
});
