import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import path from "node:path";
import vm from "node:vm";

const bootstrapSource = process.env.NIVA_BOOTSTRAP_SOURCE
  ? readFileSync(process.env.NIVA_BOOTSTRAP_SOURCE, "utf8")
  : readFileSync(new URL("../dist/bootstrap.js", import.meta.url), "utf8");
const fixtureRoot = new URL("./fixtures/cjs-loader/", import.meta.url);
const fixtureFiles = new Map([
  ["/app/cycle-a.cjs", readFileSync(new URL("cycle-a.cjs", fixtureRoot), "utf8")],
  ["/app/cycle-b.cjs", readFileSync(new URL("cycle-b.cjs", fixtureRoot), "utf8")],
  ["/app/package.json", readFileSync(new URL("package.json", fixtureRoot), "utf8")],
  ["/app/throw.cjs", "throw new Error('source path fixture');"],
]);

function bootPage({ injectCommonJs = true, injectEsm = false, webSocketThrows = 0, ipcReply, windowsIpc, fetchImpl, fakeTimers = false, local = true, userImportMap = false, iframeParent, origin = "https://niva.test", serverOrigin = "https://niva.test", ErrorConstructor, platform = "linux" } = {}) {
  const requests = [];
  const importMaps = [];
  const sockets = [];
  const pageListeners = new Map();
  let context;
  let remainingWebSocketThrows = webSocketThrows;
  function resolve(specifier, parent) {
    if (specifier === "#native-fs") return "node:fs";
    if (specifier === "#missing-builtin") return "node:inspector";
    const base = parent ? path.posix.dirname(parent) : "/app";
    const filename = path.posix.resolve(base, specifier);
    if (!fixtureFiles.has(filename)) throw new Error(`MODULE_NOT_FOUND: ${specifier}`);
    return filename;
  }
  class TestWebSocket {
    static OPEN = 1;
    constructor(url) {
      if (remainingWebSocketThrows > 0) { remainingWebSocketThrows -= 1; throw new Error("fixture WebSocket connection failed"); }
      this.url = url; this.readyState = 0; this.listeners = new Map(); this.sent = [];
      sockets.push(this);
    }
    addEventListener(name, listener) { this.listeners.set(name, listener); }
    send(value) { this.sent.push(value); }
    open() { this.readyState = 1; this.listeners.get("open")?.(); }
    receive(value) { this.listeners.get("message")?.({ data: value }); }
    close() { this.readyState = 3; this.listeners.get("close")?.(); }
  }
  function ipcResponse(request, value) {
    if (value && typeof value === "object" && typeof value.t === "string") {
      return { ...value, rid: request.rid, ...(value.id === undefined && request.id !== undefined ? { id: request.id } : {}) };
    }
    return { t: "result", rid: request.rid, ...(request.id === undefined ? {} : { id: request.id }), code: 0, data: value };
  }
  const webviewListeners = new Map();
  const mockWebview = { addEventListener(name, listener) { webviewListeners.set(name, listener); } };
  const mockIpc = windowsIpc ? { postMessage(text) {
    const request = JSON.parse(text);
    requests.push({ transport: "ipc", ...request });
    Promise.resolve(windowsIpc(request)).then((value) => {
      const response = ipcResponse(request, value);
      webviewListeners.get("message")?.({ data: JSON.stringify(response) });
    });
  } } : undefined;
  class TestXHR {
    open(method, url, async) { this.method = method; this.url = url; this.async = async; }
    setRequestHeader() {}
    send(text) {
      const envelope = JSON.parse(text);
      const [id, method, args] = envelope.request;
      requests.push({ method, args, async: this.async, token: envelope.token, url: this.url });
      let data;
      if (method === "module.resolve") data = resolve(args[0], args[1]);
      else if (method === "module.load") {
        const filename = resolve(args[0], args[1]);
        data = { filename, dirname: path.posix.dirname(filename), type: filename.endsWith(".json") ? "json" : "commonjs", source: fixtureFiles.get(filename) };
      } else if (method === "process.currentDir") data = "/app";
      else throw new Error(`Unexpected sync method ${method}`);
      this.status = 200;
      this.responseText = JSON.stringify([id, 0, "", data]);
    }
  }
  const initialMap = userImportMap ? {
    type: "importmap",
    textContent: JSON.stringify({ imports: { "node:fs": "https://user.test/fs.mjs" } }),
    replaceWith(element) { importMaps.push({ map: JSON.parse(element.textContent), nonce: element.nonce }); },
  } : null;
  const document = {
    createElement(name) { return { tagName: name, remove() {} }; },
    querySelector() { return initialMap; },
    head: {
      appendChild(element) { vm.runInContext(element.textContent, context); },
      prepend(element) { if (element.type === "importmap") importMaps.push({ map: JSON.parse(element.textContent), nonce: element.nonce }); },
    },
    documentElement: { appendChild(element) { vm.runInContext(element.textContent, context); }, prepend() {} },
  };
  let timerId = 0;
  const fakeTimerHandles = new Set();
  const setPageTimeout = fakeTimers ? function () { const id = ++timerId; fakeTimerHandles.add(id); return id; } : setTimeout;
  const clearPageTimeout = fakeTimers ? function (id) { fakeTimerHandles.delete(id); } : clearTimeout;
  const setPageInterval = fakeTimers ? function () { const id = ++timerId; fakeTimerHandles.add(id); return id; } : setInterval;
  const clearPageInterval = fakeTimers ? function (id) { fakeTimerHandles.delete(id); } : clearInterval;
  context = vm.createContext({
    console,
    ...(ErrorConstructor ? { Error: ErrorConstructor } : {}),
    document,
    location: { pathname: "/app/index.html", href: `${origin}/app/index.html`, origin },
    __niva_server_origin: iframeParent ? undefined : serverOrigin,
    __niva_window_id: iframeParent ? undefined : 4,
    __niva_node_bootstrap: iframeParent ? undefined : {
      nivaVersion: "v0.9.9",
      os: { platform, arch: "x64", homedir: "/home/test", tmpdir: "/tmp", EOL: "\n" },
      process: { version: "v0.9.9", versions: { niva: "0.9.9" }, argv: [], env: {}, stdioIsTTY: { stdin: true, stdout: false, stderr: true } },
    },
    __niva_runtime_config: iframeParent ? undefined : { injectCommonJs, injectEsm, nonce: "test-nonce" },
    webkit: ipcReply ? { messageHandlers: { nivaReply: { postMessage(text) {
      const request = JSON.parse(text);
      requests.push({ transport: "ipc", ...request });
      return Promise.resolve(ipcReply(request)).then((value) => JSON.stringify(ipcResponse(request, value)));
    } } } } : undefined,
    ...(windowsIpc ? { chrome: { webview: mockWebview }, ipc: mockIpc } : {}),
    addEventListener(name, listener) {
      const listeners = pageListeners.get(name) || [];
      listeners.push(listener);
      pageListeners.set(name, listeners);
    },
    postMessage(data, targetOrigin) {
      const targetOriginValue = context.location?.origin;
      if (targetOrigin !== "*" && targetOrigin !== targetOriginValue) return;
      const source = context.parent;
      const event = { data, origin: source?.location?.origin || targetOriginValue, source };
      for (const listener of (pageListeners.get("message") || []).slice()) listener(event);
    },
    WebSocket: TestWebSocket,
    ...(fetchImpl ? { fetch: fetchImpl } : {}),
    XMLHttpRequest: TestXHR,
    crypto: globalThis.crypto,
    performance: globalThis.performance,
    URL,
    URLSearchParams,
    TextEncoder,
    TextDecoder,
    AbortController,
    AbortSignal,
    Blob,
    DOMException,
    setTimeout: setPageTimeout,
    clearTimeout: clearPageTimeout,
    setInterval: setPageInterval,
    clearInterval: clearPageInterval,
    queueMicrotask,
    WeakRef,
    FinalizationRegistry,
  });
  const originalCaptureStackTrace = vm.runInContext("Error.captureStackTrace", context);
  context.window = context;
  if (iframeParent === "cross-origin") {
    context.parent = new Proxy({}, { get() { throw new DOMException("Cross-origin access denied", "SecurityError"); } });
    context.top = context.parent;
  } else {
    context.parent = iframeParent?.context || context;
    context.top = iframeParent?.context?.top || context.parent;
  }
  if (local && !iframeParent) {
    context.__niva_ws_url = "wss://niva.test/__niva_ws";
    context.__niva_token = "fixture-secret";
  }
  vm.runInContext(bootstrapSource, context, { filename: "bootstrap.js" });
  return { context, requests, importMaps, sockets, fakeTimerHandles, initialMap, originalCaptureStackTrace };
}

