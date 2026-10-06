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

function bootPage({ injectCommonJs = true, injectEsm = false, webSocketThrows = 0, ipcReply, ipcAttachReply, windowsIpc, fetchImpl, fakeTimers = false, local = true, userImportMap = false, iframeParent, origin = "https://niva.test", href, sourceOrigin, serverOrigin = "https://niva.test", ErrorConstructor, platform = "linux", consoleImpl = console, syncReply, syncBlock, failPagehideOnce = false, captureBootstrapError = false } = {}) {
  const requests = [];
  const importMaps = [];
  const sockets = [];
  const pageListeners = new Map();
  const pageHref = href || `${origin}/app/index.html`;
  const pageUrl = new URL(pageHref);
  let context;
  let shouldFailPagehide = failPagehideOnce;
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
      return { ...value, ...(value.id === undefined && request.id !== undefined ? { id: request.id } : {}) };
    }
    return { t: "result", ...(request.id === undefined ? {} : { id: request.id }), code: 0, data: value };
  }
  const ipcHandler = ipcReply || windowsIpc;
  const mockIpc = ipcHandler ? { postMessage(text) {
    const request = JSON.parse(text);
    const requestRecord = { transport: "ipc", ...request };
    requests.push(requestRecord);
    if (request.t === "session_close") return;
    const handled = request.t === "channelAttach"
      ? (ipcAttachReply ? ipcAttachReply(request) : { t: "channelAttached", id: request.id, accepted: true })
      : ipcHandler(request);
    Promise.resolve(handled).then((value) => {
      const response = ipcResponse(request, value);
      requestRecord.replyAccepted = context.__niva_ipc_reply?.({
        sessionId: request.sessionId,
        rid: request.rid,
        sourceOrigin: context.__niva_source_origin || context.location.origin,
        response,
      }) === true;
    });
  } } : undefined;
  class TestXHR {
    open(method, url, async) { this.method = method; this.url = url; this.async = async; }
    setRequestHeader() {}
    send(text) {
      const envelope = JSON.parse(text);
      const [id, method, args] = envelope.request;
      requests.push({ method, args, async: this.async, token: envelope.token, url: this.url });
      if (syncBlock) syncBlock({ id, method, args, url: this.url });
      let data;
      if (method === "module.resolve") data = resolve(args[0], args[1]);
      else if (method === "module.load") {
        const filename = resolve(args[0], args[1]);
        data = { filename, dirname: path.posix.dirname(filename), type: filename.endsWith(".json") ? "json" : "commonjs", source: fixtureFiles.get(filename) };
      } else if (method === "process.currentDir") data = "/app";
      else if (syncReply) data = syncReply({ id, method, args });
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
    console: consoleImpl,
    ...(ErrorConstructor ? { Error: ErrorConstructor } : {}),
    document,
    location: { pathname: pageUrl.pathname, href: pageHref, origin: pageUrl.origin },
    ...(sourceOrigin ? { __niva_source_origin: sourceOrigin } : {}),
    __niva_server_origin: iframeParent ? undefined : serverOrigin,
    __niva_window_id: iframeParent ? undefined : 4,
    __niva_node_bootstrap: iframeParent ? undefined : {
      nivaVersion: "v0.10.0-beta.1",
      os: { platform, arch: "x64", homedir: "/home/test", tmpdir: "/tmp", EOL: "\n" },
      process: { version: "v0.10.0-beta.1", versions: { niva: "0.10.0-beta.1" }, argv: [], env: {}, stdioIsTTY: { stdin: true, stdout: false, stderr: true } },
    },
    __niva_runtime_config: iframeParent ? undefined : { injectCommonJs, injectEsm, nonce: "test-nonce" },
    ...(mockIpc ? { ipc: mockIpc } : {}),
    addEventListener(name, listener) {
      if (name === "pagehide" && shouldFailPagehide) {
        shouldFailPagehide = false;
        throw new Error("fixture pagehide registration failed");
      }
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
    if (iframeParent?.context) {
      const frames = iframeParent.context.frames || (iframeParent.context.frames = []);
      frames.push(context);
    }
  }
  if (local && !iframeParent) {
    context.__niva_ws_url = "wss://niva.test/__niva_ws";
    context.__niva_token = "fixture-secret";
  }
  let bootstrapError;
  try { vm.runInContext(bootstrapSource, context, { filename: "bootstrap.js" }); }
  catch (error) {
    if (!captureBootstrapError) throw error;
    bootstrapError = error;
  }
  return { context, requests, importMaps, sockets, fakeTimerHandles, initialMap, originalCaptureStackTrace, pageListeners, bootstrapError };
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
  assert.equal(context.Niva.process.versions.niva, "0.10.0-beta.1");
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

test("root-path CommonJS bootstrap avoids sync cwd until a relative path needs it", () => {
  const { context, requests } = bootPage({
    injectCommonJs: true,
    injectEsm: false,
    href: "https://niva.test/",
    syncBlock({ method }) {
      if (method === "process.currentDir") throw new Error("CSP blocked __niva_sync by connect-src");
    },
  });

  assert.equal(context.location.pathname, "/");
  assert.equal(context.Niva.runtimeConfig.injectCommonJs, true);
  assert.equal(context.Niva.__runtimeBootstrap, true);
  assert.equal(typeof context.require, "function");
  assert.equal(requests.some((request) => request.method === "process.currentDir"), false,
    "installing absolute global module paths must not query cwd during document bootstrap");
  assert.equal(context.Niva.path.resolve("/"), "/");
  assert.equal(requests.some((request) => request.method === "process.currentDir"), false,
    "resolving an absolute path must not query cwd");

  assert.throws(() => context.Niva.path.resolve("relative"), /CSP blocked __niva_sync/,
    "relative paths still need the Native process cwd and surface a blocked sync request");
  assert.equal(requests.filter((request) => request.method === "process.currentDir").length, 1);
  assert.equal(context.Niva.__runtimeBootstrap, true,
    "a later path operation failure must not change the successfully completed bootstrap marker");
});

test("Windows path resolution queries cwd only for rooted or drive-relative paths", () => {
  const { context, requests } = bootPage({
    injectCommonJs: false,
    injectEsm: false,
    syncBlock({ method }) {
      if (method === "process.currentDir") throw new Error("CSP blocked __niva_sync by connect-src");
    },
  });
  context.navigator = { platform: "Win32" };
  const windowsPath = context[Symbol.for("niva.node-compat.runtime")].createPathModule();

  assert.equal(windowsPath.resolve("C:\\absolute\\file"), "C:\\absolute\\file");
  assert.equal(windowsPath.resolve("\\\\server\\share\\file"), "\\\\server\\share\\file");
  assert.equal(requests.some((request) => request.method === "process.currentDir"), false);
  assert.throws(() => windowsPath.resolve("\\rooted"), /CSP blocked __niva_sync/);
  assert.throws(() => windowsPath.resolve("C:drive-relative"), /CSP blocked __niva_sync/);
  assert.equal(requests.filter((request) => request.method === "process.currentDir").length, 2);
});

test("failed page initialization stays retryable until the bootstrap reaches completion", () => {
  const { context, pageListeners, bootstrapError } = bootPage({
    injectCommonJs: true,
    injectEsm: false,
    failPagehideOnce: true,
    captureBootstrapError: true,
  });

  assert.match(bootstrapError?.message || "", /pagehide registration failed/);
  assert.notEqual(context.Niva.__runtimeBootstrap, true,
    "an initialization exception must not leave the completion guard set");
  assert.equal(pageListeners.has("pagehide"), false);

  vm.runInContext(bootstrapSource, context, { filename: "bootstrap-retry.js" });
  assert.equal(context.Niva.__runtimeBootstrap, true);
  assert.equal(pageListeners.get("pagehide")?.length, 1,
    "a retry after partial initialization must install the remaining lifecycle hook");
});

test("persisted BFCache restore starts a fresh IPC session and resumes the cached process stdin", async () => {
  let releaseStaleCall;
  const { context, requests, pageListeners, importMaps } = bootPage({
    injectCommonJs: true,
    injectEsm: true,
    fakeTimers: true,
    ipcReply(request) {
      if (request.t === "api_call" && request.method === "stale.once") {
        return new Promise((resolve) => { releaseStaleCall = () => resolve({ t: "result", id: request.id, code: 0, data: "late" }); });
      }
      if (request.t === "api_call" && request.method === "process.stdin") {
        return { t: "channelOpened", id: request.id, capability: `stdin-${request.sessionId}` };
      }
      if (request.t === "channelCancel") return { t: "channelCancelled", id: request.id, accepted: true };
      if (request.t === "channelAck") return { t: "channelAckReceived", id: request.id, seq: request.seq, accepted: true };
      if (request.t === "heartbeat") return { t: "heartbeatAck", sessionId: request.sessionId };
      return { t: "result", id: request.id, code: 0, data: `ok:${request.method}` };
    },
  });
  const processObject = context.Niva.process;
  const stdin = processObject.stdin;
  const cachedCjs = context.require("./cycle-a.cjs");
  let ordinaryResourceInvalidated;
  context.Niva.__runtime.registerResource({
    __nivaInvalidate(error) { ordinaryResourceInvalidated = error; },
  });
  const initialImports = importMaps.length;
  const oldSessionId = context.Niva.bridge.sessionId;
  const stale = context.Niva.bridge.call("stale.once", []).then(
    (value) => ({ value }),
    (error) => ({ error }),
  );
  await new Promise((resolve) => setImmediate(resolve));
  const staleRequest = requests.find((request) => request.t === "api_call" && request.method === "stale.once");
  assert.ok(staleRequest);
  assert.equal(staleRequest.sessionId, oldSessionId);
  assert.equal(typeof releaseStaleCall, "function");

  const chunks = [];
  stdin.on("data", (chunk) => chunks.push(...chunk));
  stdin.resume();
  stdin.read(0);
  await new Promise((resolve) => setImmediate(resolve));
  const oldInput = requests.find((request) => request.t === "api_call" && request.method === "process.stdin");
  assert.ok(oldInput, JSON.stringify(requests.map(({ t, method, id, sessionId }) => ({ t, method, id, sessionId }))));
  assert.equal(oldInput.sessionId, oldSessionId);

  function deliverInputFrame(session, id, seq, value) {
    const frame = Buffer.alloc(19);
    frame[0] = 2;
    frame.writeUInt32BE(id, 6);
    frame.writeUInt32BE(seq, 14);
    frame[18] = value;
    return context.__niva_native_frame({
      sessionId: session,
      id,
      seq,
      frame: { t: "binary", data: frame.toString("base64") },
    });
  }
  assert.equal(deliverInputFrame(oldSessionId, oldInput.id, 1, 17), true);
  await new Promise((resolve) => setImmediate(resolve));
  assert.deepEqual(chunks, [17]);

  for (const listener of pageListeners.get("pagehide") || []) listener({ persisted: true });
  const staleOutcome = await stale;
  assert.equal(staleOutcome.error?.code, "ERR_NIVA_SESSION_EXPIRED");
  assert.ok(requests.some((request) => request.t === "session_close" && request.sessionId === oldSessionId));
  assert.equal(requests.find((request) => request.t === "session_close" && request.sessionId === oldSessionId).rid, undefined,
    "session retirement is a one-way lifecycle message, not an API call");
  assert.equal(ordinaryResourceInvalidated?.code, "ERR_NIVA_SESSION_EXPIRED",
    "ordinary handles are invalidated rather than revived with the BFCache document");
  assert.equal(stdin.destroyed, false, "BFCache suspension keeps the canonical Readable alive");
  assert.equal(deliverInputFrame(oldSessionId, oldInput.id, 2, 18), false, "old-session frames are rejected while suspended");

  for (const listener of pageListeners.get("pageshow") || []) listener({ persisted: true });
  const newSessionId = context.Niva.bridge.sessionId;
  assert.notEqual(newSessionId, oldSessionId);
  assert.strictEqual(context.Niva.process, processObject);
  assert.strictEqual(context.Niva.process.stdin, stdin);
  assert.strictEqual(context.require("./cycle-a.cjs"), cachedCjs, "CJS instances survive the restored document");
  assert.equal(importMaps.length, initialImports, "restore does not reinstall or replace the ESM import map");

  const freshResult = await context.Niva.bridge.call("after.restore", []);
  assert.equal(freshResult, "ok:after.restore");
  assert.equal(context.Niva.path.resolve("after-restore"), "/app/after-restore",
    "Node-compatible relative path resolution still uses synchronous XHR after restore");
  assert.equal(requests.at(-1).method, "process.currentDir");
  assert.equal(requests.at(-1).async, false);
  const freshRequest = requests.find((request) => request.t === "api_call" && request.method === "after.restore");
  assert.equal(freshRequest.sessionId, newSessionId);
  releaseStaleCall();
  await new Promise((resolve) => setImmediate(resolve));
  const staleRequestRecord = requests.find((request) => request.t === "api_call" && request.method === "stale.once");
  assert.equal(staleRequestRecord.replyAccepted, false, "a late old-session reply cannot settle through the new session");

  await new Promise((resolve) => setImmediate(resolve));
  const freshInput = requests.find((request) => request.t === "api_call" && request.method === "process.stdin" && request.sessionId === newSessionId);
  assert.ok(freshInput, "the existing stdin listener causes a new Native input stream to open");
  assert.notEqual(freshInput.id, oldInput.id);
  assert.equal(deliverInputFrame(oldSessionId, oldInput.id, 2, 19), false, "retired session frames stay isolated after restore");
  assert.equal(deliverInputFrame(newSessionId, freshInput.id, 1, 20), true);
  await new Promise((resolve) => setImmediate(resolve));
  assert.deepEqual(chunks, [17, 20], "the original listener receives data from the replacement stream once");
});

test("persisted BFCache restore revives canonical stdout and stderr after old-session writes fail", async () => {
  const pendingWrites = new Map();
  const { context, requests, pageListeners } = bootPage({
    ipcReply(request) {
      if (request.t === "api_call" && request.method === "process.write") {
        const [channel, encoded] = request.args;
        const text = Buffer.from(encoded, "base64").toString();
        if (text.startsWith("restored ")) return { t: "result", id: request.id, code: 0, data: null };
        return new Promise((resolve) => {
          pendingWrites.set(`${channel}:${text}`, () => resolve({ t: "result", id: request.id, code: 0, data: null }));
        });
      }
      if (request.t === "heartbeat") return { t: "heartbeatAck", sessionId: request.sessionId };
      return { t: "result", id: request.id, code: 0, data: null };
    },
  });
  const processObject = context.Niva.process;
  const stdout = processObject.stdout;
  const stderr = processObject.stderr;
  const oldSessionId = context.Niva.bridge.sessionId;
  const callbackResults = [];
  const stdoutIdentity = stdout;
  const stderrIdentity = stderr;

  stdout.write("old stdout", (error) => callbackResults.push({ channel: "stdout", error }));
  stderr.write("old stderr");
  await new Promise((resolve) => setImmediate(resolve));
  assert.deepEqual([...pendingWrites.keys()].sort(), ["stderr:old stderr", "stdout:old stdout"]);
  const stdoutClosed = new Promise((resolve) => stdout.once("close", resolve));
  const stderrClosed = new Promise((resolve) => stderr.once("close", resolve));

  for (const listener of pageListeners.get("pagehide") || []) listener({ persisted: true });
  for (const listener of pageListeners.get("pageshow") || []) listener({ persisted: true });
  const newSessionId = context.Niva.bridge.sessionId;
  assert.notEqual(newSessionId, oldSessionId);
  await Promise.all([stdoutClosed, stderrClosed]);
  assert.equal(callbackResults.length, 1, "the suspended user callback settles once");
  assert.deepEqual(callbackResults.map(({ channel, error }) => [channel, error?.code]).sort(), [
    ["stdout", "ERR_NIVA_SESSION_EXPIRED"],
  ]);
  assert.equal(stdout.destroyed, false, "the expected session error is internally handled until close drains");
  assert.equal(stderr.destroyed, false);
  assert.strictEqual(processObject.stdout, stdoutIdentity);
  assert.strictEqual(processObject.stderr, stderrIdentity);

  const freshWrites = [
    new Promise((resolve, reject) => stdout.write("restored stdout", (error) => error ? reject(error) : resolve())),
    new Promise((resolve, reject) => stderr.write("restored stderr", (error) => error ? reject(error) : resolve())),
  ];
  await Promise.all(freshWrites);
  const outputCalls = requests.filter((request) => request.t === "api_call" && request.method === "process.write");
  assert.deepEqual(outputCalls.map((request) => ({
    sessionId: request.sessionId,
    channel: request.args[0],
    text: Buffer.from(request.args[1], "base64").toString(),
  })), [
    { sessionId: oldSessionId, channel: "stdout", text: "old stdout" },
    { sessionId: oldSessionId, channel: "stderr", text: "old stderr" },
    { sessionId: newSessionId, channel: "stdout", text: "restored stdout" },
    { sessionId: newSessionId, channel: "stderr", text: "restored stderr" },
  ], "old writes are not replayed and each fresh write is sent exactly once on the new session");

  pendingWrites.get("stdout:old stdout")();
  pendingWrites.get("stderr:old stderr")();
  await new Promise((resolve) => setImmediate(resolve));
  assert.ok(outputCalls.slice(0, 2).every((request) => request.replyAccepted === false),
    "late replies from retired writes cannot settle through the restored session");
});

test("BFCache stdio recovery retries after a Promise catch writes to the already-destroyed stream", async () => {
  let releaseOldWrite;
  const { context, requests, pageListeners } = bootPage({
    ipcReply(request) {
      if (request.t === "api_call" && request.method === "process.write") {
        const text = Buffer.from(request.args[1], "base64").toString();
        if (text === "restored after promise catch") return { t: "result", id: request.id, code: 0, data: null };
        return new Promise((resolve) => {
          releaseOldWrite = () => resolve({ t: "result", id: request.id, code: 0, data: null });
        });
      }
      if (request.t === "heartbeat") return { t: "heartbeatAck", sessionId: request.sessionId };
      return { t: "result", id: request.id, code: 0, data: null };
    },
  });
  const stdout = context.Niva.process.stdout;
  const oldSessionId = context.Niva.bridge.sessionId;
  let closeObserved = false;
  stdout.once("close", () => { closeObserved = true; });

  const firstWrite = new Promise((resolve, reject) => {
    stdout.write("write before suspend", (error) => error ? reject(error) : resolve());
  });
  const handledWriteFailure = firstWrite.catch((error) => {
    assert.equal(error?.code, "ERR_NIVA_SESSION_EXPIRED");
    return new Promise((resolve) => {
      // This mirrors an application reporting a failed write from a Promise catch.
      // The catch runs after onwriteError has destroyed the Writable, while its
      // close notification can be queued ahead of this write's callback.
      stdout.write("write from promise catch", (writeError) => {
        resolve({ writeError, closeObserved });
      });
    });
  });

  await new Promise((resolve) => setImmediate(resolve));
  const oldRequest = requests.find((request) => request.t === "api_call" && request.method === "process.write");
  assert.ok(oldRequest);
  assert.equal(oldRequest.sessionId, oldSessionId);
  for (const listener of pageListeners.get("pagehide") || []) listener({ persisted: true });
  for (const listener of pageListeners.get("pageshow") || []) listener({ persisted: true });

  const { writeError, closeObserved: wasClosedBeforeRetry } = await handledWriteFailure;
  assert.equal(writeError?.code, "ERR_STREAM_DESTROYED");
  assert.equal(wasClosedBeforeRetry, true, "the post-destroy write callback settles after close was emitted");
  assert.equal(stdout.destroyed, false, "the final tracked callback retries recovery after the earlier close check");

  const newSessionId = context.Niva.bridge.sessionId;
  await new Promise((resolve, reject) => {
    stdout.write("restored after promise catch", (error) => error ? reject(error) : resolve());
  });
  const outputCalls = requests.filter((request) => request.t === "api_call" && request.method === "process.write");
  assert.deepEqual(outputCalls.map((request) => ({
    sessionId: request.sessionId,
    text: Buffer.from(request.args[1], "base64").toString(),
  })), [
    { sessionId: oldSessionId, text: "write before suspend" },
    { sessionId: newSessionId, text: "restored after promise catch" },
  ], "the rejected post-destroy write and any old-session write are never replayed");
  releaseOldWrite();
  await new Promise((resolve) => setImmediate(resolve));
  assert.equal(oldRequest.replyAccepted, false, "a late old-session reply stays detached from the restored stream");
});

test("buffered stdio writes fail once on BFCache retirement without replay", async () => {
  let releaseOldWrite;
  const { context, requests, pageListeners } = bootPage({
    ipcReply(request) {
      if (request.t === "api_call" && request.method === "process.write") {
        const text = Buffer.from(request.args[1], "base64").toString();
        if (text.startsWith("restored ")) return { t: "result", id: request.id, code: 0, data: null };
        return new Promise((resolve) => { releaseOldWrite = () => resolve({ t: "result", id: request.id, code: 0, data: null }); });
      }
      if (request.t === "heartbeat") return { t: "heartbeatAck", sessionId: request.sessionId };
      return { t: "result", id: request.id, code: 0, data: null };
    },
  });
  const stdout = context.Niva.process.stdout;
  const oldSessionId = context.Niva.bridge.sessionId;
  const callbacks = [];
  stdout.write("active old write", (error) => callbacks.push({ text: "active", error }));
  stdout.write("buffered old write", (error) => callbacks.push({ text: "buffered", error }));
  stdout.write("buffered old write without callback");
  await new Promise((resolve) => setImmediate(resolve));
  assert.equal(requests.filter((request) => request.t === "api_call" && request.method === "process.write").length, 1);
  const closed = new Promise((resolve) => stdout.once("close", resolve));

  for (const listener of pageListeners.get("pagehide") || []) listener({ persisted: true });
  for (const listener of pageListeners.get("pageshow") || []) listener({ persisted: true });
  const newSessionId = context.Niva.bridge.sessionId;
  await closed;
  assert.deepEqual(callbacks.map(({ text, error }) => [text, error?.code]).sort(), [
    ["active", "ERR_NIVA_SESSION_EXPIRED"],
    ["buffered", "ERR_NIVA_SESSION_EXPIRED"],
  ]);
  assert.equal(stdout.destroyed, false, "all buffered callbacks, including the implicit callback, have drained");
  await new Promise((resolve, reject) => stdout.write("restored output", (error) => error ? reject(error) : resolve()));
  let endError;
  await new Promise((resolve) => stdout.end((error) => { endError = error; resolve(); }));
  assert.equal(endError, undefined);
  assert.equal(stdout.writableFinished, true, "normal end accounting works after the pinned pendingcb repair");
  const outputCalls = requests.filter((request) => request.t === "api_call" && request.method === "process.write");
  assert.deepEqual(outputCalls.map((request) => ({
    sessionId: request.sessionId,
    text: Buffer.from(request.args[1], "base64").toString(),
  })), [
    { sessionId: oldSessionId, text: "active old write" },
    { sessionId: newSessionId, text: "restored output" },
  ], "buffered old writes are never sent or replayed; only the fresh-session write is sent");
  releaseOldWrite();
  await new Promise((resolve) => setImmediate(resolve));
  assert.equal(outputCalls[0].replyAccepted, false, "a late old-session response remains isolated");
});

test("BFCache stdio recovery excludes explicit destroy and ordinary write errors", async () => {
  let releaseOldWrite;
  const { context, requests, pageListeners } = bootPage({
    fakeTimers: true,
    ipcReply(request) {
      if (request.t === "api_call" && request.method === "process.write" && request.args[0] === "stdout") {
        return new Promise((resolve) => { releaseOldWrite = () => resolve({ t: "result", id: request.id, code: 0, data: null }); });
      }
      if (request.t === "api_call" && request.method === "process.write" && request.args[0] === "stderr") {
        return { t: "result", id: request.id, code: 1, message: "disk write failed" };
      }
      if (request.t === "heartbeat") return { t: "heartbeatAck", sessionId: request.sessionId };
      return { t: "result", id: request.id, code: 0, data: null };
    },
  });
  const stdout = context.Niva.process.stdout;
  const stderr = context.Niva.process.stderr;
  let stdoutCallbackError;
  let stderrCallbackError;
  stdout.write("destroy in callback", (error) => {
    stdoutCallbackError = error;
    stdout.destroy(error);
  });
  stderr.on("error", () => stderr.destroy());
  stderr.write("ordinary failure", (error) => { stderrCallbackError = error; });
  await new Promise((resolve) => setImmediate(resolve));
  assert.equal(typeof releaseOldWrite, "function");
  assert.equal(stderr.destroyed, true, "an ordinary process.write error remains fatal to that Writable");
  assert.notEqual(stderrCallbackError?.code, "ERR_NIVA_SESSION_EXPIRED");

  for (const listener of pageListeners.get("pagehide") || []) listener({ persisted: true });
  for (const listener of pageListeners.get("pageshow") || []) listener({ persisted: true });
  await new Promise((resolve) => setImmediate(resolve));
  assert.equal(stdoutCallbackError?.code, "ERR_NIVA_SESSION_EXPIRED");
  assert.equal(stdout.destroyed, true, "an explicit destroy from the failed write callback is never undone");
  assert.equal(stderr.destroyed, true, "ordinary write failure is not reclassified by a later BFCache restore");
  assert.equal(requests.filter((request) => request.t === "api_call" && request.method === "process.write").length, 2,
    "neither failed stream is revived by replaying a write");

  releaseOldWrite();
  await new Promise((resolve) => setImmediate(resolve));
  assert.equal(requests.filter((request) => request.t === "api_call" && request.method === "process.write").length, 2);
});

test("BFCache stdio recovery does not undo an explicit end", async () => {
  let releaseOldWrite;
  const { context, requests, pageListeners } = bootPage({
    ipcReply(request) {
      if (request.t === "api_call" && request.method === "process.write") {
        return new Promise((resolve) => { releaseOldWrite = () => resolve({ t: "result", id: request.id, code: 0, data: null }); });
      }
      if (request.t === "heartbeat") return { t: "heartbeatAck", sessionId: request.sessionId };
      return { t: "result", id: request.id, code: 0, data: null };
    },
  });
  const stdout = context.Niva.process.stdout;
  let writeError;
  let endError;
  stdout.write("pending before end", (error) => { writeError = error; });
  stdout.end((error) => { endError = error; });
  await new Promise((resolve) => setImmediate(resolve));
  const closed = new Promise((resolve) => stdout.once("close", resolve));
  for (const listener of pageListeners.get("pagehide") || []) listener({ persisted: true });
  for (const listener of pageListeners.get("pageshow") || []) listener({ persisted: true });
  await closed;

  assert.equal(writeError?.code, "ERR_NIVA_SESSION_EXPIRED");
  assert.equal(endError?.code, "ERR_NIVA_SESSION_EXPIRED");
  assert.equal(stdout.writableEnded, true);
  assert.equal(stdout.destroyed, true, "_undestroy must not make an intentionally ended output writable again");
  assert.equal(requests.filter((request) => request.t === "api_call" && request.method === "process.write").length, 1);
  releaseOldWrite();
});

test("same-origin iframe BFCache restore replaces its frame route and ignores old replies", async () => {
  let releaseStaleCall;
  const parent = bootPage({
    injectCommonJs: false,
    injectEsm: false,
    fakeTimers: true,
    webSocketThrows: 1,
    ipcReply(request) {
      if (request.t === "api_call" && request.method === "child.stale") {
        return new Promise((resolve) => { releaseStaleCall = () => resolve({ t: "result", id: request.id, code: 0, data: "late child" }); });
      }
      if (request.t === "heartbeat") return { t: "heartbeatAck", sessionId: request.sessionId };
      return { t: "result", id: request.id, code: 0, data: `ok:${request.method}` };
    },
  });
  const child = bootPage({
    injectCommonJs: false,
    injectEsm: false,
    fakeTimers: true,
    webSocketThrows: 1,
    iframeParent: parent,
  });
  // vm contexts do not supply a WindowProxy-backed frames collection, so set
  // the exact same-origin child object used by the runtime's route check.
  parent.context.frames = [child.context];
  const frameRouteActions = [];
  const registerFrameSession = parent.context.__niva_register_frame_session;
  parent.context.__niva_register_frame_session = function (session, frame, action) {
    const result = registerFrameSession(session, frame, action);
    frameRouteActions.push({ session, frame, action, result });
    return result;
  };
  const oldSessionId = child.context.Niva.bridge.sessionId;
  const stale = child.context.Niva.bridge.call("child.stale", []).then(
    (value) => ({ value }),
    (error) => ({ error }),
  );
  await new Promise((resolve) => setImmediate(resolve));
  assert.equal(typeof releaseStaleCall, "function");
  assert.ok(parent.requests.some((request) => request.t === "api_call" && request.sessionId === oldSessionId));

  for (const listener of parent.pageListeners.get("pagehide") || []) listener({ persisted: true });
  for (const listener of child.pageListeners.get("pagehide") || []) listener({ persisted: true });
  assert.equal((await stale).error?.code, "ERR_NIVA_SESSION_EXPIRED");
  for (const listener of parent.pageListeners.get("pageshow") || []) listener({ persisted: true });
  for (const listener of child.pageListeners.get("pageshow") || []) listener({ persisted: true });
  const newSessionId = child.context.Niva.bridge.sessionId;
  assert.ok(frameRouteActions.some((route) => route.session === oldSessionId && route.action === "unregister"));
  assert.ok(frameRouteActions.some((route) => route.session === newSessionId && route.action === "register"));
  assert.notEqual(newSessionId, oldSessionId);
  // The VM does not preserve WindowProxy identity in `frames`; mirror the
  // browser's same-origin route registration after observing the child's
  // restored-session announce.
  assert.equal(registerFrameSession(newSessionId, child.context, "register"), true);

  releaseStaleCall();
  await new Promise((resolve) => setImmediate(resolve));
  const oldRequest = parent.requests.find((request) => request.t === "api_call" && request.method === "child.stale");
  assert.equal(oldRequest.replyAccepted, false, "the parent has no route for a retired iframe session");
  const childResultPromise = child.context.Niva.bridge.call("child.afterRestore", []);
  await new Promise((resolve) => setImmediate(resolve));
  const childResult = await childResultPromise;
  assert.equal(childResult, "ok:child.afterRestore");
  const restoredRequest = parent.requests.find((request) => request.t === "api_call" && request.method === "child.afterRestore");
  assert.equal(restoredRequest.sessionId, newSessionId);
  assert.equal(restoredRequest.replyAccepted, true, "the iframe registers a fresh same-origin route before new IPC replies");
  assert.ok(parent.requests.some((request) => request.t === "session_close" && request.sessionId === oldSessionId));
});

test("retired stream API tickets cannot start an attachment after BFCache restore", async () => {
  let releaseTicket;
  const { context, requests, pageListeners } = bootPage({
    injectCommonJs: false,
    injectEsm: false,
    fakeTimers: true,
    webSocketThrows: 1,
    ipcReply(request) {
      if (request.t === "api_call") {
        return new Promise((resolve) => {
          releaseTicket = () => resolve({ t: "channelOpened", id: request.id, capability: "retired-ticket" });
        });
      }
      if (request.t === "heartbeat") return { t: "heartbeatAck", sessionId: request.sessionId };
      return { t: "channelCancelled", id: request.id, accepted: true };
    },
  });
  const oldSession = context.Niva.bridge.sessionId;
  const stream = context.Niva.bridge.stream("stream.ticketLate", []);
  const settled = stream.promise.then((value) => ({ value }), (error) => ({ error }));
  await new Promise((resolve) => setImmediate(resolve));
  const openRequest = requests.find((request) => request.t === "api_call" && request.method === "stream.ticketLate");
  assert.ok(openRequest);

  for (const listener of pageListeners.get("pagehide") || []) listener({ persisted: true });
  assert.equal((await settled).error?.code, "ERR_NIVA_SESSION_EXPIRED");
  for (const listener of pageListeners.get("pageshow") || []) listener({ persisted: true });
  const freshSession = context.Niva.bridge.sessionId;
  assert.notEqual(freshSession, oldSession);
  releaseTicket();
  await new Promise((resolve) => setImmediate(resolve));

  assert.equal(openRequest.replyAccepted, false, "the old session cannot accept the delayed ticket reply");
  assert.equal(requests.filter((request) => request.t === "api_call" && request.method === "stream.ticketLate").length, 1,
    "restore never replays an API call that may have had side effects");
  assert.equal(requests.some((request) => request.t === "channelAttach" && request.id === openRequest.id), false,
    "a ticket arriving after retirement cannot attach or begin data delivery");
});

test("retired IPC attach acknowledgements cannot open a channel in the restored session", async () => {
  let releaseAttach;
  const { context, requests, pageListeners } = bootPage({
    injectCommonJs: false,
    injectEsm: false,
    fakeTimers: true,
    webSocketThrows: 1,
    ipcReply(request) {
      if (request.t === "api_call") return { t: "channelOpened", id: request.id, capability: "retired-attach" };
      if (request.t === "channelCancel") return { t: "channelCancelled", id: request.id, accepted: true };
      if (request.t === "heartbeat") return { t: "heartbeatAck", sessionId: request.sessionId };
      return { t: "result", id: request.id, code: 0 };
    },
    ipcAttachReply(request) {
      return new Promise((resolve) => {
        releaseAttach = () => resolve({ t: "channelAttached", id: request.id, accepted: true });
      });
    },
  });
  const oldSession = context.Niva.bridge.sessionId;
  const stream = context.Niva.bridge.stream("stream.attachLate", []);
  const settled = stream.promise.then((value) => ({ value }), (error) => ({ error }));
  await new Promise((resolve) => setImmediate(resolve));
  const attachRequest = requests.find((request) => request.t === "channelAttach");
  assert.ok(attachRequest);

  for (const listener of pageListeners.get("pagehide") || []) listener({ persisted: true });
  assert.equal((await settled).error?.code, "ERR_NIVA_SESSION_EXPIRED");
  for (const listener of pageListeners.get("pageshow") || []) listener({ persisted: true });
  const freshSession = context.Niva.bridge.sessionId;
  assert.notEqual(freshSession, oldSession);
  releaseAttach();
  await new Promise((resolve) => setImmediate(resolve));

  assert.equal(attachRequest.replyAccepted, false, "the attach response belongs to the retired session");
  assert.equal(requests.filter((request) => request.t === "api_call" && request.method === "stream.attachLate").length, 1);
  assert.equal(context.__niva_native_frame({
    sessionId: oldSession,
    id: attachRequest.id,
    seq: 1,
    frame: { t: "text", data: JSON.stringify({ t: "event", id: attachRequest.id, name: "should.not.open", data: {} }) },
  }), false, "late data cannot enter the new session through the old call id");
});

test("lease expiry cannot be revived by a later persisted pagehide/pageshow pair", async () => {
  const { context, pageListeners } = bootPage({
    injectCommonJs: false,
    injectEsm: false,
    ipcReply(request) {
      if (request.t === "heartbeat") return { t: "heartbeatError", sessionId: request.sessionId, code: "ERR_NIVA_SESSION_EXPIRED", message: "lease ended" };
      return new Promise(() => {});
    },
  });
  const pendingCall = context.Niva.bridge.call("never.reply", []).then(
    (value) => ({ value }),
    (error) => ({ error }),
  );
  const outcome = await pendingCall;
  assert.equal(outcome.error?.code, "ERR_NIVA_SESSION_EXPIRED");
  const expiredSession = context.Niva.bridge.sessionId;
  for (const listener of pageListeners.get("pagehide") || []) listener({ persisted: true });
  for (const listener of pageListeners.get("pageshow") || []) listener({ persisted: true });
  assert.equal(context.Niva.bridge.sessionId, expiredSession);
  await assert.rejects(context.Niva.bridge.call("must.stay.closed", []), { code: "ERR_NIVA_SESSION_EXPIRED" });
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

test("API calls stay on IPC while an available WebSocket only attaches stream tickets", async () => {
  const warnings = [];
  const { context, sockets, requests } = bootPage({
    injectCommonJs: false,
    injectEsm: false,
    fakeTimers: true,
    consoleImpl: { warn: (message) => warnings.push(message), error() {} },
    ipcReply(request) {
      if (request.t === "api_call" && request.method === "fs.readStream") {
        return { t: "channelOpened", id: request.id, capability: "ws-capability" };
      }
      if (request.t === "channelAttach") return { t: "channelAttached", id: request.id, accepted: true };
      return { t: "result", id: request.id, code: 0, data: { method: request.method } };
    },
  });
  const socket = sockets[0];
  socket.open();
  const helloFrame = socket.sent.map((frame) => JSON.parse(frame)).find((frame) => frame.t === "hello");
  assert.equal(helloFrame.sessionId, context.Niva.bridge.sessionId);
  assert.equal(helloFrame.v, 2);
  const unaryResult = await context.Niva.bridge.call("window.getTitle", []);
  assert.deepEqual(JSON.parse(JSON.stringify(unaryResult)), { method: "window.getTitle" });
  assert.equal(requests[0].transport, "ipc");
  assert.equal(requests[0].method, "window.getTitle");
  assert.equal(requests[0].t, "api_call");
  assert.equal(socket.sent.map((frame) => JSON.parse(frame)).filter((frame) => frame.t === "call").length, 0);

  const stream = context.Niva.bridge.stream("fs.readStream", []);
  const streamResult = stream.promise;
  await new Promise((resolve) => setImmediate(resolve));
  const streamRequest = requests.find((request) => request.method === "fs.readStream");
  assert.equal(streamRequest.t, "api_call");
  const attach = socket.sent.map((text) => JSON.parse(text)).find((frame) => frame.t === "attach");
  assert.deepEqual(JSON.parse(JSON.stringify(attach)), {
    t: "attach", id: stream.id, sessionId: context.Niva.bridge.sessionId, capability: "ws-capability",
  });
  assert.equal(Object.hasOwn(attach, "method"), false);
  socket.receive(JSON.stringify({ t: "attached", id: stream.id, sessionId: context.Niva.bridge.sessionId }));
  socket.receive(JSON.stringify({
    t: "channelData", id: stream.id, seq: 1,
    frame: { t: "text", data: JSON.stringify({ t: "result", id: stream.id, code: 0, data: "stream done" }) },
  }));
  assert.equal(await streamResult, "stream done");
  assert.ok(socket.sent.map((frame) => JSON.parse(frame)).some((frame) => frame.t === "ack" && frame.id === stream.id && frame.seq === 1));
  assert.deepEqual(requests.filter((request) => request.method).map((request) => request.t), ["api_call", "api_call"]);

  assert.equal(context.Niva.bridge.callSync("process.currentDir", []), "/app");
  assert.equal(warnings.length, 1);
  assert.match(warnings[0], /synchronous XHR/);
  assert.equal(context.Niva.bridge.callSync("process.currentDir", []), "/app");
  assert.equal(warnings.length, 1, "the sync warning is emitted once per method per realm");
});

test("native stream wrappers accept small direct results without attaching a data lane", async () => {
  const { context, requests, sockets } = bootPage({
    injectCommonJs: false,
    injectEsm: false,
    fakeTimers: true,
    ipcReply(request) {
      if (request.t === "api_call" && request.method === "process.execStream") {
        return { t: "channelOpened", id: request.id, capability: "owner-capability" };
      }
      if (request.t === "api_call" && request.method === "process.signal") {
        return { t: "result", id: request.id, code: 0, data: true };
      }
      return { t: "heartbeatAck", sessionId: request.sessionId };
    },
  });
  const owner = context.Niva.bridge.stream("process.execStream", ["helper"]);
  owner.promise.catch(() => {});
  await new Promise((resolve) => setImmediate(resolve));
  const runtime = context[Symbol.for("niva.node-compat.runtime")];
  const control = runtime.streamRelated(context.Niva, owner, "process.signal", [owner.id, "SIGTERM"]);
  assert.equal(await control.promise, true);
  const signal = requests.find((request) => request.method === "process.signal");
  assert.equal(signal.t, "api_call");
  assert.equal(requests.some((request) => request.t === "channelAttach" && request.id === signal.id), false);
  assert.equal(sockets[0].sent.map((frame) => JSON.parse(frame)).some((frame) => frame.id === signal.id), false);
  owner.cancel();
});

test("IPC reply settlement checks the requested session, rid, and source origin", async () => {
  let callRequest;
  const { context } = bootPage({
    injectCommonJs: false,
    injectEsm: false,
    fakeTimers: true,
    ipcReply(request) {
      callRequest = request;
      return new Promise(() => {});
    },
  });
  const result = context.Niva.bridge.call("window.getTitle", []);
  await new Promise((resolve) => setImmediate(resolve));
  const sessionId = context.Niva.bridge.sessionId;
  const sourceOrigin = context.location.origin;
  const response = { t: "result", id: callRequest.id, code: 0, data: "settled" };
  const envelope = { sessionId, rid: callRequest.rid, sourceOrigin, response };
  assert.equal(context.__niva_ipc_reply({ ...envelope, sessionId: "other-session" }), false);
  assert.equal(context.__niva_ipc_reply({ ...envelope, rid: callRequest.rid + 1 }), false);
  assert.equal(context.__niva_ipc_reply({ ...envelope, sourceOrigin: "https://attacker.test" }), false);
  assert.equal(context.__niva_ipc_reply({ ...envelope, sourceOrigin: "null" }), false);
  assert.equal(context.__niva_ipc_reply(envelope), true);
  assert.equal(await result, "settled");
  assert.equal(context.__niva_ipc_reply(envelope), false, "a settled rid cannot be replayed");
});

test("opaque packaged origins require the matching Native-injected app origin", async () => {
  const { context, requests } = bootPage({
    injectCommonJs: false,
    injectEsm: false,
    fakeTimers: true,
    origin: "null",
    href: "niva-12345678-1234-1234-1234-123456789abc://app/index.html",
    sourceOrigin: "niva-12345678-1234-1234-1234-123456789abc://app",
    ipcReply(request) { return { t: "result", id: request.id, code: 0, data: "custom-origin" }; },
  });
  assert.equal(await context.Niva.bridge.call("window.getTitle", []), "custom-origin");
  assert.equal(requests.find((request) => request.t === "api_call").sourceOrigin, undefined, "page metadata is not sent as authorization input");

  const rejected = bootPage({
    injectCommonJs: false,
    injectEsm: false,
    origin: "null",
    href: "niva-12345678-1234-1234-1234-123456789abc://app/index.html",
    sourceOrigin: "niva-12345678-1234-1234-1234-123456789abc://other",
    ipcReply() { return null; },
  });
  await assert.rejects(rejected.context.Niva.bridge.call("window.getTitle", []), { code: "ERR_NIVA_IPC_UNAVAILABLE" });
  assert.equal(rejected.requests.length, 0);
});

test("async reply handling does not depend on platform-specific WebView reply handlers", () => {
  assert.doesNotMatch(bootstrapSource, /webkit\.messageHandlers\.nivaReply|chrome\.webview/);
});

test("sync compatibility aliases warn once by their public Node method name", () => {
  const warnings = [];
  const { context } = bootPage({
    injectCommonJs: false,
    injectEsm: false,
    consoleImpl: { warn: (message) => warnings.push(message), error() {} },
    syncReply({ method, args }) {
      if (method === "fs.node") {
        if (args[0] === "readFile") return "eA==";
        return { size: 1, mtimeMs: 1 };
      }
      if (method === "process.spawnSync") return { stdout: "", stderr: "", status: 0, signal: null };
      if (method === "process.currentDir") return "/app";
      throw new Error(`Unexpected sync method ${method}`);
    },
  });
  const fs = context.Niva.fs;
  assert.equal(fs.readFileSync("/file", "utf8"), "x");
  assert.equal(fs.readFileSync("/file", "utf8"), "x");
  assert.equal(fs.statSync("/file").size, 1);
  assert.equal(fs.statSync("/file").size, 1);

  const childProcess = context.Niva.child_process;
  assert.equal(childProcess.spawnSync("noop", []).status, 0);
  assert.equal(childProcess.spawnSync("noop", []).status, 0);
  assert.equal(childProcess.execSync("noop").length, 0);
  assert.equal(childProcess.execSync("noop").length, 0);
  assert.equal(childProcess.execFileSync("noop", []).length, 0);
  assert.equal(childProcess.execFileSync("noop", []).length, 0);
  assert.equal(context.Niva.process.cwd(), "/app");
  assert.equal(context.Niva.process.cwd(), "/app");

  const names = warnings.map((message) => /^Niva synchronous compatibility API '([^']+)'/.exec(message)?.[1]);
  assert.deepEqual(names, ["readFileSync", "statSync", "spawnSync", "execSync", "execFileSync", "process.cwd"]);
});

test("CommonJS module resolve and load warnings each emit once with their RPC names", () => {
  const warnings = [];
  const { context } = bootPage({
    injectCommonJs: true,
    injectEsm: false,
    consoleImpl: { warn: (message) => warnings.push(message), error() {} },
  });
  context.require("./cycle-a.cjs");
  context.require("./cycle-b.cjs");
  context.require.resolve("./cycle-a.cjs");
  const names = warnings.map((message) => /^Niva synchronous compatibility API '([^']+)'/.exec(message)?.[1]);
  assert.deepEqual(names, ["module.resolve", "module.load"]);
});

test("callSyncAs and related streams delegate through the current bridge methods", () => {
  const { context } = bootPage({ injectCommonJs: false, injectEsm: false });
  const runtime = context[Symbol.for("niva.node-compat.runtime")];
  const calls = [];
  context.Niva.bridge.stream = (method, args, handlers) => {
    calls.push(["stream", method, args, handlers]);
    return { id: 77, promise: Promise.resolve(null), cancel() {} };
  };
  context.Niva.bridge.callSync = (method, args, apiName) => {
    calls.push(["callSync", method, args, apiName]);
    return apiName;
  };

  const related = runtime.streamRelated(context.Niva, { id: 76 }, "resource.control", ["id"]);
  assert.equal(related.id, 77);
  assert.equal(JSON.stringify(calls[0]), JSON.stringify(["stream", "resource.control", ["id"], {}]));
  assert.equal(runtime.callSyncAs(context.Niva, "readFileSync", "fs.node", ["readFile"]), "readFileSync");
  assert.equal(JSON.stringify(calls[1]), JSON.stringify(["callSync", "fs.node", ["readFile"], "readFileSync"]));
});

test("async IPC calls remain available when WebSocket setup fails", async () => {
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
  assert.equal(requests.length, 2, "ordinary async calls always stay on IPC");
});

test("a stream started before WebSocket opens stays on IPC; later streams may use WS", async () => {
  const { context, requests, sockets } = bootPage({
    injectCommonJs: false,
    injectEsm: false,
    ipcReply(request) {
      if (request.t === "api_call") return { t: "channelOpened", id: request.id, capability: "fixture-capability" };
      if (request.t === "channelAttach") return { t: "channelAttached", id: request.id, accepted: true };
      if (request.t === "channelAck") return { t: "channelAckReceived", id: request.id, seq: request.seq, accepted: true };
      if (request.t === "heartbeat") return { t: "heartbeatAck", sessionId: request.sessionId };
      throw new Error(`Unexpected IPC message ${request.t}`);
    },
  });
  const first = context.Niva.bridge.stream("fs.readStream", []);
  await new Promise((resolve) => setImmediate(resolve));
  assert.equal(requests.find((request) => request.t === "api_call").transport, "ipc");
  assert.ok(requests.some((request) => request.t === "channelAttach" && request.id === first.id));
  assert.equal(sockets.length, 1);
  sockets[0].open();
  context.__niva_native_frame({
    sessionId: context.Niva.bridge.sessionId,
    id: first.id,
    seq: 1,
    frame: { t: "text", data: JSON.stringify({ t: "result", id: first.id, code: 0, message: "ok", data: "ipc stream done" }) },
  });
  assert.equal(await first.promise, "ipc stream done");

  const second = context.Niva.bridge.stream("fs.readStream", []);
  const secondResult = second.promise;
  await new Promise((resolve) => setImmediate(resolve));
  const wsCall = requests.find((request) => request.t === "api_call" && request.id === second.id);
  assert.equal(wsCall.method, "fs.readStream");
  const wsAttach = sockets[0].sent.map((frame) => JSON.parse(frame)).find((frame) => frame.t === "attach" && frame.id === second.id);
  assert.equal(wsAttach.capability, "fixture-capability");
  sockets[0].receive(JSON.stringify({ t: "attached", id: second.id, sessionId: context.Niva.bridge.sessionId }));
  sockets[0].receive(JSON.stringify({
    t: "channelData", id: second.id, seq: 1,
    frame: { t: "text", data: JSON.stringify({ t: "result", id: second.id, code: 0, data: "ws stream done" }) },
  }));
  assert.equal(await secondResult, "ws stream done");
  assert.equal(requests.filter((request) => request.t === "api_call").length, 2);
});

test("explicit WS attach rejection reuses the existing ticket on IPC without replaying the API", async () => {
  const { context, requests, sockets } = bootPage({
    injectCommonJs: false,
    injectEsm: false,
    fakeTimers: true,
    ipcReply(request) {
      if (request.t === "api_call") return { t: "channelOpened", id: request.id, capability: "same-ticket" };
      if (request.t === "channelAck") return { t: "channelAckReceived", id: request.id, seq: request.seq, accepted: true };
      if (request.t === "heartbeat") return { t: "heartbeatAck", sessionId: request.sessionId };
      throw new Error(`Unexpected IPC message ${request.t}`);
    },
  });
  const socket = sockets[0];
  socket.open();
  const stream = context.Niva.bridge.stream("fs.readStream", []);
  const settled = stream.promise;
  await new Promise((resolve) => setImmediate(resolve));
  socket.receive(JSON.stringify({
    t: "attachError", id: stream.id, sessionId: context.Niva.bridge.sessionId,
    code: "ERR_NIVA_WS_ATTACH", message: "not attached",
  }));
  await new Promise((resolve) => setImmediate(resolve));
  const apiCalls = requests.filter((request) => request.t === "api_call");
  const ipcAttach = requests.find((request) => request.t === "channelAttach");
  assert.equal(apiCalls.length, 1);
  assert.equal(ipcAttach.id, stream.id);
  assert.equal(ipcAttach.capability, "same-ticket");
  assert.deepEqual(socket.sent.map((frame) => JSON.parse(frame)).filter((frame) => frame.t !== "hello"), [
    { t: "attach", id: stream.id, sessionId: context.Niva.bridge.sessionId, capability: "same-ticket" },
  ]);
  context.__niva_native_frame({
    sessionId: context.Niva.bridge.sessionId,
    id: stream.id,
    seq: 1,
    frame: { t: "text", data: JSON.stringify({ t: "result", id: stream.id, code: 0, data: "ipc fallback" }) },
  });
  assert.equal(await settled, "ipc fallback");
});

test("WS binary data uses the outer sequence for its ACK and terminal result", async () => {
  const { context, requests, sockets } = bootPage({
    injectCommonJs: false,
    injectEsm: false,
    fakeTimers: true,
    ipcReply(request) {
      if (request.t === "api_call") return { t: "channelOpened", id: request.id, capability: "binary-capability" };
      if (request.t === "heartbeat") return { t: "heartbeatAck", sessionId: request.sessionId };
      throw new Error(`Unexpected IPC message ${request.t}`);
    },
  });
  const socket = sockets[0];
  socket.open();
  const chunks = [];
  const stream = context.Niva.bridge.stream("fs.handle", [3, "read", { length: 3 }], {
    onChunk: (chunk) => chunks.push(...chunk),
  });
  const settled = stream.promise;
  await new Promise((resolve) => setImmediate(resolve));
  socket.receive(JSON.stringify({ t: "attached", id: stream.id, sessionId: context.Niva.bridge.sessionId }));

  const binary = vm.runInContext("new ArrayBuffer(21)", context);
  const view = new DataView(binary);
  view.setUint8(0, 2);
  view.setUint32(6, stream.id);
  view.setUint32(14, 1);
  new Uint8Array(binary, 18).set([4, 5, 6]);
  socket.receive(binary);
  await new Promise((resolve) => setImmediate(resolve));
  assert.deepEqual(chunks, [4, 5, 6]);

  socket.receive(JSON.stringify({
    t: "channelData", id: stream.id, seq: 2,
    frame: { t: "text", data: JSON.stringify({ t: "result", id: stream.id, code: 0, data: { bytesRead: 3 } }) },
  }));
  assert.deepEqual(JSON.parse(JSON.stringify(await settled)), { bytesRead: 3 });
  const acks = socket.sent.map((frame) => JSON.parse(frame)).filter((frame) => frame.t === "ack");
  assert.deepEqual(acks.map((frame) => frame.seq), [1, 2]);
  assert.equal(requests.some((request) => request.t === "channelAck"), false, "WS receives ACKs on the same lane");
});

test("WS binary frames with a mismatched outer sequence are rejected and never ACKed", async () => {
  const { context, sockets } = bootPage({
    injectCommonJs: false,
    injectEsm: false,
    fakeTimers: true,
    ipcReply(request) {
      if (request.t === "api_call") return { t: "channelOpened", id: request.id, capability: "bad-sequence" };
      if (request.t === "heartbeat") return { t: "heartbeatAck", sessionId: request.sessionId };
      throw new Error(`Unexpected IPC message ${request.t}`);
    },
  });
  const socket = sockets[0];
  socket.open();
  const stream = context.Niva.bridge.stream("fs.handle", [3, "read", { length: 3 }]);
  const rejected = assert.rejects(stream.promise, { code: "ERR_NIVA_CHANNEL_SEQUENCE" });
  await new Promise((resolve) => setImmediate(resolve));
  socket.receive(JSON.stringify({ t: "attached", id: stream.id, sessionId: context.Niva.bridge.sessionId }));
  const binary = vm.runInContext("new ArrayBuffer(19)", context);
  const view = new DataView(binary);
  view.setUint8(0, 2);
  view.setUint32(6, stream.id);
  view.setUint32(14, 2);
  new Uint8Array(binary, 18).set([9]);
  socket.receive(binary);
  await rejected;
  const frames = socket.sent.map((frame) => typeof frame === "string" ? JSON.parse(frame) : null).filter(Boolean);
  assert.equal(frames.some((frame) => frame.t === "ack"), false);
  assert.ok(frames.some((frame) => frame.t === "cancel" && frame.id === stream.id));
});

test("related resource calls keep their IPC owner after WebSocket connects", async () => {
  const { context, requests, sockets } = bootPage({
    injectCommonJs: false,
    injectEsm: false,
    fakeTimers: true,
    ipcReply(request) {
      if (request.t === "api_call" && request.method === "fs.openHandle") return { t: "channelOpened", id: request.id, capability: "fixture-capability" };
      if (request.t === "api_call" && request.method === "fs.handleControl") return { t: "result", id: request.id, code: 0, data: 0 };
      if (request.t === "channelAttach") return { t: "channelAttached", id: request.id, accepted: true };
      if (request.t === "channelCancel") return { t: "channelCancelled", id: request.id, accepted: true };
      if (request.t === "heartbeat") return { t: "heartbeatAck", sessionId: request.sessionId };
      throw new Error(`Unexpected IPC message ${request.t}`);
    },
  });
  const runtime = context[Symbol.for("niva.node-compat.runtime")];
  const owner = context.Niva.bridge.stream("fs.openHandle", ["/tmp/owned"]);
  owner.promise.catch(() => {});
  await new Promise((resolve) => setImmediate(resolve));
  assert.equal(requests.find((request) => request.t === "api_call").transport, "ipc");

  sockets[0].open();
  const control = runtime.streamRelated(context.Niva, owner, "fs.handleControl", ["handle-id", "stat", {}]);
  control.promise.catch(() => {});
  await new Promise((resolve) => setImmediate(resolve));

  const calls = requests.filter((request) => request.t === "api_call");
  assert.deepEqual(calls.map((request) => request.transport), ["ipc", "ipc"]);
  assert.deepEqual(calls.map((request) => request.method), ["fs.openHandle", "fs.handleControl"]);
  assert.equal(sockets[0].sent.map((frame) => JSON.parse(frame)).filter((frame) => frame.t === "call").length, 0);
  assert.equal(await control.promise, 0, "small related API results do not open a data channel");

  owner.cancel();
  control.cancel();
});

test("related calls fail locally after their resource owner closes", async () => {
  const { context, requests, sockets } = bootPage({
    injectCommonJs: false,
    injectEsm: false,
    fakeTimers: true,
    ipcReply(request) {
      if (request.t === "api_call") return { t: "channelOpened", id: request.id, capability: "ws-capability" };
      if (request.t === "heartbeat") return { t: "heartbeatAck", sessionId: request.sessionId };
      throw new Error(`Unexpected IPC message ${request.t}`);
    },
  });
  const socket = sockets[0];
  socket.open();
  const owner = context.Niva.bridge.stream("process.execStream", ["helper"]);
  const ownerError = owner.promise.then(() => null, (error) => error);
  const wsCall = requests.find((request) => request.t === "api_call");
  assert.equal(wsCall.method, "process.execStream");
  await new Promise((resolve) => setImmediate(resolve));
  socket.receive(JSON.stringify({ t: "attached", id: owner.id, sessionId: context.Niva.bridge.sessionId }));

  socket.close();
  assert.equal((await ownerError).code, "ECONNRESET");
  const runtime = context[Symbol.for("niva.node-compat.runtime")];
  assert.throws(() => runtime.streamRelated(context.Niva, owner, "process.signal", [owner.id, "SIGTERM"]), { code: "ERR_NIVA_STREAM_OWNER_CLOSED" });
  assert.equal(requests.filter((request) => request.t === "api_call").length, 1, "closed resource owners do not issue control API calls");
});

test("internal route helpers honor monkey-patched bridge methods without exposing route choice", () => {
  const { context } = bootPage({ injectCommonJs: false, injectEsm: false });
  const runtime = context[Symbol.for("niva.node-compat.runtime")];
  const calls = [];
  context.Niva.bridge.stream = (method, args, handlers) => {
    calls.push(["stream", method, args, handlers]);
    return { id: 77, promise: Promise.resolve(null), cancel() {} };
  };
  context.Niva.bridge.callSync = (method, args, apiName) => {
    calls.push(["callSync", method, args, apiName]);
    return apiName;
  };

  const related = runtime.streamRelated(context.Niva, { id: 76 }, "resource.control", ["id"]);
  assert.equal(related.id, 77);
  assert.equal(JSON.stringify(calls[0]), JSON.stringify(["stream", "resource.control", ["id"], {}]));
  assert.equal(runtime.callSyncAs(context.Niva, "readFileSync", "fs.node", ["readFile"]), "readFileSync");
  assert.equal(JSON.stringify(calls[1]), JSON.stringify(["callSync", "fs.node", ["readFile"], "readFileSync"]));
});

test("WebSocket disconnect fails WS streams but preserves concurrent IPC calls and resources", async () => {
  let finishUnary;
  const { context, requests, sockets } = bootPage({
    injectCommonJs: false,
    injectEsm: false,
    fakeTimers: true,
    ipcReply(request) {
      if (request.t === "api_call" && request.method === "slow.ipc") {
        return new Promise((resolve) => { finishUnary = () => resolve({ t: "result", id: request.id, code: 0, data: "ipc survived" }); });
      }
      if (request.t === "api_call" && request.method === "fs.readStream") return { t: "channelOpened", id: request.id, capability: "fixture-capability" };
      if (request.t === "api_call") return { t: "result", id: request.id, code: 0, data: { method: request.method } };
      if (request.t === "channelAck") return { t: "channelAckReceived", id: request.id, seq: request.seq, accepted: true };
      if (request.t === "heartbeat") return { t: "heartbeatAck", sessionId: request.sessionId };
      throw new Error(`Unexpected IPC message ${request.t}`);
    },
  });
  const socket = sockets[0];
  const ipcStream = context.Niva.bridge.stream("fs.readStream", []);
  const ipcStreamSettled = ipcStream.promise.then((value) => ({ value }), (error) => ({ error }));
  await new Promise((resolve) => setImmediate(resolve));
  socket.open();
  const unary = context.Niva.bridge.call("slow.ipc", []);
  const wsStream = context.Niva.bridge.stream("fs.readStream", []);
  const wsStreamSettled = wsStream.promise.then((value) => ({ value }), (error) => ({ error }));
  const resource = { invalidated: false, __nivaInvalidate() { this.invalidated = true; } };
  const ipcResource = { invalidated: false, __nivaInvalidate() { this.invalidated = true; } };
  context.Niva.__runtime.registerResource(resource, undefined, wsStream.id);
  context.Niva.__runtime.registerResource(ipcResource, undefined, ipcStream.id);
  await new Promise((resolve) => setImmediate(resolve));
  socket.close();
  assert.equal((await wsStreamSettled).error.code, "ECONNRESET");
  assert.equal(resource.invalidated, true);
  assert.equal(ipcResource.invalidated, false);
  assert.equal(typeof finishUnary, "function");
  finishUnary();
  assert.equal(await unary, "ipc survived");
  context.__niva_native_frame({
    sessionId: context.Niva.bridge.sessionId,
    id: ipcStream.id,
    seq: 1,
    frame: { t: "text", data: JSON.stringify({ t: "result", id: ipcStream.id, code: 0, message: "ok", data: "ipc stream survived" }) },
  });
  assert.equal((await ipcStreamSettled).value, "ipc stream survived");
  assert.deepEqual(JSON.parse(JSON.stringify(await context.Niva.bridge.call("window.getTitle", []))), { method: "window.getTitle" });
  const fallbackStream = context.Niva.bridge.stream("fs.readStream", []);
  const fallbackSettled = fallbackStream.promise;
  await new Promise((resolve) => setImmediate(resolve));
  assert.equal(requests.filter((request) => request.transport === "ipc" && request.t === "api_call").length, 5);
  context.__niva_native_frame({
    sessionId: context.Niva.bridge.sessionId,
    id: fallbackStream.id,
    seq: 1,
    frame: { t: "text", data: JSON.stringify({ t: "result", id: fallbackStream.id, code: 0, message: "ok", data: "later IPC stream" }) },
  });
  assert.equal(await fallbackSettled, "later IPC stream");
  assert.equal(sockets.length, 1, "the failed WebSocket is not reconnected");
});

test("standard Wry IPC opens a Channel and native push is routed through rid-correlated ACKs", async () => {
  const { context, requests } = bootPage({
    injectCommonJs: false,
    injectEsm: false,
    fakeTimers: true,
    webSocketThrows: 1,
    windowsIpc(request) {
      if (request.t === "api_call") return { t: "channelOpened", id: request.id, capability: "fixture-capability" };
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
  const open = requests.find((request) => request.t === "api_call");
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
  const nativeRequests = [];
  const parent = bootPage({
    injectCommonJs: false,
    injectEsm: false,
    fakeTimers: true,
    webSocketThrows: 1,
    ipcReply(request) {
      nativeRequests.push(request);
      if (request.t === "api_call") return { t: "channelOpened", id: request.id, capability: "child-capability" };
      if (request.t === "channelAck") return { t: "channelAckReceived", id: request.id, seq: request.seq, accepted: true };
      return { t: "heartbeatAck", sessionId: request.sessionId };
    },
  });
  const child = bootPage({
    injectCommonJs: false,
    injectEsm: false,
    fakeTimers: true,
    webSocketThrows: 1,
    iframeParent: parent,
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
  // vm contexts do not model WindowProxy identity, so register the exact same-origin target explicitly.
  assert.equal(parent.context.__niva_register_frame_session(child.context.Niva.bridge.sessionId, child.context, "register"), true);

  const chunks = [];
  const stream = child.context.Niva.bridge.stream("fs.readStream", [], { onChunk: (bytes) => chunks.push(...bytes) });
  const settled = stream.promise.then((value) => ({ value }), (error) => ({ error }));
  await new Promise((resolve) => setImmediate(resolve));
  assert.ok(nativeRequests.some((request) => request.t === "api_call" && request.sessionId === child.context.Niva.bridge.sessionId));
  assert.ok(parent.requests.some((request) => request.t === "channelAttach" && request.sessionId === child.context.Niva.bridge.sessionId));
  const bytes = Buffer.from([0, 1, 127, 128, 255]);
  const binaryFrame = Buffer.alloc(18 + bytes.length);
  binaryFrame[0] = 2;
  binaryFrame[1] = 3;
  binaryFrame.writeUInt32BE(stream.id, 6);
  binaryFrame.writeUInt32BE(1, 14);
  bytes.copy(binaryFrame, 18);
  assert.equal(parent.context.__niva_native_frame({
    sessionId: child.context.Niva.bridge.sessionId,
    id: stream.id,
    seq: 1,
    frame: { t: "binary", data: binaryFrame.toString("base64") },
  }), true);
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
  assert.deepEqual(nativeRequests.filter((request) => request.t === "channelAck").map((request) => request.seq), [1, 2]);
  assert.equal(nativeRequests[0].sessionId, child.context.Niva.bridge.sessionId);
  assert.equal(nativeRequests[0].token, "fixture-secret");
  assert.ok(nativeRequests.some((request) => request.t === "api_call" && request.sessionId === child.context.Niva.bridge.sessionId));
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

test("IPC channel data is accepted only after the API ticket and channel attachment", async () => {
  const requests = [];
  const { context, requests: mockRequests } = bootPage({
    injectCommonJs: false,
    injectEsm: false,
    fakeTimers: true,
    webSocketThrows: 1,
    ipcReply(request) {
      requests.push(request);
      if (request.t === "api_call") return { t: "channelOpened", id: request.id, capability: "fixture-capability" };
      if (request.t === "channelAck") return { t: "channelAckReceived", id: request.id, seq: request.seq, accepted: true };
      if (request.t === "heartbeat") return { t: "heartbeatAck", sessionId: request.sessionId };
      throw new Error(`Unexpected IPC message ${request.t}`);
    },
  });
  const events = [];
  const stream = context.Niva.bridge.stream("fs.readStream", [], { onEvent: (name) => events.push(name) });
  const settled = stream.promise.then((value) => ({ value }), (error) => ({ error }));
  await new Promise((resolve) => setImmediate(resolve));
  const api = mockRequests.find((request) => request.t === "api_call");
  const attach = mockRequests.find((request) => request.t === "channelAttach");
  assert.ok(api && attach);
  assert.ok(mockRequests.indexOf(api) < mockRequests.indexOf(attach), "the channel ticket is created before the data lane attaches");
  assert.equal(context.__niva_native_frame({
    sessionId: context.Niva.bridge.sessionId,
    id: stream.id,
    seq: 1,
    frame: { t: "text", data: JSON.stringify({ t: "event", id: stream.id, seq: 1, name: "ready", data: {} }) },
  }), true);
  await new Promise((resolve) => setImmediate(resolve));
  assert.deepEqual(events, ["ready"]);
  assert.ok(requests.some((request) => request.t === "channelAck" && request.capability === "fixture-capability"));
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
    if (request.t === "api_call") return { t: "channelOpened", id: request.id, capability: "ipc-capability" };
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
    assert.equal(frame[0], 2);
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
      if (request.t === "api_call") return { t: "channelOpened", id: request.id, capability: "retry-capability" };
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
      if (request.t === "api_call") return { t: "channelOpened", id: request.id, capability: "reject-capability" };
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
