(function (root) {
  "use strict";

  var runtime = root[Symbol.for("niva.node-compat.runtime")];
  if (typeof runtime.createNetModule === "function") return;

  function createNetModule(niva) {
    var vendor = runtime.vendor;
    var Buffer = vendor && vendor.Buffer;
    var Duplex = vendor && vendor.stream && vendor.stream.Duplex;
    var EventEmitter = runtime.events && runtime.events.EventEmitter;
    if (!Buffer || !Duplex || !EventEmitter) throw new Error("Load vendor, Buffer, and events before net.");

    function bridgeStream(method, args, handlers) {
      return runtime.stream(niva, method, args, handlers || {});
    }

    function sendChunk(nivaTarget, id, value, end) {
      var target = runtime.resolveNiva(nivaTarget === undefined ? niva : nivaTarget);
      if (typeof target.streamSend !== "function") throw runtime.bridgeError("Niva.streamSend is unavailable", "ERR_METHOD_NOT_IMPLEMENTED");
      return target.streamSend(id, value, !!end);
    }

    function control(nivaTarget, socketId, action, extra) {
      var target = runtime.resolveNiva(nivaTarget === undefined ? niva : nivaTarget);
      return runtime.stream(target, "socket.control", [Object.assign({ socketId: socketId, action: action }, extra || {})], {}).promise;
    }

    function validatePort(port, allowZero) {
      if (typeof port === "string" && port.trim() !== "") port = Number(port);
      if (!Number.isInteger(port) || port < (allowZero ? 0 : 1) || port > 65535) {
        var error = new RangeError("The \"port\" argument must be an integer between " + (allowZero ? "0" : "1") + " and 65535");
        error.code = "ERR_SOCKET_BAD_PORT";
        throw error;
      }
      return port;
    }

    function normalizeHost(host) {
      if (host === undefined || host === null || host === "") return "127.0.0.1";
      if (typeof host !== "string") throw new TypeError("The \"host\" argument must be of type string");
      return host.replace(/^\[|\]$/g, "");
    }

    function socketError(message, code) {
      return runtime.bridgeError(message, code || "ERR_SOCKET");
    }

    class Socket extends Duplex {
      constructor(options) {
        options = options || {};
        var allowHalfOpen = options.allowHalfOpen === true;
        if (options.secure && allowHalfOpen) {
          throw runtime.bridgeError("TLS allowHalfOpen is not supported because Native TLS shutdown ends the session", "ENOTSUP");
        }
        super({
          allowHalfOpen: allowHalfOpen,
          readableHighWaterMark: options.readableHighWaterMark,
          writableHighWaterMark: options.writableHighWaterMark,
          autoDestroy: true,
        });
        this.allowHalfOpen = allowHalfOpen;
        this.connecting = false;
        this.pending = false;
        this.bytesRead = 0;
        this.bytesWritten = 0;
        this.remoteAddress = undefined;
        this.remotePort = undefined;
        this.remoteFamily = undefined;
        this.localAddress = undefined;
        this.localPort = undefined;
        this._niva = options.niva === undefined ? niva : options.niva;
        this._socketId = undefined;
        this._bridgeCall = undefined;
        this._bridgeMethod = undefined;
        this._connectEvent = "connect";
        this._connected = false;
        this._remoteClosed = false;
        this._manualPaused = false;
        this._unackedReadBytes = 0;
        this._writeSequence = 0;
        this._writeCallbacks = new Map();
        this._waitingWrite = null;
        this._waitingFinal = null;
        this._timeout = 0;
        this._timeoutId = undefined;
        this._hadError = false;
        this._secure = !!options.secure;
        this.authorized = this._secure ? true : undefined;
        this.authorizationError = undefined;
        this.encrypted = this._secure;
        this._onReady = options.onReady;
      }

      get readyState() {
        if (this.destroyed || this._remoteClosed) return "closed";
        if (this.connecting) return "opening";
        if (!this._connected) return "closed";
        if (this.writableEnded) return "readOnly";
        if (this.readableEnded) return "writeOnly";
        return "open";
      }

      get bufferSize() { return this.writableLength || 0; }

      connect() {
        var parsed = parseConnectArgs(arguments);
        if (parsed.callback) this.once(this._connectEvent, parsed.callback);
        return this._start(parsed.method || "socket.tcpConnect", parsed.args, parsed.event || this._connectEvent);
      }

      _start(method, args, readyEvent) {
        if (this._bridgeCall) throw socketError("Socket is already connecting", "ERR_SOCKET_DGRAM_IS_CONNECTED");
        this.connecting = true;
        this.pending = true;
        this._bridgeMethod = method;
        this._connectEvent = readyEvent || "connect";
        var self = this;
        try {
          this._bridgeCall = bridgeStream(method, args, {
            onEvent: function (name, data) { self._onBridgeEvent(name, data); },
            onChunk: function (chunk, stderr) { if (!stderr) self._onBridgeChunk(chunk); },
          });
        } catch (error) {
          this.connecting = false;
          this.pending = false;
          this.destroy(runtime.nativeError(error));
          return this;
        }
        this._bridgeCall.promise.then(function () {
          if (!self._remoteClosed) self._onBridgeClose(false);
        }, function (error) {
          self._hadError = true;
          self.destroy(runtime.nativeError(error));
        });
        return this;
      }

      _attach(socketId, secure, onReady) {
        this._secure = !!secure;
        this.encrypted = !!secure;
        this.authorized = secure ? true : undefined;
        this._onReady = onReady;
        return this._start(secure ? "socket.tlsAttach" : "socket.tcpAttach", [{ socketId: socketId }], secure ? "secureConnect" : "connect");
      }

      _onBridgeEvent(name, data) {
        data = data || {};
        if ((name === "connect" || name === "secureConnect") && data.socketId) {
          this._socketId = data.socketId;
          this.localAddress = data.localAddress;
          this.localPort = data.localPort;
          this.remoteAddress = data.remoteAddress;
          this.remotePort = data.remotePort;
          this.remoteFamily = String(data.remoteAddress || "").indexOf(":") >= 0 ? "IPv6" : "IPv4";
          this.connecting = false;
          this.pending = false;
          this._connected = true;
          this._armTimeout();
          if (name === "secureConnect") this.emit("connect");
          this.emit(name);
          if (typeof this._onReady === "function") {
            var onReady = this._onReady;
            this._onReady = undefined;
            onReady(this);
          }
          this._flushWaitingWrite();
          return;
        }
        if (name === "writeAck") {
          var callback = this._writeCallbacks.get(data.seq);
          if (callback) {
            this._writeCallbacks.delete(data.seq);
            this.bytesWritten += Number(data.bytes) || 0;
            callback(null);
          }
          if (Number(data.bytes) > 0) this._armTimeout();
          return;
        }
        if (name === "end") {
          this._onRemoteEnd();
          return;
        }
        if (name === "close") {
          this._onBridgeClose(!!data.hadError);
          return;
        }
        if (name === "timeout") this.emit("timeout");
      }

      _onBridgeChunk(chunk) {
        if (this._remoteClosed || !chunk || chunk.byteLength === 0) return;
        this.bytesRead += chunk.byteLength;
        var bytes = Buffer.from(chunk);
        this._armTimeout();
        this._unackedReadBytes += bytes.length;
        this.push(bytes);
      }

      _onBridgeClose(hadError) {
        if (this._remoteClosed) return;
        this._remoteClosed = true;
        this._hadError = this._hadError || hadError;
        this.emit("_nativeClose", this._hadError);
        this.connecting = false;
        this.pending = false;
        this._clearTimeout();
        if (!this._readableState.ended) this.push(null);
        if (!this.writableEnded) this.end();
        this._failPendingWrites(socketError("Socket closed", "ECONNRESET"));
        var self = this;
        function finishClose() {
          if (self._remoteClosed && self.readableEnded && self.writableFinished && !self.destroyed) self.destroy();
        }
        this.once("end", finishClose);
        this.once("finish", finishClose);
        finishClose();
      }

      _onRemoteEnd() {
        if (this._remoteEnded || this.destroyed) return;
        this._remoteEnded = true;
        if (!this._readableState.ended) this.push(null);
        if (!this.allowHalfOpen && !this.writableEnded) this.end();
      }

      _failPendingWrites(error) {
        if (this._waitingWrite) {
          var waiting = this._waitingWrite;
          this._waitingWrite = null;
          waiting.callback(error);
        }
        if (this._waitingFinal) {
          var finalCallback = this._waitingFinal;
          this._waitingFinal = null;
          finalCallback(error);
        }
        this._writeCallbacks.forEach(function (callback) { callback(error); });
        this._writeCallbacks.clear();
      }

      _flushWaitingWrite() {
        if (!this._connected || this._remoteClosed) return;
        var waiting = this._waitingWrite;
        if (waiting) {
          this._waitingWrite = null;
          this._sendWrite(waiting.chunk, waiting.callback);
          return;
        }
        if (this._waitingFinal) {
          var callback = this._waitingFinal;
          this._waitingFinal = null;
          this._sendEnd(callback);
        }
      }

      _sendWrite(chunk, callback) {
        if (!this._bridgeCall || this._remoteClosed) { callback(socketError("Socket is not writable", "ERR_STREAM_DESTROYED")); return; }
        var sequence = ++this._writeSequence;
        this._writeCallbacks.set(sequence, callback);
        try {
          if (!sendChunk(this._niva, this._bridgeCall.id, Buffer.from(chunk), false)) {
            this._writeCallbacks.delete(sequence);
            callback(socketError("Socket write could not be queued", "ERR_STREAM_DESTROYED"));
          }
        } catch (error) {
          this._writeCallbacks.delete(sequence);
          callback(error);
        }
      }

      _sendEnd(callback) {
        if (this._remoteClosed) { callback(); return; }
        if (!this._bridgeCall) { this._waitingFinal = callback; return; }
        var sequence = ++this._writeSequence;
        this._writeCallbacks.set(sequence, callback);
        try {
          if (!sendChunk(this._niva, this._bridgeCall.id, new Uint8Array(0), true)) {
            this._writeCallbacks.delete(sequence);
            callback(socketError("Socket shutdown could not be queued", "ERR_STREAM_DESTROYED"));
          }
        } catch (error) {
          this._writeCallbacks.delete(sequence);
          callback(error);
        }
      }

      _read() {
        if (this._manualPaused && this._socketId) {
          this._manualPaused = false;
          control(this._niva, this._socketId, "resumeRead").catch(function () {});
        }
      }

      read(size) {
        var wasFlowing = this._readableState && this._readableState.flowing;
        var chunk = Duplex.prototype.read.call(this, size);
        if (chunk !== null && !wasFlowing) {
          var byteCount = typeof chunk === "string" ? Buffer.byteLength(chunk, this.readableEncoding || "utf8") : chunk.byteLength;
          this._ackRead(byteCount);
        }
        return chunk;
      }

      emit(event) {
        var args = arguments;
        var result;
        try { result = Duplex.prototype.emit.apply(this, args); }
        finally {
          if (event === "data" && args[1] !== undefined) {
            var chunk = args[1];
            var byteCount = typeof chunk === "string" ? Buffer.byteLength(chunk, this.readableEncoding || "utf8") : chunk.byteLength;
            this._ackRead(byteCount);
          }
        }
        return result;
      }

      _ackRead(bytes) {
        if (!this._socketId || !Number.isFinite(bytes) || bytes <= 0 || this._unackedReadBytes <= 0) return;
        var ack = Math.min(Math.trunc(bytes), this._unackedReadBytes);
        if (ack <= 0) return;
        this._unackedReadBytes -= ack;
        var self = this;
        control(this._niva, this._socketId, "readAck", { bytes: ack }).catch(function (error) { self.destroy(runtime.nativeError(error)); });
      }

      _write(chunk, encoding, callback) {
        if (this._remoteClosed) { callback(socketError("Socket is not writable", "ERR_STREAM_DESTROYED")); return; }
        if (!this._connected) { this._waitingWrite = { chunk: Buffer.from(chunk), callback: callback }; return; }
        this._sendWrite(chunk, callback);
      }

      _final(callback) {
        if (this._remoteClosed) { callback(); return; }
        if (!this._connected) { this._waitingFinal = callback; return; }
        this._sendEnd(callback);
      }

      _destroy(error, callback) {
        this._clearTimeout();
        this._failPendingWrites(error || socketError("Socket closed", "ERR_STREAM_DESTROYED"));
        var call = this._bridgeCall;
        this._bridgeCall = undefined;
        if (call && !this._remoteClosed && typeof call.cancel === "function") call.cancel();
        callback(error);
      }

      pause() {
        Duplex.prototype.pause.call(this);
        if (!this._manualPaused) {
          this._manualPaused = true;
          if (this._socketId) control(this._niva, this._socketId, "pauseRead").catch(function () {});
        }
        return this;
      }

      resume() {
        Duplex.prototype.resume.call(this);
        if (this._manualPaused) {
          this._manualPaused = false;
          if (this._socketId) control(this._niva, this._socketId, "resumeRead").catch(function () {});
        }
        return this;
      }

      setTimeout(timeout, callback) {
        timeout = Number(timeout) || 0;
        this._timeout = timeout;
        if (callback) this.once("timeout", callback);
        if (this._connected) this._armTimeout();
        return this;
      }

      setNoDelay(noDelay) {
        throw runtime.bridgeError("TCP_NODELAY is not supported by the Native socket bridge", "ENOTSUP");
      }

      setKeepAlive(enable, initialDelay) {
        throw runtime.bridgeError("TCP keepalive is not supported by the Native socket bridge", "ENOTSUP");
      }

      ref() { this._refed = true; return this; }
      unref() { this._refed = false; return this; }
      hasRef() { return this._refed !== false; }

      address() {
        if (this._socketId && this.localAddress !== undefined) {
          return { address: this.localAddress, family: String(this.localAddress).indexOf(":") >= 0 ? "IPv6" : "IPv4", port: this.localPort };
        }
        return null;
      }

      _armTimeout() {
        this._clearTimeout();
        if (this._timeout > 0) {
          var self = this;
          this._timeoutId = root.setTimeout(function () { self._timeoutId = undefined; self.emit("timeout"); }, this._timeout);
        }
      }

      _clearTimeout() {
        if (this._timeoutId !== undefined) root.clearTimeout(this._timeoutId);
        this._timeoutId = undefined;
      }
    }

    function parseConnectArgs(args, method, eventName) {
      var values = Array.prototype.slice.call(args);
      var callback = typeof values[values.length - 1] === "function" ? values.pop() : undefined;
      var first = values[0];
      var options;
      if (first && typeof first === "object") options = Object.assign({}, first);
      else if (typeof first === "number" || typeof first === "string" && /^\d+$/.test(first)) options = { port: Number(first), host: typeof values[1] === "string" ? values[1] : undefined };
      else if (typeof first === "string") options = { path: first };
      else options = {};
      if (options.path || options.socketPath) throw socketError("Unix domain sockets are not available in WebView", "ENOTSUP");
      if (options.localAddress || options.localPort) throw socketError("Binding the client source address is unsupported", "ENOTSUP");
      var port = validatePort(options.port, false);
      var host = normalizeHost(options.host || options.hostname);
      var nativeArgs = { host: host, port: port };
      if (options.connectTimeoutMs !== undefined) nativeArgs.timeoutMs = Number(options.connectTimeoutMs);
      return { args: [nativeArgs], callback: callback, method: method, event: eventName, options: options };
    }

    function connect() {
      var parsed = parseConnectArgs(arguments, "socket.tcpConnect", "connect");
      var socket = new Socket(parsed.options);
      if (parsed.callback) socket.once("connect", parsed.callback);
      socket._start(parsed.method, parsed.args, parsed.event);
      if (parsed.options.timeout) socket.setTimeout(parsed.options.timeout);
      return socket;
    }

    function connectGuarded(options) {
      options = Object.assign({}, options || {});
      if (!options.host) options.host = options.hostname;
      var parsed = parseConnectArgs([options], "socket.tcpConnectGuarded", "connect");
      parsed.args[0].scheme = options.scheme || "http";
      var socket = new Socket(options);
      socket._start(parsed.method, parsed.args, parsed.event);
      if (options.timeout) socket.setTimeout(options.timeout);
      return socket;
    }

    class Server extends EventEmitter {
      constructor(options, listener, secure) {
        super();
        this._options = options || {};
        this._secure = !!secure;
        this._tlsIdentity = this._secure ? this._options.identity : undefined;
        this._listenerCall = undefined;
        this._socketId = undefined;
        this._address = null;
        this._sockets = new Set();
        this._pendingAttaches = 0;
        this._closing = false;
        this.listening = false;
        this.maxConnections = this._options.maxConnections || Infinity;
        if (listener) this.on(this._secure ? "secureConnection" : "connection", listener);
      }

      listen() {
        var parsed = parseListenArgs(arguments, this._secure, this._tlsIdentity);
        if (parsed.callback) this.once("listening", parsed.callback);
        if (this.listening || this._listenerCall) throw socketError("Server is already listening", "ERR_SERVER_ALREADY_LISTEN");
        this._options = Object.assign({}, this._options, parsed.options);
        var self = this;
        var method = this._secure ? "socket.tlsListen" : "socket.tcpListen";
        this._listenerCall = bridgeStream(method, [parsed.nativeArgs], {
          onEvent: function (name, data) { self._onListenerEvent(name, data); },
        });
        this._listenerCall.promise.then(function () {
          self.listening = false;
          self._maybeCloseEvent();
        }, function (error) {
          if (!self._closing) self.emit("error", runtime.nativeError(error));
          self.listening = false;
          self._maybeCloseEvent();
        });
        return this;
      }

      _onListenerEvent(name, data) {
        data = data || {};
        if (name === "listening") {
          this._socketId = data.socketId;
          this._address = { address: data.address, family: String(data.address).indexOf(":") >= 0 ? "IPv6" : "IPv4", port: data.port };
          this.listening = true;
          this.emit("listening");
          return;
        }
        if (name === "connection") {
          this._attachConnection(data.socketId);
          return;
        }
        if (name === "connectionError") {
          var error = socketError("Native socket accept queue is full", data.code || "ERR_SOCKET_ACCEPT_QUEUE_FULL");
          this.emit("clientError", error);
          return;
        }
        if (name === "close") {
          this.listening = false;
          this._maybeCloseEvent();
        }
      }

      _attachConnection(socketId) {
        var self = this;
        this._pendingAttaches += 1;
        var socket = new Socket({ niva: niva, secure: this._secure, allowHalfOpen: this._options.allowHalfOpen === true });
        var readyEvent = this._secure ? "secureConnect" : "connect";
        var settled = false;
        function settle() {
          if (settled) return;
          settled = true;
          self._pendingAttaches -= 1;
        }
        socket.once(readyEvent, function () {
          settle();
          self._sockets.add(socket);
          socket.once("close", function () {
            self._sockets.delete(socket);
            self._maybeCloseEvent();
          });
          if (self._secure) self.emit("secureConnection", socket);
          else self.emit("connection", socket);
          if (self._closing) socket.destroy();
        });
        socket.once("error", function (error) {
          settle();
          if (self.listenerCount("clientError")) self.emit("clientError", error, socket);
        });
        socket.once("close", function () { settle(); });
        socket.once("_nativeClose", function () {
          settle();
          self._sockets.delete(socket);
          self._maybeCloseEvent();
        });
        socket._attach(socketId, this._secure);
      }

      address() { return this._address; }
      ref() { this._refed = true; return this; }
      unref() { this._refed = false; return this; }
      hasRef() { return this._refed !== false; }

      close(callback) {
        if (callback) this.once("close", callback);
        if (!this.listening && !this._listenerCall) {
          var error = socketError("Server is not running", "ERR_SERVER_NOT_RUNNING");
          if (callback) { enqueue(function () { callback(error); }); return this; }
          throw error;
        }
        this._closing = true;
        if (this._socketId) control(niva, this._socketId, "pauseRead").catch(function () {});
        this._maybeCloseEvent();
        return this;
      }

      _maybeCloseEvent() {
        if (!this._closing || this._sockets.size || this._pendingAttaches) return;
        var call = this._listenerCall;
        this._listenerCall = undefined;
        this._socketId = undefined;
        this._address = null;
        this.listening = false;
        if (call && typeof call.cancel === "function") call.cancel();
        if (!this._closeEmitted) {
          this._closeEmitted = true;
          enqueue(() => this.emit("close"));
        }
      }

      getConnections(callback) {
        if (typeof callback !== "function") throw new TypeError("callback must be a function");
        enqueue(() => callback(null, this._sockets.size + this._pendingAttaches));
        return this;
      }
    }

    function parseListenArgs(args, secure, identity) {
      var values = Array.prototype.slice.call(args);
      var callback = typeof values[values.length - 1] === "function" ? values.pop() : undefined;
      var first = values[0];
      var options;
      if (first && typeof first === "object") options = Object.assign({}, first);
      else if (typeof first === "number" || typeof first === "string" && /^\d+$/.test(first)) {
        options = { port: Number(first), host: typeof values[1] === "string" ? values[1] : undefined };
      } else if (typeof first === "string") {
        throw socketError("Unix domain sockets are not available in WebView", "ENOTSUP");
      } else options = {};
      options.port = validatePort(options.port === undefined ? 0 : options.port, true);
      options.host = normalizeHost(options.host);
      var nativeArgs = { host: options.host, port: options.port };
      if (secure) {
        if (!identity) throw socketError("TLS server identity is required", "ERR_TLS_CERT_ALTNAME_INVALID");
        nativeArgs.identity = identity;
      }
      return { callback: callback, options: options, nativeArgs: nativeArgs };
    }

    function createServer(options, listener) {
      if (typeof options === "function") { listener = options; options = {}; }
      return new Server(options || {}, listener, false);
    }

    function createTlsServer(options, listener) {
      if (typeof options === "function") { listener = options; options = {}; }
      return new Server(options || {}, listener, true);
    }

    function isIPv4(value) {
      if (typeof value !== "string") return false;
      var parts = value.split(".");
      if (parts.length !== 4) return false;
      return parts.every(function (part) {
        if (!/^\d{1,3}$/.test(part) || part.length > 1 && part[0] === "0") return false;
        return Number(part) <= 255;
      });
    }
    function isIPv6(value) {
      if (typeof value !== "string" || !value) return false;
      var zoneIndex = value.indexOf("%");
      if (zoneIndex >= 0 && !/^[A-Za-z0-9_.%-]+$/.test(value.slice(zoneIndex + 1))) return false;
      var address = value.split("%")[0];
      if (!address || address.indexOf(":") < 0) return false;
      var compressed = address.indexOf("::");
      if (compressed !== address.lastIndexOf("::")) return false;
      var halves = compressed < 0 ? [address] : [address.slice(0, compressed), address.slice(compressed + 2)];
      var count = 0;
      for (var hi = 0; hi < halves.length; hi += 1) {
        if (!halves[hi]) continue;
        var groups = halves[hi].split(":");
        for (var gi = 0; gi < groups.length; gi += 1) {
          var group = groups[gi];
          if (group.indexOf(".") >= 0) {
            if (hi !== halves.length - 1 || gi !== groups.length - 1 || !isIPv4(group)) return false;
            count += 2;
          } else {
            if (!/^[0-9a-fA-F]{1,4}$/.test(group)) return false;
            count += 1;
          }
        }
      }
      return compressed < 0 ? count === 8 : count < 8;
    }

    var module = {
      Socket: Socket,
      Server: Server,
      connect: connect,
      createConnection: connect,
      connectGuarded: connectGuarded,
      createServer: createServer,
      createTlsServer: createTlsServer,
      isIP: function (input) { return isIPv4(input) ? 4 : isIPv6(input) ? 6 : 0; },
      isIPv4: isIPv4,
      isIPv6: isIPv6,
    };
    return module;
  }

  function enqueue(callback) {
    if (typeof root.queueMicrotask === "function") root.queueMicrotask(callback);
    else Promise.resolve().then(callback);
  }

  runtime.createNetModule = createNetModule;
  runtime.net = createNetModule();
})(globalThis);
