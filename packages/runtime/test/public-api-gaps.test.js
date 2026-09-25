import "./setup-runtime.js";
import test from "node:test";
import assert from "node:assert/strict";
import nodeBuffer from "node:buffer";
import nodeCrypto from "node:crypto";
import nodeOs from "node:os";
import nodePath from "node:path";
import nodeQuerystring from "node:querystring";
import nodeAssert from "node:assert/strict";
import path, { posix, win32, getCwd, setCwd } from "../dist/source/path.js";
import os from "../dist/source/os.js";
import childProcess from "../dist/source/child_process.js";
import events, { EventEmitter, defaultMaxListeners } from "../dist/source/events.js";
import querystring from "../dist/source/querystring.js";
import bufferModule from "../dist/source/buffer.js";
import urlModule from "../dist/source/url.js";
import crypto from "../dist/source/crypto.js";
import http from "../dist/source/http.js";
import https from "../dist/source/https.js";
import assertApi, {
  AssertionError,
  fail as assertFail,
  ok as assertOk,
  equal as assertEqual,
  notEqual as assertNotEqual,
  strictEqual as assertStrictEqual,
  notStrictEqual as assertNotStrictEqual,
  deepEqual as assertDeepEqual,
  notDeepEqual as assertNotDeepEqual,
  deepStrictEqual as assertDeepStrictEqual,
  notDeepStrictEqual as assertNotDeepStrictEqual,
  throws as assertThrows,
  doesNotThrow as assertDoesNotThrow,
  rejects as assertRejects,
  doesNotReject as assertDoesNotReject,
  ifError as assertIfError,
  match as assertMatch,
  doesNotMatch as assertDoesNotMatch,
  strict as strictAssert,
} from "../dist/source/assert.js";
import strictAssertDefault, { equal as strictEqualExport, deepEqual as strictDeepEqualExport } from "../dist/source/assert-strict.js";
import streamModule from "../dist/source/stream.js";
import streamPromises from "../dist/source/stream-promises.js";
import { pipeline as exportedStreamPromisesPipeline } from "@niva/runtime/stream/promises";

const { Buffer } = bufferModule;
const runtime = globalThis[Symbol.for("niva.node-compat.runtime")];

test("path.toNamespacedPath resolves drive and UNC paths without double-prefixing", () => {
  const originalCwd = getCwd();
  try {
    setCwd("C:\\workspace\\project");
    assert.equal(win32.toNamespacedPath("src\\index.js"), "\\\\?\\C:\\workspace\\project\\src\\index.js");
    assert.equal(win32.toNamespacedPath("\\\\server\\share\\folder\\file.txt"), "\\\\?\\UNC\\server\\share\\folder\\file.txt");
    assert.equal(win32.toNamespacedPath("\\\\?\\C:\\long\\path"), "\\\\?\\C:\\long\\path");
    assert.equal(win32.toNamespacedPath(12), 12);
    assert.equal(posix.toNamespacedPath("/tmp/file.txt"), "/tmp/file.txt");
  } finally {
    setCwd(originalCwd);
  }
  assert.equal(path.toNamespacedPath, path.sep === "\\" ? path.win32.toNamespacedPath : path.posix.toNamespacedPath);
});

test("EventEmitter aliases, once raw listeners, listener filtering, and errorMonitor work", () => {
  assert.equal(defaultMaxListeners, 10);
  assert.equal(events.errorMonitor, EventEmitter.errorMonitor);
  const previousDefault = events.defaultMaxListeners;
  try {
    events.defaultMaxListeners = 3;
    assert.equal(new EventEmitter().getMaxListeners(), 3);
  } finally {
    events.defaultMaxListeners = previousDefault;
  }

  const emitter = new EventEmitter();
  const received = [];
  const regular = (value) => received.push(["regular", value]);
  const oneShot = (value) => received.push(["once", value]);
  assert.equal(emitter.addListener("data", regular), emitter);
  emitter.prependListener("data", (value) => received.push(["first", value]));
  emitter.once("data", oneShot);
  const raw = emitter.rawListeners("data");
  assert.equal(raw.length, 3);
  assert.equal(raw[2].listener, oneShot);
  assert.deepEqual(emitter.listeners("data"), [raw[0], raw[1], oneShot]);
  assert.equal(emitter.listenerCount("data", oneShot), 1);
  assert.equal(EventEmitter.listenerCount(emitter, "data"), 3);
  emitter.emit("data", 1);
  assert.equal(emitter.listenerCount("data", oneShot), 0);
  assert.equal(emitter.removeListener("data", regular), emitter);
  assert.deepEqual(received, [["first", 1], ["regular", 1], ["once", 1]]);

  const monitored = [];
  emitter.on(events.errorMonitor, (error) => monitored.push(error));
  const failure = new Error("observed");
  assert.throws(() => emitter.emit("error", failure), (error) => error === failure);
  assert.deepEqual(monitored, [failure]);
});

