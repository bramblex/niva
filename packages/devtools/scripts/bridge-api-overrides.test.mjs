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

for (const [method, invoke, expected] of [
  ["get", (api) => api.http.get("https://example.test/a"), { method: "GET", url: "https://example.test/a", headers: null }],
  ["post", (api) => api.http.post("https://example.test/a", "payload"), { method: "POST", url: "https://example.test/a", body: "payload", headers: null }],
  ["request", (api) => api.http.request({ method: "PUT", url: "https://example.test/a" }), { method: "PUT", url: "https://example.test/a" }],
]) {
  test(`http.${method} returns response head and streamed body`, async () => {
    const page = createPage();
    const operation = invoke(page.api);
    const request = page.call();
    assert.equal(request.method, "http.requestStream");
    assert.deepEqual(request.args[0], expected);
    page.text({ t: "event", id: request.id, name: "head", data: { status: 201, headers: { "x-test": "yes" } } });
    page.chunk(request.id, "done");
    page.result(request.id, { status: 201 });
    const response = await operation;
    assert.equal(response.status, 201);
    assert.equal(response.headers["x-test"], "yes");
    assert.equal(response.body, "done");
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
