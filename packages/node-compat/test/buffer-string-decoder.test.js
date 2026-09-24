import test from "node:test";
import assert from "node:assert/strict";
import { Buffer } from "../src/buffer.js";
import { StringDecoder } from "../src/string_decoder.js";

const runtime = globalThis[Symbol.for("niva.node-compat.runtime")];

test("Buffer supports Node base64url across constructors, writes, search, and byteLength", () => {
  assert.equal(Buffer.isEncoding("BASE64URL"), true);
  assert.equal(Buffer.from("Zm9v", "base64url").toString(), "foo");
  assert.equal(Buffer.from([0xfb, 0xff]).toString("base64url"), "-_8");
  assert.equal(Buffer.byteLength("Zm9v", "base64url"), 3);

  const target = Buffer.alloc(4);
  assert.equal(target.write("Zm9v", 0, "base64url"), 3);
  assert.equal(target.toString(), "foo\0");
  assert.equal(Buffer.from("foo").indexOf("Zm9v", "base64url"), 0);
  assert.equal(Buffer.from("foo").indexOf(Buffer.from("Zm9v", "base64url")), 0);
});

test("Buffer preserves Node argument errors and typed-array copyBytesFrom semantics", () => {
  assert.throws(() => Buffer.byteLength(32), {
    code: "ERR_INVALID_ARG_TYPE",
    name: "TypeError",
    message: "The \"string\" argument must be of type string or an instance of Buffer or ArrayBuffer. Received type number (32)",
  });
  assert.throws(() => Buffer.alloc(1).copy(), {
    code: "ERR_INVALID_ARG_TYPE",
    name: "TypeError",
    message: "The \"target\" argument must be an instance of Buffer or Uint8Array. Received undefined",
  });

  const words = new Uint16Array([0x1234, 0xffff]);
  const bytes = Buffer.copyBytesFrom(words, 1, 4);
  words[1] = 0;
  assert.deepEqual([...bytes], [0xff, 0xff]);
});

test("Buffer fill exposes a checked Niva primitive to the internal test binding", () => {
  const buffer = Buffer.alloc(1);
  assert.throws(() => runtime.bufferBinding.fill(buffer, 1, -1, 0, 1), { code: "ERR_OUT_OF_RANGE" });
  assert.throws(() => runtime.bufferBinding.fill(buffer, 1, 1, -2, 1), { code: "ERR_OUT_OF_RANGE" });
  assert.throws(() => buffer.fill("a", 0, 2), { code: "ERR_OUT_OF_RANGE" });
});

test("StringDecoder matches Node malformed UTF-8 and base64url chunk boundaries", () => {
  const utf8 = new StringDecoder("utf8");
  assert.equal(utf8.write(Buffer.from("f0b8", "hex")), "");
  assert.equal(utf8.lastNeed, 2);
  assert.equal(utf8.lastTotal, 4);
  assert.equal(utf8.write(Buffer.from("41", "hex")), "\ufffdA");
  assert.equal(utf8.end(), "");

  const invalidLead = new StringDecoder("utf8");
  assert.equal(invalidLead.write(Buffer.from("f69b", "hex")), "");
  assert.equal(invalidLead.write(Buffer.from("d1", "hex")), "\ufffd\ufffd");
  assert.equal(invalidLead.end(), "\ufffd");

  const base64url = new StringDecoder("base64url");
  assert.equal(base64url.write(Buffer.from([0xfb])), "");
  assert.equal(base64url.write(Buffer.from([0xff])), "");
  assert.equal(base64url.end(), "-_8");
});

test("StringDecoder reports Node type and encoding errors", () => {
  assert.throws(() => new StringDecoder("unknown"), {
    code: "ERR_UNKNOWN_ENCODING",
    name: "TypeError",
    message: "Unknown encoding: unknown",
  });
  assert.throws(() => new StringDecoder("utf8").write(null), {
    code: "ERR_INVALID_ARG_TYPE",
    name: "TypeError",
    message: "The \"buf\" argument must be an instance of Buffer, TypedArray, or DataView. Received null",
  });
});