const bootCommonJsPage = () => bootPage({ injectCommonJs: true, injectEsm: false });

test("base Native page initializes without injecting CommonJS globals", () => {
  const { context, importMaps } = bootPage({ injectCommonJs: false, injectEsm: false });
  assert.equal(context.Niva.bridgeVersion, 1);
  assert.equal(context.Niva.runtimeConfig.injectCommonJs, false);
  assert.equal(context.Niva.runtimeConfig.injectEsm, false);
  assert.equal(context.require, undefined);
  assert.equal(context.module, undefined);
  assert.equal(context.global, undefined);
  assert.equal(context.exports, undefined);
  assert.equal(context.process, undefined);
  assert.equal(context.Buffer, undefined);
  assert.equal(context.setImmediate, undefined);
  assert.equal(importMaps.length, 0);
  assert.equal(typeof context.Niva.fs.readFile, "function");
  assert.equal(context.Niva.process.stdin.fd, 0);
  assert.equal(context.Niva.process.version, "v22.14.0");
  assert.equal(context.Niva.process.versions.node, "22.14.0");
  assert.equal(context.Niva.process.versions.nodeCompat, "22.14.0");
  assert.equal(context.Niva.process.versions.niva, "0.9.9");
  assert.equal(context.Niva.process.stdout.fd, 1);
  assert.equal(context.Niva.process.stderr.fd, 2);
  assert.equal(context.Niva.process.stdin.isTTY, true);
  assert.equal(context.Niva.tty.isatty(0), true);
  assert.equal(context.Niva.tty.isatty(1), false);
  assert.equal(context.Niva.tty.isatty(2), true);
  assert.equal(context.Niva.tty.isatty(3), false);
  assert.equal(context.Niva.tty.isatty(-1), false);
  assert.throws(() => new context.Niva.tty.ReadStream(0), { code: "ENOTSUP" });
  assert.throws(() => new context.Niva.tty.WriteStream(1), { code: "ENOTSUP" });
});

