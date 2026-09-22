(function () {
  // Disable drag and open file
  window.addEventListener("dragover", function (ev) { ev.preventDefault(); }, false);

  var Niva = {};

  // === Event ===
  var eventListeners = {};

  function addEventListener(event, listener) {
    if (!eventListeners[event]) {
      eventListeners[event] = [];
    }
    eventListeners[event].push(listener);
  }

  function removeEventListener(event, listener) {
    if (!eventListeners[event]) {
      return;
    }
    var listeners = eventListeners[event];
    var newListeners = [];
    for (var i = 0; i < listeners.length; i++) {
      if (listeners[i] !== listener) {
        newListeners.push(listeners[i]);
      }
    }
    eventListeners[event] = newListeners;
  }

  function removeAllEventListeners(event) {
    if (!eventListeners[event]) {
      return;
    }
    eventListeners[event] = [];
  }

  function emit(event, data) {
    setTimeout(function () {
      var keys = [event, event.split('.')[0] + '.*', '*'];

      for (var i = 0; i < keys.length; i++) {
        var key = keys[i];

        if (eventListeners[key]) {
          var listeners = eventListeners[key];
          for (var j = 0; j < listeners.length; j++) {
            listeners[j](event, data);
          }
        }
      }

    }, 0);
  }

  Niva.addEventListener = addEventListener;
  Niva.removeEventListener = removeEventListener;
  Niva.removeAllEventListeners = removeAllEventListeners;
  EventListener = removeEventListener;
  Niva.__emit__ = emit;

  // === API calls over our own WebSocket (wire protocol, see docs/bridge.md) ===
  // The native side prepends a bootstrap snippet defining:
  //   window.__niva_ws_url / window.__niva_window_id / window.__niva_token
  Niva.bridgeVersion = 1;
  //
  // Text frames are JSON objects:
  //   {t:"call", id, method, args}      client -> server
  //   {t:"result", id, code, message, data}  server -> client (terminal)
  //   {t:"event", id?, seq, name, data}      server -> client (stream push)
  //   {t:"cancel", id}                  client -> server
  // Binary frames: [ver=1][flags][id u64 BE][seq u64 BE][payload]
  //   flags: 0x01 START, 0x02 END
  var getNextCallbackId = (function () {
    var callbackId = 0;
    return function () {
      if (callbackId >= Number.MAX_SAFE_INTEGER) {
        callbackId = 0;
      }
      return ++callbackId;
    }
  })();

  var pendings = {};
  var sendQueue = [];
  var socket = null;

  function flushQueue() {
    if (socket && socket.readyState === WebSocket.OPEN) {
      while (sendQueue.length > 0) {
        socket.send(sendQueue.shift());
      }
    }
  }

  function handleTextMessage(msg) {
    if (!msg || typeof msg !== "object") {
      return;
    }
    if (msg.t === "result") {
      var pending = pendings[msg.id];
      if (pending) {
        delete pendings[msg.id];
        if (msg.code === 0) {
          pending.resolve(msg.data);
        } else {
          pending.reject(msg);
        }
      }
    } else if (msg.t === "event") {
      var target = (msg.id != null) ? pendings[msg.id] : null;
      if (target && target.onEvent) {
        target.onEvent(msg.name, msg.data);
      } else {
        emit(msg.name, msg.data);
      }
    }
  }

  function handleBinaryMessage(buffer) {
    var view = new DataView(buffer);
    if (view.byteLength < 18 || view.getUint8(0) !== 1) {
      return;
    }
    var flags = view.getUint8(1);
    var idHigh = view.getUint32(2);
    var idLow = view.getUint32(6);
    var seqHigh = view.getUint32(10);
    var seqLow = view.getUint32(14);
    // ids stay within 2^53 on this bridge; combine for map keys
    var id = idHigh * 4294967296 + idLow;
    var seq = seqHigh * 4294967296 + seqLow;
    var pending = pendings[id];
    if (!pending) {
      return;
    }
    // Groups are keyed by sub-stream (stderr flag): pipes interleave chunks
    // but each END flushes only its own group.
    var isStderr = (flags & 0x04) !== 0;
    pending.groups = pending.groups || new Map();
    var group = pending.groups.get(isStderr);
    if (!group) {
      group = new Map();
      pending.groups.set(isStderr, group);
    }
    group.set(seq, buffer.slice(18));
    if (flags & 0x02) {
      // END: concatenate in seq order and flush one blob per group
      var keys = Array.from(group.keys()).sort(function (a, b) { return a - b; });
      var parts = keys.map(function (k) { return group.get(k); });
      pending.groups.delete(isStderr);
      if (pending.onBlob) {
        pending.onBlob(new Blob(parts), isStderr);
      }
    }
  }

  function connectSocket() {
    var url = window.__niva_ws_url;
    if (!url || typeof WebSocket === 'undefined') {
      return;
    }
    try {
      socket = new WebSocket(url + "?token=" + encodeURIComponent(window.__niva_token || ""));
    } catch (e) {
      socket = null;
      return;
    }
    socket.binaryType = "arraybuffer";
    socket.addEventListener("open", function () {
      try {
        socket.send(JSON.stringify({ t: "hello", wid: window.__niva_window_id || 0, v: 1 }));
      } catch (e) {}
      flushQueue();
      flushBinQueue();
    });
    socket.addEventListener("message", function (ev) {
      try {
        if (typeof ev.data === "string") {
          handleTextMessage(JSON.parse(ev.data));
        } else if (ev.data instanceof ArrayBuffer) {
          handleBinaryMessage(ev.data);
        }
      } catch (e) {}
    });
    socket.addEventListener("close", function () {
      socket = null;
      // Reconnect (e.g. dev-server HMR should not kill the bridge).
      setTimeout(connectSocket, 1000);
    });
  }

  function trackPending(id, entry) {
    pendings[id] = entry;
    return entry;
  }

  // Unary call: same promise behavior as before.
  function call(method, args) {
    var stream = streamCall(method, args, {});
    return stream.promise;
  }

  // Stateful call: { promise, cancel }. onEvent receives stream pushes tied
  // to this call id; onBlob receives one Blob per END-terminated group.
  function streamCall(method, args, handlers) {
    handlers = handlers || {};
    var callbackId = getNextCallbackId();
    var pending = trackPending(callbackId, { groups: new Map(), seqOut: 0 });
    var promise = new Promise(function (resolve, reject) {
      pending.resolve = resolve;
      pending.reject = reject;
    });
    pending.onEvent = handlers.onEvent;
    pending.onBlob = handlers.onBlob;
    sendQueue.push(JSON.stringify({ t: "call", id: callbackId, method: method, args: args }));
    flushQueue();
    return {
      id: callbackId,
      promise: promise,
      cancel: function () {
        delete pendings[callbackId];
        binQueue = binQueue.filter(function (item) { return item.id !== callbackId; });
        sendQueue.push(JSON.stringify({ t: "cancel", id: callbackId }));
        flushQueue();
      },
    };
  }

  // Binary frames queue alongside text when the socket is still opening.
  var binQueue = [];

  function flushBinQueue() {
    if (socket && socket.readyState === WebSocket.OPEN) {
      while (binQueue.length > 0) {
        var item = binQueue.shift();
        if (pendings[item.id]) {
          try {
            socket.send(item.frame);
          } catch (e) {}
        }
      }
    }
  }

  // Send a binary chunk for a stateful call id (stdin-style input).
  // data: ArrayBuffer | Uint8Array | string. Queued while connecting.
  function streamSend(id, data, end) {
    var pending = pendings[id];
    if (!pending) {
      return false;
    }
    var bytes;
    if (data instanceof ArrayBuffer) {
      bytes = new Uint8Array(data);
    } else if (data instanceof Uint8Array) {
      bytes = data;
    } else {
      bytes = new TextEncoder().encode(String(data));
    }
    pending.seqOut = (pending.seqOut || 0) + 1;
    var frame = new Uint8Array(18 + bytes.length);
    frame[0] = 1;
    var flags = 0;
    if (pending.seqOut === 1) {
      flags |= 0x01;
    }
    if (end) {
      flags |= 0x02;
    }
    frame[1] = flags;
    var view = new DataView(frame.buffer);
    view.setUint32(2, Math.floor(id / 4294967296));
    view.setUint32(6, id % 4294967296);
    view.setUint32(10, Math.floor(pending.seqOut / 4294967296));
    view.setUint32(14, pending.seqOut % 4294967296);
    frame.set(bytes, 18);
    if (socket && socket.readyState === WebSocket.OPEN) {
      try {
        socket.send(frame.buffer);
        return true;
      } catch (e) {
        return false;
      }
    }
    binQueue.push({ id: id, frame: frame.buffer });
    return true;
  }

  // High-level overrides: streaming-native implementations behind the
  // exact legacy signatures (devtools and user projects call these).
  var apiOverrides = {};
  function overrideApi(namespace, method, fn) {
    if (!apiOverrides[namespace]) {
      apiOverrides[namespace] = {};
    }
    apiOverrides[namespace][method] = fn;
  }

  var namespaceCache = {};
  if (typeof Proxy !== 'undefined') {
    Niva.api = new Proxy({}, {
      get: function (_, namespace) {
        if (!namespaceCache[namespace]) {
          namespaceCache[namespace] = new Proxy({}, {
            get: function (_, method) {
              var overrides = apiOverrides[namespace] || {};
              if (overrides[method]) {
                return overrides[method];
              }
              return function () {
                return Niva.call(namespace + '.' + method, Array.prototype.slice.call(arguments))
              }
            }
          });
        }
        return namespaceCache[namespace];
      }
    });
  } else {
    console.log('Proxy not supported, please use Niva.call instead');
  }

  // Collect one stream call into { result, blob } (blobs concatenated).
  function collectStream(method, args, handlers) {
    handlers = handlers || {};
    return new Promise(function (resolve, reject) {
      var blobs = [];
      var st = streamCall(method, args, {
        onEvent: handlers.onEvent,
        onBlob: function (blob) {
          blobs.push(blob);
        },
      });
      st.promise.then(function (res) {
        resolve({ res: res, blob: new Blob(blobs) });
      }, reject);
    });
  }

  function bytesToBase64(bytes) {
    var binary = "";
    var chunk = 32768;
    for (var i = 0; i < bytes.length; i += chunk) {
      binary += String.fromCharCode.apply(null, bytes.subarray(i, i + chunk));
    }
    return btoa(binary);
  }

  function base64ToBytes(b64) {
    var binary = atob(b64);
    var bytes = new Uint8Array(binary.length);
    for (var i = 0; i < binary.length; i++) {
      bytes[i] = binary.charCodeAt(i);
    }
    return bytes;
  }

  function sendAll(st, bytes, chunkSize) {
    chunkSize = chunkSize || 65536;
    for (var offset = 0; offset < bytes.length; offset += chunkSize) {
      var end = offset + chunkSize >= bytes.length;
      streamSend(st.id, bytes.subarray(offset, offset + chunkSize), end);
    }
    if (bytes.length === 0) {
      streamSend(st.id, new Uint8Array(0), true);
    }
  }

  // fs.read/write/append over fs.readStream/fs.writeStream.
  overrideApi("fs", "read", function (path, encode) {
    return collectStream("fs.readStream", [path]).then(function (out) {
      if ((encode || "utf8").toLowerCase() === "base64") {
        return out.blob.arrayBuffer().then(function (buf) {
          return bytesToBase64(new Uint8Array(buf));
        });
      }
      return out.blob.text();
    });
  });

  overrideApi("fs", "write", function (path, content, encode) {
    var bytes = (encode && encode.toLowerCase() === "base64")
      ? base64ToBytes(content || "")
      : new TextEncoder().encode(content || "");
    var st = Niva.stream("fs.writeStream", [path], {});
    sendAll(st, bytes);
    return st.promise.then(function () { return null; });
  });

  overrideApi("fs", "append", function (path, content, encode) {
    var bytes = (encode && encode.toLowerCase() === "base64")
      ? base64ToBytes(content || "")
      : new TextEncoder().encode(content || "");
    var st = Niva.stream("fs.writeStream", [path, true], {});
    sendAll(st, bytes);
    return st.promise.then(function () { return null; });
  });

  // http.get/post/request over http.requestStream.
  function httpStreamCollect(options) {
    var head = null;
    return collectStream("http.requestStream", [options], {
      onEvent: function (name, data) {
        if (name === "head") {
          head = data;
        }
      },
    }).then(function (out) {
      return out.blob.text().then(function (body) {
        return { status: head.status, headers: head.headers, body: body };
      });
    });
  }

  overrideApi("http", "get", function (url, headers) {
    return httpStreamCollect({ method: "GET", url: url, headers: headers || null });
  });

  overrideApi("http", "post", function (url, body, headers) {
    return httpStreamCollect({ method: "POST", url: url, body: body, headers: headers || null });
  });

  overrideApi("http", "request", function (options) {
    return httpStreamCollect(options);
  });

  // process.exec over process.execStream (byte-exact stdout/stderr).
  overrideApi("process", "exec", function (cmd, args, options) {
    options = options || {};
    if (options.detached) {
      return Niva.call("process.execStream", [cmd, args || null, options]);
    }
    var out = { stdout: [], stderr: [] };
    var st = Niva.stream("process.execStream", [cmd, args || null, options], {
      onBlob: function (blob, isStderr) {
        (isStderr ? out.stderr : out.stdout).push(blob);
      },
    });
    function blobsToText(blobs) {
      return Promise.all(blobs.map(function (b) { return b.text(); }))
        .then(function (parts) { return parts.join(""); });
    }
    return st.promise.then(function (res) {
      return blobsToText(out.stdout).then(function (stdout) {
        return blobsToText(out.stderr).then(function (stderr) {
          return { status: res.status, stdout: stdout, stderr: stderr };
        });
      });
    });
  });

  // resource.read over resource.readStream.
  overrideApi("resource", "read", function (path, encode) {
    return collectStream("resource.readStream", [path]).then(function (out) {
      if ((encode || "utf8").toLowerCase() === "base64") {
        return out.blob.arrayBuffer().then(function (buf) {
          return bytesToBase64(new Uint8Array(buf));
        });
      }
      return out.blob.text();
    });
  });

  Niva.call = call;
  Niva.stream = streamCall;
  Niva.streamSend = streamSend;

  // === Modules: require / registerModule / Niva.import ===
  //
  // `require(id)` is a synchronous registry lookup (CJS shape). Native
  // namespaces are pre-registered under the `niva:` prefix; userland
  // (e.g. the standalone node-compat package, or inline shims) adds more
  // via Niva.registerModule. Unknown ids throw.
  //
  // NOTE on ESM `import`: static import syntax is resolved by the browser
  // before any runtime code runs, so it cannot be shimmed. Single-file
  // pages use require()/Niva.import(); bundled projects (Vite/webpack)
  // consume the node-compat npm package instead, which bundlers resolve
  // natively (same pattern as Tauri's @tauri-apps/api).
  var moduleRegistry = {};

  function builtinNamespaces() {
    // Snapshot the live Niva.api namespaces (overrides included).
    var out = {};
    ["fs", "http", "process", "os", "resource", "dialog", "window",
     "clipboard", "tray", "shortcut", "monitor", "webview", "extra",
     "windowExtra"].forEach(function (ns) {
      try {
        out[ns] = Niva.api[ns];
      } catch (e) {}
    });
    return out;
  }

  Niva.registerModule = function (id, impl) {
    moduleRegistry[id] = impl;
  };

  function nivaRequire(id) {
    if (Object.prototype.hasOwnProperty.call(moduleRegistry, id)) {
      return moduleRegistry[id];
    }
    if (id.indexOf("niva:") === 0) {
      var ns = id.slice(5);
      var namespaces = builtinNamespaces();
      if (namespaces[ns]) {
        return namespaces[ns];
      }
    }
    throw new Error(
      "niva: unknown module '" + id + "'. " +
      "Available: niva:<namespace> (fs, os, ...) or modules added via Niva.registerModule(). " +
      "For Node-shaped APIs, load the node-compat package."
    );
  }

  Niva.require = nivaRequire;
  if (typeof window.require === "undefined") {
    window.require = nivaRequire;
  }

  // Async module shape for single-file pages: `await Niva.import(id)`.
  Niva.import = function (id) {
    return Promise.resolve().then(function () {
      return nivaRequire(id);
    });
  };

  delete window.close;
  delete window.open;

  // === Tauri API ===
  window.Niva = Niva;
  console.log('Niva loaded');

  connectSocket();

  (function docReady(func) {
    if (document.readyState === "complete" || document.readyState === "interactive") {
      setTimeout(func);
    } else {
      document.addEventListener("DOMContentLoaded", func);
    }
  })(function () {
  });

}());
