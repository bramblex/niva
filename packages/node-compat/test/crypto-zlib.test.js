import test from "node:test";
import assert from "node:assert/strict";
import nodeCrypto from "node:crypto";
import nodeZlib from "node:zlib";
import crypto from "../src/crypto.js";
import zlib from "../src/zlib.js";
import bufferModule from "../src/buffer.js";

test("async random APIs return correctly shaped secure values", async () => {
  const random = await crypto.randomBytes(40);
  assert.equal(bufferModule.Buffer.isBuffer(random), true);
  assert.equal(random.length, 40);
  assert.ok(random.some((byte) => byte !== 0));
  assert.match(await crypto.randomUUID(), /^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/i);
  await assert.rejects(crypto.randomBytes(-1), RangeError);
});

test("Web Crypto hash supports chunked updates and Node encodings", async () => {
  const hash = crypto.createHash("sha-256").update("hello ").update(Buffer.from("world"));
  const result = await hash.digest("hex");
  assert.equal(result, nodeCrypto.createHash("sha256").update("hello world").digest("hex"));
  assert.equal(await crypto.createHash("sha1").update("x").digest("base64"), nodeCrypto.createHash("sha1").update("x").digest("base64"));
  assert.throws(() => crypto.createHash("md5"), { code: "ERR_CRYPTO_HASH_UNSUPPORTED" });
  const finalized = crypto.createHash("sha256");
  await finalized.digest();
  assert.throws(() => finalized.update("late"), { code: "ERR_CRYPTO_HASH_FINALIZED" });
});

test("gzip and gunzip round-trip bytes compatible with Node zlib", async () => {
  assert.equal(typeof CompressionStream, "function");
  assert.equal(typeof DecompressionStream, "function");
  const original = Buffer.from("Niva compression\n".repeat(300));
  const compressed = await zlib.gzip(original);
  assert.equal(bufferModule.Buffer.isBuffer(compressed), true);
  assert.equal(nodeZlib.gunzipSync(compressed).toString(), original.toString());
  const restored = await zlib.gunzip(nodeZlib.gzipSync(original));
  assert.deepEqual([...restored], [...original]);
});