test("constants builtin aliases fs.constants and exposes host open flags", () => {
  for (const [platform, createFlag] of [["darwin", 512], ["linux", 64], ["win32", 256]]) {
    const { context } = bootPage({ injectCommonJs: true, platform });
    const constants = context.require("constants");
    assert.strictEqual(constants, context.require("node:constants"));
    assert.strictEqual(constants, context.Niva.fs.constants);
    assert.equal(constants.O_RDONLY, 0);
    assert.equal(constants.O_RDWR, 2);
    assert.equal(constants.O_CREAT, createFlag);
  }
});

test("cluster builtin reports the Niva primary process and rejects worker creation", async () => {
  const { context } = bootPage({ injectCommonJs: true });
  const cluster = context.require("cluster");
  assert.strictEqual(cluster, context.require("node:cluster"));
  assert.ok(cluster instanceof context.Niva.events.EventEmitter);
  assert.equal(cluster.isPrimary, true);
  assert.equal(cluster.isMaster, true);
  assert.equal(cluster.isWorker, false);
  assert.equal(cluster.worker, undefined);
  assert.deepEqual(Object.keys(cluster.workers), []);
  assert.equal(cluster.SCHED_NONE, 1);
  assert.equal(cluster.SCHED_RR, 2);
  assert.throws(() => cluster.fork(), { code: "ERR_NIVA_CLUSTER_UNAVAILABLE" });
  let called = false;
  assert.strictEqual(cluster.disconnect(() => { called = true; }), cluster);
  await new Promise((resolve) => queueMicrotask(resolve));
  assert.equal(called, true);
});

test("CallSite compatibility installer leaves Error globals alone until CommonJS is enabled", () => {
  function BrowserError(message) {
    this.message = message || "Error";
    this.stack = this.message + "\n    at fixture (/app/index.js:1:1)";
  }
  BrowserError.prototype = Object.create(Error.prototype);
  const base = bootPage({ injectCommonJs: false, injectEsm: false, ErrorConstructor: BrowserError });
  assert.equal(base.originalCaptureStackTrace, undefined);
  assert.equal(base.context.Error.captureStackTrace, base.originalCaptureStackTrace);

  const commonJs = bootPage({ injectCommonJs: true, injectEsm: false, ErrorConstructor: BrowserError });
  assert.equal(typeof commonJs.context.Error.captureStackTrace, "function");
  assert.notEqual(commonJs.context.Error.captureStackTrace, commonJs.originalCaptureStackTrace);
});

test("external injectEsm leaves import maps to the page bundler", () => {
  const { context, importMaps, initialMap } = bootPage({ injectCommonJs: false, injectEsm: true, local: false, userImportMap: true });
  assert.equal(context.Niva.runtimeConfig.injectEsm, true);
  assert.equal(context.require, undefined);
  assert.equal(context.global, undefined);
  assert.equal(importMaps.length, 0);
  assert.deepEqual(JSON.parse(initialMap.textContent), { imports: { "node:fs": "https://user.test/fs.mjs" } });
  const localPage = bootPage({ injectCommonJs: false, injectEsm: true, local: true });
  assert.equal(localPage.importMaps.length, 0, "Native owns the local page import map");
});

test("same-origin iframe inherits only transport credentials, flags, nonce, and OS metadata", () => {
  const parent = bootPage({ injectCommonJs: true, injectEsm: true, fakeTimers: true });
  const child = bootPage({ iframeParent: parent, fakeTimers: true });
  assert.equal(child.context.__niva_ws_url, parent.context.__niva_ws_url);
  assert.equal(child.context.__niva_token, parent.context.__niva_token);
  assert.equal(child.context.__niva_window_id, parent.context.__niva_window_id);
  assert.equal(child.context.Niva.runtimeConfig.injectCommonJs, true);
  assert.equal(child.context.Niva.runtimeConfig.injectEsm, true);
  assert.equal(child.context.Niva.runtimeConfig.nonce, "test-nonce");
  assert.deepEqual(child.context.Niva.bootstrap.os, parent.context.Niva.bootstrap.os);
  assert.equal(child.context.Niva.bootstrap.process, undefined);
  assert.equal(child.context.process, undefined, "main-process globals are not inherited by a frame");
  assert.notEqual(child.context.Niva.bridge.sessionId, parent.context.Niva.bridge.sessionId);
});

test("cross-origin iframe receives no parent bridge credentials or process metadata", () => {
  const child = bootPage({ iframeParent: "cross-origin", local: false, fakeTimers: true, origin: "https://remote.test" });
  assert.equal(child.context.__niva_ws_url, undefined);
  assert.equal(child.context.__niva_token, undefined);
  assert.equal(child.context.__niva_server_origin, undefined);
  assert.equal(child.context.require, undefined);
  assert.equal(child.context.process, undefined);
  assert.equal(child.context.Niva.bootstrap.process, undefined);
  assert.equal(child.context.Niva.bridge.isTrustedLocal(), false);
});

