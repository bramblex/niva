(function (root) {
  "use strict";

  var runtime = root[Symbol.for("niva.node-compat.runtime")];
  var supportedHashes = ["sha1", "sha256", "sha384", "sha512"];

  function cryptoProvider() {
    if (!root.crypto || typeof root.crypto.getRandomValues !== "function") {
      throw runtime.bridgeError("Web Crypto is unavailable in this page", "ENOTSUP");
    }
    return root.crypto;
  }

  function toBytes(value, encoding) {
    if (typeof value === "string") return runtime.buffer.Buffer.from(value, encoding || "utf8");
    if (value instanceof ArrayBuffer) return new Uint8Array(value);
    if (ArrayBuffer.isView(value)) return new Uint8Array(value.buffer, value.byteOffset, value.byteLength).slice();
    if (Array.isArray(value)) return Uint8Array.from(value);
    throw new TypeError("data must be a string, Buffer, ArrayBuffer, or typed array");
  }

  function concatenate(parts) {
    return runtime.buffer.Buffer.concat(parts);
  }

  function randomBytes(size) {
    if (!Number.isSafeInteger(size) || size < 0 || size > 0x7fffffff) {
      return Promise.reject(new RangeError("size must be a non-negative safe integer no larger than 2^31 - 1"));
    }
    return Promise.resolve().then(function () {
      var provider = cryptoProvider();
      var output = new Uint8Array(size);
      for (var offset = 0; offset < size; offset += 65536) {
        provider.getRandomValues(output.subarray(offset, Math.min(offset + 65536, size)));
      }
      return runtime.buffer.Buffer.from(output);
    });
  }

  function randomUUID() {
    return Promise.resolve().then(function () {
      var provider = cryptoProvider();
      if (typeof provider.randomUUID === "function") return provider.randomUUID();
      var bytes = provider.getRandomValues(new Uint8Array(16));
      bytes[6] = (bytes[6] & 0x0f) | 0x40;
      bytes[8] = (bytes[8] & 0x3f) | 0x80;
      var hex = Array.prototype.map.call(bytes, function (byte) { return byte.toString(16).padStart(2, "0"); }).join("");
      return hex.slice(0, 8) + "-" + hex.slice(8, 12) + "-" + hex.slice(12, 16) + "-" + hex.slice(16, 20) + "-" + hex.slice(20);
    });
  }

  function createHash(algorithm) {
    var normalized = String(algorithm || "").toLowerCase().replace(/-/g, "");
    if (supportedHashes.indexOf(normalized) < 0) {
      throw runtime.bridgeError("Web Crypto digest does not support algorithm: " + algorithm, "ERR_CRYPTO_HASH_UNSUPPORTED");
    }
    var webAlgorithm = normalized.toUpperCase().replace(/^SHA/, "SHA-");
    var chunks = [];
    var digested = false;
    var hash = {
      update: function (data, inputEncoding) {
        if (digested) throw runtime.bridgeError("Digest already called", "ERR_CRYPTO_HASH_FINALIZED");
        chunks.push(runtime.buffer.Buffer.from(toBytes(data, inputEncoding)));
        return hash;
      },
      digest: function (encoding) {
        if (digested) throw runtime.bridgeError("Digest already called", "ERR_CRYPTO_HASH_FINALIZED");
        digested = true;
        var bytes = concatenate(chunks);
        chunks = [];
        return Promise.resolve().then(function () {
          var provider = cryptoProvider();
          if (!provider.subtle || typeof provider.subtle.digest !== "function") {
            throw runtime.bridgeError("Web Crypto digest is unavailable in this page", "ENOTSUP");
          }
          var copy = bytes.buffer.slice(bytes.byteOffset, bytes.byteOffset + bytes.byteLength);
          return provider.subtle.digest(webAlgorithm, copy);
        }).then(function (result) {
          var output = runtime.buffer.Buffer.from(result);
          return encoding === undefined ? output : output.toString(encoding);
        });
      },
    };
    return hash;
  }

  var module = {
    randomUUID: randomUUID,
    randomBytes: randomBytes,
    createHash: createHash,
    getHashes: function () { return supportedHashes.slice(); },
  };
  runtime.createCryptoModule = function () { return module; };
  runtime.crypto = module;
})(globalThis);
