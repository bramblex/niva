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

type StreamRoute = { transport: "ipc" } | { transport: "ws"; socket: WebSocket };

type Pending = {
  id: number;
  sessionId: string;
  epoch: number;
  transport: "ws" | "ipc";
  route: StreamRoute;
  resolve?: (value: any) => void;
  reject?: (error: any) => void;
  timer?: ReturnType<typeof setTimeout>;
  onEvent?: (name: string, data: any) => void;
  onChunk?: (chunk: Uint8Array, isStderr: boolean) => void;
  onBlob?: (blob: Blob, isStderr: boolean) => void;
  groups?: Map<boolean, Map<number, ArrayBuffer>>;
  seqOut?: number;
  seqIn?: number;
  inboundFrames?: Array<{ message: any; size: number }>;
  inboundFrameBytes?: number;
  inboundAcking?: boolean;
  active?: boolean;
  opened?: boolean;
  ticketReady?: boolean;
  attachState?: "none" | "ws-pending" | "ipc-pending" | "attached";
  dataStarted?: boolean;
  cancelRequested?: boolean;
  cancelSent?: boolean;
  capability?: string;
  outbound?: Array<{ seq: number; frame: ArrayBuffer; end: boolean }>;
  outboundBytes?: number;
  outboundSending?: boolean;
  outboundRetryStarted?: number;
  outboundRetryAttempt?: number;
  outboundRetryTimer?: ReturnType<typeof setTimeout>;
};
type IpcRequest = {
  resolve: (value: any) => void;
  reject: (error: Error) => void;
  timer: ReturnType<typeof setTimeout>;
  sessionId: string;
  epoch: number;
  sourceOrigin: string;
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
  const local = typeof root.__niva_token === "string" && root.__niva_token.length > 0
    && root.__niva_window_id !== undefined && root.__niva_window_id !== null;
  const sourceOrigin = pageSourceOrigin(root);
  const canIpc = !!sourceOrigin && isSupportedFrame(root) && !!ipcHost(root);
  let sessionId = makeSessionId(root);
  let sessionEpoch = 0;
  let nextId = 0;
  let nextRid = 0;
  let socket: WebSocket | null = null;
  let heartbeatTimer: ReturnType<typeof setTimeout> | null = null;
  let heartbeatPending = false;
  let lastHeartbeatAck = 0;
  let sessionExpired = false;
  let bfcacheSuspended = false;
  let socketOpen = false;
  const pending: Record<string, Pending> = Object.create(null);
  const ipcRequests = new Map<string, IpcRequest>();
  const ipcActiveCalls = new Set<number>();
  let queuedFrames = 0;
  let queuedBytes = 0;
  const streamTransports = new Map<number, "ws" | "ipc">();
  const streamCalls = new WeakSet<object>();
  const streamCallIds = new WeakMap<object, number>();
  const syncWarnings = new Set<string>();
  const resourceOwners = new Map<number, {
    ref?: WeakRef<any>;
    fallback?: any;
    unregisterToken: object;
    finalizeNative?: () => unknown;
    streamId?: number | (() => number | undefined);
    transport?: "ws" | "ipc";
  }>();
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

  const frameRoutes = new Map<string, any>();

  function makeSessionId(host: any): string {
    const bytes = new Uint8Array(16);
    if (!host.crypto || typeof host.crypto.getRandomValues !== "function") {
      throw new Error("Secure random source is required for Niva session IDs");
    }
    host.crypto.getRandomValues(bytes);
    return Array.from(bytes, (value) => value.toString(16).padStart(2, "0")).join("");
  }

  function pageSourceOrigin(host: any, inheritedOrigin?: string): string | null {
    try {
      const location = host.location;
      const actualOrigin = location?.origin;
      if (typeof actualOrigin === "string" && actualOrigin.length > 0 && actualOrigin !== "null") return actualOrigin;

      const candidate = typeof host.__niva_source_origin === "string" ? host.__niva_source_origin : inheritedOrigin;
      if (typeof candidate !== "string" || !candidate) return null;
      const URLConstructor = host.URL || URL;
      const pageUrl = new URLConstructor(location?.href);
      const originUrl = new URLConstructor(candidate);
      const customOrigin = originUrl.protocol !== "http:" && originUrl.protocol !== "https:";
      if (!customOrigin || originUrl.username || originUrl.password || originUrl.port
        || originUrl.hostname !== "app" || pageUrl.protocol !== originUrl.protocol
        || pageUrl.hostname !== originUrl.hostname || pageUrl.username || pageUrl.password || pageUrl.port) return null;
      return originUrl.protocol + "//" + originUrl.host;
    } catch (_) {
      return null;
    }
  }

  function sameOriginTop(host: any): any | null {
    try {
      const origin = pageSourceOrigin(host);
      if (!origin) return null;
      const expectedTop = host.top;
      const visited = new Set<any>();
      let current = host;
      while (current && !visited.has(current)) {
        visited.add(current);
        if (pageSourceOrigin(current, origin) !== origin) return null;
        const parent = current.parent;
        if (!parent || parent === current) return current === expectedTop ? current : null;
        if (pageSourceOrigin(parent, origin) !== origin) return null;
        current = parent;
      }
    } catch (_) {
      // A cross-origin ancestor makes this frame unsupported.
    }
    return null;
  }

  function isSupportedFrame(host: any): boolean {
    return sameOriginTop(host) !== null;
  }

  function ipcHost(host: any): any | null {
    const top = sameOriginTop(host);
    if (!top) return null;
    try {
      return top.ipc && typeof top.ipc.postMessage === "function" ? top : null;
    } catch (_) { /* Cross-origin parent access is deliberately unavailable. */ }
    return null;
  }

  function newId(): number {
    nextId = nextId >= Number.MAX_SAFE_INTEGER ? 1 : nextId + 1;
    return nextId;
  }

  function isTrustedLocal(): boolean {
    return local;
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
    streamTransports.delete(Number(key));
    entry.active = false;
    if (entry.timer) clearTimeout(entry.timer);
    if (entry.outboundRetryTimer) clearTimeout(entry.outboundRetryTimer);
    clearOutbound(entry);
    if (entry.inboundFrames) entry.inboundFrames.length = 0;
    entry.inboundFrameBytes = 0;
    endIpcCall(Number(key));
    if (ok) entry.resolve?.(value);
    else entry.reject?.(runtime.nativeError ? runtime.nativeError(value) : value);
    return true;
  }

  function postSessionClose(retiringSessionId: string) {
    if (!canIpc || !retiringSessionId) return false;
    try {
      return postIpc(JSON.stringify({
        t: "session_close",
        sessionId: retiringSessionId,
        ...(local ? { token: root.__niva_token } : {}),
      }));
    } catch (_) {
      return false;
    }
  }

  function closeSocketForSession() {
    const owner = socket;
    socket = null;
    socketOpen = false;
    if (!owner) return;
    try {
      if (owner.readyState < 2) owner.close();
    } catch (_) { /* the old session is already detached locally */ }
  }

  function expireSession(
    error = nivaError("Niva IPC session lease expired", "ERR_NIVA_SESSION_EXPIRED"),
    options: { retireNative?: boolean; preserveProcessStdio?: boolean } = {},
  ) {
    if (sessionExpired) return false;
    for (const entry of Object.values(pending)) sendNativeCancel(entry);
    if (options.retireNative) postSessionClose(sessionId);
    sessionExpired = true;
    rejectAll(pending, error);
    for (const [rid, request] of ipcRequests) {
      clearTimeout(request.timer);
      request.reject(error);
      ipcRequests.delete(rid);
    }
    ipcActiveCalls.clear();
    if (options.preserveProcessStdio) {
      suspendResources(error);
      modules.process?.__nivaSuspendSession?.(error);
    } else {
      invalidateResources(error);
    }
    if (heartbeatTimer) clearTimeout(heartbeatTimer);
    heartbeatTimer = null;
    heartbeatPending = false;
    closeSocketForSession();
    return true;
  }

  function updateHeartbeat() {
    if (ipcActiveCalls.size === 0) {
      if (heartbeatTimer) clearTimeout(heartbeatTimer);
      heartbeatTimer = null;
      heartbeatPending = false;
      return;
    }
    if (lastHeartbeatAck === 0) lastHeartbeatAck = Date.now();
    if (heartbeatTimer || heartbeatPending) return;
    heartbeatTimer = setTimeout(sendHeartbeat, 1000);
  }

  function nextRequestId(): number {
    nextRid = nextRid >= Number.MAX_SAFE_INTEGER ? 1 : nextRid + 1;
    return nextRid;
  }

  function postIpc(text: string): boolean {
    const host = ipcHost(root);
    if (!host) return false;
    host.ipc.postMessage(text);
    return true;
  }

  function sendHeartbeat() {
    heartbeatTimer = null;
    if (ipcActiveCalls.size === 0 || sessionExpired) return;
    if (!canIpc) return expireSession(nivaError("Niva IPC transport is unavailable", "ERR_NIVA_IPC_UNAVAILABLE"));
    const heartbeatSessionId = sessionId;
    const heartbeatEpoch = sessionEpoch;
    heartbeatPending = true;
    sendIpcRequest({ t: "heartbeat", sessionId: heartbeatSessionId, ...(local ? { token: root.__niva_token } : {}) }, 5000)
      .then((message) => {
        if (sessionId !== heartbeatSessionId || sessionEpoch !== heartbeatEpoch || sessionExpired) return;
        if (message.t === "heartbeatError") throw nivaError(message.message || "Niva IPC session expired", message.code || "ERR_NIVA_SESSION_EXPIRED");
        if (message.t !== "heartbeatAck") throw nivaError("Invalid Niva IPC heartbeat response", "ERR_NIVA_IPC_RESPONSE");
        lastHeartbeatAck = Date.now();
        heartbeatPending = false;
        updateHeartbeat();
      })
      .catch((error) => {
        if (sessionId !== heartbeatSessionId || sessionEpoch !== heartbeatEpoch || sessionExpired) return;
        heartbeatPending = false;
        if (ipcActiveCalls.size) expireSession(nivaError(String(error), "ERR_NIVA_SESSION_EXPIRED"));
      });
  }

  function parseIpc(raw: any): any {
    return typeof raw === "string" ? JSON.parse(raw) : raw;
  }

  function handleIpcMessage(message: any) {
    if (!message || typeof message !== "object") return;
    if (message.rid === undefined || message.rid === null) return;
    const rid = String(message.rid);
    const request = ipcRequests.get(rid);
    if (!request) return;
    ipcRequests.delete(rid);
    clearTimeout(request.timer);
    request.resolve(message);
  }

  function currentOriginMatches(source: any, origin: any): boolean {
    if (typeof origin !== "string" || !origin) return false;
    const actual = pageSourceOrigin(source);
    return actual !== null && actual === origin;
  }

  function forwardIpcReply(envelope: any): boolean {
    const origin = pageSourceOrigin(root);
    const target = frameRoutes.get(envelope.sessionId);
    if (!origin || !target || pageSourceOrigin(target, origin) !== origin || typeof target.__niva_ipc_reply !== "function") return false;
    try {
      return target.__niva_ipc_reply(envelope) === true;
    } catch (_) { return false; }
  }

  function settleIpcReply(envelope: any): boolean {
    if (!envelope || typeof envelope !== "object" || typeof envelope.sessionId !== "string"
      || !Number.isSafeInteger(envelope.rid) || typeof envelope.sourceOrigin !== "string"
      || !envelope.response || typeof envelope.response !== "object") return false;
    if (!currentOriginMatches(root, envelope.sourceOrigin)) return false;
    if (envelope.sessionId === sessionId) {
      const request = ipcRequests.get(String(envelope.rid));
      if (!request || request.sessionId !== envelope.sessionId || request.epoch !== sessionEpoch
        || request.sourceOrigin !== envelope.sourceOrigin) return false;
      handleIpcMessage({ ...envelope.response, rid: envelope.rid });
      return true;
    }
    return forwardIpcReply(envelope);
  }

  root.__niva_ipc_reply = function (envelope: any) {
    try { return settleIpcReply(envelope); }
    catch (_) { return false; }
  };

  function isDirectSameOriginChild(child: any, origin: string): boolean {
    try {
      const frames = root.frames;
      if (!frames || !child || pageSourceOrigin(child, origin) !== origin) return false;
      for (let index = 0; index < frames.length; index += 1) {
        if (frames[index] === child) return true;
      }
      return false;
    } catch (_) { return false; }
  }

  function registerFrameRoute(targetSessionId: string, child: any, action: "register" | "unregister"): boolean {
    try {
      const origin = pageSourceOrigin(root);
      if (!origin || !targetSessionId || !isDirectSameOriginChild(child, origin)) return false;
      if (action === "register") frameRoutes.set(targetSessionId, child);
      else if (frameRoutes.get(targetSessionId) === child) frameRoutes.delete(targetSessionId);
      const parent = root.parent;
      if (parent && parent !== root && pageSourceOrigin(parent, origin) === origin
        && typeof parent.__niva_register_frame_session === "function") {
        parent.__niva_register_frame_session(targetSessionId, root, action);
      }
      return true;
    } catch (_) { return false; }
  }

  root.__niva_register_frame_session = function (targetSessionId: string, child: any, action: "register" | "unregister") {
    return registerFrameRoute(targetSessionId, child, action);
  };

  function announceFrameSession(action: "register" | "unregister") {
    try {
      const parent = root.parent;
      const origin = pageSourceOrigin(root);
      if (!parent || parent === root || !origin || pageSourceOrigin(parent, origin) !== origin
        || typeof parent.__niva_register_frame_session !== "function") return false;
      return parent.__niva_register_frame_session(sessionId, root, action) === true;
    } catch (_) { return false; }
  }

  function sendIpcRequest(payload: any, timeoutMs = 60000): Promise<any> {
    if (!canIpc) return Promise.reject(nivaError("Niva IPC transport is unavailable", "ERR_NIVA_IPC_UNAVAILABLE"));
    const expectedOrigin = pageSourceOrigin(root);
    if (!expectedOrigin) return Promise.reject(nivaError("Niva IPC source origin is unavailable", "ERR_NIVA_IPC_UNAVAILABLE"));
    const requestSessionId = sessionId;
    const requestEpoch = sessionEpoch;
    const rid = nextRequestId();
    const ridKey = String(rid);
    const text = JSON.stringify({ ...payload, rid });
    if (new TextEncoder().encode(text).byteLength > 256 * 1024) {
      return Promise.reject(nivaError("Niva IPC request exceeds 256 KiB", "ERR_NIVA_IPC_TOO_LARGE"));
    }
    return new Promise((resolve, reject) => {
      const timer = setTimeout(() => {
        ipcRequests.delete(ridKey);
        reject(nivaError("Niva IPC reply timed out", "ETIMEDOUT"));
      }, timeoutMs);
      ipcRequests.set(ridKey, { resolve, reject, timer, sessionId: requestSessionId, epoch: requestEpoch, sourceOrigin: expectedOrigin });
      try {
        if (!postIpc(text)) throw nivaError("Niva IPC is unavailable", "ERR_NIVA_IPC_UNAVAILABLE");
      } catch (error) {
        const request = ipcRequests.get(ridKey);
        if (!request) return;
        ipcRequests.delete(ridKey);
        clearTimeout(request.timer);
        reject(error instanceof Error ? error : nivaError(String(error), "ERR_NIVA_IPC_ERROR"));
      }
    });
  }

  function beginIpcCall(id: number) {
    ipcActiveCalls.add(id);
    updateHeartbeat();
  }

  function endIpcCall(id: number) {
    ipcActiveCalls.delete(id);
    updateHeartbeat();
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

  root.__niva_native_event = function (encoded: any) {
    try {
      const message = parseIpc(encoded);
      if (!message || typeof message !== "object") return false;
      handleTextMessage(message);
      forwardNativeEvent(message);
      return true;
    } catch (_) {
      return false;
    }
  };

  function handleBinaryMessage(buffer: ArrayBuffer) {
    if (buffer.byteLength < 18) return;
    const view = new DataView(buffer);
    if (view.getUint8(0) !== 2) return;
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

  function socketIsReady(connection: WebSocket | null | undefined): connection is WebSocket {
    return !!connection && socket === connection && socketOpen && connection.readyState === 1;
  }

  function socketReady(): boolean {
    return socketIsReady(socket);
  }

  function loseSocket(owner: WebSocket, message = "Niva WebSocket connection closed") {
    if (socket !== owner) return;
    socket = null;
    socketOpen = false;
    const error = nivaError(message, "ECONNRESET");
    for (const id of Object.keys(pending)) {
      if (pending[id].transport === "ws") finish(pending, id, false, error);
    }
    invalidateResourcesForTransport("ws", error);
  }

  function handleWebSocketMessage(owner: WebSocket, message: any) {
    if (!message || typeof message !== "object") return;
    const id = Number(message.id);
    const entry = pending[String(id)];
    if (entry && (entry.sessionId !== sessionId || entry.epoch !== sessionEpoch)) return;
    if (message.t === "attached") {
      if (message.sessionId !== sessionId || !entry || entry.route.transport !== "ws"
        || entry.route.socket !== owner || entry.attachState !== "ws-pending") return;
      if (entry.timer) clearTimeout(entry.timer);
      entry.timer = undefined;
      entry.attachState = "attached";
      entry.opened = true;
      associateResourcesWithStream(id, "ws");
      if (entry.cancelRequested) sendNativeCancel(entry);
      else {
        pumpIncomingFrames(id, entry);
        pumpUpload(String(id), entry);
      }
      return;
    }
    if (message.t === "attachError") {
      if (message.sessionId !== sessionId || !entry || entry.route.transport !== "ws"
        || entry.route.socket !== owner || entry.attachState !== "ws-pending") return;
      if (entry.timer) clearTimeout(entry.timer);
      entry.timer = undefined;
      // Native explicitly rejected this attach before moving any data. Reuse
      // the already-created ticket on IPC; never issue the API call again.
      entry.route = { transport: "ipc" };
      entry.transport = "ipc";
      entry.attachState = "none";
      associateResourcesWithStream(id, "ipc");
      startIpcAttach(id, entry);
      return;
    }
    if (message.t === "channelData") {
      if (message.sessionId !== undefined && message.sessionId !== sessionId) return;
      if (!entry || entry.route.transport !== "ws" || entry.route.socket !== owner
        || entry.attachState !== "attached"
        || message.frame?.t !== "text" || typeof message.frame.data !== "string") return;
      frameInCurrentRealm({ id, seq: message.seq, frame: message.frame }, "ws");
      return;
    }
    handleTextMessage(message);
  }

  function startIpcAttach(id: number, entry: Pending) {
    if (!entry.active || !entry.ticketReady || !entry.capability || sessionExpired
      || entry.sessionId !== sessionId || entry.epoch !== sessionEpoch) return;
    entry.route = { transport: "ipc" };
    entry.transport = "ipc";
    entry.attachState = "ipc-pending";
    associateResourcesWithStream(id, "ipc");
    sendIpcRequest({
      t: "channelAttach",
      id,
      sessionId: entry.sessionId,
      token: root.__niva_token,
      capability: entry.capability,
    }, 15000).then((message) => {
      if (!entry.active || entry.sessionId !== sessionId || entry.epoch !== sessionEpoch || sessionExpired) return;
      if (message.t === "channelError") throw nivaError(message.message || "Niva IPC channel attach failed", message.code || "ERR_NIVA_CHANNEL_ATTACH");
      if (message.t !== "channelAttached" || message.id !== id || message.accepted !== true) {
        throw nivaError("Invalid Niva IPC channel attach response", "ERR_NIVA_IPC_RESPONSE");
      }
      entry.attachState = "attached";
      entry.opened = true;
      if (entry.cancelRequested || !entry.active) sendNativeCancel(entry);
      else {
        pumpIncomingFrames(id, entry);
        pumpUpload(String(id), entry);
      }
    }).catch((error) => {
      if (entry.active) failChannel(id, entry, error);
    });
  }

  function startWsAttach(id: number, entry: Pending, owner: WebSocket) {
    if (!entry.active || !entry.ticketReady || !entry.capability || sessionExpired
      || entry.sessionId !== sessionId || entry.epoch !== sessionEpoch) return;
    if (!socketIsReady(owner)) {
      startIpcAttach(id, entry);
      return;
    }
    entry.route = { transport: "ws", socket: owner };
    entry.transport = "ws";
    entry.attachState = "ws-pending";
    associateResourcesWithStream(id, "ws");
    entry.timer = setTimeout(() => {
      // Once sent, attachment outcome is uncertain. Do not bind the ticket to
      // IPC or replay the API call after a timeout.
      failChannel(id, entry, nivaError("Niva WebSocket channel attach timed out", "ETIMEDOUT"));
    }, 15000);
    try {
      owner.send(JSON.stringify({ t: "attach", id, sessionId: entry.sessionId, capability: entry.capability }));
    } catch (error) {
      failChannel(id, entry, error);
    }
  }

  function connectSocket() {
    if (!local || typeof root.__niva_ws_url !== "string" || !root.__niva_ws_url || !root.WebSocket || socket || sessionExpired) return;
    let connecting: WebSocket;
    try {
      connecting = new root.WebSocket(root.__niva_ws_url + "?token=" + encodeURIComponent(root.__niva_token));
      socket = connecting;
    } catch (_) {
      socket = null;
      return;
    }
    connecting.binaryType = "arraybuffer";
    connecting.addEventListener("open", () => {
      if (socket !== connecting || sessionExpired) return;
      try { connecting.send(JSON.stringify({ t: "hello", wid: root.__niva_window_id || 0, v: 2, sessionId })); }
      catch (_) { loseSocket(connecting, "Niva WebSocket authentication failed"); }
      if (socket === connecting) socketOpen = true;
    });
    connecting.addEventListener("message", (event) => {
      if (socket !== connecting) return;
      try {
        if (typeof event.data === "string") handleWebSocketMessage(connecting, JSON.parse(event.data));
        else if (event.data instanceof ArrayBuffer) {
          const buffer = event.data;
          if (buffer.byteLength < 18) return;
          const view = new DataView(buffer);
          if (view.getUint8(0) !== 2) return;
          const id = view.getUint32(2) * 0x100000000 + view.getUint32(6);
          const seq = view.getUint32(10) * 0x100000000 + view.getUint32(14);
          const entry = pending[String(id)];
          if (!entry || entry.sessionId !== sessionId || entry.epoch !== sessionEpoch
            || entry.route.transport !== "ws" || entry.route.socket !== connecting
            || entry.attachState !== "attached") return;
          frameInCurrentRealm({ id, seq, frame: { t: "binary", data: buffer } }, "ws");
        }
      } catch (error) { console.error("Invalid Niva bridge frame", error); }
    });
    connecting.addEventListener("close", () => loseSocket(connecting));
    connecting.addEventListener("error", () => {
      if (socket === connecting && connecting.readyState !== 1) loseSocket(connecting, "Niva WebSocket connection failed");
    });
  }

  function call(method: string, args: any[] = []): Promise<any> {
    if (sessionExpired) return Promise.reject(nivaError("Niva page session is no longer active", "ERR_NIVA_SESSION_EXPIRED"));
    return sendIpcCall(newId(), method, args);
  }

  function streamCall(method: string, args: any[] = [], handlers: any = {}) {
    if (sessionExpired) throw nivaError("Niva page session is no longer active", "ERR_NIVA_SESSION_EXPIRED");
    if (!local) throw nivaError("This API requires a trusted local Niva page", "ERR_NIVA_LOCAL_PAGE_REQUIRED");
    const id = newId();
    const key = String(id);
    let resolve!: (value: any) => void;
    let reject!: (error: any) => void;
    const promise = new Promise((res, rej) => { resolve = res; reject = rej; });
    const entry: Pending = {
      id,
      sessionId,
      epoch: sessionEpoch,
      transport: "ipc",
      route: { transport: "ipc" },
      active: true,
      attachState: "none",
      resolve,
      reject,
      onEvent: handlers.onEvent,
      onChunk: handlers.onChunk,
      onBlob: handlers.onBlob,
      groups: new Map(),
      seqOut: 0,
      outbound: [],
      outboundBytes: 0,
    };
    pending[key] = entry;
    associateResourcesWithStream(id, "ipc");
    try { dispatchStreamCall(id, method, args, entry); }
    catch (error) { finish(pending, id, false, error); }
    Promise.resolve().then(() => associateResourcesWithStream(id, entry.transport));
    const call = {
      id,
      promise,
      cancel() {
        return cancelStream(id, nivaError("Niva call was cancelled", "ABORT_ERR"));
      },
    };
    streamCalls.add(call);
    streamCallIds.set(call, id);
    return call;
  }

  function relatedStreamCall(this: any, source: object, method: string, args: any[] = [], handlers: any = {}) {
    if (streamCalls.has(source)) {
      const sourceId = streamCallIds.get(source);
      if (sourceId !== undefined && pending[String(sourceId)]?.active) return streamCall(method, args, handlers);
      throw nivaError("Niva stream has no active owner route", "ERR_NIVA_STREAM_OWNER_CLOSED");
    }
    if (typeof this?.stream === "function") return this.stream(method, args, handlers);
    throw nivaError("Niva stream has no active owner route", "ERR_NIVA_STREAM_OWNER_CLOSED");
  }

  function dispatchStreamCall(id: number, method: string, args: any[], entry: Pending) {
    if (!entry.active || sessionExpired || entry.sessionId !== sessionId || entry.epoch !== sessionEpoch) return;
    if (!canIpc) {
      finish(pending, id, false, nivaError("Niva IPC transport is unavailable", "ERR_NIVA_IPC_UNAVAILABLE"));
      return;
    }
    beginIpcCall(id);
    sendIpcRequest({ t: "api_call", id, method, args, sessionId: entry.sessionId, token: root.__niva_token }).then((message) => {
      if (!entry.active || entry.sessionId !== sessionId || entry.epoch !== sessionEpoch || sessionExpired) return;
      if (message.id !== id) throw nivaError("Mismatched Niva IPC stream id", "ERR_NIVA_IPC_RESPONSE");
      if (message.t === "channelOpened") {
        if (typeof message.capability !== "string" || !message.capability) throw nivaError("Invalid Niva IPC channel capability", "ERR_NIVA_IPC_RESPONSE");
        entry.capability = message.capability;
        entry.ticketReady = true;
        if (entry.cancelRequested || !entry.active) sendNativeCancel(entry);
        else if (socketReady() && socket) startWsAttach(id, entry, socket);
        else startIpcAttach(id, entry);
        return;
      }
      if (message.t === "result") {
        finish(pending, id, message.code === 0, message.code === 0 ? message.data : message);
        return;
      }
      throw nivaError("Invalid Niva IPC stream open response", "ERR_NIVA_IPC_RESPONSE");
    }).catch((error) => {
      if (entry.active) failChannel(id, entry, error);
    });
  }

  function cancelStream(idValue: number | string, error = nivaError("Niva stream resource was disposed", "ERR_NIVA_RESOURCE_CLOSED")) {
    const id = String(idValue);
    const entry = pending[id];
    if (!entry) return false;
    entry.cancelRequested = true;
    sendNativeCancel(entry);
    return finish(pending, id, false, error);
  }

  function streamSend(idValue: number | string, data: ArrayBuffer | Uint8Array | string, end = false): boolean {
    if (sessionExpired) return false;
    const id = String(idValue);
    const entry = pending[id];
    if (!entry || !entry.active) return false;
    let bytes: Uint8Array;
    if (data instanceof ArrayBuffer || Object.prototype.toString.call(data) === "[object ArrayBuffer]") {
      bytes = new Uint8Array(data as ArrayBuffer);
    } else if (ArrayBuffer.isView(data)) {
      const view = data as ArrayBufferView;
      bytes = new Uint8Array(view.buffer, view.byteOffset, view.byteLength);
    } else {
      bytes = new TextEncoder().encode(String(data));
    }
    return queueUpload(id, entry, bytes, end);
  }

  function sendIpcCall(id: number, method: string, args: any[]): Promise<any> {
    if (!canIpc) return Promise.reject(nivaError("Niva IPC transport is unavailable", "ERR_NIVA_IPC_UNAVAILABLE"));
    const callSessionId = sessionId;
    const callEpoch = sessionEpoch;
    beginIpcCall(id);
    return sendIpcRequest({ t: "api_call", id, method, args, sessionId: callSessionId, ...(local ? { token: root.__niva_token } : {}) })
      .then((message) => {
        if (sessionId !== callSessionId || sessionEpoch !== callEpoch || sessionExpired) {
          throw nivaError("Niva page session is no longer active", "ERR_NIVA_SESSION_EXPIRED");
        }
        if (message.id !== id || message.t !== "result") throw nivaError("Invalid Niva IPC call response", "ERR_NIVA_IPC_RESPONSE");
        if (message.code === 0) return message.data;
        throw message;
      })
      .finally(() => endIpcCall(id));
  }

  function encodeBinaryFrame(id: number, seq: number, bytes: Uint8Array, end: boolean): ArrayBuffer {
    const frame = new Uint8Array(18 + bytes.byteLength);
    frame[0] = 2;
    frame[1] = (seq === 1 ? 1 : 0) | (end ? 2 : 0);
    const view = new DataView(frame.buffer);
    view.setUint32(2, Math.floor(id / 0x100000000));
    view.setUint32(6, id >>> 0);
    view.setUint32(10, Math.floor(seq / 0x100000000));
    view.setUint32(14, seq >>> 0);
    frame.set(bytes, 18);
    return frame.buffer;
  }

  function queueUpload(id: string, entry: Pending, bytes: Uint8Array, end: boolean): boolean {
    const chunks = Math.max(1, Math.ceil(bytes.byteLength / 16384));
    const framedBytes = bytes.byteLength + chunks * 18;
    const queue = entry.outbound || (entry.outbound = []);
    if (queue.length + chunks > 64 || (entry.outboundBytes || 0) + framedBytes > 1024 * 1024
      || queuedFrames + chunks > 256
      || queuedBytes + framedBytes > 4 * 1024 * 1024) return false;

    let offset = 0;
    let seq = entry.seqOut || 0;
    for (let index = 0; index < chunks; index += 1) {
      const length = bytes.byteLength === 0 ? 0 : Math.min(16384, bytes.byteLength - offset);
      const payload = bytes.subarray(offset, offset + length);
      offset += length;
      seq += 1;
      const frameEnd = end && index === chunks - 1;
      queue.push({ seq, frame: encodeBinaryFrame(Number(id), seq, payload, frameEnd), end: frameEnd });
    }
    entry.seqOut = seq;
    entry.outboundBytes = (entry.outboundBytes || 0) + framedBytes;
    queuedFrames += chunks;
    queuedBytes += framedBytes;
    pumpUpload(id, entry);
    return true;
  }

  function clearOutbound(entry: Pending) {
    const queue = entry.outbound;
    if (entry.outboundRetryTimer) clearTimeout(entry.outboundRetryTimer);
    entry.outboundRetryTimer = undefined;
    if (queue) {
      for (const item of queue) {
        queuedFrames = Math.max(0, queuedFrames - 1);
        queuedBytes = Math.max(0, queuedBytes - item.frame.byteLength);
      }
      queue.length = 0;
    }
    entry.outboundBytes = 0;
    entry.outboundSending = false;
  }

  function encodeBase64(bytes: Uint8Array): string {
    const alphabet = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let output = "";
    for (let offset = 0; offset < bytes.length; offset += 3) {
      const a = bytes[offset];
      const hasB = offset + 1 < bytes.length;
      const hasC = offset + 2 < bytes.length;
      const b = hasB ? bytes[offset + 1] : 0;
      const c = hasC ? bytes[offset + 2] : 0;
      output += alphabet[a >> 2]
        + alphabet[((a & 3) << 4) | (b >> 4)]
        + (hasB ? alphabet[((b & 15) << 2) | (c >> 6)] : "=")
        + (hasC ? alphabet[c & 63] : "=");
    }
    return output;
  }

  function decodeBase64(encoded: string): ArrayBuffer {
    if (!/^(?:[A-Za-z0-9+/]{4})*(?:[A-Za-z0-9+/]{2}==|[A-Za-z0-9+/]{3}=)?$/.test(encoded)) {
      throw nivaError("Invalid Niva binary frame encoding", "ERR_NIVA_IPC_RESPONSE");
    }
    const alphabet = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    const padding = encoded.endsWith("==") ? 2 : encoded.endsWith("=") ? 1 : 0;
    const bytes = new Uint8Array((encoded.length / 4) * 3 - padding);
    let output = 0;
    for (let offset = 0; offset < encoded.length; offset += 4) {
      const a = alphabet.indexOf(encoded[offset]);
      const b = alphabet.indexOf(encoded[offset + 1]);
      const c = encoded[offset + 2] === "=" ? 0 : alphabet.indexOf(encoded[offset + 2]);
      const d = encoded[offset + 3] === "=" ? 0 : alphabet.indexOf(encoded[offset + 3]);
      if (output < bytes.length) bytes[output++] = (a << 2) | (b >> 4);
      if (output < bytes.length) bytes[output++] = ((b & 15) << 4) | (c >> 2);
      if (output < bytes.length) bytes[output++] = ((c & 3) << 6) | d;
    }
    return bytes.buffer;
  }

  function pumpUpload(id: string, entry: Pending) {
    const queue = entry.outbound;
    if (!entry.active || !entry.opened || !entry.capability
      || entry.outboundSending || entry.outboundRetryTimer || !queue || queue.length === 0) return;
    if (entry.outboundRetryStarted !== undefined && Date.now() - entry.outboundRetryStarted >= 10000) {
      failChannel(id, entry, nivaError("Niva IPC channel send remained backpressured", "ENOBUFS", { retryable: true }));
      return;
    }
    const item = queue[0];
    if (entry.route.transport === "ws") {
      const owner = entry.route.socket;
      if (!socketIsReady(owner)) {
        failChannel(id, entry, nivaError("Niva WebSocket owner connection is closed", "ECONNRESET"));
        return;
      }
      const buffered = Number((owner as any).bufferedAmount) || 0;
      if (buffered + item.frame.byteLength > 1024 * 1024) {
        if (entry.outboundRetryStarted === undefined) entry.outboundRetryStarted = Date.now();
        if (Date.now() - entry.outboundRetryStarted >= 10000) {
          failChannel(id, entry, nivaError("Niva WebSocket channel send remained backpressured", "ENOBUFS", { retryable: true }));
          return;
        }
        entry.outboundRetryTimer = setTimeout(() => {
          entry.outboundRetryTimer = undefined;
          pumpUpload(id, entry);
        }, 10);
        return;
      }
      try {
        owner.send(item.frame);
        queue.shift();
        entry.outboundBytes = Math.max(0, (entry.outboundBytes || 0) - item.frame.byteLength);
        queuedFrames = Math.max(0, queuedFrames - 1);
        queuedBytes = Math.max(0, queuedBytes - item.frame.byteLength);
        entry.outboundRetryStarted = undefined;
        pumpUpload(id, entry);
      } catch (error) { failChannel(id, entry, error); }
      return;
    }
    entry.outboundSending = true;
    sendIpcRequest({
      t: "channelSend",
      id: Number(id),
      frame: encodeBase64(new Uint8Array(item.frame)),
      sessionId,
      token: root.__niva_token,
      capability: entry.capability,
    }, 60000).then((message) => {
      if (!entry.active) return;
      if (message.t === "channelError") {
        throw nivaError(message.message || "Niva IPC channel send failed", message.code || "ERR_NIVA_CHANNEL_SEND", { retryable: !!message.retryable });
      }
      if (message.t !== "channelAck" || message.id !== Number(id) || message.seq !== item.seq || message.accepted !== true) {
        throw nivaError("Niva IPC channel send was rejected or acknowledged out of order", "ERR_NIVA_CHANNEL_SEND");
      }
      if (queue[0] !== item) throw nivaError("Niva IPC channel acknowledgement is out of order", "ERR_NIVA_CHANNEL_SEQUENCE");
      queue.shift();
      entry.outboundBytes = Math.max(0, (entry.outboundBytes || 0) - item.frame.byteLength);
      queuedFrames = Math.max(0, queuedFrames - 1);
      queuedBytes = Math.max(0, queuedBytes - item.frame.byteLength);
      entry.outboundSending = false;
      entry.outboundRetryStarted = undefined;
      entry.outboundRetryAttempt = 0;
      pumpUpload(id, entry);
    }).catch((error) => {
      entry.outboundSending = false;
      if (!entry.active) return;
      const failure = error instanceof Error ? error : nivaError(String(error), "ERR_NIVA_CHANNEL_SEND");
      if ((failure as any).retryable === true) retryIpcSend(id, entry, failure);
      else failChannel(id, entry, failure);
    });
  }

  function retryIpcSend(id: string, entry: Pending, error: Error) {
    const now = Date.now();
    if (entry.outboundRetryStarted === undefined) entry.outboundRetryStarted = now;
    const elapsed = now - entry.outboundRetryStarted;
    if (elapsed >= 10000) {
      failChannel(id, entry, error);
      return;
    }
    const attempt = entry.outboundRetryAttempt || 0;
    entry.outboundRetryAttempt = attempt + 1;
    const delay = Math.min(250, 10 * Math.pow(2, Math.min(attempt, 5)), 10000 - elapsed);
    entry.outboundRetryTimer = setTimeout(() => {
      entry.outboundRetryTimer = undefined;
      pumpUpload(id, entry);
    }, delay);
  }

  function frameInCurrentRealm(message: any, transport: "ipc" | "ws" = "ipc"): boolean {
    const id = Number(message.id);
    const seq = Number(message.seq);
    const entry = pending[String(id)];
    if (!Number.isSafeInteger(id) || !Number.isSafeInteger(seq) || seq < 1 || !entry
      || entry.sessionId !== sessionId || entry.epoch !== sessionEpoch
      || entry.transport !== transport || !entry.active) return false;
    if (seq !== (entry.seqIn || 0) + 1) {
      failChannel(id, entry, nivaError("Niva IPC channel frame is out of sequence", "ERR_NIVA_CHANNEL_SEQUENCE"));
      return false;
    }
    const frame = message.frame;
    if (!frame || typeof frame !== "object") {
      failChannel(id, entry, nivaError("Invalid Niva IPC channel frame", "ERR_NIVA_IPC_RESPONSE"));
      return false;
    }
    let frameBytes: number;
    try {
      frameBytes = frame.t === "binary" && frame.data instanceof ArrayBuffer
        ? frame.data.byteLength
        : new TextEncoder().encode(JSON.stringify(frame)).byteLength;
    } catch (error) {
      failChannel(id, entry, error);
      return false;
    }
    const inbound = entry.inboundFrames || (entry.inboundFrames = []);
    if (inbound.length >= 8 || (entry.inboundFrameBytes || 0) + frameBytes > 256 * 1024) {
      failChannel(id, entry, nivaError("Niva IPC channel receive queue is full", "ENOBUFS"));
      return false;
    }
    inbound.push({ message, size: frameBytes });
    entry.inboundFrameBytes = (entry.inboundFrameBytes || 0) + frameBytes;
    entry.seqIn = seq;
    entry.dataStarted = true;
    pumpIncomingFrames(id, entry);
    return true;
  }

  function pumpIncomingFrames(id: number, entry: Pending) {
    const queue = entry.inboundFrames;
    if (!entry.active || !entry.opened || !entry.capability || entry.inboundAcking || !queue || queue.length === 0) return;
    const item = queue[0];
    const message = item.message;
    const seq = Number(message.seq);
    const frame = message.frame;
    const transport = entry.route.transport;
    let terminalText: any;
    let eventText: any;
    try {
      if (frame.t === "text" && typeof frame.data === "string") {
        const serverMessage = JSON.parse(frame.data);
        if (!serverMessage || typeof serverMessage !== "object"
          || (serverMessage.id !== undefined && serverMessage.id !== null && Number(serverMessage.id) !== id)) {
          throw nivaError("Mismatched Niva channel text frame", "ERR_NIVA_IPC_RESPONSE");
        }
        if (serverMessage.t === "result") terminalText = serverMessage;
        else if (serverMessage.t === "event") eventText = serverMessage;
        else throw nivaError("Unsupported Niva channel text frame", "ERR_NIVA_IPC_RESPONSE");
      } else if (frame.t === "binary" && ((transport === "ipc" && typeof frame.data === "string")
        || (transport === "ws" && frame.data instanceof ArrayBuffer))) {
        const buffer = transport === "ipc" ? decodeBase64(frame.data) : frame.data as ArrayBuffer;
        if (buffer.byteLength < 18) throw nivaError("Truncated Niva binary frame", "ERR_NIVA_IPC_RESPONSE");
        const view = new DataView(buffer);
        const version = view.getUint8(0);
        const frameId = view.getUint32(2) * 0x100000000 + view.getUint32(6);
        const frameSeq = view.getUint32(10) * 0x100000000 + view.getUint32(14);
        if (version !== 2 || frameId !== id) throw nivaError("Mismatched Niva binary frame header", "ERR_NIVA_IPC_RESPONSE");
        if (transport === "ws" && frameSeq !== seq) throw nivaError("Mismatched Niva WebSocket frame sequence", "ERR_NIVA_CHANNEL_SEQUENCE");
        handleBinaryMessage(buffer);
      } else {
        throw nivaError("Unsupported Niva IPC Channel frame", "ERR_NIVA_IPC_RESPONSE");
      }
    } catch (error) {
      failChannel(id, entry, error);
      return;
    }
    entry.inboundAcking = true;
    const acknowledge = transport === "ws"
      ? Promise.resolve().then(() => {
        const owner = entry.route.transport === "ws" ? entry.route.socket : null;
        if (!owner || !socketIsReady(owner)) throw nivaError("Niva WebSocket owner connection is closed", "ECONNRESET");
        owner.send(JSON.stringify({ t: "ack", id, sessionId, seq }));
      })
      : sendIpcRequest({
        t: "channelAck",
        id,
        sessionId,
        token: root.__niva_token,
        capability: entry.capability,
        seq,
      }, 3000).then((ack) => {
        if (ack.t === "channelError") throw nivaError(ack.message || "Niva IPC channel acknowledgement failed", ack.code || "ERR_NIVA_CHANNEL_ACK", { retryable: !!ack.retryable });
        if (ack.t !== "channelAckReceived" || ack.id !== id || ack.seq !== seq || ack.accepted !== true) {
          throw nivaError("Invalid Niva IPC channel acknowledgement", "ERR_NIVA_CHANNEL_ACK");
        }
      });
    acknowledge.then(() => {
      if (queue[0] !== item) throw nivaError("Niva IPC channel receive order changed", "ERR_NIVA_CHANNEL_SEQUENCE");
      queue.shift();
      entry.inboundFrameBytes = Math.max(0, (entry.inboundFrameBytes || 0) - item.size);
      entry.inboundAcking = false;
      if (terminalText) handleTextMessage(terminalText);
      else if (eventText) handleTextMessage(eventText);
      pumpIncomingFrames(id, entry);
    }).catch((error) => {
      entry.inboundAcking = false;
      if (entry.active) failChannel(id, entry, error);
    });
  }

  function forwardNativeFrame(message: any) {
    const origin = pageSourceOrigin(root);
    const target = frameRoutes.get(message.sessionId);
    if (!origin || !target || pageSourceOrigin(target, origin) !== origin || typeof target.__niva_native_frame !== "function") return false;
    try { return target.__niva_native_frame(message) === true; }
    catch (_) { return false; }
  }

  function forwardNativeEvent(message: any) {
    const origin = pageSourceOrigin(root);
    const frames = root.frames;
    if (!origin || !frames) return false;
    let forwarded = false;
    for (let index = 0; index < frames.length; index += 1) {
      try {
        const child = frames[index];
        if (!isDirectSameOriginChild(child, origin) || typeof child.__niva_native_event !== "function") continue;
        if (child.__niva_native_event(message) === true) forwarded = true;
      } catch (_) { /* IPC window events never cross an origin boundary. */ }
    }
    return forwarded;
  }

  root.__niva_native_frame = function (message: any) {
    if (!message || typeof message !== "object" || typeof message.sessionId !== "string" || !message.frame) return false;
    if (!sessionExpired && message.sessionId === sessionId) return frameInCurrentRealm(message);
    return forwardNativeFrame(message);
  };

  announceFrameSession("register");

  function sendNativeCancel(entry: Pending) {
    if (entry.cancelSent) return;
    if (!entry.active || entry.sessionId !== sessionId || entry.epoch !== sessionEpoch) return;
    if (entry.route.transport === "ws") {
      const owner = entry.route.socket;
      if (!socketIsReady(owner)) return;
      try {
        owner.send(JSON.stringify({ t: "cancel", id: entry.id, sessionId: entry.sessionId }));
        entry.cancelSent = true;
      }
      catch (_) { /* the local promise still settles */ }
      return;
    }
    if (!entry.ticketReady || !entry.capability) return;
    const id = entry.id;
    if (!Number.isSafeInteger(id)) return;
    entry.cancelSent = true;
    sendIpcRequest({ t: "channelCancel", id, sessionId: entry.sessionId, token: root.__niva_token, capability: entry.capability }, 10000)
      .then((message) => {
        if (message.t !== "channelCancelled" || message.id !== id || message.accepted !== true) throw nivaError("Invalid Niva channel cancel acknowledgement", "ERR_NIVA_IPC_RESPONSE");
      })
      .catch(() => { entry.cancelSent = false; });
  }

  function failChannel(id: number | string, entry: Pending, error: any) {
    if (!entry.active) return;
    entry.cancelRequested = true;
    sendNativeCancel(entry);
    finish(pending, id, false, error);
  }

  function callSync(method: string, args: any[] = [], warningApiName = method): any {
    if (sessionExpired) throw nivaError("Niva page session is no longer active", "ERR_NIVA_SESSION_EXPIRED");
    if (!local) throw nivaError("Synchronous Niva bridge calls require a trusted local page", "ERR_NIVA_SYNC_UNAVAILABLE");
    if (!syncWarnings.has(warningApiName)) {
      syncWarnings.add(warningApiName);
      root.console?.warn?.("Niva synchronous compatibility API '" + warningApiName + "' blocks the WebView through synchronous XHR.");
    }
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

  const bridgeApi: any = {
    call,
    callSync,
    stream: streamCall,
    streamSend,
    isTrustedLocal,
  };
  Object.defineProperty(bridgeApi, "sessionId", { enumerable: true, get: () => sessionId });
  Object.defineProperty(bridgeApi, Symbol.for("niva.internal.bridge.streamRelated"), {
    value: relatedStreamCall,
  });
  Object.defineProperty(bridgeApi, Symbol.for("niva.internal.bridge.callSyncApi"), {
    value(this: any, apiName: string, method: string, args: any[] = []) {
      if (typeof this?.callSync === "function") return this.callSync(method, args, apiName);
      return callSync(method, args, apiName);
    },
  });

  const Niva: any = {
    bridgeVersion: 1,
    bootstrap: root.__niva_node_bootstrap || {},
    bridge: bridgeApi,
    __bridge: { isTrustedLocal },
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
  Niva.runtimeConfig = Object.freeze({ injectCommonJs, injectEsm, trustedLocal: local, nonce: config.nonce });
  // Native HTML owns import maps for trusted local pages. External pages must
  // use their own bundler: this runtime has no cross-origin asset URL/CORS
  // contract and must not install a relative map that silently points elsewhere.

  function getResourceStreamId(streamId?: number | (() => number | undefined)): number | undefined {
    try {
      const value = typeof streamId === "function" ? streamId() : streamId;
      return Number.isSafeInteger(value) && Number(value) > 0 ? Number(value) : undefined;
    } catch (_) { return undefined; }
  }

  function associateResourcesWithStream(streamId: number, transport: "ws" | "ipc") {
    streamTransports.set(streamId, transport);
    for (const record of resourceOwners.values()) {
      if (getResourceStreamId(record.streamId) === streamId) record.transport = transport;
    }
  }

  function invalidateResourcesForTransport(transport: "ws" | "ipc", error: Error) {
    for (const [id, record] of Array.from(resourceOwners.entries())) {
      if (record.transport !== transport) continue;
      resourceOwners.delete(id);
      resourceFinalizer?.unregister(record.unregisterToken);
      const resource = record.ref?.deref() || record.fallback;
      if (!resource) continue;
      try { resource.__nivaInvalidate?.(error); } catch (_) { /* continue invalidating sibling resources */ }
    }
  }

  function registerResource<T extends object>(
    resource: T,
    finalizeNative?: () => unknown,
    streamId?: number | (() => number | undefined),
  ) {
    const id = ++nextResourceOwner;
    const unregisterToken = {};
    const ref = typeof WeakRef === "function" ? new WeakRef(resource) : undefined;
    const associatedStreamId = getResourceStreamId(streamId);
    resourceOwners.set(id, {
      ref,
      fallback: ref ? undefined : resource,
      unregisterToken,
      finalizeNative,
      streamId,
      transport: associatedStreamId === undefined ? undefined : streamTransports.get(associatedStreamId),
    });
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
  function suspendResources(error: Error) {
    const records = Array.from(resourceOwners.values());
    resourceOwners.clear();
    for (const record of records) {
      resourceFinalizer?.unregister(record.unregisterToken);
      const resource = record.ref?.deref() || record.fallback;
      if (!resource) continue;
      try {
        if (typeof resource.__nivaSuspend === "function") resource.__nivaSuspend(error);
        else resource.__nivaInvalidate?.(error);
      } catch (_) { /* continue suspending or invalidating sibling resources */ }
    }
  }
  runtime.registerResource = registerResource;
  runtime.invalidateResources = invalidateResources;
  runtime.cancelStream = cancelStream;
  Niva.__runtime = Object.freeze({ registerResource, isMainProcess: mainProcessAvailable });

  function handlePageHide(event: any) {
    announceFrameSession("unregister");
    frameRoutes.clear();
    if (sessionExpired) {
      bfcacheSuspended = false;
      return;
    }
    const preserveProcessStdio = event?.persisted === true;
    bfcacheSuspended = preserveProcessStdio;
    const ended = expireSession(
      nivaError("Niva page session ended", "ERR_NIVA_SESSION_EXPIRED"),
      { retireNative: true, preserveProcessStdio },
    );
    if (!ended) bfcacheSuspended = false;
  }

  function handlePageShow(event: any) {
    if (!bfcacheSuspended) return;
    if (event?.persisted !== true || !sessionExpired) {
      bfcacheSuspended = false;
      return;
    }
    const restoredOrigin = pageSourceOrigin(root);
    if (!restoredOrigin || restoredOrigin !== sourceOrigin || !isSupportedFrame(root) || !ipcHost(root)) {
      bfcacheSuspended = false;
      return;
    }

    sessionId = makeSessionId(root);
    sessionEpoch += 1;
    sessionExpired = false;
    heartbeatPending = false;
    lastHeartbeatAck = 0;
    ipcActiveCalls.clear();
    streamTransports.clear();
    queuedFrames = 0;
    queuedBytes = 0;
    bfcacheSuspended = false;
    announceFrameSession("register");
    if (local) connectSocket();
    modules.process?.__nivaResumeSession?.();
  }

  root.addEventListener?.("pagehide", handlePageHide);
  root.addEventListener?.("pageshow", handlePageShow);
  if (local) connectSocket();
  Niva.__runtimeBootstrap = true;

  function inheritSameOriginRuntime(host: any) {
    try {
      const parent = host.parent;
      if (!parent || parent === host) return;
      const parentOrigin = pageSourceOrigin(parent);
      const childOrigin = pageSourceOrigin(host, parentOrigin || undefined);
      if (!childOrigin || childOrigin !== parentOrigin) return;
      if (host.__niva_source_origin === undefined) host.__niva_source_origin = parentOrigin;
      if (host.__niva_token && host.__niva_window_id !== undefined && host.__niva_window_id !== null) return;
      const token = parent.__niva_token;
      const windowId = parent.__niva_window_id;
      if (typeof token !== "string" || !token || windowId === undefined || windowId === null) return;

      host.__niva_token = token;
      if (host.__niva_window_id === undefined) host.__niva_window_id = windowId;
      if (host.__niva_ws_url === undefined) host.__niva_ws_url = parent.__niva_ws_url;
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
      const resolvedFilename = runtime.callSyncAs(niva, "module.resolve", "module.resolve", [specifier, parent]);
      const resolvedBuiltin = typeof niva.__getModule === "function" ? niva.__getModule(resolvedFilename) : { found: false, value: undefined };
      if (resolvedBuiltin.found) return resolvedBuiltin.value;
      if (typeof resolvedFilename === "string" && resolvedFilename.startsWith("node:")) {
        throw nivaError("Unsupported Node builtin module: " + resolvedFilename, "ERR_UNKNOWN_BUILTIN_MODULE");
      }
      if (typeof resolvedFilename !== "string") throw nivaError("Native module resolver returned an invalid result", "ERR_NIVA_MODULE_RESOLVE_RESPONSE");
      if (resolvedFilename.endsWith(".node")) throw nivaError("Native Node addons are not supported: " + resolvedFilename, "ERR_DLOPEN_FAILED");
      if (resolvedFilename.endsWith(".mjs") || resolvedFilename.endsWith(".mts")) throw nivaError("require() of ES modules is not supported: " + resolvedFilename, "ERR_REQUIRE_ESM");
      if (cache[resolvedFilename]) return cache[resolvedFilename].exports;
      const loaded = runtime.callSyncAs(niva, "module.load", "module.load", [specifier, parent]);
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
      const resolved = runtime.callSyncAs(niva, "module.resolve", "module.resolve", [specifier, parentFilename]);
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
