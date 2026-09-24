import test from "node:test";
import assert from "node:assert/strict";
import crypto from "../src/crypto.js";

test("PBKDF2 accepts shared byte buffers and rejects invalid arguments with Node codes", () => {
  const shared = new SharedArrayBuffer(10);
  assert.equal(crypto.pbkdf2Sync(shared, "salt", 1, 8, "sha256").length, 8);
  assert.throws(() => crypto.pbkdf2Sync("password", "salt", 1, "8", "sha256"), {
    name: "TypeError",
    code: "ERR_INVALID_ARG_TYPE",
    message: 'The "keylen" argument must be of type number. Received type string (\'8\')',
  });
  assert.throws(() => crypto.pbkdf2Sync("password", "salt", 1, NaN, "sha256"), {
    name: "RangeError",
    code: "ERR_OUT_OF_RANGE",
    message: 'The value of "keylen" is out of range. It must be an integer. Received NaN',
  });
  assert.throws(() => crypto.pbkdf2Sync("password", "salt", 1, 8, null), {
    name: "TypeError",
    code: "ERR_INVALID_ARG_TYPE",
    message: 'The "digest" argument must be of type string. Received null',
  });
  assert.throws(() => crypto.pbkdf2Sync("password", "salt", 1, 8, "md55"), {
    name: "TypeError",
    code: "ERR_CRYPTO_INVALID_DIGEST",
    message: "Invalid digest: md55",
  });
});

test("randomUUID validates the documented disableEntropyCache option", () => {
  assert.match(crypto.randomUUID({ disableEntropyCache: true }), /^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/);
  assert.throws(() => crypto.randomUUID({ disableEntropyCache: "yes" }), {
    name: "TypeError",
    code: "ERR_INVALID_ARG_TYPE",
  });
});
