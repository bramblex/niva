import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";
import vm from "node:vm";

const script = readFileSync(
  new URL("../../../crates/niva/assets/initialize_script.js", import.meta.url),
  "utf8",
);

function createPage() {
  const timers = [];
  class FakeWebSocket {
    static OPEN = 1;

    constructor() {
      this.readyState = FakeWebSocket.OPEN;
      this.handlers = new Map();
      this.sent = [];
      FakeWebSocket.latest = this;
    }

    addEventListener(name, handler) { this.handlers.set(name, handler); }
    send(value) { this.sent.push(value); }
    emit(name, event = {}) { this.handlers.get(name)?.(event); }
  }

  const window = {
    addEventListener() {},
    __niva_ws_url: "ws://127.0.0.1:1/__niva_ws",
    __niva_token: "test-token",
    __niva_window_id: 0,
    __niva_compat_imports: { "test-module": "/should-not-reload-registered-module.js" },
  };
  window.top = window;
  vm.runInNewContext(script, {
    window,
    document: { readyState: "complete", addEventListener() {} },
    location: { origin: "niva://app" },
    WebSocket: FakeWebSocket,
    Blob, TextEncoder, TextDecoder, ArrayBuffer, Uint8Array, DataView, Map, URL,
    atob, btoa,
    setTimeout(callback) { timers.push(callback); return timers.length; }, clearTimeout() {},
    console: { log() {}, error() {} },
  });

  const socket = FakeWebSocket.latest;
  socket.emit("open");
  socket.sent.length = 0; // Drop the hello frame.
  const call = () => {
    const frame = socket.sent.find((value) => typeof value === "string" && JSON.parse(value).t === "call");
    assert.ok(frame, "expected a native bridge call");
    return JSON.parse(frame);
  };
  const text = (message) => socket.emit("message", { data: JSON.stringify(message) });
  const result = (id, data) => text({ t: "result", id, code: 0, data });
  const chunk = (id, bytes, flags = 0x03, seq = 1) => {
    const payload = typeof bytes === "string" ? new TextEncoder().encode(bytes) : bytes;
    const frame = new Uint8Array(18 + payload.length);
    const view = new DataView(frame.buffer);
    view.setUint8(0, 1);
    view.setUint8(1, flags);
    view.setUint32(6, id);
    view.setUint32(14, seq);
    frame.set(payload, 18);
    socket.emit("message", { data: frame.buffer });
  };
  const flushTimers = () => {
    while (timers.length) timers.shift()();
  };
  return { niva: window.Niva, api: window.Niva.api, socket, call, text, result, chunk, flushTimers };
}

test("Niva event subscriptions match exact, namespace, and wildcard names", () => {
  const page = createPage();
  const received = [];
  const exact = (name, data) => received.push(["exact", name, data.value]);
  const namespace = (name) => received.push(["namespace", name]);
  const wildcard = (name) => received.push(["wildcard", name]);
  page.niva.addEventListener("demo.ready", exact);
  page.niva.addEventListener("demo.*", namespace);
  page.niva.addEventListener("*", wildcard);
  page.text({ t: "event", name: "demo.ready", data: { value: 7 } });
  page.flushTimers();
  assert.deepEqual(received, [
    ["exact", "demo.ready", 7],
    ["namespace", "demo.ready"],
    ["wildcard", "demo.ready"],
  ]);

  page.niva.removeEventListener("demo.ready", exact);
  page.niva.removeAllEventListeners("demo.*");
  received.length = 0;
  page.text({ t: "event", name: "demo.ready", data: { value: 8 } });
  page.flushTimers();
  assert.deepEqual(received, [["wildcard", "demo.ready"]]);
});

test("Niva module registration, require, and import preserve module identity", async () => {
  const page = createPage();
  const module = { value: 42 };
  page.niva.registerModule("test-module", module);
  assert.equal(page.niva.require("test-module"), module);
  assert.equal(await page.niva.import("test-module"), module);
  assert.equal(page.niva.require("niva:fs"), page.api.fs);
  assert.throws(() => page.niva.require("missing-module"), /unknown module/);
});

test("Niva.call resolves successful replies and rejects native errors", async () => {
  const page = createPage();
  const success = page.niva.call("os.sep", []);
  const request = page.call();
  assert.equal(request.method, "os.sep");
  page.result(request.id, "/");
  assert.equal(await success, "/");

  page.socket.sent.length = 0;
  const failure = page.niva.call("os.sep", []);
  const rejected = page.call();
  page.text({ t: "result", id: rejected.id, code: -1, message: "native failure", data: null });
  await assert.rejects(failure, (error) => error.code === -1 && error.message === "native failure");
});

test("Niva.stream cancellation and streamSend use the call ID and binary frame", () => {
  const page = createPage();
  const stream = page.niva.stream("process.execStream", ["tool", [], null]);
  const request = page.call();
  assert.equal(stream.id, request.id);
  assert.equal(page.niva.streamSend(stream.id, "input", true), true);
  const frame = page.socket.sent.find((value) => value instanceof ArrayBuffer);
  assert.ok(frame);
  const view = new DataView(frame);
  assert.equal(view.getUint8(0), page.niva.bridgeVersion);
  assert.equal(view.getUint8(1) & 0x03, 0x03);
  assert.equal(view.getUint32(6), stream.id);
  assert.equal(new TextDecoder().decode(new Uint8Array(frame, 18)), "input");
  stream.cancel();
  assert.ok(page.socket.sent.some((value) => typeof value === "string" && JSON.parse(value).t === "cancel" && JSON.parse(value).id === stream.id));
});

