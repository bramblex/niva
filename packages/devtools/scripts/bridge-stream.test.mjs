import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { test } from "node:test";
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
      FakeWebSocket.latest = this;
    }
    addEventListener(name, callback) { this.handlers.set(name, callback); }
    send() {}
    emit(name, event = {}) { this.handlers.get(name)?.(event); }
  }
  const window = {
    addEventListener() {},
    __niva_ws_url: "ws://127.0.0.1:1/__niva_ws",
    __niva_token: "test-token",
    __niva_window_id: 0,
  };
  window.top = window;
  const sandbox = {
    window,
    document: { readyState: "complete", addEventListener() {} },
    location: { origin: "niva://app" },
    WebSocket: FakeWebSocket,
    Blob,
    TextEncoder,
    TextDecoder,
    ArrayBuffer,
    Uint8Array,
    DataView,
    Map,
    URL,
    setTimeout() {},
    clearTimeout() {},
    console: { log() {}, error() {} },
  };
  vm.runInNewContext(script, sandbox);
  const socket = FakeWebSocket.latest;
  socket.emit("open");
  return { niva: window.Niva, socket };
}

function binaryFrame(id, seq, flags, payload) {
  const frame = new Uint8Array(18 + payload.length);
  const view = new DataView(frame.buffer);
  view.setUint8(0, 1);
  view.setUint8(1, flags);
  view.setUint32(6, id);
  view.setUint32(14, seq);
  frame.set(payload, 18);
  return frame.buffer;
}

test("onChunk arrives before END while onBlob retains grouped output", async () => {
  const { niva, socket } = createPage();
  const chunks = [];
  const blobs = [];
  const stream = niva.stream("process.execStream", ["test", [], {}], {
    onChunk(bytes, stderr) { chunks.push({ bytes: [...bytes], stderr }); },
    onBlob(blob, stderr) { blobs.push({ blob, stderr }); },
  });

  socket.emit("message", { data: binaryFrame(stream.id, 1, 0x01, Uint8Array.of(65, 66)) });
  assert.deepEqual(chunks, [{ bytes: [65, 66], stderr: false }]);
  assert.equal(blobs.length, 0);

  socket.emit("message", { data: binaryFrame(stream.id, 2, 0x02, Uint8Array.of(67)) });
  assert.deepEqual(chunks, [
    { bytes: [65, 66], stderr: false },
    { bytes: [67], stderr: false },
  ]);
  assert.equal(blobs.length, 1);
  assert.equal(await blobs[0].blob.text(), "ABC");

  socket.emit("message", { data: binaryFrame(stream.id, 3, 0x07, Uint8Array.of(69)) });
  assert.deepEqual(chunks[2], { bytes: [69], stderr: true });
  assert.equal(await blobs[1].blob.text(), "E");
  assert.equal(blobs[1].stderr, true);
});
