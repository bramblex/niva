import test from "node:test";
import assert from "node:assert/strict";
import nodeBuffer from "node:buffer";
import nodeCrypto from "node:crypto";
import nodePath from "node:path";
import nodeQuerystring from "node:querystring";
import nodeUrl from "node:url";
import nodeUtil from "node:util";
import nodeZlib from "node:zlib";
import bufferModule from "../src/buffer.js";
import stream from "../src/stream.js";
import streamPromises from "../src/stream-promises.js";
import crypto from "../src/crypto.js";
import zlib from "../src/zlib.js";
import events from "../src/events.js";
import util from "../src/util.js";
import assertCompat from "../src/assert.js";
import path, { posix, win32 } from "../src/path.js";
import querystring from "../src/querystring.js";
import url from "../src/url.js";
import stringDecoder from "../src/string_decoder.js";
import timers from "../src/timers.js";

const { Buffer } = bufferModule;
const runtime = globalThis[Symbol.for("niva.node-compat.runtime")];

 test("Buffer selected methods match Node views, encodings, and validation", () => {
  const values = ["", "hello 🌍", "00ff10", "AQIDBA==", "café", "snow"];
  const encodings = ["utf8", "utf8", "hex", "base64", "latin1", "utf16le"];
  values.forEach((value, index) => {
    const ours = Buffer.from(value, encodings[index]);
    const expected = nodeBuffer.Buffer.from(value, encodings[index]);
    assert.deepEqual([...ours], [...expected]);
    assert.equal(ours.toString(encodings[index]), expected.toString(encodings[index]));
  });
  assert.deepEqual([...Buffer.from(Uint16Array.of(0x1234))], [...nodeBuffer.Buffer.from(Uint16Array.of(0x1234))]);
  assert.equal(Buffer.isEncoding("base64url"), nodeBuffer.Buffer.isEncoding("base64url"));
  const bom = Buffer.from([0xef, 0xbb, 0xbf, 0x61]);
  assert.equal(bom.toString(), nodeBuffer.Buffer.from(bom).toString());
  assert.throws(() => Buffer.alloc(2).write(12), TypeError);

  const viewSource = Buffer.from("abcd");
  const slice = viewSource.slice(1, 3);
  slice[0] = 0x78;
  assert.equal(viewSource.toString(), "axcd");
  const subarray = viewSource.subarray(2, 4);
  subarray[0] = 0x79;
  assert.equal(viewSource.toString(), "axyd");

  const source = Buffer.from("abcd");
  const target = Buffer.alloc(4);
  assert.equal(source.copy(target, 1, 1, 4), nodeBuffer.Buffer.from("abcd").copy(nodeBuffer.Buffer.alloc(4), 1, 1, 4));
  assert.throws(() => source.copy(target, -1), { code: "ERR_OUT_OF_RANGE" });
  assert.throws(() => source.copy(target, 1, -2, 4), { code: "ERR_OUT_OF_RANGE" });
  assert.equal(Buffer.from("abcd").indexOf(""), 0);
  assert.equal(Buffer.from("abcd").lastIndexOf(""), 4);
});

test("stream classes, finished, callback pipeline, and Promise pipeline follow Node returns", async () => {
  assert.equal(typeof stream.Readable, "function");
  assert.equal(typeof stream.Writable, "function");
  assert.equal(typeof stream.Duplex, "function");
  assert.equal(typeof stream.Transform, "function");
  assert.equal(typeof stream.PassThrough, "function");
  assert.equal(typeof stream.finished, "function");
  assert.equal(typeof streamPromises.finished, "function");

  const chunks = [];
  const source = stream.Readable.from([Buffer.from("a"), Buffer.from("b")]);
  const destination = new stream.Writable({ write(chunk, _encoding, callback) { chunks.push(Buffer.from(chunk)); callback(); } });
  assert.equal(await streamPromises.pipeline(source, destination), undefined);
  assert.equal(Buffer.concat(chunks).toString(), "ab");

  const callbackSource = stream.Readable.from([Buffer.from("callback")]);
  const callbackDestination = new stream.PassThrough();
  const finished = new Promise((resolve, reject) => {
    callbackDestination.on("data", (chunk) => chunks.push(Buffer.from(chunk)));
    callbackDestination.on("error", reject);
    const returned = stream.pipeline(callbackSource, callbackDestination, (error) => error ? reject(error) : resolve());
    assert.equal(returned, callbackDestination);
  });
  await finished;

  const failingSource = new stream.Readable({ read() { this.destroy(new Error("source failed")); } });
  const failingDestination = new stream.Writable({ write(_chunk, _encoding, callback) { callback(); } });
  await assert.rejects(streamPromises.pipeline(failingSource, failingDestination), /source failed/);
  assert.equal(failingDestination.destroyed, true);
});

