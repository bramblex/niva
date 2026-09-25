import "./runtime/bridge";
import "./runtime/callsite";
import "./runtime/vendor.js";
import "./runtime/path";
import "./runtime/events";
import "./runtime/querystring";
import "./runtime/buffer";
import "./runtime/util";
import "./runtime/os";
import "./runtime/fs";
import "./runtime/child_process";
import "./runtime/url";
import "./runtime/crypto";
import "./runtime/zlib";
import "./runtime/source";
import "./runtime/assert";
import "./runtime/stream";
import "./runtime/process";
import "./runtime/net";
import "./runtime/dgram";
import "./runtime/tls";
import "./runtime/http";
import "./runtime/dns";
import "./runtime/string_decoder";
import "./runtime/timers";
import "./runtime/tty";
import "./runtime/registration";

type Pending = {
  resolve?: (value: any) => void;
  reject?: (error: any) => void;
  timer?: ReturnType<typeof setTimeout>;
  connectTimer?: ReturnType<typeof setTimeout>;
  onEvent?: (name: string, data: any) => void;
  onChunk?: (chunk: Uint8Array, isStderr: boolean) => void;
  onBlob?: (blob: Blob, isStderr: boolean) => void;
  groups?: Map<boolean, Map<number, ArrayBuffer>>;
  seqOut?: number;
};
type TransportWaiter = {
  resolve: (ipcOnly: boolean) => void;
  reject: (error: Error) => void;
  timer: ReturnType<typeof setTimeout>;
};

