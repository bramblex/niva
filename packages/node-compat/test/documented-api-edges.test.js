import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import vm from "node:vm";
import compatAssert from "../src/assert.js";
import strictAssert from "../src/assert-strict.js";
import { inspect } from "../src/util.js";

test("util.inspect honors depth and ANSI color options", () => {
  const nested = { outer: { inner: 1 } };
  assert.match(inspect(nested, { depth: 0 }), /outer: \[Object\]/);
  assert.match(inspect(nested, { depth: 1 }), /outer: \{ inner: 1 \}/);
  assert.match(inspect({ value: "green" }, { colors: true }), /\u001b\[32m'green'\u001b\[39m/);
});

const assertionCases = [
  ["ok", () => compatAssert.ok(1), () => compatAssert.ok(0)],
  ["fail", null, () => compatAssert.fail("expected failure")],
  ["equal", () => compatAssert.equal(1, "1"), () => compatAssert.equal(1, 2)],
  ["notEqual", () => compatAssert.notEqual(1, 2), () => compatAssert.notEqual(1, "1")],
  ["strictEqual", () => compatAssert.strictEqual(NaN, NaN), () => compatAssert.strictEqual(1, "1")],
  ["notStrictEqual", () => compatAssert.notStrictEqual(1, "1"), () => compatAssert.notStrictEqual(1, 1)],
  ["deepEqual", () => compatAssert.deepEqual({ n: 1 }, { n: "1" }), () => compatAssert.deepEqual({ n: 1 }, { n: 2 })],
  ["notDeepEqual", () => compatAssert.notDeepEqual({ n: 1 }, { n: 2 }), () => compatAssert.notDeepEqual({ n: 1 }, { n: "1" })],
  ["deepStrictEqual", () => compatAssert.deepStrictEqual({ n: 1 }, { n: 1 }), () => compatAssert.deepStrictEqual({ n: 1 }, { n: "1" })],
  ["notDeepStrictEqual", () => compatAssert.notDeepStrictEqual({ n: 1 }, { n: "1" }), () => compatAssert.notDeepStrictEqual({ n: 1 }, { n: 1 })],
  ["throws", () => compatAssert.throws(() => { throw new TypeError("boom"); }, TypeError), () => compatAssert.throws(() => {})],
  ["doesNotThrow", () => compatAssert.doesNotThrow(() => 3), () => compatAssert.doesNotThrow(() => { throw new Error("boom"); })],
  ["ifError", () => compatAssert.ifError(null), () => compatAssert.ifError(new Error("boom"))],
  ["match", () => compatAssert.match("abc", /b/), () => compatAssert.match("abc", /z/)],
  ["doesNotMatch", () => compatAssert.doesNotMatch("abc", /z/), () => compatAssert.doesNotMatch("abc", /b/)],
];

for (const [name, pass, fail] of assertionCases) {
  test(`assert.${name} accepts and rejects its documented inputs`, () => {
    if (pass) pass();
    assert.throws(fail, name === "ifError" ? /boom/ : { code: "ERR_ASSERTION" });
  });
}

test("assert.rejects and assert.doesNotReject check asynchronous outcomes", async () => {
  await compatAssert.rejects(Promise.reject(new TypeError("boom")), TypeError);
  await assert.rejects(compatAssert.rejects(Promise.resolve(1)), { code: "ERR_ASSERTION" });
  await compatAssert.doesNotReject(Promise.resolve(1));
  await assert.rejects(compatAssert.doesNotReject(Promise.reject(new Error("boom"))), { code: "ERR_ASSERTION" });
});

test("callable assert, AssertionError and assert/strict aliases keep their public semantics", () => {
  compatAssert(true);
  assert.throws(() => compatAssert(false), compatAssert.AssertionError);
  assert.equal(new compatAssert.AssertionError({ actual: 1, expected: 2 }).code, "ERR_ASSERTION");
  compatAssert.equal(1, "1");
  assert.throws(() => strictAssert.equal(1, "1"), { code: "ERR_ASSERTION" });
  strictAssert.deepEqual({ n: 1 }, { n: 1 });
  assert.throws(() => strictAssert.deepEqual({ n: 1 }, { n: "1" }), { code: "ERR_ASSERTION" });
  strictAssert(true);
});

test("crypto and zlib reject clearly when browser primitives are unavailable", async () => {
  const sandbox = { ArrayBuffer, DataView, Uint8Array, TextEncoder, TextDecoder, Symbol };
  const context = vm.createContext(sandbox);
  for (const name of ["bridge", "buffer", "crypto", "zlib"]) {
    const source = readFileSync(new URL(`../src/runtime/${name}.js`, import.meta.url), "utf8");
    vm.runInContext(source, context);
  }
  const runtime = vm.runInContext('globalThis[Symbol.for("niva.node-compat.runtime")]', context);
  await assert.rejects(runtime.crypto.randomBytes(4), { code: "ENOTSUP" });
  await assert.rejects(runtime.crypto.randomUUID(), { code: "ENOTSUP" });
  await assert.rejects(runtime.crypto.createHash("sha256").update("data").digest(), { code: "ENOTSUP" });
  await assert.rejects(runtime.zlib.gzip("data"), { code: "ENOTSUP" });
  await assert.rejects(runtime.zlib.gunzip("data"), { code: "ENOTSUP" });
});