test("an already-open WebSocket is selected for every async call and stream in the realm", async () => {
  const { context, sockets, requests } = bootPage({ injectCommonJs: false, injectEsm: false, fakeTimers: true, ipcReply: () => ({ unreachable: true }) });
  const socket = sockets[0];
  socket.open();
  const reply = context.Niva.bridge.call("window.getTitle", []);
  await Promise.resolve();
  const helloFrame = socket.sent.map((frame) => JSON.parse(frame)).find((frame) => frame.t === "hello");
  assert.equal(helloFrame.sessionId, context.Niva.bridge.sessionId);
  const callFrame = socket.sent.map((frame) => JSON.parse(frame)).find((frame) => frame.t === "call");
  assert.ok(callFrame);
  assert.equal(typeof callFrame.id, "number");
  assert.equal(callFrame.sessionId, context.Niva.bridge.sessionId);
  socket.receive(JSON.stringify({ t: "result", id: callFrame.id, code: 0, data: { ok: true } }));
  assert.equal((await reply).ok, true);

  const stream = context.Niva.bridge.stream("fs.readStream", []);
  const streamResult = stream.promise;
  await Promise.resolve();
  const streamCallFrame = socket.sent.map((text) => JSON.parse(text)).filter((frame) => frame.t === "call").at(-1);
  assert.equal(streamCallFrame.method, "fs.readStream");
  socket.receive(JSON.stringify({ t: "result", id: stream.id, code: 0, data: "stream done" }));
  assert.equal(await streamResult, "stream done");
  assert.equal(requests.length, 0, "the selected WebSocket call is not also sent over IPC");
});

test("macOS IPC replies correlate by numeric rid after WebSocket setup fails", async () => {
  const { context, requests, sockets } = bootPage({
    injectCommonJs: false,
    injectEsm: false,
    fakeTimers: true,
    webSocketThrows: 1,
    ipcReply(request) { return { t: "result", id: request.id, code: 0, message: "ok", data: { method: request.method } }; },
  });
  const ipcCall = context.Niva.bridge.call("window.getTitle", []);
  assert.equal((await ipcCall).method, "window.getTitle");
  const request = requests[0];
  assert.equal(request.transport, "ipc");
  assert.equal(typeof request.rid, "number");
  assert.equal(request.id, 1);
  assert.equal(request.token, "fixture-secret");
  assert.equal(sockets.length, 0);
  assert.equal((await context.Niva.bridge.call("window.getSize", [])).method, "window.getSize");
  assert.equal(requests.length, 2, "the realm stays on IPC after transport selection");
});

test("IPC selection times out once and a late WebSocket cannot upgrade the realm", async () => {
  const { context, requests, sockets } = bootPage({
    injectCommonJs: false,
    injectEsm: false,
    ipcReply(request) { return { t: "result", id: request.id, code: 0, message: "ok", data: request.method }; },
  });
  assert.equal(context.Niva.bridge.callSync("process.currentDir", []), "/app", "sync XHR does not select an async transport");
  const first = context.Niva.bridge.call("window.getTitle", []);
  assert.equal(await first, "window.getTitle");
  assert.equal(requests.filter((request) => request.transport === "ipc").length, 1);
  assert.equal(sockets.length, 1);
  assert.equal(sockets[0].readyState, 3, "locking IPC closes the still-connecting WebSocket");

  sockets[0].open();
  assert.equal(sockets[0].sent.length, 0, "a late open sends no hello and does not bind a Native WebSocket session");
  assert.equal(await context.Niva.bridge.call("window.getSize", []), "window.getSize");
  assert.equal(requests.filter((request) => request.transport === "ipc").length, 2, "all later calls in the realm remain on IPC");
});

test("a selected WebSocket disconnect fails pending work, invalidates resources, then future calls use IPC", async () => {
  const { context, requests, sockets } = bootPage({
    injectCommonJs: false,
    injectEsm: false,
    fakeTimers: true,
    ipcReply(request) { return { t: "result", id: request.id, code: 0, message: "ok", data: { method: request.method } }; },
  });
  const socket = sockets[0];
  socket.open();
  const unary = context.Niva.bridge.call("window.getTitle", []);
  const stream = context.Niva.bridge.stream("fs.readStream", []);
  const streamSettled = stream.promise.then((value) => ({ value }), (error) => ({ error }));
  const resource = { invalidated: false, __nivaInvalidate() { this.invalidated = true; } };
  context.Niva.__runtime.registerResource(resource);
  await new Promise((resolve) => setImmediate(resolve));
  socket.close();
  await assert.rejects(unary, { code: "ECONNRESET" });
  assert.equal((await streamSettled).error.code, "ECONNRESET");
  assert.equal(resource.invalidated, true);
  assert.deepEqual(JSON.parse(JSON.stringify(await context.Niva.bridge.call("window.getTitle", []))), { method: "window.getTitle" });
  assert.equal(requests.filter((request) => request.transport === "ipc").length, 1);
  assert.equal(sockets.length, 1, "the failed WebSocket is not reconnected");
});

