import test from "node:test";
import assert from "node:assert/strict";
import nodeBuffer from "node:buffer";
import nodeCrypto from "node:crypto";
import nodeOs from "node:os";
import nodePath from "node:path";
import nodeQuerystring from "node:querystring";
import nodeAssert from "node:assert/strict";
import path, { posix, win32, getCwd, setCwd } from "../src/path.js";
import os from "../src/os.js";
import childProcess from "../src/child_process.js";
import events, { EventEmitter, defaultMaxListeners } from "../src/events.js";
import querystring from "../src/querystring.js";
import bufferModule from "../src/buffer.js";
import urlModule from "../src/url.js";
import crypto from "../src/crypto.js";
import http from "../src/http.js";
import https from "../src/https.js";
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
} from "../src/assert.js";
import strictAssertDefault, { equal as strictEqualExport, deepEqual as strictDeepEqualExport } from "../src/assert-strict.js";
import streamModule from "../src/stream.js";
import streamPromises from "../src/stream-promises.js";
import { pipeline as exportedStreamPromisesPipeline } from "@niva/node-compat/stream/promises";

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

test("OS synchronous constants agree with the host path convention", () => {
  assert.equal(os.EOL, nodeOs.EOL);
  assert.equal(os.devNull, nodePath.sep === "\\" ? "\\\\.\\NUL" : "/dev/null");
  assert.equal(os.sep, nodePath.sep);
  assert.equal(os.delimiter, nodePath.delimiter);
});

function makeProcessNiva(onStart) {
  const calls = [];
  const sent = [];
  let nextId = 20;
  const niva = {
    stream(method, args, handlers) {
      const id = ++nextId;
      calls.push([method, args]);
      onStart?.(handlers);
      return {
        id,
        cancel() { calls.push(["cancel", id]); },
        promise: Promise.resolve().then(async () => {
          if (onStart?.blobs) {
            for (const [bytes, isStderr] of onStart.blobs) handlers.onBlob(new Blob([bytes]), isStderr);
          }
          return onStart?.result ?? { status: 0 };
        }),
      };
    },
    streamSend(id, data, end) { sent.push([id, [...data], end]); return true; },
  };
  return { niva, calls, sent, setBlobs(blobs) { onStart.blobs = blobs; }, setResult(result) { onStart.result = result; } };
}

test("child output setEncoding incrementally decodes split UTF-8 and validates encodings", async () => {
  const fake = makeProcessNiva(() => {});
  fake.setBlobs([
    [Uint8Array.from([65, 0xf0, 0x9f]), false],
    [Uint8Array.from([0x8c, 0x8d, 90]), false],
  ]);
  const child = runtime.createChildProcessModule(fake.niva).spawn("emit", []);
  assert.equal(child.stdout.setEncoding("utf8"), child.stdout);
  const output = [];
  child.stdout.on("data", (chunk) => output.push(chunk));
  await child.completion;
  assert.deepEqual(output, ["A", "🌍Z"]);
  assert.equal(output.join(""), "A🌍Z");
  assert.throws(() => child.stderr.setEncoding("latin1"), /Only UTF-8 and binary/);
});

test("detached spawn passes its mode to the bridge and reports the detached pid", async () => {
  const fake = makeProcessNiva(() => {});
  fake.setResult(4321);
  const child = runtime.createChildProcessModule(fake.niva).spawn("worker", [], { detached: true });
  assert.equal(child.stdin.write("ignored"), false);
  child.stdin.end();
  assert.deepEqual(await child.completion, { pid: 4321, status: null, detached: true });
  assert.equal(child.pid, 4321);
  assert.equal(fake.calls[0][1][2].detached, true);
  assert.deepEqual(fake.sent, []);
});

