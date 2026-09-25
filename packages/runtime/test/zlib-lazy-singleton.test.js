import "./setup-runtime.js";
import test from "node:test";
import assert from "node:assert/strict";

test("zlib is initialized on first product access and shared by CJS and ESM", async () => {
  const niva = globalThis.Niva;
  const bufferModule = niva.buffer;
  const BufferCompat = bufferModule.Buffer;
  const originalMaxLength = bufferModule.kMaxLength;
  const zlibDescriptor = Object.getOwnPropertyDescriptor(niva, "zlib");
  assert.equal(typeof zlibDescriptor?.get, "function");

  try {
    bufferModule.kMaxLength = 1;
    const productRequire = niva.module.createRequire("/niva-zlib-lazy-fixture.js");
    const cjsZlib = productRequire("zlib");
    assert.throws(() => cjsZlib.gzipSync("payload"), { code: "ERR_BUFFER_TOO_LARGE" });
    assert.strictEqual(productRequire("node:zlib"), cjsZlib);
    assert.strictEqual(niva.zlib, cjsZlib);

    const esmZlib = await import("../dist/esm/zlib.mjs");
    assert.strictEqual(esmZlib.default, cjsZlib);
    assert.strictEqual(esmZlib.gzip, cjsZlib.gzip);
  } finally {
    bufferModule.kMaxLength = originalMaxLength;
  }
});
