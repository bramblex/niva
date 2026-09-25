import "./setup-runtime.js";
import test from "node:test";
import assert from "node:assert/strict";
import zlib from "../dist/source/zlib.js";
import bufferModule from "../dist/source/buffer.js";

const { Buffer } = bufferModule;

test("gunzip joins concatenated members and ignores trailing zero padding", async () => {
  const input = Buffer.concat([
    zlib.gzipSync("abc"),
    zlib.gzipSync("def"),
    Buffer.alloc(10),
  ]);
  assert.equal(zlib.gunzipSync(input).toString(), "abcdef");
  const asyncResult = await new Promise((resolve, reject) => {
    assert.equal(zlib.gunzip(input, (error, value) => error ? reject(error) : resolve(value)), undefined);
  });
  assert.equal(asyncResult.toString(), "abcdef");
});

test("gunzip reports a malformed following member and enforces maxOutputLength", async () => {
  const malformed = Buffer.concat([
    zlib.gzipSync("abc"),
    zlib.gzipSync("def"),
    Buffer.from([0x1f, 0x8b, 0xff, 0xff]),
    Buffer.alloc(10),
  ]);
  assert.throws(() => zlib.gunzipSync(malformed), /^Error: unknown compression method$/);
  await new Promise((resolve, reject) => zlib.gunzip(malformed, (error, value) => {
    try {
      assert.equal(error?.code, "Z_DATA_ERROR");
      assert.equal(error?.message, "unknown compression method");
      assert.equal(value, undefined);
      resolve();
    } catch (failure) { reject(failure); }
  }));

  const compressed = zlib.gzipSync("abcdef");
  assert.throws(() => zlib.gunzipSync(compressed, { maxOutputLength: 5 }), {
    name: "RangeError",
    code: "ERR_BUFFER_TOO_LARGE",
  });
});