test("Windows postMessage IPC opens a Channel and native push is routed through rid-correlated ACKs", async () => {
  const { context, requests } = bootPage({
    injectCommonJs: false,
    injectEsm: false,
    fakeTimers: true,
    webSocketThrows: 1,
    windowsIpc(request) {
      if (request.t === "call") return { t: "channelOpened", id: request.id, capability: "fixture-capability" };
      if (request.t === "channelAck") return { t: "channelAckReceived", id: request.id, seq: request.seq, accepted: true };
      if (request.t === "channelCancel") return { t: "channelCancelled", id: request.id, accepted: true };
      if (request.t === "heartbeat") return { t: "heartbeatAck", sessionId: request.sessionId };
      throw new Error(`Unexpected Windows IPC message ${request.t}`);
    },
  });
  const events = [];
  const stream = context.Niva.bridge.stream("fs.readStream", [], { onEvent: (name, data) => events.push([name, data]) });
  const settled = stream.promise.then((value) => ({ value }), (error) => ({ error }));
  await new Promise((resolve) => setImmediate(resolve));
  assert.equal(requests.filter((request) => request.t === "channelPoll").length, 0, "Native pushes frames without a polling loop");
  const open = requests.find((request) => request.t === "call");
  assert.equal(typeof open.rid, "number");
  assert.equal(open.token, "fixture-secret");

  assert.equal(context.__niva_native_frame({
    sessionId: context.Niva.bridge.sessionId,
    id: stream.id,
    seq: 1,
    frame: { t: "text", data: JSON.stringify({ t: "event", id: stream.id, seq: 1, name: "chunk", data: { bytes: 3 } }) },
  }), true);
  await new Promise((resolve) => setImmediate(resolve));
  assert.deepEqual(JSON.parse(JSON.stringify(events)), [["chunk", { bytes: 3 }]]);
  assert.ok(requests.some((request) => request.t === "channelAck" && request.seq === 1 && request.capability === "fixture-capability"));

  context.__niva_native_frame({
    sessionId: context.Niva.bridge.sessionId,
    id: stream.id,
    seq: 2,
    frame: { t: "text", data: JSON.stringify({ t: "result", id: stream.id, code: 0, message: "ok", data: "stream done" }) },
  });
  const outcome = await settled;
  assert.equal(outcome.value, "stream done");
  const acknowledgements = requests.filter((request) => request.t === "channelAck");
  assert.deepEqual(acknowledgements.map((request) => request.seq), [1, 2]);
  assert.notEqual(acknowledgements[0].rid, acknowledgements[1].rid);
});

test("main-frame Channel push reaches only the matching same-origin child session and decodes binary frames", async () => {
  const parent = bootPage({
    injectCommonJs: false,
    injectEsm: false,
    fakeTimers: true,
    webSocketThrows: 1,
    ipcReply(request) {
      if (request.t === "channelAck") return { t: "channelAckReceived", id: request.id, seq: request.seq, accepted: true };
      return { t: "heartbeatAck", sessionId: request.sessionId };
    },
  });
  const childRequests = [];
  const child = bootPage({
    injectCommonJs: false,
    injectEsm: false,
    fakeTimers: true,
    webSocketThrows: 1,
    iframeParent: parent,
    ipcReply(request) {
      childRequests.push(request);
      if (request.t === "call") return { t: "channelOpened", id: request.id, capability: "child-capability" };
      if (request.t === "channelAck") return { t: "channelAckReceived", id: request.id, seq: request.seq, accepted: true };
      if (request.t === "heartbeat") return { t: "heartbeatAck", sessionId: request.sessionId };
      throw new Error(`Unexpected child IPC message ${request.t}`);
    },
  });
  const crossOrigin = bootPage({
    injectCommonJs: false,
    injectEsm: false,
    fakeTimers: true,
    iframeParent: parent,
    local: false,
    origin: "https://untrusted.test",
  });
  let crossOriginMessageCount = 0;
  crossOrigin.context.addEventListener("message", () => { crossOriginMessageCount += 1; });
  parent.context.frames = [child.context, crossOrigin.context];

  const chunks = [];
  const stream = child.context.Niva.bridge.stream("fs.readStream", [], { onChunk: (bytes) => chunks.push(...bytes) });
  const settled = stream.promise.then((value) => ({ value }), (error) => ({ error }));
  await new Promise((resolve) => setImmediate(resolve));
  const bytes = Buffer.from([0, 1, 127, 128, 255]);
  const binaryFrame = Buffer.alloc(18 + bytes.length);
  binaryFrame[0] = 1;
  binaryFrame[1] = 3;
  binaryFrame.writeUInt32BE(stream.id, 6);
  binaryFrame.writeUInt32BE(1, 14);
  bytes.copy(binaryFrame, 18);
  parent.context.__niva_native_frame({
    sessionId: child.context.Niva.bridge.sessionId,
    id: stream.id,
    seq: 1,
    frame: { t: "binary", data: binaryFrame.toString("base64") },
  });
  await new Promise((resolve) => setImmediate(resolve));
  assert.deepEqual(chunks, Array.from(bytes));
  assert.equal(crossOriginMessageCount, 0, "the native frame relay skips another origin");

  parent.context.__niva_native_frame({
    sessionId: child.context.Niva.bridge.sessionId,
    id: stream.id,
    seq: 2,
    frame: { t: "text", data: JSON.stringify({ t: "result", id: stream.id, code: 0, message: "ok", data: "done" }) },
  });
  assert.equal((await settled).value, "done");
  assert.deepEqual(childRequests.filter((request) => request.t === "channelAck").map((request) => request.seq), [1, 2]);
  assert.equal(childRequests[0].sessionId, child.context.Niva.bridge.sessionId);
  assert.equal(childRequests[0].token, "fixture-secret");
});