test("crypto hash, HMAC, random, KDF and timingSafeEqual use synchronous Node shapes", async () => {
  for (const algorithm of ["md5", "sha1", "sha256", "sha384", "sha512"]) {
    const actual = crypto.createHash(algorithm).update("chunk ").update("boundary").digest("hex");
    const expected = nodeCrypto.createHash(algorithm).update("chunk boundary").digest("hex");
    assert.equal(actual, expected);
  }
  const hmac = crypto.createHmac("sha256", "secret").update("message").digest("hex");
  assert.equal(hmac, nodeCrypto.createHmac("sha256", "secret").update("message").digest("hex"));
  const hash = crypto.createHash("sha256");
  const raw = hash.update("data").digest();
  assert.equal(Buffer.isBuffer(raw), true);
  assert.throws(() => hash.digest(), { code: "ERR_CRYPTO_HASH_FINALIZED" });

  const random = crypto.randomBytes(24);
  assert.equal(Buffer.isBuffer(random), true);
  assert.equal(random.length, 24);
  assert.match(crypto.randomUUID(), /^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/i);
  assert.throws(() => crypto.randomBytes(-1), RangeError);
  let callbackRan = false;
  const returned = crypto.randomBytes(8, (error, value) => {
    assert.ifError(error);
    assert.equal(value.length, 8);
    callbackRan = true;
  });
  assert.equal(returned, undefined);
  assert.equal(callbackRan, false);
  await new Promise((resolve) => setTimeout(resolve, 0));
  assert.equal(callbackRan, true);

  const pbkdf = crypto.pbkdf2Sync("password", "salt", 2, 24, "sha256");
  assert.deepEqual([...pbkdf], [...nodeCrypto.pbkdf2Sync("password", "salt", 2, 24, "sha256")]);
  const pbkdfAsync = await new Promise((resolve, reject) => crypto.pbkdf2("password", "salt", 2, 24, "sha256", (error, value) => error ? reject(error) : resolve(value)));
  assert.deepEqual([...pbkdfAsync], [...nodeCrypto.pbkdf2Sync("password", "salt", 2, 24, "sha256")]);
  const options = { N: 16, r: 1, p: 1, maxmem: 1024 * 1024 };
  const scrypt = await new Promise((resolve, reject) => crypto.scrypt("password", "salt", 16, options, (error, value) => error ? reject(error) : resolve(value)));
  assert.deepEqual([...scrypt], [...nodeCrypto.scryptSync("password", "salt", 16, options)]);

  const originalNiva = globalThis.Niva;
  const calls = [];
  globalThis.Niva = { callSync(method, args) {
    calls.push(method);
    return nodeCrypto.timingSafeEqual(Buffer.from(args[0], "base64"), Buffer.from(args[1], "base64"));
  } };
  try {
    assert.equal(crypto.timingSafeEqual(Buffer.from([1, 2]), Buffer.from([1, 2])), true);
    assert.equal(crypto.timingSafeEqual(Buffer.from([1, 2]), Buffer.from([1, 3])), false);
    assert.deepEqual(calls, ["os.timingSafeEqual", "os.timingSafeEqual"]);
    assert.throws(() => crypto.timingSafeEqual(Buffer.from([1]), Buffer.from([1, 2])), { code: "ERR_CRYPTO_TIMING_SAFE_EQUAL_LENGTH" });
  } finally {
    if (originalNiva === undefined) delete globalThis.Niva;
    else globalThis.Niva = originalNiva;
  }
});

