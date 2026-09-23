(function (root) {
  "use strict";

  var runtime = root[Symbol.for("niva.node-compat.runtime")];

  function bytes(value) {
    if (typeof value === "string") return new TextEncoder().encode(value);
    if (value instanceof ArrayBuffer) return new Uint8Array(value);
    if (ArrayBuffer.isView(value)) return new Uint8Array(value.buffer, value.byteOffset, value.byteLength).slice();
    if (Array.isArray(value)) return Uint8Array.from(value);
    throw new TypeError("data must be a string, Buffer, ArrayBuffer, or typed array");
  }

  async function transform(data, Constructor, format) {
    if (typeof Constructor !== "function") {
      throw runtime.bridgeError("Web Compression Streams are unavailable in this page", "ENOTSUP");
    }
    var transformer = new Constructor(format);
    var reader = transformer.readable.getReader();
    var writer = transformer.writable.getWriter();
    var chunks = [];
    var readAll = (async function () {
      while (true) {
        var result = await reader.read();
        if (result.done) break;
        chunks.push(new Uint8Array(result.value));
      }
    })();
    try {
      await writer.write(bytes(data));
      await writer.close();
      await readAll;
    } catch (error) {
      try { await writer.abort(error); } catch (_) {}
      try { await reader.cancel(error); } catch (_) {}
      throw error;
    }
    return runtime.buffer.Buffer.concat(chunks);
  }

  var module = {
    gzip: function (data) { return transform(data, root.CompressionStream, "gzip"); },
    gunzip: function (data) { return transform(data, root.DecompressionStream, "gzip"); },
  };
  runtime.createZlibModule = function () { return module; };
  runtime.zlib = module;
})(globalThis);
