(function (root: any) {
  "use strict";

  var runtime = root[Symbol.for("niva.node-compat.runtime")];
  if (typeof runtime.createCryptoModule === "function") return;
  var vendor = runtime.vendor;
  var Buffer = runtime.buffer && runtime.buffer.Buffer;
  var hashes = vendor && vendor.hashes;
  if (!Buffer || !hashes) throw new Error("Load the Niva Buffer and browser vendor adapters before crypto.");

  var hashFunctions = {
    md5: hashes.md5,
    sha1: hashes.sha1,
    sha256: hashes.sha256,
    sha384: hashes.sha384,
    sha512: hashes.sha512,
  };
  var supportedHashes = Object.keys(hashFunctions).filter(function (name) { return typeof hashFunctions[name] === "function"; }).sort();

  function cryptoError(message, code?, extra?) {
    return runtime.bridgeError(message, code, extra);
  }

  function codedTypeError(message, code?) {
    var error = new TypeError(message);
    error.code = code || "ERR_INVALID_ARG_TYPE";
    return error;
  }

  function codedRangeError(message) {
    var error = new RangeError(message);
    error.code = "ERR_OUT_OF_RANGE";
    return error;
  }

  function received(value) {
    if (value === null || value === undefined) return String(value);
    if (typeof value === "function") return "function " + (value.name || "");
    if (typeof value === "object") {
      var name = value.constructor && value.constructor.name;
      return "an instance of " + (name || "Object");
    }
    if (typeof value === "string") return "type string ('" + value.replace(/'/g, "\\'") + "')";
    return "type " + typeof value + " (" + String(value) + ")";
  }

  function argumentTypeError(argument, expected, value) {
    return codedTypeError('The "' + argument + '" argument must be of type ' + expected + '. Received ' + received(value));
  }

  function kdfInput(value, argument) {
    if (typeof value === "string") return Buffer.from(value);
    if (ArrayBuffer.isView(value)) return Buffer.from(value.buffer, value.byteOffset, value.byteLength);
    if (value instanceof ArrayBuffer || (typeof root.SharedArrayBuffer === "function" && value instanceof root.SharedArrayBuffer)) {
      return Buffer.from(new Uint8Array(value));
    }
    throw codedTypeError('The "' + argument + '" argument must be of type string or an instance of ArrayBuffer, Buffer, TypedArray, or DataView. Received ' + received(value));
  }

  var keyMaterial = new WeakMap();
  var keyConstructorToken = {};
  function secretMaterial(key) {
    var material = keyMaterial.get(key);
    if (!material) throw codedTypeError("Value must be a Niva KeyObject", "ERR_INVALID_THIS");
    return material;
  }
  class KeyObject {
    constructor(token, material) {
      if (token !== keyConstructorToken) throw codedTypeError("Illegal constructor", "ERR_ILLEGAL_CONSTRUCTOR");
      keyMaterial.set(this, Buffer.from(material));
    }
    get type() { secretMaterial(this); return "secret"; }
    equals(other) {
      if (!keyMaterial.has(other)) throw codedTypeError('The "otherKeyObject" argument must be an instance of KeyObject');
      return compareNivaKeyObjects(this, other);
    }
    get [Symbol.toStringTag]() { return "KeyObject"; }
  }
  class SecretKeyObject extends KeyObject {
    get symmetricKeySize() { return secretMaterial(this).length; }
    export(options) {
      var material = secretMaterial(this);
      if (options !== undefined && (options === null || typeof options !== "object" || Array.isArray(options))) throw codedTypeError('The "options" argument must be of type object');
      var format = options && options.format;
      if (format === undefined || format === "buffer") return Buffer.from(material);
      if (format === "jwk") return {kty: "oct", k: material.toString("base64url")};
      throw codedTypeError('The "options.format" property must be "buffer" or "jwk"', "ERR_INVALID_ARG_VALUE");
    }
  }
  function createSecretKey(key, encoding) {
    var material = typeof key === "string" ? Buffer.from(key, encoding) : kdfInput(key, "key");
    return new SecretKeyObject(keyConstructorToken, material);
  }
  function compareNivaKeyObjects(left, right) {
    if (!keyMaterial.has(left) || !keyMaterial.has(right)) return false;
    var a = keyMaterial.get(left), b = keyMaterial.get(right);
    if (a.length !== b.length) return false;
    var difference = 0;
    for (var index = 0; index < a.length; index++) difference |= a[index] ^ b[index];
    return difference === 0;
  }

  function normalizeHash(algorithm) {
    if (typeof algorithm !== "string") throw new TypeError("The \"algorithm\" argument must be of type string");
    var normalized = algorithm.toLowerCase().replace(/[-_]/g, "");
    if (normalized === "sha1") normalized = "sha1";
    if (normalized === "sha1" || normalized === "sha256" || normalized === "sha384" || normalized === "sha512" || normalized === "md5") {
      if (typeof hashFunctions[normalized] === "function") return normalized;
    }
    throw cryptoError("Digest method not supported: " + algorithm, "ERR_CRYPTO_HASH_UNSUPPORTED");
  }

  function bytes(value, encoding?) {
    if (typeof value === "string") return Buffer.from(value, encoding || "utf8");
    if (ArrayBuffer.isView(value)) return Buffer.from(value.buffer, value.byteOffset, value.byteLength);
    if (value instanceof ArrayBuffer || Array.isArray(value)) return Buffer.from(value);
    throw new TypeError("data must be a string, Buffer, ArrayBuffer, or typed array");
  }

  function output(data, encoding) {
    var buffer = Buffer.from(data);
    return encoding === undefined ? buffer : buffer.toString(encoding);
  }

  function makeIncremental(state) {
    var finalized = false;
    var hash = {
      update: function (data, inputEncoding) {
        if (finalized) throw cryptoError("Digest already called", "ERR_CRYPTO_HASH_FINALIZED");
        state.update(bytes(data, inputEncoding));
        return hash;
      },
      digest: function (encoding) {
        if (finalized) throw cryptoError("Digest already called", "ERR_CRYPTO_HASH_FINALIZED");
        finalized = true;
        return output(state.digest(), encoding);
      },
    };
    return hash;
  }

  function createHash(algorithm, options) {
    var name = normalizeHash(algorithm);
    if (options !== undefined && (options === null || typeof options !== "object")) {
      throw new TypeError("The \"options\" argument must be an object");
    }
    return makeIncremental(hashFunctions[name].create());
  }

  function createHmac(algorithm, key, options) {
    if (keyMaterial.has(key)) key = secretMaterial(key);
    var name = normalizeHash(algorithm);
    if (typeof hashes.hmac !== "function" || typeof hashes.hmac.create !== "function") {
      throw cryptoError("HMAC is unavailable in this build", "ENOTSUP");
    }
    if (options !== undefined && (options === null || typeof options !== "object")) {
      throw new TypeError("The \"options\" argument must be an object");
    }
    return makeIncremental(hashes.hmac.create(hashFunctions[name], bytes(key)));
  }

  function randomSource() {
    var provider = root.crypto;
    if (!provider || typeof provider.getRandomValues !== "function") {
      throw cryptoError("Web Crypto is unavailable in this page", "ENOTSUP");
    }
    return provider;
  }

  function validateSize(size, label?) {
    if (!Number.isInteger(size)) throw new TypeError((label || "size") + " must be an integer");
    if (size < 0 || size > 0x7fffffff) throw new RangeError((label || "size") + " is out of range");
  }

  function fillRandom(size) {
    validateSize(size);
    var provider = randomSource();
    var result = Buffer.allocUnsafe(size);
    for (var offset = 0; offset < size; offset += 65536) {
      provider.getRandomValues(result.subarray(offset, Math.min(offset + 65536, size)));
    }
    return result;
  }

  function randomBytes(size, callback) {
    validateSize(size);
    if (callback !== undefined && typeof callback !== "function") throw new TypeError("The \"callback\" argument must be of type function");
    if (callback === undefined) return fillRandom(size);
    enqueue(function () {
      var result;
      try { result = fillRandom(size); }
      catch (error) { callback(error); return; }
      callback(null, result);
    });
    return undefined;
  }

  function randomUUID(options) {
    if (options !== undefined && (options === null || typeof options !== "object")) throw argumentTypeError("options", "Object", options);
    if (options && options.disableEntropyCache !== undefined && typeof options.disableEntropyCache !== "boolean") {
      throw argumentTypeError("options.disableEntropyCache", "boolean", options.disableEntropyCache);
    }
    var provider = randomSource();
    if (typeof provider.randomUUID === "function") return provider.randomUUID();
    var data = provider.getRandomValues(new Uint8Array(16));
    data[6] = (data[6] & 0x0f) | 0x40;
    data[8] = (data[8] & 0x3f) | 0x80;
    var hex = Array.prototype.map.call(data, function (byte) { return byte.toString(16).padStart(2, "0"); }).join("");
    return hex.slice(0, 8) + "-" + hex.slice(8, 12) + "-" + hex.slice(12, 16) + "-" + hex.slice(16, 20) + "-" + hex.slice(20);
  }

  function validateKdf(iterations, keylen, digest, callback?) {
    if (typeof iterations !== "number") throw argumentTypeError("iterations", "number", iterations);
    if (!Number.isInteger(iterations)) throw codedRangeError('The value of "iterations" is out of range. It must be an integer. Received ' + String(iterations));
    if (iterations < 1 || iterations > 0x7fffffff) throw codedRangeError('The value of "iterations" is out of range. It must be >= 1 and <= 2147483647. Received ' + String(iterations));

    if (typeof keylen !== "number") throw argumentTypeError("keylen", "number", keylen);
    if (!Number.isInteger(keylen)) throw codedRangeError('The value of "keylen" is out of range. It must be an integer. Received ' + String(keylen));
    var maxLength = 0x7fffffff;
    if (keylen < 1 || keylen > maxLength) throw codedRangeError('The value of "keylen" is out of range. It must be >= 1 and <= ' + maxLength + '. Received ' + String(keylen));

    if (typeof digest !== "string") throw argumentTypeError("digest", "string", digest);
    var hash = digest.toLowerCase().replace(/[-_]/g, "");
    if (!hashFunctions[hash]) {
      var digestError = codedTypeError("Invalid digest: " + digest, "ERR_CRYPTO_INVALID_DIGEST");
      throw digestError;
    }
    if (callback !== undefined && typeof callback !== "function") throw argumentTypeError("callback", "Function", callback);
    return { hash: hash };
  }

  function pbkdf2(password, salt, iterations, keylen, digest, callback) {
    if (typeof digest === "function" && callback === undefined) {
      callback = digest;
      digest = undefined;
    }
    var valid = validateKdf(iterations, keylen, digest, callback);
    if (typeof callback !== "function") throw codedTypeError('The "callback" argument must be of type function');
    var passwordBytes = kdfInput(password, "password");
    var saltBytes = kdfInput(salt, "salt");
    if (typeof hashes.pbkdf2Async !== "function") throw cryptoError("PBKDF2 is unavailable in this build", "ENOTSUP");
    hashes.pbkdf2Async(hashFunctions[valid.hash], passwordBytes, saltBytes, { c: iterations, dkLen: keylen })
      .then(function (result) { callback(null, Buffer.from(result)); }, function (error) { callback(error); });
    return undefined;
  }

  function pbkdf2Sync(password, salt, iterations, keylen, digest) {
    var valid = validateKdf(iterations, keylen, digest);
    var hash = valid.hash;
    var passwordBytes = kdfInput(password, "password");
    var saltBytes = kdfInput(salt, "salt");
    if (typeof hashes.pbkdf2 !== "function") throw cryptoError("PBKDF2 is unavailable in this build", "ENOTSUP");
    return Buffer.from(hashes.pbkdf2(hashFunctions[hash], passwordBytes, saltBytes, { c: iterations, dkLen: keylen }));
  }

  function scryptOptions(keylen, options) {
    validateSize(keylen, "keylen");
    options = options || {};
    if (options === null || typeof options !== "object") throw new TypeError("The \"options\" argument must be an object");
    var normalized: Record<string, any> = {
      N: options.N === undefined ? 16384 : options.N,
      r: options.r === undefined ? 8 : options.r,
      p: options.p === undefined ? 1 : options.p,
      dkLen: keylen,
      maxmem: options.maxmem === undefined ? 32 * 1024 * 1024 : options.maxmem,
    };
    if (options.asyncTick !== undefined) normalized.asyncTick = options.asyncTick;
    if (options.onProgress !== undefined) normalized.onProgress = options.onProgress;
    if (!Number.isInteger(normalized.N) || normalized.N < 2 || (normalized.N & (normalized.N - 1)) !== 0) {
      throw new RangeError("The \"N\" argument must be a power of 2 greater than 1");
    }
    if (!Number.isInteger(normalized.r) || normalized.r < 1) throw new RangeError("The \"r\" argument must be a positive integer");
    if (!Number.isInteger(normalized.p) || normalized.p < 1) throw new RangeError("The \"p\" argument must be a positive integer");
    if (!Number.isSafeInteger(normalized.maxmem) || normalized.maxmem < 0) throw new RangeError("The \"maxmem\" argument is out of range");
    return normalized;
  }

  function scrypt(password, salt, keylen, options, callback) {
    if (typeof options === "function") { callback = options; options = undefined; }
    if (typeof callback !== "function") throw new TypeError("The \"callback\" argument must be of type function");
    var normalized = scryptOptions(keylen, options);
    if (typeof hashes.scryptAsync !== "function") throw cryptoError("scrypt is unavailable in this build", "ENOTSUP");
    hashes.scryptAsync(bytes(password), bytes(salt), normalized)
      .then(function (result) { callback(null, Buffer.from(result)); }, function (error) { callback(error); });
    return undefined;
  }

  function timingSafeEqual(a, b) {
    root.console?.warn?.("Niva.crypto.timingSafeEqual is a JavaScript compatibility helper and is not constant-time.");
    var left = bytes(a);
    var right = bytes(b);
    if (left.length !== right.length) throw cryptoError("Input buffers must have the same byte length", "ERR_CRYPTO_TIMING_SAFE_EQUAL_LENGTH");
    var difference = 0;
    for (var index = 0; index < left.length; index += 1) difference |= left[index] ^ right[index];
    return difference === 0;
  }

  function enqueue(callback) {
    if (typeof root.queueMicrotask === "function") root.queueMicrotask(callback);
    else Promise.resolve().then(callback);
  }

  var module = {
    KeyObject: KeyObject,
    createSecretKey: createSecretKey,
    randomUUID: randomUUID,
    randomBytes: randomBytes,
    createHash: createHash,
    createHmac: createHmac,
    getHashes: function () { return supportedHashes.slice(); },
    pbkdf2: pbkdf2,
    pbkdf2Sync: pbkdf2Sync,
    scrypt: scrypt,
    timingSafeEqual: timingSafeEqual,
  };
  Object.defineProperties(module, {
    __proto__: null,
    isNivaKeyObject: {__proto__: null, value: function (value) { return keyMaterial.has(value); }},
    compareNivaKeyObjects: {__proto__: null, value: compareNivaKeyObjects}
  });
  runtime.createCryptoModule = function () { return module; };
  runtime.crypto = module;
})(globalThis);