test("querystring aliases and custom codec options preserve their documented behavior", () => {
  assert.deepEqual(querystring.decode("a=one+two&x=1&x=2"), nodeQuerystring.parse("a=one+two&x=1&x=2"));
  assert.equal(querystring.unescape("a+b"), nodeQuerystring.unescape("a+b"));
  assert.equal(querystring.escape("a b"), nodeQuerystring.escape("a b"));
  assert.equal(querystring.encode({ a: [1, 2] }), nodeQuerystring.stringify({ a: [1, 2] }));
  assert.deepEqual(querystring.parse("a=1&b=2&c=3", "&", "=", { maxKeys: 0 }), Object.assign(Object.create(null), { a: "1", b: "2", c: "3" }));

  const decodedInputs = [];
  const parsed = querystring.parse("a+b=%41", "&", "=", {
    decodeURIComponent(value) { decodedInputs.push(value); return `decoded:${value}`; },
  });
  assert.deepEqual(decodedInputs, ["a%20b", "%41"]);
  assert.equal(parsed["decoded:a%20b"], "decoded:%41");
  assert.equal(querystring.stringify({ "a b": "x/y" }, "&", "=", {
    encodeURIComponent(value) { return `[${value}]`; },
  }), "[a b]=[x/y]");
});

test("Buffer allocation, sizing, encoding checks, comparisons, and copies match Node", () => {
  assert.equal(Buffer.isBuffer(bufferModule.SlowBuffer(3)), true);
  assert.deepEqual([...bufferModule.SlowBuffer(3)], [...nodeBuffer.SlowBuffer(3)]);
  assert.equal(bufferModule.INSPECT_MAX_BYTES, 50);
  assert.equal(bufferModule.kMaxLength, nodeBuffer.kMaxLength);
  assert.equal(Buffer.allocUnsafeSlow(2).length, 2);
  const unsafe = Buffer.allocUnsafe(5);
  assert.equal(unsafe.length, 5);
  assert.equal(Buffer.isBuffer(unsafe), true);
  unsafe.set([1, 2, 3, 4, 5]);
  assert.equal(unsafe.readUInt8(4), 5);
  assert.equal(Buffer.byteLength("🌍 café", "utf8"), nodeBuffer.Buffer.byteLength("🌍 café", "utf8"));
  assert.equal(Buffer.byteLength(Uint16Array.of(0x1234, 0x5678)), 4);
  for (const encoding of ["utf8", "UTF-16LE", "base64url", "latin1", "binary", "hex", "ascii", "ucs2"]) {
    assert.equal(Buffer.isEncoding(encoding), nodeBuffer.Buffer.isEncoding(encoding), encoding);
  }
  assert.equal(Buffer.isEncoding("rot13"), false);
  assert.equal(Buffer.compare(Buffer.from([1, 2]), Buffer.from([1, 3])), -1);

  const left = Buffer.from([1, 2, 3, 4]);
  const same = Buffer.from([1, 2, 3, 4]);
  assert.equal(left.equals(same), true);
  assert.equal(left.equals(Buffer.from([1, 2, 4])), false);
  assert.equal(left.compare(Buffer.from([2, 3]), 0, 2, 1, 3), 0);
  const target = Buffer.alloc(3);
  assert.equal(left.copy(target, 1, 1, 4), 2);
  assert.deepEqual([...target], [0, 2, 3]);
  assert.equal(left.includes(Buffer.from([2, 3])), true);
  assert.equal(Buffer.from("abc").includes("b"), true);
  assert.equal(left.includes(4), true);
  assert.equal(left.includes("24"), false);
  const writable = Buffer.from("aabbcc", "hex");
  assert.equal(writable.write("XY", 1, 1), 1);
  assert.deepEqual([...writable], [0xaa, 0x58, 0xcc]);
});