test("exec with encoding null returns binary Buffers", async () => {
  const fake = makeProcessNiva(() => {});
  fake.setBlobs([
    [Uint8Array.from([0, 0xff, 65]), false],
    [Uint8Array.from([0xfe]), true],
  ]);
  const child = runtime.createChildProcessModule(fake.niva).exec("binary-output", { encoding: null });
  const result = await child.result;
  assert.equal(Buffer.isBuffer(result.stdout), true);
  assert.equal(Buffer.isBuffer(result.stderr), true);
  assert.deepEqual([...result.stdout], [0, 0xff, 65]);
  assert.deepEqual([...result.stderr], [0xfe]);
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
  assert.deepEqual(decodedInputs, ["a b", "%41"]);
  assert.equal(parsed["decoded:a b"], "decoded:%41");
  assert.equal(querystring.stringify({ "a b": "x/y" }, "&", "=", {
    encodeURIComponent(value) { return `[${value}]`; },
  }), "[a b]=[x/y]");
});

test("Buffer allocation, sizing, encoding checks, comparisons, and copies match Node", () => {
  assert.equal(bufferModule.SlowBuffer, Buffer);
  assert.equal(bufferModule.INSPECT_MAX_BYTES, 50);
  assert.equal(bufferModule.kMaxLength, 0x7fffffff);
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

test("crypto reports its supported hashes and hashes SHA-384/SHA-512 data", async () => {
  const supported = crypto.getHashes();
  assert.deepEqual(supported, ["sha1", "sha256", "sha384", "sha512"]);
  supported.pop();
  assert.deepEqual(crypto.getHashes(), ["sha1", "sha256", "sha384", "sha512"]);
  for (const algorithm of ["sha384", "sha512"]) {
    const actual = await crypto.createHash(algorithm).update("chunk ").update("boundary").digest("hex");
    const expected = nodeCrypto.createHash(algorithm).update("chunk boundary").digest("hex");
    assert.equal(actual, expected, algorithm);
  }
  const raw = await crypto.createHash("sha384").update("bytes").digest();
  assert.equal(Buffer.isBuffer(raw), true);
  assert.equal(raw.toString("hex"), nodeCrypto.createHash("sha384").update("bytes").digest("hex"));
  assert.equal(
    await crypto.createHash("sha256").update("6869", "hex").digest("hex"),
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

test("http.post submits its body and IncomingMessage exposes buffer and response events", async () => {
  const { niva, calls } = makeHttpNiva([
    { status: 404, headers: { "content-type": "text/plain", "x-origin": "mock" }, body: "not found" },
    { status: 202, headers: {}, body: "queued" },
  ]);
  const api = runtime.createHttpModule("http", niva);
  const responseEvents = [];
  let responseFromEvent;
  const request = api.post("http://api.example.test/jobs", "payload", { headers: { "content-type": "text/plain" } });
  request.on("response", (response) => {
    responseFromEvent = response;
    responseEvents.push("response");
    response.setEncoding("utf8");
    response.on("data", (chunk) => responseEvents.push(["data", chunk]));
    response.on("end", () => responseEvents.push("end"));
  });
  const response = await request.response;
  assert.equal(responseFromEvent, response);
  assert.equal(response.statusCode, 404);
  assert.equal(response.statusMessage, "Not Found");
  assert.equal((await response.buffer()).toString(), "not found");
  assert.equal(response.complete, true);
  assert.deepEqual(responseEvents, ["response", ["data", "not found"], "end"]);
  assert.equal(new TextDecoder().decode(await response.arrayBuffer()), "not found");
  assert.equal(calls[0][1][0].method, "POST");
  assert.equal(calls[0][1][0].body, "payload");

  const objectPost = api.post({ hostname: "api.example.test", path: "/jobs" }, "second");
  assert.equal((await objectPost.result).statusCode, 202);
  assert.equal(calls[1][1][0].url, "http://api.example.test/jobs");
  assert.equal(calls[1][1][0].body, "second");
});

test("HTTP request accepts an options object followed by request overrides", async () => {
  const { niva, calls } = makeHttpNiva([{ status: 201, headers: {}, body: "created" }]);
  const api = runtime.createHttpModule("http", niva);
  let callbackResponse;
  const request = api.request(
    { hostname: "api.example.test", path: "/items", headers: { "x-base": "base" } },
    { method: "PATCH", headers: { "x-extra": "extra" } },
    (response) => { callbackResponse = response; },
  );
  assert.equal(request.method, "PATCH");
  assert.equal(request.getHeader("x-base"), "base");
  assert.equal(request.hasHeader("x-extra"), true);
  assert.deepEqual(Object.fromEntries(Object.entries(request.getHeaders())), { "x-base": "base", "x-extra": "extra" });
  request.setHeader("x-later", "later");
  assert.equal(request.getHeader("x-later"), "later");
  request.removeHeader("x-later");
  assert.equal(request.hasHeader("x-later"), false);
  request.end();
  const response = await request.response;
  assert.equal(callbackResponse, response);
  assert.equal(response.statusCode, 201);
  assert.equal(calls[0][1][0].method, "PATCH");
  assert.equal(calls[0][1][0].url, "http://api.example.test/items");
  assert.deepEqual(Object.fromEntries(Object.entries(calls[0][1][0].headers)), { "x-base": "base", "x-extra": "extra" });
  assert.throws(() => request.setHeader("x-late", "no"), { code: "ERR_HTTP_HEADERS_SENT" });
});

test("https.get and https.request use HTTPS and preserve request callbacks", async () => {
  const { niva, calls } = makeHttpNiva([
    { status: 200, headers: { "x-secure": "yes" }, body: "secure get" },
    { status: 204, headers: {}, body: "" },
  ]);
  const api = runtime.createHttpModule("https", niva);
  const getRequest = api.get("https://secure.example.test/read");
  assert.equal(await (await getRequest.result).text(), "secure get");
  const request = api.request({ hostname: "secure.example.test", path: "/write", method: "PUT" });
  request.end("body");
  assert.equal((await request.result).statusCode, 204);
  assert.equal(calls[0][1][0].method, "GET");
  assert.equal(calls[0][1][0].url, "https://secure.example.test/read");
  assert.equal(calls[1][1][0].method, "PUT");
  assert.equal(calls[1][1][0].body, "body");
});

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
  assert.equal(assertThrows(() => { throw new TypeError("bad"); }, TypeError).message, "bad");
  assertDoesNotThrow(() => 42);
  assert.equal(await assertRejects(() => { throw new Error("async"); }, /async/).then((error) => error.message), "async");
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

function makeWritable() {
  const events = new EventEmitter();
  const writable = {
    chunks: [],
    destroyError: undefined,
    on: events.on.bind(events),
    once: events.once.bind(events),
    off: events.off.bind(events),
    emit: events.emit.bind(events),
    write(value) { writable.chunks.push(value); return true; },
    end() { events.emit("finish"); },
    destroy(error) { writable.destroyError = error || true; },
  };
  return writable;
}

test("stream/promises pipeline exports the Promise API and rejects stream errors", async () => {
  assert.equal(streamPromises, streamModule.promises);
  assert.equal(streamPromises.pipeline, streamModule.pipeline);
  assert.equal(exportedStreamPromisesPipeline, streamModule.pipeline);

  const sourceEvents = new EventEmitter();
  const destination = makeWritable();
  const source = Object.assign(sourceEvents, {
    pipe(target) { target.write(Buffer.from("piped")); target.end(); return target; },
  });
  assert.equal(await exportedStreamPromisesPipeline(source, destination), destination);
  assert.equal(Buffer.from(destination.chunks[0]).toString(), "piped");

  const failingSource = Object.assign(new EventEmitter(), { pipe() {} });
  const failingDestination = makeWritable();
  failingSource.pipe = (target) => { target.emit("error", new Error("sink failed")); return target; };
  await assert.rejects(streamPromises.pipeline(failingSource, failingDestination), /sink failed/);
  assert.match(String(failingDestination.destroyError), /sink failed/);
});

function aString() { return "different"; }
