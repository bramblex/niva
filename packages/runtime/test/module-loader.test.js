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

function bootPage({ injectCommonJs = true, injectEsm = false, webSocketThrows = 0, ipcReply, fakeTimers = false, local = true, userImportMap = false, iframeParent, origin = "https://niva.test", ErrorConstructor, platform = "linux" } = {}) {
  const requests = [];
  const importMaps = [];
  const sockets = [];
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
    close() {}
  }
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
    __niva_server_origin: iframeParent ? undefined : "https://niva.test",
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
      return JSON.stringify({ t: "result", id: request.id, code: 0, data: ipcReply(request) });
    } } } } : undefined,
    WebSocket: TestWebSocket,
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
  assert.equal(child.context.Niva.bridge.isIpcOnly(), false);
});

test("a local call waits for WebSocket readiness and sends numeric Rust-compatible IDs", async () => {
  const { context, sockets } = bootPage({ injectCommonJs: false, injectEsm: false, fakeTimers: true });
  const socket = sockets[0];
  const reply = context.Niva.bridge.call("fs.node", ["stat", { path: "/app/file" }]);
  await Promise.resolve();
  assert.deepEqual(socket.sent.map((frame) => JSON.parse(frame)).filter((frame) => frame.t === "call"), []);

  socket.open();
  await new Promise((resolve) => setImmediate(resolve));
  const helloFrame = socket.sent.map((frame) => JSON.parse(frame)).find((frame) => frame.t === "hello");
  assert.equal(helloFrame.sessionId, context.Niva.bridge.sessionId);
  const callFrame = socket.sent.map((frame) => JSON.parse(frame)).find((frame) => frame.t === "call");
  assert.ok(callFrame);
  assert.equal(typeof callFrame.id, "number");
  assert.equal(callFrame.sessionId, context.Niva.bridge.sessionId);
  socket.receive(JSON.stringify({ t: "result", id: callFrame.id, code: 0, data: { ok: true } }));
  assert.equal((await reply).ok, true);
});

test("a confirmed local WebSocket failure selects text IPC before dispatch and never downgrades stdio", async () => {
  const { context, requests } = bootPage({
    injectCommonJs: false,
    injectEsm: false,
    webSocketThrows: 1,
    fakeTimers: true,
    ipcReply(request) { return request.method === "fs.readText" ? "hello from IPC" : null; },
  });
  assert.equal(context.Niva.bridge.isIpcOnly(), true);
  assert.equal(await context.Niva.fs.promises.readFile("notes.txt", "utf8"), "hello from IPC");
  const fsRequest = requests.find((request) => request.transport === "ipc" && request.method === "fs.readText");
  assert.equal(typeof fsRequest.id, "number");

  const requestCount = requests.length;
  await assert.rejects(context.Niva.fs.promises.readFile("notes.txt"), { code: "ERR_NIVA_IPC_BINARY_UNSUPPORTED" });
  assert.equal(requests.length, requestCount, "Buffer-default reads are rejected before any transport is selected");

  const stdoutError = await new Promise((resolve) => context.Niva.process.stdout.write("startup output", resolve));
  assert.equal(stdoutError.code, "ERR_NIVA_IPC_METHOD_UNSUPPORTED");
  assert.equal(requests.some((request) => request.method === "process.write"), false);
  assert.equal(requests.some((request) => request.method === "fs.node"), false);
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