test("zlib sync calls and callback APIs exchange Node Buffers", async () => {
  const input = Buffer.from("Niva compression\n".repeat(30));
  const compressed = zlib.gzipSync(input, { level: 9 });
  assert.equal(Buffer.isBuffer(compressed), true);
  assert.deepEqual([...zlib.gunzipSync(compressed)], [...input]);
  assert.deepEqual([...nodeZlib.gunzipSync(compressed)], [...input]);
  const asyncCompressed = await new Promise((resolve, reject) => {
    const result = zlib.gzip(input, { level: 6 }, (error, value) => error ? reject(error) : resolve(value));
    assert.equal(result, undefined);
  });
  assert.deepEqual([...zlib.gunzipSync(asyncCompressed)], [...input]);
  const asyncPlain = await new Promise((resolve, reject) => zlib.gunzip(asyncCompressed, (error, value) => error ? reject(error) : resolve(value)));
  assert.deepEqual([...asyncPlain], [...input]);
  assert.throws(() => zlib.gunzipSync(Buffer.from("invalid")), { code: "Z_DATA_ERROR" });
});

test("EventEmitter captureRejections and top-level once/on handle errors and abort", async () => {
  const emitter = new events.EventEmitter({ captureRejections: true });
  const rejected = new Promise((resolve) => {
    emitter.once("error", resolve);
    emitter.emit("data");
  });
  emitter.on("data", () => Promise.reject(new Error("captured rejection")));
  // A second emission is needed because the error listener above was registered before data.
  emitter.emit("data");
  assert.match((await rejected).message, /captured rejection/);

  const onceEmitter = new events.EventEmitter();
  const oncePromise = events.once(onceEmitter, "ready");
  onceEmitter.emit("ready", "a", "b");
  assert.deepEqual(await oncePromise, ["a", "b"]);
  const failed = new events.EventEmitter();
  const pending = events.once(failed, "ready");
  failed.emit("error", new Error("failed"));
  await assert.rejects(pending, /failed/);

  const iterableEmitter = new events.EventEmitter();
  const iterable = events.on(iterableEmitter, "data", { close: ["close"] });
  iterableEmitter.emit("data", 1);
  iterableEmitter.emit("close");
  assert.deepEqual(await iterable.next(), { value: [1], done: false });
  assert.deepEqual(await iterable.next(), { value: undefined, done: true });

  const controller = new AbortController();
  const abortIterable = events.on(new events.EventEmitter(), "data", { signal: controller.signal });
  controller.abort();
  await assert.rejects(abortIterable.next(), { name: "AbortError", code: "ABORT_ERR" });
});

test("util.inspect options and deep strict built-in comparisons follow Node", () => {
  const value = { b: 2, a: { x: 1 } };
  assert.equal(util.inspect(value, { sorted: true }), nodeUtil.inspect(value, { sorted: true }));
  assert.equal(util.inspect([1, 2], { maxArrayLength: 1 }), nodeUtil.inspect([1, 2], { maxArrayLength: 1 }));
  assert.equal(util.inspect(value, { breakLength: 5 }), nodeUtil.inspect(value, { breakLength: 5 }));
  assert.equal(util.isDeepStrictEqual(new Error("x", { cause: { a: 1 } }), new Error("x", { cause: { a: 2 } })), false);
  assert.equal(util.isDeepStrictEqual(Object.assign(/x/g, { lastIndex: 1 }), /x/g), false);
  assert.equal(util.isDeepStrictEqual(new Date(NaN), new Date(NaN)), false);
  assert.equal(util.isDeepStrictEqual(new Set([1, 2]), new Set([2, 1])), true);
});

test("assertions resolve with Node return values", async () => {
  assert.equal(assertCompat.strictEqual(1, 1), undefined);
  assert.equal(assertCompat.throws(() => { throw new Error("x"); }), undefined);
  assert.equal(await assertCompat.rejects(Promise.reject(new Error("x"))), undefined);
  assert.equal(await assertCompat.doesNotReject(Promise.resolve(1)), undefined);
});

