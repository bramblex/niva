(function (root) {
  "use strict";

  var runtime = root[Symbol.for("niva.node-compat.runtime")];
  if (typeof runtime.createTlsModule === "function") return;

  function createTlsModule(niva) {
    var net = runtime.createNetModule(niva);
    var Buffer = runtime.vendor.Buffer;

    function tlsError(message, code) { return runtime.bridgeError(message, code || "ERR_TLS_INVALID_ARGUMENT"); }

    function isPemBytes(value) {
      return Buffer.isBuffer(value) || ArrayBuffer.isView(value) || value instanceof ArrayBuffer;
    }

    function pemBytes(value) {
      if (value instanceof DataView) {
        return Buffer.from(value.buffer, value.byteOffset, value.byteLength);
      }
      return Buffer.from(value);
    }

    function invalidArgType(name, value) {
      var received;
      if (value == null) received = "Received " + value;
      else if (typeof value === "function") received = "Received function " + (value.name || "");
      else if (typeof value === "object") received = "Received an instance of " + (value.constructor && value.constructor.name || "Object");
      else received = "Received type " + typeof value + " (" + String(value) + ")";
      var error = new TypeError("The \"options." + name + "\" property must be of type string or an instance of Buffer, TypedArray, or DataView." + (received ? " " + received : ""));
      error.code = "ERR_INVALID_ARG_TYPE";
      return error;
    }

    function pem(value, name) {
      if (typeof value === "string") return value;
      if (isPemBytes(value)) {
        return pemBytes(value).toString("utf8");
      }
      throw new TypeError("The \"" + name + "\" option must be a string or byte array");
    }

    function caList(value) {
      if (value === undefined) return undefined;
      var values = Array.isArray(value) ? value : [value];
      return values.map(function (item) { return pem(item, "ca"); });
    }

    function normalizeServerCa(value) {
      if (value === undefined || value === false) return;
      var values = Array.isArray(value) ? value : [value];
      values.forEach(function (item) {
        if (item === false) return;
        if (typeof item !== "string" && !isPemBytes(item)) throw invalidArgType("ca", item);
      });
    }

    function validateClientOptions(options, host) {
      if (options.rejectUnauthorized === false) throw tlsError("Disabling TLS certificate verification is not supported", "ENOTSUP");
      if (options.allowHalfOpen === true) throw tlsError("TLS allowHalfOpen is not supported because Native TLS shutdown ends the session", "ENOTSUP");
      if (options.ALPNProtocols !== undefined) throw tlsError("TLS ALPN is not supported by this bridge", "ENOTSUP");
      if (options.checkServerIdentity !== undefined) throw tlsError("Custom TLS server identity checks are not supported", "ENOTSUP");
      if (options.key !== undefined || options.cert !== undefined || options.pfx !== undefined) {
        throw tlsError("Client TLS identities are not supported", "ENOTSUP");
      }
      ["minVersion", "maxVersion", "ciphers", "secureContext", "session", "sigalgs", "ecdhCurve", "secureProtocol", "secureOptions"]
        .forEach(function (name) {
          if (options[name] !== undefined) throw tlsError("TLS option " + name + " is not supported by this bridge", "ENOTSUP");
        });
      if (options.servername !== undefined) {
        if (typeof options.servername !== "string") {
          throw tlsError("TLS servername must be a string", "ENOTSUP");
        }
        if (options.servername.toLowerCase() !== String(host).toLowerCase()) {
          throw tlsError("A separate TLS servername is not supported", "ENOTSUP");
        }
      }
    }

    function parseConnectArgs(args) {
      var values = Array.prototype.slice.call(args);
      var callback = typeof values[values.length - 1] === "function" ? values.pop() : undefined;
      var first = values[0];
      var options;
      if (first && typeof first === "object") options = Object.assign({}, first);
      else if (typeof first === "number" || typeof first === "string" && /^\d+$/.test(first)) {
        options = { port: Number(first) };
        if (typeof values[1] === "string") options.host = values[1];
        else if (values[1] && typeof values[1] === "object") Object.assign(options, values[1]);
        if (values[2] && typeof values[2] === "object") Object.assign(options, values[2]);
      } else options = {};
      if (!Number.isInteger(Number(options.port)) || Number(options.port) < 1 || Number(options.port) > 65535) {
        var portError = new RangeError("The \"port\" argument must be an integer between 1 and 65535");
        portError.code = "ERR_SOCKET_BAD_PORT";
        throw portError;
      }
      var host = options.host || options.hostname || "localhost";
      validateClientOptions(options, host);
      var nativeArgs = { host: String(host).replace(/^\[|\]$/g, ""), port: Number(options.port) };
      if (options.timeoutMs !== undefined) nativeArgs.timeoutMs = Number(options.timeoutMs);
      var ca = caList(options.ca);
      if (ca !== undefined) nativeArgs.ca = ca;
      return { callback: callback, options: options, args: [nativeArgs] };
    }

    function connect() {
      var parsed = parseConnectArgs(arguments);
      var socket = new net.Socket({ secure: true, allowHalfOpen: parsed.options.allowHalfOpen });
      if (parsed.callback) socket.once("secureConnect", parsed.callback);
      socket._start("socket.tlsConnect", parsed.args, "secureConnect");
      if (parsed.options.timeout) socket.setTimeout(parsed.options.timeout);
      return socket;
    }

    function connectGuarded(options) {
      options = Object.assign({}, options || {});
      var host = options.host || options.hostname || "localhost";
      validateClientOptions(options, host);
      var nativeArgs = { host: String(host).replace(/^\[|\]$/g, ""), port: Number(options.port) };
      if (options.timeoutMs !== undefined) nativeArgs.timeoutMs = Number(options.timeoutMs);
      var ca = caList(options.ca);
      if (ca !== undefined) nativeArgs.ca = ca;
      var socket = new net.Socket({ secure: true });
      socket._start("socket.tlsConnectGuarded", [nativeArgs], "secureConnect");
      if (options.timeout) socket.setTimeout(options.timeout);
      return socket;
    }

    // native-tls accepts PKCS#8 identities; Node also accepts unencrypted
    // PKCS#1 RSA keys. Wrap the existing DER key without changing key material.
    function privateKey(value) {
      var text = pem(value, "key");
      var match = /-----BEGIN RSA PRIVATE KEY-----([\s\S]*?)-----END RSA PRIVATE KEY-----/.exec(text);
      if (!match) return text;
      function der(tag, body) {
        var length = body.length, encoded = [];
        if (length < 128) encoded.push(length);
        else { var bytes = []; while (length) { bytes.unshift(length & 255); length >>>= 8; } encoded.push(128 | bytes.length, ...bytes); }
        return Buffer.concat([Buffer.from([tag, ...encoded]), body]);
      }
      var rsa = Buffer.from(match[1].replace(/\s/g, ""), "base64");
      var algorithm = Buffer.from("300d06092a864886f70d0101010500", "hex");
      var wrapped = der(0x30, Buffer.concat([Buffer.from([2, 1, 0]), algorithm, der(4, rsa)]));
      return "-----BEGIN PRIVATE KEY-----\n" + wrapped.toString("base64").match(/.{1,64}/g).join("\n") + "\n-----END PRIVATE KEY-----\n";
    }

    function serverIdentity(options) {
      if (options.identity) return options.identity;
      if (options.pfx !== undefined) {
        return { pfx: Array.from(Buffer.from(options.pfx)), passphrase: options.passphrase };
      }

      function normalize(value, name, pemObjects) {
        if (value === undefined || value === false) return { present: false, value: undefined };
        if (Array.isArray(value)) {
          var entries = value.map(function (item) {
            if (item === false) return undefined;
            if (pemObjects && item && typeof item === "object" && !isPemBytes(item) && Object.prototype.hasOwnProperty.call(item, "pem")) {
              if (typeof item.pem === "string" || isPemBytes(item.pem)) return item.pem;
              throw invalidArgType(name, item);
            }
            if (typeof item !== "string" && !isPemBytes(item)) throw invalidArgType(name, item);
            return item;
          });
          return { present: true, value: entries };
        }
        if (typeof value !== "string" && !isPemBytes(value)) throw invalidArgType(name, value);
        return { present: true, value: value };
      }

      // Node accepts false as an omitted credential and permits arrays of
      // alternative key/certificate entries. The Native TLS bridge has one
      // identity slot, so defer missing or multi-identity credentials until
      // listen() instead of rejecting createServer() prematurely.
      var cert = normalize(options.cert, "cert", true);
      var key = normalize(options.key, "key", true);
      function collapseSingle(values) {
        if (!Array.isArray(values.value)) return values;
        var present = values.value.filter(function (item) { return item !== undefined; });
        if (present.length > 1) {
          options._nivaMultiIdentity = true;
          return { present: false, value: undefined };
        }
        return { present: present.length === 1, value: present[0] };
      }
      key = collapseSingle(key);
      cert = collapseSingle(cert);
      if (!key.present || !cert.present) return undefined;
      return { key: privateKey(key.value), cert: pem(cert.value, "cert") };
    }

    function createServer(options, listener) {
      if (typeof options === "function") { listener = options; options = {}; }
      options = Object.assign({}, options || {});
      if (options.ALPNProtocols !== undefined) throw tlsError("TLS ALPN is not supported by this bridge", "ENOTSUP");
      if (options.allowHalfOpen === true) throw tlsError("TLS allowHalfOpen is not supported because Native TLS shutdown ends the session", "ENOTSUP");
      normalizeServerCa(options.ca);
      options.identity = serverIdentity(options);
      var multiIdentity = options._nivaMultiIdentity === true;
      delete options._nivaMultiIdentity;
      var server = net.createTlsServer(options, listener);
      if (multiIdentity) {
        server.listen = function () {
          throw tlsError("Multiple TLS server identities are not supported by this bridge", "ENOTSUP");
        };
      }
      return server;
    }

    return {
      connect: connect,
      createServer: createServer,
      TLSSocket: net.Socket,
      Server: net.Server,
      connectGuarded: connectGuarded,
    };
  }

  runtime.createTlsModule = createTlsModule;
  runtime.tls = createTlsModule();
})(globalThis);
