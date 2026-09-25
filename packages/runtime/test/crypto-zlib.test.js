import "./setup-runtime.js";
import test from "node:test";
import assert from "node:assert/strict";
import nodeCrypto from "node:crypto";
import nodeZlib from "node:zlib";
import crypto from "../dist/source/crypto.js";
import zlib from "../dist/source/zlib.js";
import bufferModule from "../dist/source/buffer.js";

const { Buffer } = bufferModule;

test("randomBytes and randomUUID use synchronous Node return shapes and callback overload", async () => {
  const random = crypto.randomBytes(40);
  assert.equal(Buffer.isBuffer(random), true);
  assert.equal(random.length, 40);
  assert.ok(random.some((byte) => byte !== 0));
  assert.match(crypto.randomUUID(), /^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/i);
  assert.throws(() => crypto.randomBytes(-1), RangeError);
  let called = false;
  assert.equal(crypto.randomBytes(4, (error, value) => {
    assert.ifError(error);
    assert.equal(value.length, 4);
    called = true;
  }), undefined);
  assert.equal(called, false);
  await new Promise((resolve) => setTimeout(resolve, 0));
  assert.equal(called, true);
});

test("hash, HMAC, PBKDF2 and scrypt use synchronous or callback Node contracts", async () => {
  const hash = crypto.createHash("sha-256").update("hello ").update(Buffer.from("world"));
  const result = hash.digest("hex");
  assert.equal(result, nodeCrypto.createHash("sha256").update("hello world").digest("hex"));
  assert.equal(crypto.createHash("sha1").update("x").digest("base64"), nodeCrypto.createHash("sha1").update("x").digest("base64"));
  assert.equal(crypto.createHash("md5").update("x").digest("hex"), nodeCrypto.createHash("md5").update("x").digest("hex"));
  assert.equal(crypto.createHmac("sha256", "secret").update("x").digest("hex"), nodeCrypto.createHmac("sha256", "secret").update("x").digest("hex"));
  assert.throws(() => crypto.createHash("unknown"), { code: "ERR_CRYPTO_HASH_UNSUPPORTED" });
  const finalized = crypto.createHash("sha256");
  finalized.digest();
  assert.throws(() => finalized.update("late"), { code: "ERR_CRYPTO_HASH_FINALIZED" });

  const actualPbkdf = crypto.pbkdf2Sync("password", "salt", 2, 24, "sha256");
  assert.deepEqual([...actualPbkdf], [...nodeCrypto.pbkdf2Sync("password", "salt", 2, 24, "sha256")]);
  const pbkdfCallback = await new Promise((resolve, reject) => crypto.pbkdf2("password", "salt", 2, 24, "sha256", (error, value) => error ? reject(error) : resolve(value)));
  assert.deepEqual([...pbkdfCallback], [...actualPbkdf]);
  const scryptOptions = { N: 16, r: 1, p: 1, maxmem: 1024 * 1024 };
  const scryptCallback = await new Promise((resolve, reject) => crypto.scrypt("password", "salt", 16, scryptOptions, (error, value) => error ? reject(error) : resolve(value)));
  assert.deepEqual([...scryptCallback], [...nodeCrypto.scryptSync("password", "salt", 16, scryptOptions)]);
});

test("gzipSync/gunzipSync and callback gzip/gunzip round-trip Node buffers", async () => {
  const original = Buffer.from("Niva compression\n".repeat(300));
  const compressed = zlib.gzipSync(original);
  assert.equal(Buffer.isBuffer(compressed), true);
  assert.deepEqual([...zlib.gunzipSync(compressed)], [...original]);
  assert.deepEqual([...nodeZlib.gunzipSync(compressed)], [...original]);

  const callbackGzip = await new Promise((resolve, reject) => {
    assert.equal(zlib.gzip(original, { level: 6 }, (error, result) => error ? reject(error) : resolve(result)), undefined);
  });
  const callbackGunzip = await new Promise((resolve, reject) => zlib.gunzip(callbackGzip, (error, result) => error ? reject(error) : resolve(result)));
  assert.deepEqual([...callbackGunzip], [...original]);
  assert.throws(() => zlib.gunzipSync(Buffer.from("invalid")), { code: "Z_DATA_ERROR" });
});