test("fs.read returns streamed UTF-8 and base64 bytes", async () => {
  const page = createPage();
  const read = page.api.fs.read("notes.txt");
  const request = page.call();
  assert.equal(request.method, "fs.readStream");
  assert.deepEqual(request.args, ["notes.txt"]);
  page.chunk(request.id, "hello");
  page.result(request.id, { size: 5 });
  assert.equal(await read, "hello");

  const base64Page = createPage();
  const binary = base64Page.api.fs.read("bytes.bin", "base64");
  const binaryCall = base64Page.call();
  base64Page.chunk(binaryCall.id, Uint8Array.of(0, 255));
  base64Page.result(binaryCall.id, { size: 2 });
  assert.equal(await binary, "AP8=");
});

for (const [method, expectedArgs] of [
  ["write", ["notes.txt"]],
  ["append", ["notes.txt", true]],
]) {
  test(`fs.${method} sends content through writeStream`, async () => {
    const page = createPage();
    const operation = page.api.fs[method]("notes.txt", "hello");
    const request = page.call();
    assert.equal(request.method, "fs.writeStream");
    assert.deepEqual(request.args, expectedArgs);
    const frame = page.socket.sent.find((value) => value instanceof ArrayBuffer);
    assert.ok(frame, "expected a binary upload frame");
    assert.equal(new TextDecoder().decode(new Uint8Array(frame, 18)), "hello");
    page.result(request.id, { bytes: 5 });
    assert.equal(await operation, null);
  });
}

test("process.exec collects stdout and stderr into separate strings", async () => {
  const page = createPage();
  const operation = page.api.process.exec("tool", ["--version"]);
  const request = page.call();
  assert.equal(request.method, "process.execStream");
  assert.deepEqual(request.args, ["tool", ["--version"], {}]);
  page.chunk(request.id, "output", 0x03);
  page.chunk(request.id, "warning", 0x07, 2);
  page.result(request.id, { status: 0 });
  const response = await operation;
  assert.equal(response.status, 0);
  assert.equal(response.stdout, "output");
  assert.equal(response.stderr, "warning");
});

test("resource.read returns streamed resource bytes", async () => {
  const page = createPage();
  const operation = page.api.resource.read("config.json");
  const request = page.call();
  assert.equal(request.method, "resource.readStream");
  page.chunk(request.id, '{"ok":true}');
  page.result(request.id, { size: 11 });
  assert.equal(await operation, '{"ok":true}');
});

test("the public proxy keeps responding after 300 native calls", async () => {
  const page = createPage();
  for (let id = 1; id <= 300; id += 1) {
    page.socket.sent.length = 0;
    const operation = page.api.os.sep();
    const request = page.call();
    assert.equal(request.id, id);
    assert.equal(request.method, "os.sep");
    page.result(id, "/");
    assert.equal(await operation, "/");
  }
});

test("a file upload after many unary calls keeps its call ID and END frame", async () => {
  const page = createPage();
  for (let id = 1; id <= 18; id += 1) {
    page.socket.sent.length = 0;
    const operation = page.api.os.sep();
    page.result(id, "/");
    assert.equal(await operation, "/");
  }
  page.socket.sent.length = 0;
  const operation = page.api.fs.write("fixture.txt", "content");
  const call = page.call();
  assert.equal(call.id, 19);
  assert.equal(call.method, "fs.writeStream");
  const frame = page.socket.sent.find((value) => value instanceof ArrayBuffer);
  assert.ok(frame, "file upload must send a binary frame");
  const view = new DataView(frame);
  assert.equal(view.getUint32(6), 19);
  assert.equal(view.getUint8(1) & 0x03, 0x03);
  assert.equal(new TextDecoder().decode(new Uint8Array(frame, 18)), "content");
  page.result(19, { bytes: 7 });
  assert.equal(await operation, null);
});

test("chunk consumers cannot mutate the bytes retained for Blob subscribers", async () => {
  const page = createPage();
  let complete;
  const body = new Promise((resolve) => { complete = resolve; });
  const operation = page.niva.stream("test.bytes", [], {
    onChunk(bytes) { bytes.fill(0); },
    onBlob(blob) { complete(blob.text()); },
  });
  const request = page.call();
  page.chunk(request.id, "unchanged");
  page.result(request.id, null);
  await operation.promise;
  assert.equal(await body, "unchanged");
});

test("disconnect rejects connection-owned calls rather than replaying them", async () => {
  const page = createPage();
  const unary = page.niva.call("os.sep", []);
  const stream = page.niva.stream("socket.tcpConnect", [{host: "example.test", port: 80}]);
  const unaryRejected = assert.rejects(unary, error => error.data.code === "ECONNRESET");
  const streamRejected = assert.rejects(stream.promise, error => error.data.code === "ECONNRESET");
  page.socket.emit("close");
  await Promise.all([unaryRejected, streamRejected]);
  assert.equal(page.niva.streamSend(stream.id, new Uint8Array([1]), false), false);
});

test('module factories initialize on first require/import, memoize and retry failures', async () => {
  const { niva } = createPage();
  let calls = 0;
  const value = {};
  niva.registerModuleFactory('lazy', () => { calls++; return value; });
  assert.equal(calls, 0);
  assert.equal(await niva.import('lazy'), value);
  assert.equal(niva.require('lazy'), value);
  assert.equal(calls, 1);
  niva.registerModule('lazy', 7);
  assert.equal(niva.require('lazy'), 7);
  let failures = 0;
  niva.registerModuleFactory('retry', () => { if (!failures++) throw new Error('retry'); return value; });
  await assert.rejects(niva.import('retry'), /retry/);
  assert.equal(niva.require('retry'), value);
  assert.equal(failures, 2);
});
