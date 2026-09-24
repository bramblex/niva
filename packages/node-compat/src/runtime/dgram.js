(function (root) {
  "use strict";

  var runtime = root[Symbol.for("niva.node-compat.runtime")];
  if (typeof runtime.createDgramModule === "function") return;

  function createDgramModule(niva) {
    var Buffer = runtime.vendor.Buffer;
    var EventEmitter = runtime.events.EventEmitter;

    function dgramError(message, code) { return runtime.bridgeError(message, code || "ERR_SOCKET_DGRAM_NOT_RUNNING"); }
    function callStream(method, args, handlers) { return runtime.stream(niva, method, args, handlers || {}); }
    function sendBytes(id, bytes) {
      var target = runtime.resolveNiva(niva);
      if (typeof target.streamSend !== "function") throw runtime.bridgeError("Niva.streamSend is unavailable", "ERR_METHOD_NOT_IMPLEMENTED");
      if (!target.streamSend(id, bytes, true)) throw dgramError("UDP datagram could not be queued", "ERR_SOCKET_DGRAM_NOT_RUNNING");
    }
    function control(socketId, action, extra) {
      return callStream("socket.control", [Object.assign({ socketId: socketId, action: action }, extra || {})], {}).promise;
    }
    function enqueue(callback) {
      if (typeof root.queueMicrotask === "function") root.queueMicrotask(callback);
      else Promise.resolve().then(callback);
    }

    class Socket extends EventEmitter {
      constructor(options, callback) {
        super();
        if (typeof options === "string") options = { type: options };
        options = options || {};
        if (options === null || typeof options !== "object") throw new TypeError("options must be an object or socket type string");
        this.type = options.type || "udp4";
        if (this.type !== "udp4" && this.type !== "udp6") throw dgramError("Unsupported socket type", "ERR_SOCKET_BAD_TYPE");
        if (options.reuseAddr || options.ipv6Only || options.recvBufferSize || options.sendBufferSize) {
          throw runtime.bridgeError("Requested UDP socket options are not supported", "ENOTSUP");
        }
        this._socketId = undefined;
        this._bindCall = undefined;
        this._bound = false;
        this._closed = false;
        this._localAddress = undefined;
        this._localPort = undefined;
        this._pendingDatagram = undefined;
        this._closing = false;
        this._closeSent = false;
        this._closeEmitted = false;
        this._refed = true;
        if (callback) this.once("message", callback);
      }

      bind(port, address, callback) {
        if (this._closed) throw dgramError("Socket is closed");
        if (this._bindCall) throw dgramError("Socket is already bound", "ERR_SOCKET_DGRAM_IS_BOUND");
        if (typeof port === "function") { callback = port; port = undefined; }
        var options;
        if (port && typeof port === "object") {
          options = Object.assign({}, port);
          callback = typeof address === "function" ? address : callback;
        } else {
          if (typeof address === "function") { callback = address; address = undefined; }
          options = { port: port === undefined ? 0 : port, host: address };
        }
        options.port = Number(options.port === undefined ? 0 : options.port);
        if (!Number.isInteger(options.port) || options.port < 0 || options.port > 65535) throw dgramError("Invalid UDP port", "ERR_SOCKET_BAD_PORT");
        options.host = options.host || options.address || (this.type === "udp6" ? "::" : "0.0.0.0");
        if (callback) this.once("listening", callback);
        var self = this;
        this._bindCall = callStream("socket.udpBind", [{ host: options.host, port: options.port }], {
          onEvent: function (name, data) {
            if (name === "listening") {
              self._socketId = data.socketId;
              self._localAddress = data.address;
              self._localPort = data.port;
              self._bound = true;
              self.emit("listening");
              if (self._closing) self._closeNative();
              return;
            }
            if (name === "datagram") {
              self._pendingDatagram = data;
              return;
            }
            if (name === "close") {
              self._finishClose();
            }
          },
          onBlob: function (blob) {
            var info = self._pendingDatagram || {};
            self._pendingDatagram = undefined;
            Promise.resolve(blob.arrayBuffer()).then(function (arrayBuffer) {
              var message = Buffer.from(arrayBuffer);
              var remoteInfo = {
                address: info.address,
                port: info.port,
                family: String(info.address || "").indexOf(":") >= 0 ? "IPv6" : "IPv4",
                size: Number(info.size) || message.length,
              };
              self.emit("message", message, remoteInfo);
              if (self._socketId) {
                control(self._socketId, "readAck", { bytes: Math.max(1, message.length) }).catch(function (error) {
                  if (!self._closing) self.emit("error", runtime.nativeError(error));
                });
              }
            }, function (error) { self.emit("error", error); });
          },
        });
        this._bindCall.promise.then(function () { self._finishClose(); }, function (error) {
          if (!self._closing) self.emit("error", runtime.nativeError(error));
          self._finishClose();
        });
        return this;
      }

      address() {
        if (!this._bound) throw dgramError("Socket is not bound", "ERR_SOCKET_DGRAM_NOT_RUNNING");
        return { address: this._localAddress, family: String(this._localAddress).indexOf(":") >= 0 ? "IPv6" : "IPv4", port: this._localPort };
      }

      send(message, offset, length, port, address, callback) {
        var args = Array.prototype.slice.call(arguments);
        var cb = typeof args[args.length - 1] === "function" ? args.pop() : undefined;
        var data = args[0];
        var destinationPort;
        var destinationAddress;
        if (typeof args[1] === "number" && typeof args[2] === "number") {
          offset = args[1]; length = args[2]; destinationPort = args[3]; destinationAddress = args[4];
          if (!Number.isInteger(offset) || !Number.isInteger(length) || offset < 0 || length < 0) throw new RangeError("Invalid UDP message range");
        } else {
          destinationPort = args[1]; destinationAddress = args[2];
          offset = 0;
          length = undefined;
        }
        destinationPort = Number(destinationPort);
        if (!Number.isInteger(destinationPort) || destinationPort < 0 || destinationPort > 65535) throw dgramError("Invalid UDP port", "ERR_SOCKET_BAD_PORT");
        if (typeof destinationAddress !== "string" || !destinationAddress) throw new TypeError("UDP send requires a destination address");
        var bytes;
        if (typeof data === "string") bytes = Buffer.from(data);
        else if (ArrayBuffer.isView(data)) bytes = Buffer.from(data.buffer, data.byteOffset, data.byteLength);
        else if (data instanceof ArrayBuffer || Array.isArray(data)) bytes = Buffer.from(data);
        else throw new TypeError("UDP message must be a string, Buffer, or byte array");
        if (offset !== undefined && (offset > bytes.length || length !== undefined && offset + length > bytes.length)) throw new RangeError("UDP message range is outside the buffer");
        if (offset || length !== undefined) bytes = bytes.subarray(offset, length === undefined ? bytes.length : offset + length);
        if (bytes.length > 65507) throw dgramError("UDP datagram exceeds 65507 bytes", "EMSGSIZE");
        var self = this;
        function sendBound() {
          if (!self._bound || !self._socketId) {
            var notBound = dgramError("Socket is not bound", "ERR_SOCKET_DGRAM_NOT_RUNNING");
            if (cb) cb(notBound); else self.emit("error", notBound);
            return;
          }
          var call;
          try {
            call = callStream("socket.udpSend", [{ socketId: self._socketId, address: destinationAddress, port: destinationPort }], {});
            sendBytes(call.id, bytes);
          } catch (error) {
            if (cb) cb(error); else self.emit("error", error);
            return;
          }
          call.promise.then(function (result) {
            var sent = result && Number(result.sent);
            if (cb) cb(null, Number.isFinite(sent) ? sent : bytes.length);
          }, function (error) {
            error = runtime.nativeError(error);
            if (cb) cb(error); else self.emit("error", error);
          });
        }
        if (!this._bound) this.bind(0, this.type === "udp6" ? "::" : "0.0.0.0", sendBound);
        else sendBound();
        return undefined;
      }

      close(callback) {
        if (callback) this.once("close", callback);
        if (this._closed || this._closing) return this;
        if (!this._bindCall) throw dgramError("Socket is not running", "ERR_SOCKET_DGRAM_NOT_RUNNING");
        this._closing = true;
        if (this._socketId) this._closeNative();
        return this;
      }

      _closeNative() {
        if (!this._socketId || this._closeSent || this._closed) return;
        this._closeSent = true;
        var self = this;
        control(this._socketId, "close").catch(function (error) {
          if (self._closed) return;
          self._closeSent = false;
          self._closing = false;
          self.emit("error", runtime.nativeError(error));
        });
      }

      ref() { this._refed = true; return this; }
      unref() { this._refed = false; return this; }
      hasRef() { return this._refed; }

      setBroadcast(value) {
        if (value) throw runtime.bridgeError("UDP broadcast is not supported", "ENOTSUP");
        return this;
      }
      addMembership() { throw runtime.bridgeError("UDP multicast is not supported", "ENOTSUP"); }
      dropMembership() { throw runtime.bridgeError("UDP multicast is not supported", "ENOTSUP"); }
      setTTL() { throw runtime.bridgeError("UDP TTL options are not supported", "ENOTSUP"); }
      setMulticastTTL() { throw runtime.bridgeError("UDP multicast is not supported", "ENOTSUP"); }
      setMulticastLoopback() { throw runtime.bridgeError("UDP multicast is not supported", "ENOTSUP"); }

      _finishClose() {
        if (this._closed) return;
        this._closed = true;
        this._bound = false;
        if (!this._closeEmitted) {
          this._closeEmitted = true;
          this.emit("close");
        }
      }
    }

    function createSocket(options, callback) { return new Socket(options, callback); }
    return { Socket: Socket, createSocket: createSocket };
  }

  runtime.createDgramModule = createDgramModule;
  runtime.dgram = createDgramModule();
})(globalThis);