(function initializeNivaPage(root: any) {
  "use strict";
  if (root.Niva && root.Niva.bridgeVersion === 1 && root.Niva.__runtimeBootstrap === true) return;

  inheritSameOriginRuntime(root);

  const runtime: any = root[Symbol.for("niva.node-compat.runtime")];
  if (!runtime || typeof runtime.registerNodeCompat !== "function") {
    throw new Error("Niva runtime bundle is incomplete");
  }

  const config = root.__niva_runtime_config || {};
  const local = !!(root.__niva_ws_url && root.__niva_token);
  const canIpc = typeof root.__niva_server_origin === "string" && root.__niva_server_origin.length > 0;
  const sessionId = makeSessionId(root);
  let nextId = 0;
  let socket: WebSocket | null = null;
  let reconnectTimer: ReturnType<typeof setTimeout> | null = null;
  let heartbeatTimer: ReturnType<typeof setTimeout> | null = null;
  let lastHeartbeatAck = 0;
  let sessionExpired = false;
  let socketFailed = false;
  let socketOpen = false;
  let ipcFallbackReady = !local && canIpc;
  const transportWaiters = new Set<TransportWaiter>();
  const pending: Record<string, Pending> = Object.create(null);
  const ipcPending: Record<string, Pending> = Object.create(null);
  let sendQueue: string[] = [];
  let binQueue: Array<{ id: string; frame: ArrayBuffer }> = [];
  const resourceOwners = new Map<number, { ref?: WeakRef<any>; fallback?: any; unregisterToken: object; finalizeNative?: () => unknown }>();
  let nextResourceOwner = 0;
  const resourceFinalizer = typeof FinalizationRegistry === "function"
    ? new FinalizationRegistry<number>((id) => {
      const record = resourceOwners.get(id);
      if (!record) return;
      resourceOwners.delete(id);
      try {
        const result = record.finalizeNative?.();
        if (result && typeof (result as any).then === "function") (result as Promise<unknown>).catch(() => {});
      } catch (_) { /* GC cleanup is best effort; never surface on the finalizer queue. */ }
    })
    : null;
  const eventListeners: Record<string, Function[]> = Object.create(null);
  const moduleRegistry: Record<string, any> = Object.create(null);

  function makeSessionId(host: any): string {
    const bytes = new Uint8Array(16);
    if (!host.crypto || typeof host.crypto.getRandomValues !== "function") {
      throw new Error("Secure random source is required for Niva session IDs");
    }
    host.crypto.getRandomValues(bytes);
    return Array.from(bytes, (value) => value.toString(16).padStart(2, "0")).join("");
  }

  function newId(): number {
    nextId = nextId >= Number.MAX_SAFE_INTEGER ? 1 : nextId + 1;
    return nextId;
  }

  function isIpcOnly(): boolean {
    return canIpc && (!local || ipcFallbackReady && !socketOpen);
  }

  function settleTransportWaiters(ipcOnly: boolean, error?: Error) {
    for (const waiter of transportWaiters) {
      clearTimeout(waiter.timer);
      if (error) waiter.reject(error);
      else waiter.resolve(ipcOnly);
    }
    transportWaiters.clear();
  }

  function waitForTransport(): Promise<boolean> {
    if (sessionExpired) return Promise.reject(nivaError("Niva page session is no longer active", "ERR_NIVA_SESSION_EXPIRED"));
    if (socketOpen) return Promise.resolve(false);
    if (!local || ipcFallbackReady || !canIpc) return Promise.resolve(isIpcOnly());
    return new Promise((resolve, reject) => {
      let waiter: TransportWaiter;
      const timer = setTimeout(() => {
        if (!transportWaiters.delete(waiter)) return;
        ipcFallbackReady = canIpc;
        resolve(isIpcOnly());
      }, 8000);
      waiter = { resolve, reject, timer };
      transportWaiters.add(waiter);
    });
  }

  function nivaError(message: string, code = "NIVA_BRIDGE_ERROR", extra?: Record<string, any>): Error {
    const error: any = new Error(message);
    error.code = code;
    if (extra) Object.assign(error, extra);
    return error;
  }

  function rejectAll(table: Record<string, Pending>, error: Error) {
    for (const id of Object.keys(table)) finish(table, id, false, error);
  }

  function finish(table: Record<string, Pending>, id: string | number, ok: boolean, value: any) {
    const key = String(id);
    const entry = table[key];
    if (!entry) return false;
    delete table[key];
    if (entry.timer) clearTimeout(entry.timer);
    if (entry.connectTimer) clearTimeout(entry.connectTimer);
    if (ok) entry.resolve?.(value);
    else entry.reject?.(runtime.nativeError ? runtime.nativeError(value) : value);
    if (table === ipcPending) updateHeartbeat();
    return true;
  }

  function expireSession(error = nivaError("Niva IPC session lease expired", "ERR_NIVA_SESSION_EXPIRED")) {
    sessionExpired = true;
    settleTransportWaiters(false, error);
    rejectAll(ipcPending, error);
    rejectAll(pending, error);
    sendQueue = [];
    binQueue = [];
    invalidateResources(error);
    if (heartbeatTimer) clearTimeout(heartbeatTimer);
    heartbeatTimer = null;
  }

  function updateHeartbeat() {
    const active = Object.keys(ipcPending).length > 0;
    if (!active) {
      if (heartbeatTimer) clearTimeout(heartbeatTimer);
      heartbeatTimer = null;
      return;
    }
    if (Object.keys(ipcPending).length === 1 && lastHeartbeatAck === 0) lastHeartbeatAck = Date.now();
    if (heartbeatTimer) return;
    heartbeatTimer = setTimeout(sendHeartbeat, 1000);
  }

  function postIpc(payload: any): Promise<any> | null {
    const text = JSON.stringify(payload);
    const handler = root.webkit?.messageHandlers?.nivaReply;
    if (handler && typeof handler.postMessage === "function") return Promise.resolve(handler.postMessage(text));
    const webview = root.chrome?.webview;
    if (root.ipc && typeof root.ipc.postMessage === "function" && webview?.addEventListener) {
      root.ipc.postMessage(text);
      return null;
    }
    return null;
  }

  function sendHeartbeat() {
    heartbeatTimer = null;
    if (Object.keys(ipcPending).length === 0) return;
    if (!canIpc) {
      expireSession(nivaError("Niva IPC transport is unavailable", "ERR_NIVA_IPC_UNAVAILABLE"));
      return;
    }
    if (Date.now() - lastHeartbeatAck >= 3000) {
      expireSession(nivaError("Niva IPC session heartbeat acknowledgement expired", "ERR_NIVA_SESSION_EXPIRED"));
      return;
    }
    const payload = { t: "heartbeat", sessionId, ...(local ? { token: root.__niva_token } : {}) };
    try {
      const reply = postIpc(payload);
      if (reply) reply.then((value) => handleIpcMessage(parseIpc(value)), (error) => {
        if (Object.keys(ipcPending).length) expireSession(nivaError(String(error), "ERR_NIVA_SESSION_EXPIRED"));
      });
    } catch (error) {
      expireSession(nivaError(String(error), "ERR_NIVA_SESSION_EXPIRED"));
      return;
    }
    heartbeatTimer = setTimeout(sendHeartbeat, 1000);
  }

  function parseIpc(raw: any): any {
    return typeof raw === "string" ? JSON.parse(raw) : raw;
  }

  function handleIpcMessage(message: any) {
    if (!message || typeof message !== "object") return;
    if (message.t === "heartbeatAck" && message.sessionId === sessionId) {
      lastHeartbeatAck = Date.now();
      updateHeartbeat();
      return;
    }
    if (message.t === "heartbeatError" && message.sessionId === sessionId) {
      expireSession(nivaError(message.message || "Niva IPC session expired", message.code || "ERR_NIVA_SESSION_EXPIRED"));
      return;
    }
    if (message.t === "result" && message.id != null) finish(ipcPending, String(message.id), message.code === 0, message.code === 0 ? message.data : message);
  }

  function handleWindowsIpc(event: MessageEvent) {
    try { handleIpcMessage(parseIpc(event.data)); } catch (_) { /* invalid messages do not settle unrelated requests */ }
  }

  let windowsListenerInstalled = false;
  function sendIpcCall(method: string, args: any[]): Promise<any> {
    if (sessionExpired) return Promise.reject(nivaError("Niva IPC session lease expired", "ERR_NIVA_SESSION_EXPIRED"));
    if (!canIpc) return Promise.reject(nivaError("Niva IPC transport is unavailable", "ERR_NIVA_IPC_UNAVAILABLE"));
    const id = newId();
    const request = JSON.stringify({ t: "call", id, method, args, sessionId, ...(local ? { token: root.__niva_token } : {}) });
    if (new TextEncoder().encode(request).byteLength > 256 * 1024) {
      return Promise.reject(nivaError("Niva IPC request exceeds 256 KiB", "ERR_NIVA_IPC_TOO_LARGE"));
    }
    return new Promise((resolve, reject) => {
      const timer = setTimeout(() => finish(ipcPending, id, false, nivaError("Niva IPC reply timed out", "ETIMEDOUT")), 60000);
      ipcPending[id] = { resolve, reject, timer };
      updateHeartbeat();
      try {
        const handler = root.webkit?.messageHandlers?.nivaReply;
        if (handler && typeof handler.postMessage === "function") {
          Promise.resolve(handler.postMessage(request)).then((raw) => {
            try { handleIpcMessage(parseIpc(raw)); }
            catch (_) { finish(ipcPending, id, false, nivaError("Invalid Niva IPC response", "ERR_NIVA_IPC_RESPONSE")); }
          }, (error) => finish(ipcPending, id, false, error));
          return;
        }
        const webview = root.chrome?.webview;
        if (root.ipc && typeof root.ipc.postMessage === "function" && webview?.addEventListener) {
          if (!windowsListenerInstalled) {
            webview.addEventListener("message", handleWindowsIpc);
            windowsListenerInstalled = true;
          }
          root.ipc.postMessage(request);
          return;
        }
        finish(ipcPending, id, false, nivaError("Niva IPC is unavailable", "ERR_NIVA_IPC_UNAVAILABLE"));
      } catch (error) {
        finish(ipcPending, id, false, error);
      }
    });
  }

  function flushTextQueue() {
    if (!socket || socket.readyState !== WebSocket.OPEN) return;
    while (sendQueue.length) {
      const text = sendQueue.shift()!;
      try {
        const frame = JSON.parse(text);
        if (frame.t === "call" && frame.id != null) {
          const entry = pending[String(frame.id)];
          if (entry) {
            if (entry.connectTimer) clearTimeout(entry.connectTimer);
            entry.connectTimer = undefined;
            entry.timer = setTimeout(() => finish(pending, String(frame.id), false, nivaError("Niva WebSocket call timed out", "ETIMEDOUT")), 60000);
          }
        }
      } catch (_) { /* malformed internal frame is sent for Native protocol diagnostics */ }
      socket.send(text);
    }
  }

  function flushBinaryQueue() {
    if (!socket || socket.readyState !== WebSocket.OPEN) return;
    while (binQueue.length) {
      const item = binQueue.shift()!;
      if (pending[item.id]) socket.send(item.frame);
    }
  }

  function handleTextMessage(message: any) {
    if (message?.t === "result" && message.id != null) {
      finish(pending, String(message.id), message.code === 0, message.code === 0 ? message.data : message);
    } else if (message?.t === "event") {
      const entry = message.id == null ? undefined : pending[String(message.id)];
      if (entry?.onEvent) entry.onEvent(message.name, message.data);
      else emit(message.name, message.data);
    }
  }

  function handleBinaryMessage(buffer: ArrayBuffer) {
    if (buffer.byteLength < 18) return;
    const view = new DataView(buffer);
    if (view.getUint8(0) !== 1) return;
    const id = String(view.getUint32(2) * 0x100000000 + view.getUint32(6));
    const seq = view.getUint32(10) * 0x100000000 + view.getUint32(14);
    const entry = pending[id];
    if (!entry) return;
    const flags = view.getUint8(1);
    const stderr = !!(flags & 4);
    const payload = buffer.slice(18);
    if (entry.onChunk && payload.byteLength) entry.onChunk(new Uint8Array(payload), stderr);
    if (entry.onBlob) {
      const groups = entry.groups || (entry.groups = new Map());
      let group: Map<number, ArrayBuffer> | undefined = groups.get(stderr);
      if (!group) { group = new Map<number, ArrayBuffer>(); groups.set(stderr, group); }
      group.set(seq, payload);
      if (flags & 2) {
        entry.onBlob(new Blob(Array.from(group.keys()).sort((a, b) => a - b).map((key) => group!.get(key)!)), stderr);
        groups.delete(stderr);
      }
    }
  }

  function loseSocket(message = "Niva WebSocket connection closed") {
    socket = null;
    socketOpen = false;
    socketFailed = true;
    ipcFallbackReady = canIpc;
    settleTransportWaiters(isIpcOnly());
    rejectAll(pending, nivaError(message, "ECONNRESET"));
    sendQueue = [];
    binQueue = [];
    invalidateResources(nivaError(message, "ECONNRESET"));
    if (local && !sessionExpired && !reconnectTimer) reconnectTimer = setTimeout(() => { reconnectTimer = null; connectSocket(); }, 1000);
  }

  function connectSocket() {
    if (!local || typeof WebSocket === "undefined") return;
    try {
      socket = new WebSocket(root.__niva_ws_url + "?token=" + encodeURIComponent(root.__niva_token));
    } catch (error) {
      socket = null;
      socketFailed = true;
      ipcFallbackReady = canIpc;
      settleTransportWaiters(isIpcOnly());
      if (!reconnectTimer) reconnectTimer = setTimeout(() => { reconnectTimer = null; connectSocket(); }, 1000);
      return;
    }
    socket.binaryType = "arraybuffer";
    socket.addEventListener("open", () => {
      socketFailed = false;
      socketOpen = true;
      ipcFallbackReady = false;
      settleTransportWaiters(false);
      socket?.send(JSON.stringify({ t: "hello", wid: root.__niva_window_id || 0, v: 1, sessionId }));
      flushTextQueue();
      flushBinaryQueue();
    });
    socket.addEventListener("message", (event) => {
      try {
        if (typeof event.data === "string") handleTextMessage(JSON.parse(event.data));
        else if (event.data instanceof ArrayBuffer) handleBinaryMessage(event.data);
      } catch (error) { console.error("Invalid Niva bridge frame", error); }
    });
    socket.addEventListener("close", () => loseSocket());
    socket.addEventListener("error", () => {
      if (socket?.readyState !== WebSocket.OPEN) loseSocket("Niva WebSocket connection failed");
    });
  }

  function call(method: string, args: any[] = []): Promise<any> {
    if (sessionExpired) return Promise.reject(nivaError("Niva page session is no longer active", "ERR_NIVA_SESSION_EXPIRED"));
    return waitForTransport().then((ipcOnly) => {
      if (ipcOnly) {
        if (method === "process.write") throw nivaError("stdio writes require the local WebSocket transport", "ERR_NIVA_IPC_METHOD_UNSUPPORTED");
        return sendIpcCall(method, args);
      }
      if (!local) throw nivaError("Niva bridge is unavailable", "ERR_NIVA_BRIDGE_UNAVAILABLE");
      return streamCall(method, args).promise;
    });
  }

  function streamCall(method: string, args: any[] = [], handlers: any = {}) {
    if (sessionExpired) throw nivaError("Niva page session is no longer active", "ERR_NIVA_SESSION_EXPIRED");
    if (!local) throw nivaError("This API requires a trusted local WebSocket page", "ERR_NIVA_IPC_STREAM_UNSUPPORTED");
    const id = newId();
    const key = String(id);
    let resolve!: (value: any) => void;
    let reject!: (error: any) => void;
    const promise = new Promise((res, rej) => { resolve = res; reject = rej; });
    const entry: Pending = { resolve, reject, onEvent: handlers.onEvent, onChunk: handlers.onChunk, onBlob: handlers.onBlob, groups: new Map(), seqOut: 0 };
    pending[key] = entry;
    const text = JSON.stringify({ t: "call", id, method, args, sessionId });
    if (sendQueue.length >= 256) finish(pending, id, false, nivaError("Niva WebSocket send queue is full", "ENOBUFS"));
    else {
      if (socketOpen && socket?.readyState === WebSocket.OPEN) {
        entry.timer = setTimeout(() => finish(pending, id, false, nivaError("Niva WebSocket call timed out", "ETIMEDOUT")), 60000);
        socket.send(text);
      } else {
        sendQueue.push(text);
        entry.connectTimer = setTimeout(() => {
          sendQueue = sendQueue.filter((item) => item !== text);
          binQueue = binQueue.filter((item) => item.id !== key);
          finish(pending, id, false, nivaError("Niva WebSocket did not connect before the call deadline", "ERR_NIVA_WS_UNAVAILABLE"));
        }, 8000);
      }
    }
    return {
      id,
      promise,
      cancel() {
        return cancelStream(id, nivaError("Niva call was cancelled", "ABORT_ERR"));
      },
    };
  }

  function cancelStream(idValue: number | string, error = nivaError("Niva stream resource was disposed", "ERR_NIVA_RESOURCE_CLOSED")) {
    const id = String(idValue);
    if (!pending[id]) return false;
    sendQueue = sendQueue.filter((item) => { try { return String(JSON.parse(item).id) !== id; } catch (_) { return true; } });
    binQueue = binQueue.filter((item) => item.id !== id);
    if (socket?.readyState === WebSocket.OPEN) {
      try { socket.send(JSON.stringify({ t: "cancel", id: Number(id), sessionId })); } catch (_) { /* finish locally */ }
    }
    return finish(pending, id, false, error);
  }

  function streamSend(idValue: number | string, data: ArrayBuffer | Uint8Array | string, end = false): boolean {
    if (sessionExpired) return false;
    if (!local) throw nivaError("Binary streams are unavailable over Niva IPC", "ERR_NIVA_IPC_STREAM_UNSUPPORTED");
    const id = String(idValue);
    const entry = pending[id];
    if (!entry) return false;
    const bytes = data instanceof ArrayBuffer ? new Uint8Array(data) : data instanceof Uint8Array ? data : new TextEncoder().encode(String(data));
    entry.seqOut = (entry.seqOut || 0) + 1;
    const frame = new Uint8Array(18 + bytes.length);
    frame[0] = 1;
    frame[1] = (entry.seqOut === 1 ? 1 : 0) | (end ? 2 : 0);
    const view = new DataView(frame.buffer);
    const asNumber = Number(id);
    view.setUint32(2, Math.floor(asNumber / 0x100000000));
    view.setUint32(6, asNumber >>> 0);
    view.setUint32(10, Math.floor(entry.seqOut / 0x100000000));
    view.setUint32(14, entry.seqOut >>> 0);
    frame.set(bytes, 18);
    if (socket?.readyState === WebSocket.OPEN) { socket.send(frame.buffer); return true; }
    if (binQueue.length >= 256) return false;
    binQueue.push({ id, frame: frame.buffer });
    return true;
  }

  function callSync(method: string, args: any[] = []): any {
    if (sessionExpired) throw nivaError("Niva page session is no longer active", "ERR_NIVA_SESSION_EXPIRED");
    if (!local) throw nivaError("Synchronous Niva bridge calls require a trusted local page", "ERR_NIVA_SYNC_UNAVAILABLE");
    const xhr = new XMLHttpRequest();
    xhr.open("POST", root.__niva_server_origin + "/__niva_sync", false);
    xhr.setRequestHeader("Content-Type", "text/plain;charset=UTF-8");
    xhr.send(JSON.stringify({ token: root.__niva_token, sessionId, request: [0, method, args] }));
    if (xhr.status !== 200) throw nivaError("Synchronous Niva bridge HTTP " + xhr.status, "ERR_NIVA_SYNC_HTTP");
    const response = JSON.parse(xhr.responseText);
    if (response[1] !== 0) throw nivaError(response[2] || "Niva call failed", "NIVA_BRIDGE_ERROR", { bridgeCode: response[1], data: response[3] });
    return response[3];
  }

  function emit(name: string, data: any) {
    setTimeout(() => {
      for (const key of [name, name.split(".")[0] + ".*", "*"]) {
        for (const listener of (eventListeners[key] || []).slice()) listener(name, data);
      }
    }, 0);
  }

  function addEventListener(name: string, listener: Function) {
    if (typeof listener !== "function") throw new TypeError("listener must be a function");
    (eventListeners[name] || (eventListeners[name] = [])).push(listener);
  }

  function removeEventListener(name: string, listener: Function) {
    const list = eventListeners[name];
    if (list) eventListeners[name] = list.filter((candidate) => candidate !== listener);
  }

  function nativeNamespace(name: string): any {
    return new Proxy(Object.create(null), {
      get(_target, method) {
        if (method === "then") return undefined;
        if (method === Symbol.toStringTag) return "NivaNativeNamespace";
        if (typeof method !== "string") return undefined;
        if (name === "resource" && method === "read") return resourceRead;
        return (...args: any[]) => call(name + "." + method, args);
      },
    });
  }

  function resourceRead(path: string, encoding: "utf8" | "base64" = "utf8"): Promise<string> {
    if (typeof path !== "string") return Promise.reject(nivaError("resource.read path must be a string", "ERR_INVALID_ARG_TYPE"));
    if (encoding !== "utf8" && encoding !== "base64") return Promise.reject(nivaError("resource.read supports only utf8 and base64 encodings", "ERR_UNKNOWN_ENCODING"));
    const chunks: Blob[] = [];
    let stream: ReturnType<typeof streamCall>;
    try { stream = streamCall("resource.readStream", [path], { onBlob: (blob: Blob) => chunks.push(blob) }); }
    catch (error) { return Promise.reject(error); }
    return stream.promise.then(() => new Blob(chunks)).then((blob) => {
      if (encoding === "utf8") return blob.text();
      return blob.arrayBuffer().then((buffer) => {
        const bytes = new Uint8Array(buffer);
        let binary = "";
        for (let offset = 0; offset < bytes.length; offset += 32768) {
          binary += String.fromCharCode.apply(null, Array.from(bytes.subarray(offset, offset + 32768)));
        }
        return root.btoa(binary);
      });
    });
  }

  const Niva: any = {
    bridgeVersion: 1,
    __runtimeBootstrap: true,
    bootstrap: root.__niva_node_bootstrap || {},
    bridge: {
      call,
      callSync,
      stream: streamCall,
      streamSend,
      isIpcOnly,
      waitForTransport,
      sessionId,
    },
    __bridge: { isIpcOnly },
    addEventListener,
    removeEventListener,
    removeAllEventListeners(name: string) { if (name === undefined) Object.keys(eventListeners).forEach((key) => delete eventListeners[key]); else delete eventListeners[name]; },
    __emit__: emit,
    __registerModule(name: string, implementation: any) {
      Object.defineProperty(moduleRegistry, name, { value: implementation, writable: true, configurable: true, enumerable: true });
    },
    __registerModuleFactory(name: string, factory: () => any) {
      let initialized = false;
      let value: any;
      Object.defineProperty(moduleRegistry, name, { configurable: true, enumerable: true, get() {
        if (!initialized) {
          value = factory();
          initialized = true;
          Object.defineProperty(moduleRegistry, name, { value, writable: true, configurable: true, enumerable: true });
        }
        return value;
      } });
    },
    __getModule(name: string) {
      return Object.prototype.hasOwnProperty.call(moduleRegistry, name)
        ? { found: true, value: moduleRegistry[name] }
        : { found: false, value: undefined };
    },
    dialog: nativeNamespace("dialog"),
    window: nativeNamespace("window"),
    clipboard: nativeNamespace("clipboard"),
    tray: nativeNamespace("tray"),
    shortcut: nativeNamespace("shortcut"),
    monitor: nativeNamespace("monitor"),
    webview: nativeNamespace("webview"),
    extra: nativeNamespace("extra"),
    windowExtra: nativeNamespace("windowExtra"),
    resource: nativeNamespace("resource"),
  };
  root.Niva = Niva;

  const modules = runtime.registerNodeCompat(Niva);
  // The compatibility modules are also the page's public Node-shaped API.
  for (const name of Object.keys(modules)) {
    // registerNodeCompat installs zlib as a lazy property and lazy require
    // factory; reading it here would defeat Node's first-require semantics.
    if (name === "zlib") continue;
    if (modules[name] !== undefined) Niva[name] = modules[name];
  }
  Niva.module = createModuleBuiltin(Niva, modules);
  modules.module = Niva.module;
  modules["node:module"] = Niva.module;
  Niva.__registerModule("module", Niva.module);
  Niva.__registerModule("node:module", Niva.module);

  const injectCommonJs = config.injectCommonJs === true;
  const injectEsm = config.injectEsm === true;
  const mainProcessAvailable = local && !!Niva.bootstrap.process;
  if (injectCommonJs) {
    runtime.installCallSiteCompat?.();
    const globalFilename = root.location?.pathname || "";
    const globalModule: any = new Niva.module(globalFilename || ".", null);
    globalModule.filename = globalFilename;
    globalModule.path = globalFilename ? Niva.path.dirname(globalFilename) : "/";
    globalModule.paths = modulePaths(globalModule.path);
    globalModule.loaded = true;
    const require = makeRequire(Niva, modules, "", globalModule);
    globalModule.require = require;
    root.require = require;
    root.module = globalModule;
    root.global = root;
    root.exports = globalModule.exports;
    root.setTimeout = modules.timers.setTimeout;
    root.clearTimeout = modules.timers.clearTimeout;
    root.setInterval = modules.timers.setInterval;
    root.clearInterval = modules.timers.clearInterval;
    root.setImmediate = modules.timers.setImmediate;
    root.clearImmediate = modules.timers.clearImmediate;
    if (typeof root.Buffer === "undefined") root.Buffer = modules.buffer.Buffer;
    if (mainProcessAvailable) root.process = modules.process;
  }
  Niva.runtimeConfig = Object.freeze({ injectCommonJs, injectEsm, get ipcOnly() { return isIpcOnly(); }, nonce: config.nonce });
  // Native HTML owns import maps for trusted local pages. External pages must
  // use their own bundler: this runtime has no cross-origin asset URL/CORS
  // contract and must not install a relative map that silently points elsewhere.

  function registerResource<T extends object>(resource: T, finalizeNative?: () => unknown) {
    const id = ++nextResourceOwner;
    const unregisterToken = {};
    const ref = typeof WeakRef === "function" ? new WeakRef(resource) : undefined;
    resourceOwners.set(id, { ref, fallback: ref ? undefined : resource, unregisterToken, finalizeNative });
    if (ref) resourceFinalizer?.register(resource, id, unregisterToken);
    const owner = Object.freeze({
      id,
      release() {
        const record = resourceOwners.get(id);
        if (!record) return false;
        resourceOwners.delete(id);
        resourceFinalizer?.unregister(record.unregisterToken);
        return true;
      },
    });
    Object.defineProperty(resource, "__nivaResourceOwner", { value: owner, configurable: true });
    return owner;
  }

  function invalidateResources(error: Error) {
    const records = Array.from(resourceOwners.values());
    resourceOwners.clear();
    for (const record of records) {
      resourceFinalizer?.unregister(record.unregisterToken);
      const resource = record.ref?.deref() || record.fallback;
      if (!resource) continue;
      try { resource.__nivaInvalidate?.(error); } catch (_) { /* continue invalidating sibling resources */ }
    }
  }
  runtime.registerResource = registerResource;
  runtime.invalidateResources = invalidateResources;
  runtime.cancelStream = cancelStream;
  Niva.__runtime = Object.freeze({ registerResource, isMainProcess: mainProcessAvailable });

  root.addEventListener?.("pagehide", () => expireSession(nivaError("Niva page session ended", "ERR_NIVA_SESSION_EXPIRED")), { once: true });
  if (local) connectSocket();

  function inheritSameOriginRuntime(host: any) {
    try {
      const parent = host.parent;
      if (!parent || parent === host || host.__niva_ws_url || host.__niva_token) return;
      const childOrigin = host.location?.origin;
      const parentOrigin = parent.location?.origin;
      if (typeof childOrigin !== "string" || childOrigin !== parentOrigin) return;
      const wsUrl = parent.__niva_ws_url;
      const token = parent.__niva_token;
      if (typeof wsUrl !== "string" || !wsUrl || typeof token !== "string" || !token) return;

      host.__niva_ws_url = wsUrl;
      host.__niva_token = token;
      if (host.__niva_window_id === undefined) host.__niva_window_id = parent.__niva_window_id;
      if (host.__niva_server_origin === undefined) host.__niva_server_origin = parent.__niva_server_origin;

      const parentConfig = parent.__niva_runtime_config || {};
      const childConfig = { ...(host.__niva_runtime_config || {}) };
      for (const key of ["injectCommonJs", "injectEsm", "nonce"]) {
        if (childConfig[key] === undefined && parentConfig[key] !== undefined) childConfig[key] = parentConfig[key];
      }
      host.__niva_runtime_config = childConfig;

      if (host.__niva_node_bootstrap === undefined) {
        const parentBootstrap = parent.__niva_node_bootstrap;
        host.__niva_node_bootstrap = parentBootstrap && parentBootstrap.os !== undefined
          ? { os: parentBootstrap.os }
          : {};
      }
    } catch (_) {
      // A cross-origin WindowProxy intentionally exposes no credentials here.
    }
  }

  function createModuleBuiltin(niva: any, available: any) {
    const builtinModules = builtinModulesFor(available);
    const builtinNames = builtinModules.flatMap((name) => [name, "node:" + name]);
    const Module: any = function NivaModule(id: string, parent?: any) {
      this.id = id;
      this.filename = id;
      this.loaded = false;
      this.parent = parent || null;
      this.children = [];
      this.exports = {};
      this.paths = modulePaths(niva.path.dirname(id));
    };
    Module.builtinModules = builtinModules;
    Module.Module = Module;
    Module.isBuiltin = (specifier: string) => builtinNames.includes(specifier);
    Module.createRequire = (filename: string | URL) => makeRequire(niva, available, filename instanceof URL ? niva.url.fileURLToPath(filename) : String(filename));
    Module._cache = Object.create(null);
    Module._nodeModulePaths = modulePaths;
    Module.prototype.require = function (specifier: string) { return makeRequire(niva, available, this.filename || "", this)(specifier); };
    return Module;
  }

  function builtinModulesFor(available: any): string[] {
    return Object.keys(available).concat(["assert/strict", "dns/promises", "fs/promises", "module", "stream/promises", "timers/promises"]);
  }

  function builtinNamesFor(available: any): string[] {
    return builtinModulesFor(available).flatMap((name) => [name, "node:" + name]);
  }

  function modulePaths(directory: string): string[] {
    const path = Niva.path;
    const start = directory || (root.location?.pathname ? path.dirname(root.location.pathname) : "/");
    const result: string[] = [];
    let current = path.resolve(start || "/");
    while (true) {
      if (path.basename(current) !== "node_modules") result.push(path.join(current, "node_modules"));
      const parent = path.dirname(current);
      if (parent === current) break;
      current = parent;
    }
    return result;
  }

  function makeRequire(niva: any, available: any, parentFilename = "", parentModule?: any) {
    const cache: Record<string, any> = Niva.module?._cache || Object.create(null);
    const require: any = function nivaRequire(specifier: string) {
      if (typeof specifier !== "string") throw new TypeError("require specifier must be a string");
      const registered = typeof niva.__getModule === "function" ? niva.__getModule(specifier) : { found: false, value: undefined };
      if (registered.found) return registered.value;
      if (Object.prototype.hasOwnProperty.call(available, specifier)) return available[specifier];
      if (specifier.startsWith("node:") && Object.prototype.hasOwnProperty.call(available, specifier.slice(5))) return available[specifier.slice(5)];
      const parent = parentFilename;
      const resolvedFilename = niva.bridge.callSync("module.resolve", [specifier, parent]);
      const resolvedBuiltin = typeof niva.__getModule === "function" ? niva.__getModule(resolvedFilename) : { found: false, value: undefined };
      if (resolvedBuiltin.found) return resolvedBuiltin.value;
      if (typeof resolvedFilename === "string" && resolvedFilename.startsWith("node:")) {
        throw nivaError("Unsupported Node builtin module: " + resolvedFilename, "ERR_UNKNOWN_BUILTIN_MODULE");
      }
      if (typeof resolvedFilename !== "string") throw nivaError("Native module resolver returned an invalid result", "ERR_NIVA_MODULE_RESOLVE_RESPONSE");
      if (resolvedFilename.endsWith(".node")) throw nivaError("Native Node addons are not supported: " + resolvedFilename, "ERR_DLOPEN_FAILED");
      if (resolvedFilename.endsWith(".mjs") || resolvedFilename.endsWith(".mts")) throw nivaError("require() of ES modules is not supported: " + resolvedFilename, "ERR_REQUIRE_ESM");
      if (cache[resolvedFilename]) return cache[resolvedFilename].exports;
      const loaded = niva.bridge.callSync("module.load", [specifier, parent]);
      const filename = loaded.filename || resolvedFilename;
      if (filename !== resolvedFilename) throw nivaError("Module resolver returned inconsistent filenames", "ERR_NIVA_MODULE_RESOLVE_MISMATCH");
      if (cache[filename]) return cache[filename].exports;
      if (loaded.type === "json") {
        const jsonModule: any = new niva.module(filename, parentModule || Niva.module.main || null);
        jsonModule.filename = filename;
        jsonModule.path = loaded.dirname;
        jsonModule.paths = modulePaths(loaded.dirname);
        jsonModule.loaded = false;
        jsonModule.require = makeRequire(niva, available, filename, jsonModule);
        cache[filename] = jsonModule;
        if (jsonModule.parent && !jsonModule.parent.children.includes(jsonModule)) jsonModule.parent.children.push(jsonModule);
        try { jsonModule.exports = JSON.parse(loaded.source); jsonModule.loaded = true; return jsonModule.exports; }
        catch (error) { delete cache[filename]; throw error; }
      }
      if (loaded.type !== "commonjs") throw nivaError("Unsupported module type: " + loaded.type, "ERR_UNKNOWN_MODULE_FORMAT");
      validateCommonJs(loaded.source, filename);
      const module: any = new niva.module(filename, parentModule || Niva.module.main || null);
      module.filename = filename;
      module.path = loaded.dirname;
      module.paths = modulePaths(loaded.dirname);
      module.loaded = false;
      module.require = makeRequire(niva, available, filename, module);
      cache[filename] = module;
      if (module.parent && !module.parent.children.includes(module)) module.parent.children.push(module);
      if (!Niva.module.main) Niva.module.main = module;
      const nonce = niva.runtimeConfig?.nonce || root.__niva_runtime_config?.nonce;
      if (!nonce) { delete cache[filename]; throw nivaError("CommonJS module execution requires the document CSP nonce", "ERR_NIVA_CSP_NONCE_REQUIRED"); }
      const execution: any = { id: filename, error: null, ran: false };
      (root as any).__nivaCjsExecution = execution;
      const tag = document.createElement("script");
      tag.nonce = nonce;
      const sourceUrl = encodeURI(filename).replace(/\r/g, "%0D").replace(/\n/g, "%0A");
      tag.textContent = `(()=>{const c=globalThis.__nivaCjsExecution;try{c.ran=true;(function(exports,require,module,__filename,__dirname){\n${loaded.source}\n}).call(c.module.exports,c.module.exports,c.module.require,c.module,c.module.filename,c.module.path)}catch(e){c.error=e}})();\n//# sourceURL=${sourceUrl}`;
      execution.module = module;
      (document.head || document.documentElement).appendChild(tag);
      tag.remove();
      delete (root as any).__nivaCjsExecution;
      if (!execution.ran) { delete cache[filename]; throw nivaError("CSP blocked the inline CommonJS module factory", "ERR_NIVA_CSP_BLOCKED"); }
      if (execution.error) { delete cache[filename]; throw execution.error; }
      module.loaded = true;
      return module.exports;
    };
    require.resolve = (specifier: string) => {
      const registered = typeof niva.__getModule === "function" ? niva.__getModule(specifier) : { found: false, value: undefined };
      if (registered.found) return specifier;
      if (Object.prototype.hasOwnProperty.call(available, specifier)) return specifier;
      const resolved = niva.bridge.callSync("module.resolve", [specifier, parentFilename]);
      if (typeof resolved !== "string") throw nivaError("Native module resolver returned an invalid result", "ERR_NIVA_MODULE_RESOLVE_RESPONSE");
      const resolvedBuiltin = typeof niva.__getModule === "function" ? niva.__getModule(resolved) : { found: false, value: undefined };
      if (resolvedBuiltin.found) return resolved;
      if (resolved.startsWith("node:")) throw nivaError("Unsupported Node builtin module: " + resolved, "ERR_UNKNOWN_BUILTIN_MODULE");
      return resolved;
    };
    require.cache = cache;
    Object.defineProperty(require, "main", { configurable: true, enumerable: true, get() { return niva.module.main || null; } });
    require.resolve.paths = (specifier: string) => builtinNamesFor(available).includes(specifier) ? null : modulePaths(parentFilename ? niva.path.dirname(parentFilename) : Niva.path.dirname(root.location?.pathname || "/"));
    return require;
  }

  function validateCommonJs(source: string, filename: string) {
    const parser = runtime.vendor?.acorn;
    if (!parser || typeof parser.parse !== "function") {
      throw nivaError("The CommonJS syntax validator is unavailable", "ERR_NIVA_CJS_VALIDATOR_UNAVAILABLE");
    }
    try {
      parser.parse(source, { ecmaVersion: "latest", sourceType: "script", allowReturnOutsideFunction: true });
    } catch (error: any) {
      if (error && typeof error === "object") {
        error.filename = filename;
        if (typeof error.message === "string" && !error.message.includes(filename)) error.message += ` (${filename})`;
      }
      throw error;
    }
  }

})(globalThis as any);
