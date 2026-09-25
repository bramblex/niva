(function (root: any) {
  "use strict";

  var runtime = root[Symbol.for("niva.node-compat.runtime")];
  if (typeof runtime.createZlibModule === "function") return;
  var initialized;
  function createZlibModule() {
  var maxBufferLength = runtime.buffer.kMaxLength;
  var vendor = runtime.vendor;
  var Buffer = runtime.buffer && runtime.buffer.Buffer;
  var compression = vendor && vendor.compression;
  if (!Buffer || !compression) throw new Error("Load the Niva Buffer and browser vendor adapters before zlib.");

  function inputBytes(value) {
    if (typeof value === "string") return Buffer.from(value);
    if (ArrayBuffer.isView(value)) return Buffer.from(value.buffer, value.byteOffset, value.byteLength);
    if (value instanceof ArrayBuffer || Array.isArray(value)) return Buffer.from(value);
    throw new TypeError("The \"buffer\" argument must be a string or an instance of Buffer, TypedArray, DataView, or ArrayBuffer");
  }

  function fflateOptions(options) {
    options = options || {};
    if (options === null || typeof options !== "object") throw new TypeError("The \"options\" argument must be an object");
    var result: Record<string, any> = {};
    if (options.level !== undefined) {
      if (!Number.isInteger(options.level) || options.level < -1 || options.level > 9) throw runtime.bridgeError("The \"level\" argument is out of range", "ERR_OUT_OF_RANGE");
      if (options.level !== -1) result.level = options.level;
    }
    if (options.memLevel !== undefined) {
      if (!Number.isInteger(options.memLevel) || options.memLevel < 1 || options.memLevel > 9) throw runtime.bridgeError("The \"memLevel\" argument is out of range", "ERR_OUT_OF_RANGE");
      result.mem = options.memLevel;
    }
    if (options.strategy !== undefined) {
      if (!Number.isInteger(options.strategy) || options.strategy < 0 || options.strategy > 4) throw runtime.bridgeError("The \"strategy\" argument is out of range", "ERR_OUT_OF_RANGE");
      result.strategy = options.strategy;
    }
    if (options.mtime !== undefined) result.mtime = options.mtime;
    if (options.filename !== undefined) result.filename = options.filename;
    if (options.comment !== undefined) result.comment = options.comment;
    return result;
  }

  function wrapError(error, format) {
    if (error && typeof error.code === "string" && error.code.startsWith("Z_")) return error;
    if (error instanceof RangeError || error && error.code === "ERR_BUFFER_TOO_LARGE") return error;
    var wrapped = runtime.bridgeError(error && error.message || "Invalid " + format + " data", "Z_DATA_ERROR", { cause: error });
    return wrapped;
  }

  function rangeError(message, code?) {
    var error = new RangeError(message);
    error.code = code || "ERR_OUT_OF_RANGE";
    return error;
  }

  function outputLimit(options) {
    var limit = options && options.maxOutputLength;
    if (limit !== undefined && (!Number.isSafeInteger(limit) || limit < 1)) throw rangeError('The "maxOutputLength" argument is out of range');
    return Math.min(maxBufferLength, limit === undefined ? maxBufferLength : limit);
  }

  function checkOutput(bytes, limit) {
    if (bytes.length > limit) throw rangeError("Cannot create a Buffer larger than the configured maxOutputLength", "ERR_BUFFER_TOO_LARGE");
    return Buffer.from(bytes);
  }

  function decodeGunzip(input, options, maxOutputLength) {
    maxOutputLength = Math.min(maxBufferLength, maxOutputLength === undefined ? maxBufferLength : maxOutputLength);
    if (typeof compression.Gunzip !== "function") return Buffer.from(compression.gunzipSync(input, options));
    var trailingZeros = 0;
    while (trailingZeros < input.length && input[input.length - trailingZeros - 1] === 0) trailingZeros++;
    var lastError;

    function decode(candidate) {
      var chunks = [];
      var total = 0;
      var decoder = new compression.Gunzip(options, function (chunk) {
        if (!chunk || !chunk.length) return;
        total += chunk.length;
        if (maxOutputLength !== undefined && total > maxOutputLength) {
          var tooLarge = new RangeError("Cannot create a Buffer larger than the configured maxOutputLength");
          tooLarge.code = "ERR_BUFFER_TOO_LARGE";
          throw tooLarge;
        }
        chunks.push(Buffer.from(chunk));
      });
      decoder.push(candidate, true);
      return Buffer.concat(chunks, total);
    }

    try { return decode(input); }
    catch (error) { lastError = error; }

    // Node's gunzip accepts zero padding after complete members. Try candidate
    // ends that carry the final member's ISIZE instead of repeatedly inflating
    // every possible byte prefix.
    if (trailingZeros > 0) {
      var lastLength = 0;
      try {
        var probe = new compression.Gunzip(options, function (chunk) {
          if (chunk && chunk.length) lastLength = chunk.length;
        });
        probe.push(input, true);
      } catch (_) {}
      for (var trim = 1; trim <= trailingZeros; trim++) {
        var end = input.length - trim;
        if (end < 8) continue;
        var isize = (input[end - 4] | input[end - 3] << 8 | input[end - 2] << 16 | input[end - 1] << 24) >>> 0;
        if (isize !== (lastLength >>> 0)) continue;
        try { return decode(input.subarray(0, end)); }
        catch (error) { lastError = error; }
      }
    }

    var endWithoutPadding = input.length - trailingZeros;
    if (endWithoutPadding >= 4 && input[endWithoutPadding - 4] === 0x1f &&
        input[endWithoutPadding - 3] === 0x8b && input[endWithoutPadding - 2] !== 8) {
      var headerError = runtime.bridgeError("unknown compression method", "Z_DATA_ERROR");
      throw headerError;
    }
    throw lastError || runtime.bridgeError("Invalid gzip data", "Z_DATA_ERROR");
  }

  function gzipSync(data, options) {
    var input = inputBytes(data);
    var opts = fflateOptions(options);
    var limit = outputLimit(options);
    try { return checkOutput(compression.gzipSync(input, opts), limit); }
    catch (error) { throw wrapError(error, "gzip"); }
  }

  function gunzipSync(data, options) {
    options = options || {};
    var input = inputBytes(data);
    var opts = fflateOptions(options);
    if (options.maxOutputLength !== undefined && (!Number.isSafeInteger(options.maxOutputLength) || options.maxOutputLength < 1)) {
      throw rangeError('The "maxOutputLength" argument is out of range');
    }
    try { return decodeGunzip(input, opts, options.maxOutputLength); }
    catch (error) { throw wrapError(error, "gzip"); }
  }

  function compress(data, options, callback, method) {
    if (typeof options === "function") { callback = options; options = undefined; }
    if (typeof callback !== "function") throw new TypeError("The \"callback\" argument must be of type function");
    var input = inputBytes(data);
    var opts = fflateOptions(options);
    if (options && options.maxOutputLength !== undefined && (!Number.isSafeInteger(options.maxOutputLength) || options.maxOutputLength < 1)) {
      throw rangeError('The "maxOutputLength" argument is out of range');
    }
    var limit = outputLimit(options);
    var transform = compression[method];
    if (typeof transform !== "function") throw runtime.bridgeError(method + " is unavailable in this build", "ENOTSUP");
    var syncTransform = compression[method + "Sync"];
    var called = false;
    function finish(error, result?, fallback?) {
      if (called) return;
      if (error && fallback && typeof syncTransform === "function") {
        try { finish(null, syncTransform(input, opts)); return; }
        catch (syncError) { error = syncError; }
      }
      called = true;
      enqueue(function () {
        if (error) { callback(wrapError(error, method)); return; }
        var buffer;
        try { buffer = checkOutput(result, limit); }
        catch (tooLarge) { callback(tooLarge); return; }
        callback(null, buffer);
      });
    }
    if (method === "gunzip") {
      enqueue(function () {
        try { finish(null, decodeGunzip(input, opts, options && options.maxOutputLength)); }
        catch (error) { finish(error); }
      });
      return undefined;
    }
    if (typeof root.Worker !== "function") {
      enqueue(function () {
        try { finish(null, syncTransform(input, opts)); }
        catch (error) { finish(error); }
      });
      return undefined;
    }
    try { transform(input, opts, function (error, output) { finish(error, output, true); }); }
    catch (_) {
      enqueue(function () {
        try { finish(null, syncTransform(input, opts)); }
        catch (error) { finish(error); }
      });
    }
    return undefined;
  }

  function enqueue(callback) {
    if (typeof root.queueMicrotask === "function") root.queueMicrotask(callback);
    else Promise.resolve().then(callback);
  }

  var module = {
    gzip: function (data, options, callback) { return compress(data, options, callback, "gzip"); },
    gunzip: function (data, options, callback) { return compress(data, options, callback, "gunzip"); },
    gzipSync: gzipSync,
    gunzipSync: gunzipSync,
  };
  return module;
  }
  runtime.createZlibModule = function () { return initialized || (initialized = createZlibModule()); };
  Object.defineProperty(runtime, "zlib", {__proto__: null, get: runtime.createZlibModule, configurable: true});
})(globalThis);