test("Native IPC window events relay recursively only through exact same-origin frames", async () => {
  const parent = bootPage({
    injectCommonJs: false,
    injectEsm: false,
    webSocketThrows: 1,
    ipcReply() { return null; },
  });
  const child = bootPage({
    injectCommonJs: false,
    injectEsm: false,
    iframeParent: parent,
    webSocketThrows: 1,
  });
  const crossOrigin = bootPage({
    injectCommonJs: false,
    injectEsm: false,
    iframeParent: parent,
    local: false,
    origin: "https://untrusted.test",
  });
  parent.context.frames = [child.context, crossOrigin.context];
  const seen = { parent: [], child: [], crossOrigin: [] };
  parent.context.Niva.addEventListener("app.ready", (name, data) => seen.parent.push([name, data]));
  child.context.Niva.addEventListener("app.ready", (name, data) => seen.child.push([name, data]));
  crossOrigin.context.Niva.addEventListener("app.ready", (name, data) => seen.crossOrigin.push([name, data]));

  assert.equal(parent.context.__niva_native_event(JSON.stringify({ t: "event", seq: 1, name: "app.ready", data: { ok: true } })), true);
  await new Promise((resolve) => setTimeout(resolve, 5));
  assert.deepEqual(JSON.parse(JSON.stringify(seen)), {
    parent: [["app.ready", { ok: true }]],
    child: [["app.ready", { ok: true }]],
    crossOrigin: [],
  });
});

test("native output arriving before channelOpened is bounded and acknowledged after capability setup", async () => {
  let openChannel;
  const requests = [];
  const { context } = bootPage({
    injectCommonJs: false,
    injectEsm: false,
    fakeTimers: true,
    webSocketThrows: 1,
    ipcReply(request) {
      requests.push(request);
      if (request.t === "call") return new Promise((resolve) => { openChannel = resolve; });
      if (request.t === "channelAck") return { t: "channelAckReceived", id: request.id, seq: request.seq, accepted: true };
      if (request.t === "heartbeat") return { t: "heartbeatAck", sessionId: request.sessionId };
      throw new Error(`Unexpected IPC message ${request.t}`);
    },
  });
  const events = [];
  const stream = context.Niva.bridge.stream("fs.readStream", [], { onEvent: (name) => events.push(name) });
  const settled = stream.promise.then((value) => ({ value }), (error) => ({ error }));
  await new Promise((resolve) => setImmediate(resolve));
  assert.equal(typeof openChannel, "function");
  assert.equal(context.__niva_native_frame({
    sessionId: context.Niva.bridge.sessionId,
    id: stream.id,
    seq: 1,
    frame: { t: "text", data: JSON.stringify({ t: "event", id: stream.id, seq: 1, name: "ready", data: {} }) },
  }), true);
  assert.equal(requests.some((request) => request.t === "channelAck"), false, "the frame waits until its capability arrives");

  openChannel({ t: "channelOpened", id: stream.id, capability: "late-capability" });
  await new Promise((resolve) => setImmediate(resolve));
  assert.deepEqual(events, ["ready"]);
  assert.ok(requests.some((request) => request.t === "channelAck" && request.capability === "late-capability"));
  context.__niva_native_frame({
    sessionId: context.Niva.bridge.sessionId,
    id: stream.id,
    seq: 2,
    frame: { t: "text", data: JSON.stringify({ t: "result", id: stream.id, code: 0, message: "ok", data: "done" }) },
  });
  assert.equal((await settled).value, "done");
});