test("path glob syntax, dynamic cwd and file URL platform options match Node", () => {
  const globs = [
    ["a.js", "*.js"], ["dir/a.js", "*.js"], ["dir/a.js", "**/*.js"], [".env", "*"],
    [".env", ".*"], ["file7.txt", "file?.txt"], ["foo.txt", "foo[!a].txt"],
    ["foo.txt", "foo{.txt,.js}"], ["foo.js", "!(foo).js"], ["bar.js", "!(foo).js"],
    ["foo.js", "foo.+(js|ts)"], ["a/b", "a/**/b"], ["a/x/b", "a/**/b"],
    ["foo/", "foo"], ["foo", "foo/"], ["", "**"], ["src/.hidden.js", "src/**/*.js"],
  ];
  for (const [candidate, pattern] of globs) {
    assert.equal(posix.matchesGlob(candidate, pattern), nodePath.posix.matchesGlob(candidate, pattern), `${candidate} vs ${pattern}`);
  }

  const originalNiva = globalThis.Niva;
  const originalRuntimeCallSync = runtime.callSync;
  let cwd = "/first";
  globalThis.Niva = { callSync(method, args) { assert.equal(method, "process.currentDir"); assert.deepEqual(args, []); return cwd; } };
  runtime.callSync = (niva, method, args) => niva.callSync(method, args);
  try {
    const dynamicPath = runtime.createPathModule();
    assert.equal(dynamicPath.resolve("item"), nodePath.posix.resolve("/first", "item"));
    cwd = "/second";
    assert.equal(dynamicPath.resolve("item"), nodePath.posix.resolve("/second", "item"));
  } finally {
    runtime.callSync = originalRuntimeCallSync;
    if (originalNiva === undefined) delete globalThis.Niva;
    else globalThis.Niva = originalNiva;
  }

  for (const file of ["C:\\Program Files\\a b.txt", "\\\\server\\share\\a b.txt"]) {
    assert.equal(url.pathToFileURL(file, { windows: true }).href, nodeUrl.pathToFileURL(file, { windows: true }).href);
    const fileURL = nodeUrl.pathToFileURL(file, { windows: true });
    assert.equal(url.fileURLToPath(fileURL, { windows: true }), nodeUrl.fileURLToPath(fileURL, { windows: true }));
  }
});

test("querystring and StringDecoder retain custom codecs and split multibyte state", () => {
  const input = "a=one+two&x=1&x=2&__proto__=safe";
  assert.deepEqual(Object.fromEntries(Object.entries(querystring.parse(input))), Object.fromEntries(Object.entries(nodeQuerystring.parse(input))));
  assert.equal(querystring.stringify({ a: ["one two", "x"], empty: null }), nodeQuerystring.stringify({ a: ["one two", "x"], empty: null }));
  const decoder = new stringDecoder.StringDecoder("utf8");
  assert.equal(decoder.write(Buffer.from([0xf0, 0x9f])), "");
  assert.equal(decoder.write(Buffer.from([0x8c, 0x8d])), "🌍");
  assert.equal(decoder.end(), "");
});

test("timers module returns cancellable Node-shaped handles without a global process", async () => {
  const timeout = timers.setTimeout(() => {}, 10000);
  assert.equal(typeof timeout.ref, "function");
  assert.equal(typeof timeout.unref, "function");
  assert.equal(typeof timeout.hasRef, "function");
  assert.equal(timeout.hasRef(), true);
  assert.equal(timeout.unref(), timeout);
  assert.equal(timeout.hasRef(), false);
  assert.equal(timeout.ref(), timeout);
  assert.equal(timeout.refresh(), timeout);
  timers.clearTimeout(timeout);

  const immediate = timers.setImmediate(() => assert.fail("cleared immediate should not run"));
  assert.equal(typeof immediate.hasRef, "function");
  timers.clearImmediate(immediate);
  await new Promise((resolve) => setTimeout(resolve, 5));
});
