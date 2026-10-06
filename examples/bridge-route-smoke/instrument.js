(() => {
  "use strict";
  const events = [];
  let order = 0;
  const replySessions = new Set();
  const record = (kind, detail = {}) => {
    events.push({ order: ++order, at: performance.now(), kind, ...detail });
  };
  Object.defineProperty(window, "__bridgeRouteEvents", { configurable: true, value: events });
  Object.defineProperty(window, "__bridgeRouteRecord", { configurable: true, value: record });
  Object.defineProperty(window, "__bridgeRouteSessionCount", { configurable: true, value: () => replySessions.size });
  Object.defineProperty(window, "__bridgeRouteSessionValues", { configurable: true, value: () => Array.from(replySessions) });
  Object.defineProperty(window, "__bridgeRouteImportEvents", { configurable: true, value: (items, sourceFrame) => {
    for (const item of items) {
      const { order: sourceOrder, at: sourceAt, kind, ...detail } = item;
      record(kind, { ...detail, sourceFrame, sourceOrder, sourceAt });
    }
  }});
  record("instrumentation-start", { phase: "document-script", websocketHelloMayPrecede: true });
  const decode = value => {
    if (typeof value !== "string") return value;
    try { return JSON.parse(value); } catch (_) { return { raw: value }; }
  };
  const safeMessage = value => {
    if (Array.isArray(value)) return value.map(safeMessage);
    if (!value || typeof value !== "object") return value;
    const result = {};
    for (const [key, item] of Object.entries(value)) {
      if (key === "token" || key === "capability" || key === "sessionId") {
        result[key === "sessionId" ? "hasSessionId" : `has${key[0].toUpperCase()}${key.slice(1)}`] = !!item;
      } else if (key === "args") {
        result.args = {
          count: Array.isArray(item) ? item.length : null,
          serializedBytes: JSON.stringify(item).length
        };
      } else if (key === "frame" && typeof item === "string") {
        result.frameEncodedLength = item.length;
      } else if (key === "data") {
        result.dataShape = typeof item;
        if (typeof item === "string") result.dataEncodedLength = item.length;
        else if (Array.isArray(item)) result.dataElementCount = item.length;
        else if (item && typeof item === "object") result.dataFieldCount = Object.keys(item).length;
      } else {
        result[key] = safeMessage(item);
      }
    }
    return result;
  };
  const largeDataFields = value => {
    if (Array.isArray(value)) return value.reduce((total, item) => total + largeDataFields(item), 0);
    if (!value || typeof value !== "object") return 0;
    return Object.entries(value).reduce((total, [key, item]) =>
      total + (key === "data" && typeof item === "string" && item.length > 1024 ? 1 : 0) + largeDataFields(item), 0);
  };
  const safeUrl = value => {
    try {
      const parsed = new URL(String(value), location.href);
      return `${parsed.origin}${parsed.pathname}`;
    } catch (_) {
      return String(value).split(/[?#]/, 1)[0];
    }
  };
  const wrapMethod = (target, key, makeWrapper) => {
    try {
      const descriptor = Object.getOwnPropertyDescriptor(target, key);
      const original = target[key];
      if (typeof original !== "function") return false;
      const wrapped = makeWrapper(original);
      Object.defineProperty(target, key, {
        configurable: descriptor ? descriptor.configurable : true,
        enumerable: descriptor ? descriptor.enumerable : true,
        writable: descriptor ? descriptor.writable !== false : true,
        value: wrapped
      });
      return target[key] === wrapped;
    } catch (_) {
      return false;
    }
  };

  const recordIpcPayload = (message, kind = "ipc-send", serializedBytes = null) => {
    const decoded = decode(message);
    record(kind, {
      message: safeMessage(decoded),
      serializedBytes: serializedBytes === null ? JSON.stringify(decoded).length : serializedBytes,
      largeDataFields: largeDataFields(decoded)
    });
  };
  function installWryIpcHook() {
    const handler = window.webkit && window.webkit.messageHandlers && window.webkit.messageHandlers.ipc;
    if (!handler || typeof handler.postMessage !== "function") {
      return installWireSerializationHook();
    }
    let candidate = handler;
    let depth = 0;
    while (candidate) {
      if (Object.prototype.hasOwnProperty.call(candidate, "postMessage")
        && wrapMethod(candidate, "postMessage", original => function (message) {
          if (this === handler) recordIpcPayload(message);
          return original.apply(this, arguments);
        })) {
        record("ipc-hook-installed", { target: depth === 0 ? "wry-handler" : "wry-handler-prototype", depth });
        return true;
      }
      candidate = Object.getPrototypeOf(candidate);
      depth += 1;
    }
    return installWireSerializationHook();
  }
  function installWireSerializationHook() {
    if (!wrapMethod(JSON, "stringify", stringify => function (value) {
        const serialized = stringify.apply(this, arguments);
        if (value && typeof value === "object" && ["api_call", "channelAttach", "channelSend", "channelAck", "channelCancel"].includes(value.t)) {
          recordIpcPayload(value, "wire-serialized", serialized.length);
        }
        return serialized;
      })) {
      record("instrumentation-error", { message: "Wry IPC handler is read-only and JSON.stringify cannot be safely wrapped" });
      return false;
    }
    record("wire-serialization-hook-installed", { evidence: "wire-serialized; serialization is not proof of IPC send" });
    return true;
  }
  installWryIpcHook();

  const observedSockets = new WeakSet();
  const recordWsReceive = (socket, data) => {
    if (typeof data === "string") {
      const decoded = decode(data);
      record("ws-recv-text", { message: safeMessage(decoded), url: safeUrl(socket.url) });
      return;
    }
    let bytes;
    if (data instanceof ArrayBuffer) bytes = new Uint8Array(data);
    else if (ArrayBuffer.isView(data)) bytes = new Uint8Array(data.buffer, data.byteOffset, data.byteLength);
    else if (data instanceof Blob) {
      record("ws-recv-binary", { url: safeUrl(socket.url), byteLength: data.size, decodedLater: true });
      data.arrayBuffer().then(buffer => recordWsReceive(socket, buffer));
      return;
    } else return;
    let id = null;
    let seq = null;
    if (bytes.byteLength >= 18) {
      const view = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength);
      id = view.getUint32(2) * 0x100000000 + view.getUint32(6);
      seq = view.getUint32(10) * 0x100000000 + view.getUint32(14);
    }
    record("ws-recv-binary", {
      url: safeUrl(socket.url),
      byteLength: bytes.byteLength,
      protocolVersion: bytes.byteLength ? bytes[0] : null,
      id,
      seq,
      payloadLength: Math.max(0, bytes.byteLength - 18)
    });
  };
  const observeAttachedSocket = socket => {
    if (observedSockets.has(socket)) return;
    observedSockets.add(socket);
    try {
      socket.addEventListener("message", event => recordWsReceive(socket, event.data), true);
      record("ws-incoming-observer-installed", { url: safeUrl(socket.url), readyState: socket.readyState });
    } catch (error) {
      record("instrumentation-error", { message: `WS incoming observer failed: ${error && error.message || error}` });
    }
  };

  if (!wrapMethod(WebSocket.prototype, "send", original => function (data) {
      const decoded = typeof data === "string" ? decode(data) : null;
      if (decoded && decoded.t === "attach") observeAttachedSocket(this);
      if (typeof data === "string") {
        record("ws-send-text", { message: safeMessage(decoded), url: safeUrl(this.url), readyState: this.readyState });
      } else {
        let bytes;
        if (data instanceof ArrayBuffer) bytes = new Uint8Array(data);
        else if (ArrayBuffer.isView(data)) bytes = new Uint8Array(data.buffer, data.byteOffset, data.byteLength);
        else bytes = new Uint8Array(0);
        let id = null;
        let seq = null;
        if (bytes.byteLength >= 18) {
          const view = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength);
          id = view.getUint32(2) * 0x100000000 + view.getUint32(6);
          seq = view.getUint32(10) * 0x100000000 + view.getUint32(14);
        }
        record("ws-send-binary", {
          url: safeUrl(this.url),
          byteLength: bytes.byteLength,
          protocolVersion: bytes.byteLength ? bytes[0] : null,
          id,
          seq,
          payloadLength: Math.max(0, bytes.byteLength - 18)
        });
      }
      return original.call(this, data);
    })) {
    record("instrumentation-error", { message: "WebSocket.prototype.send cannot be safely wrapped" });
  }

  if (!wrapMethod(window, "__niva_ipc_reply", reply => function (envelope) {
      const response = envelope && envelope.response;
      if (envelope && typeof envelope.sessionId === "string") replySessions.add(envelope.sessionId);
      record("ipc-reply", {
        hasSessionId: !!(envelope && envelope.sessionId),
        rid: envelope && envelope.rid,
        sourceOrigin: envelope && envelope.sourceOrigin,
        responseType: response && response.t,
        id: response && response.id,
        hasCapability: !!(response && response.capability)
      });
      return reply.apply(this, arguments);
    })) {
    record("instrumentation-error", { message: "__niva_ipc_reply unavailable or cannot be safely wrapped" });
  }

  if (!wrapMethod(XMLHttpRequest.prototype, "open", xhrOpen => function (method, url, async = true) {
      record("xhr-open", { method: String(method), url: safeUrl(url), async: async !== false });
      return xhrOpen.apply(this, arguments);
    })) {
    record("instrumentation-error", { message: "XMLHttpRequest.prototype.open cannot be safely wrapped" });
  }
})();