test("IPC stream sends base64 full binary frames in order and bounds the queued backlog", async () => {
  const sends = [];
  const reply = (request) => {
    if (request.t === "call") return { t: "channelOpened", id: request.id, capability: "ipc-capability" };
    if (request.t === "channelSend") return new Promise((resolve) => sends.push({ request, resolve }));
    if (request.t === "channelCancel") return { t: "channelCancelled", id: request.id, accepted: true };
    if (request.t === "heartbeat") return { t: "heartbeatAck", sessionId: request.sessionId };
    throw new Error(`Unexpected macOS IPC message ${request.t}`);
  };
  const { context, requests } = bootPage({
    injectCommonJs: false,
    injectEsm: false,
    fakeTimers: true,
    webSocketThrows: 1,
    ipcReply: reply,
  });
  const stream = context.Niva.bridge.stream("socket.udpSend", []);
  stream.promise.catch(() => {});
  await new Promise((resolve) => setImmediate(resolve));
  const payload = new Uint8Array(40000);
  for (let index = 0; index < payload.length; index += 1) payload[index] = index % 251;
  assert.equal(context.Niva.bridge.streamSend(stream.id, payload, true), true);
  await new Promise((resolve) => setImmediate(resolve));
    assert.equal(sends.length, 1, "only one IPC send is outstanding until its acknowledgement");

  for (let index = 0; index < 3; index += 1) {
    const send = sends[index];
    const request = send.request;
    assert.equal(request.t, "channelSend");
    assert.equal(request.id, stream.id);
    assert.equal(typeof request.rid, "number");
    assert.equal(request.sessionId, context.Niva.bridge.sessionId);
    assert.equal(request.token, "fixture-secret");
    assert.equal(request.capability, "ipc-capability");
    const frame = Buffer.from(request.frame, "base64");
    assert.equal(frame[0], 1);
    assert.equal(frame.readUInt32BE(14), index + 1);
    assert.equal(!!(frame[1] & 2), index === 2);
    const expectedLength = index < 2 ? 16384 : 40000 - 32768;
    assert.equal(frame.byteLength, 18 + expectedLength);
    assert.deepEqual(frame.subarray(18), Buffer.from(payload.subarray(index * 16384, index * 16384 + expectedLength)));
    send.resolve({ t: "channelAck", id: stream.id, seq: index + 1, accepted: true });
    await new Promise((resolve) => setImmediate(resolve));
    if (index < 2) assert.equal(sends.length, index + 2, "the next seq starts only after the prior IPC ACK");
  }
  assert.equal(context.Niva.bridge.streamSend(stream.id, new Uint8Array(5 * 1024 * 1024), false), false, "oversized backlog is rejected atomically");
  assert.equal(sends.length, 3);
  assert.equal(stream.cancel(), true);
  await assert.rejects(stream.promise, { code: "ABORT_ERR" });
  assert.ok(requests.some((request) => request.t === "channelCancel" && request.capability === "ipc-capability"));
});

test("retryable IPC backpressure retries the same frame and sequence before continuing", async () => {
  const sends = [];
  const requests = [];
  const { context } = bootPage({
    injectCommonJs: false,
    injectEsm: false,
    webSocketThrows: 1,
    ipcReply(request) {
      requests.push(request);
      if (request.t === "call") return { t: "channelOpened", id: request.id, capability: "retry-capability" };
      if (request.t === "channelSend") {
        sends.push(request);
        const seq = Buffer.from(request.frame, "base64").readUInt32BE(14);
        return sends.length === 1
          ? { t: "channelError", id: request.id, seq, code: "EAGAIN", message: "temporarily full", retryable: true }
          : { t: "channelAck", id: request.id, seq, accepted: true };
      }
      if (request.t === "channelAck") return { t: "channelAckReceived", id: request.id, seq: request.seq, accepted: true };
      if (request.t === "channelCancel") return { t: "channelCancelled", id: request.id, accepted: true };
      if (request.t === "heartbeat") return { t: "heartbeatAck", sessionId: request.sessionId };
      throw new Error(`Unexpected IPC message ${request.t}`);
    },
  });
  const stream = context.Niva.bridge.stream("process.stdin", []);
  const settled = stream.promise.then((value) => ({ value }), (error) => ({ error }));
  await new Promise((resolve) => setImmediate(resolve));
  assert.equal(context.Niva.bridge.streamSend(stream.id, new Uint8Array([4, 5, 6]), false), true);
  await new Promise((resolve) => setTimeout(resolve, 40));
  assert.equal(sends.length, 2);
  assert.equal(Buffer.from(sends[0].frame, "base64").readUInt32BE(14), 1);
  assert.equal(Buffer.from(sends[1].frame, "base64").readUInt32BE(14), 1);
  assert.equal(sends[0].frame, sends[1].frame);
  assert.notEqual(sends[0].rid, sends[1].rid);
  context.__niva_native_frame({
    sessionId: context.Niva.bridge.sessionId,
    id: stream.id,
    seq: 1,
    frame: { t: "text", data: JSON.stringify({ t: "result", id: stream.id, code: 0, message: "ok", data: "accepted" }) },
  });
  assert.equal((await settled).value, "accepted");
});

test("a non-retryable IPC send rejection fails and cancels the stream", async () => {
  const requests = [];
  const { context } = bootPage({
    injectCommonJs: false,
    injectEsm: false,
    fakeTimers: true,
    webSocketThrows: 1,
    ipcReply(request) {
      requests.push(request);
      if (request.t === "call") return { t: "channelOpened", id: request.id, capability: "reject-capability" };
      if (request.t === "channelSend") return { t: "channelError", id: request.id, seq: Buffer.from(request.frame, "base64").readUInt32BE(14), code: "EPIPE", message: "send closed", retryable: false };
      if (request.t === "channelCancel") return { t: "channelCancelled", id: request.id, accepted: true };
      return { t: "heartbeatAck", sessionId: request.sessionId };
    },
  });
  const stream = context.Niva.bridge.stream("process.stdin", []);
  await new Promise((resolve) => setImmediate(resolve));
  assert.equal(context.Niva.bridge.streamSend(stream.id, new Uint8Array([1, 2, 3]), false), true);
  await assert.rejects(stream.promise, { code: "EPIPE" });
  assert.equal(requests.filter((request) => request.t === "channelCancel").length, 1);
});