test("Buffer swaps and signed/unsigned integer accessors match Node byte-for-byte", () => {
  for (const [method, input] of [
    ["swap16", [0, 1, 2, 3]],
    ["swap32", [0, 1, 2, 3, 4, 5, 6, 7]],
    ["swap64", [0, 1, 2, 3, 4, 5, 6, 7]],
  ]) {
    const actual = Buffer.from(input);
    const expected = nodeBuffer.Buffer.from(input);
    actual[method]();
    expected[method]();
    assert.deepEqual([...actual], [...expected], method);
    assert.throws(() => Buffer.from([1, 2, 3])[method](), RangeError, method);
  }

  for (const [writeMethod, readMethod, width, value] of [
    ["writeUInt8", "readUInt8", 1, 255],
    ["writeUInt16LE", "readUInt16LE", 2, 0x1234],
    ["writeUInt16BE", "readUInt16BE", 2, 0xabcd],
    ["writeUInt32LE", "readUInt32LE", 4, 0xfedcba98],
    ["writeUInt32BE", "readUInt32BE", 4, 0x87654321],
    ["writeInt8", "readInt8", 1, -128],
    ["writeInt16LE", "readInt16LE", 2, -12345],
    ["writeInt16BE", "readInt16BE", 2, 23456],
    ["writeInt32LE", "readInt32LE", 4, -123456789],
    ["writeInt32BE", "readInt32BE", 4, 123456789],
  ]) {
    const actual = Buffer.alloc(width);
    const expected = nodeBuffer.Buffer.alloc(width);
    assert.equal(actual[writeMethod](value, 0), expected[writeMethod](value, 0), writeMethod);
    assert.deepEqual([...actual], [...expected], writeMethod);
    assert.equal(actual[readMethod](0), expected[readMethod](0), readMethod);
  }
});

test("Windows file URL helpers round-trip drive and UNC paths", () => {
  const originalSep = runtime.path.sep;
  const originalResolve = runtime.path.resolve;
  const originalCwd = getCwd();
  try {
    runtime.path.sep = "\\";
    runtime.path.resolve = win32.resolve;
    setCwd("C:\\workspace");

    const drivePath = "C:\\Program Files\\museum\\map.json";
    const driveUrl = urlModule.pathToFileURL(drivePath);
    assert.equal(driveUrl.href, "file:///C:/Program%20Files/museum/map.json");
    assert.equal(urlModule.fileURLToPath(driveUrl), drivePath);

    const uncPath = "\\\\museum-server\\shared\\floor plan\\map.json";
    const uncUrl = urlModule.pathToFileURL(uncPath);
    assert.equal(uncUrl.href, "file://museum-server/shared/floor%20plan/map.json");
    assert.equal(urlModule.fileURLToPath(uncUrl), uncPath);
  } finally {
    runtime.path.sep = originalSep;
    runtime.path.resolve = originalResolve;
    setCwd(originalCwd);
  }
});

test("crypto exposes the bundled hashes and Node-compatible synchronous digest", () => {
  const supported = crypto.getHashes();
  assert.deepEqual(supported, ["md5", "sha1", "sha256", "sha384", "sha512"]);
  supported.pop();
  assert.deepEqual(crypto.getHashes(), ["md5", "sha1", "sha256", "sha384", "sha512"]);
  for (const algorithm of ["sha384", "sha512"]) {
    const actual = crypto.createHash(algorithm).update("chunk ").update("boundary").digest("hex");
    const expected = nodeCrypto.createHash(algorithm).update("chunk boundary").digest("hex");
    assert.equal(actual, expected, algorithm);
  }
  const raw = crypto.createHash("sha384").update("bytes").digest();
  assert.equal(Buffer.isBuffer(raw), true);
  assert.equal(raw.toString("hex"), nodeCrypto.createHash("sha384").update("bytes").digest("hex"));
  assert.equal(
    crypto.createHash("sha256").update("6869", "hex").digest("hex"),
    nodeCrypto.createHash("sha256").update("hi").digest("hex"),
  );
});

