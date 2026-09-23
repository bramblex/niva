import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";
import vm from "node:vm";

const script = readFileSync(
  new URL("../../../crates/niva/assets/initialize_script.js", import.meta.url),
  "utf8",
);

function createPage() {
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
    setTimeout() {}, clearTimeout() {},
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
  return { api: window.Niva.api, socket, call, text, result, chunk };
}

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