test("remote pages use only the restricted unary IPC surface", async () => {
  const { context, requests } = bootPage({
    injectCommonJs: false,
    injectEsm: false,
    fakeTimers: true,
    local: false,
    ipcReply(request) { return request.method === "fs.readText" ? "hello from IPC" : null; },
  });
  assert.equal(context.Niva.bridge.isTrustedLocal(), false);
  assert.equal(await context.Niva.fs.promises.readFile("notes.txt", "utf8"), "hello from IPC");
  const fsRequest = requests.find((request) => request.transport === "ipc" && request.method === "fs.readText");
  assert.equal(typeof fsRequest.id, "number");
  assert.equal(typeof fsRequest.rid, "number");
  assert.equal(fsRequest.token, undefined);

  const requestCount = requests.length;
  await assert.rejects(context.Niva.fs.promises.readFile("notes.txt"), { code: "ERR_NIVA_FS_DATA_UNSUPPORTED" });
  assert.equal(requests.length, requestCount, "Buffer-default reads are rejected before any transport is selected");
  assert.throws(() => context.Niva.bridge.stream("process.execStream", ["echo", []]), { code: "ERR_NIVA_LOCAL_PAGE_REQUIRED" });
});

test("CJS fixture preserves wrapper, cache, cycle, parent, and resolver semantics", () => {
  const { context, requests } = bootCommonJsPage();
  assert.equal(vm.runInContext("global === globalThis", context), true);
  assert.equal(context.exports, context.module.exports);
  assert.equal(typeof context.setImmediate, "function");
  const timeout = context.setTimeout(() => {}, 100);
  assert.equal(typeof timeout.ref, "function");
  context.clearTimeout(timeout);
  const immediate = context.setImmediate(() => {});
  assert.equal(typeof immediate.ref, "function");
  context.clearImmediate(immediate);
  assert.equal(context.require.main, null);
  const first = context.require("./cycle-a.cjs");

  assert.equal(first.name, "a");
  assert.equal(first.otherName, "b");
  assert.equal(first.sawPartialExports, true);
  assert.equal(first.topLevelThisIsExports, true);
  assert.equal(first.wrapperArgumentCount, 5);
  assert.equal(first.filename, "/app/cycle-a.cjs");
  assert.equal(first.dirname, "/app");
  assert.equal(first.parentId, "/app/index.html");
  assert.equal(first.mainId, "/app/cycle-a.cjs");
  assert.equal(first.isMain, true);
  assert.equal(context.require.main, context.Niva.module.main);
  assert.equal(context.require.main.id, "/app/cycle-a.cjs");
  assert.equal(first.moduleIsInstance, true);
  assert.equal(first.cacheEntryIsModule, true);
  assert.deepEqual(Array.from(first.childIds), ["/app/cycle-b.cjs"]);
  assert.ok(Array.from(first.modulePaths).includes("/app/node_modules"));
  assert.equal(context.require("./cycle-a.cjs"), first);
  assert.equal(context.require("#native-fs"), context.Niva.fs);
  assert.equal(context.require.resolve("#native-fs"), "node:fs");
  assert.equal(context.require("module"), context.Niva.module);
  assert.equal(context.module instanceof context.require("module"), true);
  assert.ok(Array.from(context.require.resolve.paths("./cycle-a.cjs")).includes("/app/node_modules"));
  assert.equal(context.require.resolve.paths("fs"), null);
  assert.throws(() => context.require("#missing-builtin"), { code: "ERR_UNKNOWN_BUILTIN_MODULE" });

  const resolverCalls = requests.filter((request) => request.method === "module.resolve" || request.method === "module.load");
  assert.ok(resolverCalls.length >= 4);
  assert.equal(resolverCalls[0].args[1], "", "the page's first require starts at the resource root");
  assert.equal(resolverCalls[1].args[1], "", "resolve and load share the same root parent");
  assert.equal(resolverCalls.find((request) => request.args[0] === "./cycle-b.cjs").args[1], "/app/cycle-a.cjs");
  assert.ok(resolverCalls.every((request) => request.async === false && request.token === "fixture-secret"));
});

test("CJS syntax errors stay syntax errors and evaluated module stacks carry the filename", () => {
  const { context } = bootCommonJsPage();
  fixtureFiles.set("/app/syntax-error.cjs", "const = ;");
  try {
    assert.throws(() => context.require("./syntax-error.cjs"), (error) => error.name === "SyntaxError" && error.filename === "/app/syntax-error.cjs" && !/CSP blocked/.test(error.message));
  } finally {
    fixtureFiles.delete("/app/syntax-error.cjs");
  }
  assert.throws(() => context.require("./throw.cjs"), (error) => error.message === "source path fixture" && error.stack.includes("/app/throw.cjs"));
});

test("CJS loader parses JSON through the same module cache", () => {
  const { context, requests } = bootCommonJsPage();
  fixtureFiles.set("/app/data.json", '{"value":42}');
  const first = context.require("./data.json");
  assert.deepEqual(JSON.parse(JSON.stringify(first)), { value: 42 });
  assert.equal(context.require("./data.json"), first);
  assert.ok(requests.some((request) => request.method === "module.load" && request.args[0] === "./data.json"));
  fixtureFiles.delete("/app/data.json");
});