function makeHttpNiva(responses) {
  const calls = [];
  let nextId = 0;
  return {
    calls,
    niva: {
      stream(method, args, handlers) {
        calls.push([method, args]);
        const response = responses.shift() || { status: 200, headers: {}, body: "" };
        return {
          id: ++nextId,
          cancel() {},
          promise: Promise.resolve().then(() => {
            handlers.onEvent("head", { status: response.status, headers: response.headers || {} });
            if (response.body) handlers.onBlob(new Blob([response.body]));
            return { status: response.status };
          }),
        };
      },
    },
  };
}

test("assert exports cover negative checks, regex checks, and sync/async no-throw helpers", async () => {
  assert.equal(assertApi.strict, strictAssert);
  assert.equal(strictAssertDefault, strictAssert);
  assertOk(true);
  assertEqual(1, "1");
  assertNotEqual(1, 2);
  assertStrictEqual(NaN, NaN);
  assertNotStrictEqual(1, "1");
  assertDeepEqual({ a: 1 }, { a: "1" });
  assertNotDeepEqual({ a: 1 }, { a: aString() });
  assertDeepStrictEqual({ a: [1, 2] }, { a: [1, 2] });
  assertNotDeepStrictEqual({ a: 1 }, { a: 2 });
  assert.equal(strictEqualExport, strictAssert.equal);
  assert.equal(strictDeepEqualExport, strictAssert.deepEqual);
  assert.equal(assertThrows(() => { throw new TypeError("bad"); }, TypeError), undefined);
  assertDoesNotThrow(() => 42);
  await assert.rejects(assertRejects(() => { throw new Error("async"); }, /async/), { name: "Error", message: "async" });
  await assertDoesNotReject(Promise.resolve("ok"));
  assertIfError(null);
  assertIfError(undefined);
  assertMatch("museum map", /map/);
  assertDoesNotMatch("museum map", /floor/);
  assert.throws(() => assertFail("expected failure"), AssertionError);
  assert.throws(() => assertIfError(new Error("callback failure")), /callback failure/);
  assert.throws(() => strictAssertDefault.equal(1, "1"), AssertionError);
  assert.throws(() => nodeAssert.notEqual(1, 1), nodeAssert.AssertionError);
});

test("stream classes and pipeline match the Node Promise API", async () => {
  assert.equal(streamPromises, streamModule.promises);
  assert.equal(typeof streamModule.Readable, "function");
  assert.equal(typeof streamModule.Writable, "function");
  assert.equal(typeof streamModule.Transform, "function");
  assert.equal(typeof streamModule.PassThrough, "function");
  assert.equal(typeof streamModule.finished, "function");
  assert.equal(typeof streamPromises.pipeline, "function");
  assert.equal(typeof exportedStreamPromisesPipeline, "function");

  const chunks = [];
  const source = streamModule.Readable.from([Buffer.from("piped")]);
  const destination = new streamModule.Writable({
    write(chunk, _encoding, callback) { chunks.push(Buffer.from(chunk)); callback(); },
  });
  assert.equal(await exportedStreamPromisesPipeline(source, destination), undefined);
  assert.equal(Buffer.concat(chunks).toString(), "piped");

  const failingSource = new streamModule.Readable({
    read() { this.destroy(new Error("sink failed")); },
  });
  const failingDestination = new streamModule.Writable({ write(_chunk, _encoding, callback) { callback(); } });
  await assert.rejects(streamPromises.pipeline(failingSource, failingDestination), /sink failed/);
  assert.equal(failingDestination.destroyed, true);
});

function aString() { return "different"; }
