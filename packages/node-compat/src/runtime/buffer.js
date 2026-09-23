(function (root) {
  "use strict";

  var runtime = root[Symbol.for("niva.node-compat.runtime")];

  function normalizeEncoding(encoding) {
    var value = String(encoding || "utf8").toLowerCase().replace(/[-_]/g, "");
    if (["utf8", "hex", "base64", "base64url", "ascii", "latin1", "binary", "utf16le", "ucs2"].indexOf(value) < 0) {
      throw new TypeError("Unknown encoding: " + encoding);
    }
    return value;
  }

  function encode(value, encoding) {
    encoding = normalizeEncoding(encoding);
    if (encoding === "utf8") return new TextEncoder().encode(value);
    if (encoding === "hex") {
      var count = Math.floor(value.length / 2);
      for (var i = 0; i < count; i += 1) {
        var pair = value.slice(i * 2, i * 2 + 2);
        if (!/^[0-9a-f]{2}$/i.test(pair)) { count = i; break; }
      }
      var hex = new Uint8Array(count);
      for (var j = 0; j < count; j += 1) hex[j] = parseInt(value.slice(j * 2, j * 2 + 2), 16);
      return hex;
    }
    if (encoding === "base64" || encoding === "base64url") {
      var source = value.replace(/\s/g, "");
      if (encoding === "base64url") source = source.replace(/-/g, "+").replace(/_/g, "/");
      source = source.replace(/[^A-Za-z0-9+/=]/g, "");
      while (source.length % 4) source += "=";
      if (typeof atob === "function") {
        var binary = atob(source);
        var bytes = new Uint8Array(binary.length);
        for (var j = 0; j < binary.length; j += 1) bytes[j] = binary.charCodeAt(j);
        return bytes;
      }
      if (typeof Buffer !== "undefined") return new Uint8Array(Buffer.from(source, "base64"));
      throw new Error("base64 decoding is unavailable");
    }
    if (encoding === "ascii" || encoding === "latin1" || encoding === "binary") {
      var mask = encoding === "ascii" ? 0x7f : 0xff;
      var single = new Uint8Array(value.length);
      for (var k = 0; k < value.length; k += 1) single[k] = value.charCodeAt(k) & mask;
      return single;
    }
    var wide = new Uint8Array(value.length * 2);
    for (var m = 0; m < value.length; m += 1) {
      var code = value.charCodeAt(m);
      wide[m * 2] = code & 0xff;
      wide[m * 2 + 1] = code >>> 8;
    }
    return wide;
  }

  function decode(bytes, encoding, start, end) {
    encoding = normalizeEncoding(encoding);
    var view = bytes.subarray(start, end);
    if (encoding === "hex") {
      var hex = new Array(view.length);
      for (var i = 0; i < view.length; i += 1) hex[i] = view[i].toString(16).padStart(2, "0");
      return hex.join("");
    }
    if (encoding === "base64" || encoding === "base64url") {
      var binary = "";
      for (var j = 0; j < view.length; j += 0x8000) binary += String.fromCharCode.apply(null, view.subarray(j, j + 0x8000));
      var output = typeof btoa === "function" ? btoa(binary) : Buffer.from(view).toString("base64");
      return encoding === "base64url" ? output.replace(/\+/g, "-").replace(/\//g, "_").replace(/=+$/, "") : output;
    }
    if (encoding === "ascii" || encoding === "latin1" || encoding === "binary") {
      var mask = encoding === "ascii" ? 0x7f : 0xff;
      var chars = new Array(view.length);
      for (var k = 0; k < view.length; k += 1) chars[k] = String.fromCharCode(view[k] & mask);
      return chars.join("");
    }
    if (encoding === "utf16le" || encoding === "ucs2") {
      var wide = "";
      for (var n = 0; n + 1 < view.length; n += 2) wide += String.fromCharCode(view[n] | (view[n + 1] << 8));
      return wide;
    }
    return new TextDecoder("utf-8").decode(view);
  }

  function asBytes(value, encoding, offset, length) {
    if (typeof value === "string") return encode(value, encoding);
    if (typeof value === "number") throw new TypeError("The \"value\" argument must not be of type number");
    if (value instanceof ArrayBuffer) {
      offset = offset === undefined ? 0 : Number(offset);
      length = length === undefined ? value.byteLength - offset : Number(length);
      if (!Number.isInteger(offset) || !Number.isInteger(length) || offset < 0 || length < 0 || offset + length > value.byteLength) throw new RangeError("ArrayBuffer view is out of bounds");
      return new Uint8Array(value, offset, length);
    }
    if (ArrayBuffer.isView(value)) return new Uint8Array(value.buffer, value.byteOffset, value.byteLength).slice();
    if (Array.isArray(value)) return Uint8Array.from(value);
    if (value && value.type === "Buffer" && Array.isArray(value.data)) return Uint8Array.from(value.data);
    if (value && typeof value.length === "number") return Uint8Array.from(value);
    throw new TypeError("The first argument must be a string, Buffer, ArrayBuffer, Array, or array-like object");
  }

  class NivaBuffer extends Uint8Array {
    static get [Symbol.species]() { return NivaBuffer; }

    static from(value, encodingOrOffset, length) {
      if (typeof value === "string") return new NivaBuffer(encode(value, encodingOrOffset));
      var bytes = asBytes(value, undefined, encodingOrOffset, length);
      if (value instanceof ArrayBuffer) return new NivaBuffer(value, encodingOrOffset || 0, length === undefined ? value.byteLength - (encodingOrOffset || 0) : length);
      return new NivaBuffer(bytes);
    }

    static alloc(size, fill, encoding) {
      validateSize(size);
      var output = new NivaBuffer(size);
      if (fill !== undefined && fill !== 0) output.fill(fill, 0, size, encoding);
      return output;
    }

    static allocUnsafe(size) { validateSize(size); return new NivaBuffer(size); }
    static allocUnsafeSlow(size) { return NivaBuffer.allocUnsafe(size); }

    static concat(list, totalLength) {
      if (!Array.isArray(list)) throw new TypeError("list must be an Array of Buffers");
      var length = totalLength === undefined ? list.reduce(function (sum, item) { return sum + item.length; }, 0) : Number(totalLength);
      validateSize(length);
      var output = new NivaBuffer(length);
      var offset = 0;
      list.forEach(function (item) {
        var bytes = asBytes(item);
        var count = Math.min(bytes.length, length - offset);
        if (count > 0) output.set(bytes.subarray(0, count), offset);
        offset += count;
      });
      return output;
    }

    static byteLength(value, encoding) {
      if (typeof value !== "string") return asBytes(value, encoding).byteLength;
      return encode(value, encoding).byteLength;
    }

    static isBuffer(value) { return value instanceof NivaBuffer || !!(value && value._isBuffer === true); }
    static isEncoding(value) {
      try { normalizeEncoding(value); return true; }
      catch (_) { return false; }
    }
    static compare(a, b) { return compareBytes(asBytes(a), asBytes(b)); }

    toString(encoding, start, end) {
      start = start === undefined ? 0 : Math.trunc(Number(start));
      end = end === undefined ? this.length : Math.trunc(Number(end));
      start = Math.max(0, Math.min(this.length, start));
      end = Math.max(start, Math.min(this.length, end));
      return decode(this, encoding, start, end);
    }

    write(value, offset, length, encoding) {
      if (typeof offset === "string") { encoding = offset; offset = 0; length = this.length; }
      else if (typeof length === "string") { encoding = length; length = this.length - (offset || 0); }
      offset = offset === undefined ? 0 : Number(offset);
      length = length === undefined ? this.length - offset : Number(length);
      if (!Number.isInteger(offset) || offset < 0 || offset > this.length) throw new RangeError("offset is outside buffer bounds");
      if (!Number.isInteger(length) || length < 0) throw new RangeError("length must be a non-negative integer");
      var bytes = encode(String(value), encoding);
      var count = Math.min(length, bytes.length, this.length - offset);
      this.set(bytes.subarray(0, count), offset);
      return count;
    }

    equals(other) { return compareBytes(this, asBytes(other)) === 0; }
    compare(other, targetStart, targetEnd, sourceStart, sourceEnd) {
      var right = asBytes(other);
      targetStart = targetStart || 0;
      targetEnd = targetEnd === undefined ? right.length : targetEnd;
      sourceStart = sourceStart || 0;
      sourceEnd = sourceEnd === undefined ? this.length : sourceEnd;
      return compareBytes(this.subarray(sourceStart, sourceEnd), right.subarray(targetStart, targetEnd));
    }
    copy(target, targetStart, sourceStart, sourceEnd) {
      if (!target || typeof target.set !== "function") throw new TypeError("target must be a Buffer or Uint8Array");
      targetStart = targetStart || 0;
      sourceStart = sourceStart || 0;
      sourceEnd = sourceEnd === undefined ? this.length : sourceEnd;
      var bytes = this.subarray(sourceStart, sourceEnd);
      var count = Math.min(bytes.length, target.length - targetStart);
      if (count > 0) target.set(bytes.subarray(0, count), targetStart);
      return count;
    }
    indexOf(value, byteOffset, encoding) { return indexBytes(this, value, byteOffset, encoding); }
    lastIndexOf(value, byteOffset, encoding) { return indexBytes(this, value, byteOffset, encoding, true); }
    includes(value, byteOffset, encoding) { return this.indexOf(value, byteOffset, encoding) !== -1; }
    fill(value, start, end, encoding) {
      start = start === undefined ? 0 : Number(start);
      end = end === undefined ? this.length : Number(end);
      if (!Number.isInteger(start) || !Number.isInteger(end) || start < 0 || end > this.length || end < start) throw new RangeError("fill range is outside buffer bounds");
      var bytes = typeof value === "number" ? Uint8Array.of(value & 0xff)
        : (ArrayBuffer.isView(value) || value instanceof ArrayBuffer ? asBytes(value) : encode(String(value), encoding));
      if (bytes.length === 0) bytes = Uint8Array.of(0);
      for (var i = start; i < end; i += 1) this[i] = bytes[(i - start) % bytes.length];
      return this;
    }
    toJSON() { return { type: "Buffer", data: Array.from(this) }; }
    swap16() { return swap(this, 2); }
    swap32() { return swap(this, 4); }
    swap64() { return swap(this, 8); }
    readUInt8(offset) { return readInteger(this, offset, 1, false, false); }
    readUInt16LE(offset) { return readInteger(this, offset, 2, true, false); }
    readUInt16BE(offset) { return readInteger(this, offset, 2, false, false); }
    readUInt32LE(offset) { return readInteger(this, offset, 4, true, false); }
    readUInt32BE(offset) { return readInteger(this, offset, 4, false, false); }
    readInt8(offset) { return readInteger(this, offset, 1, false, true); }
    readInt16LE(offset) { return readInteger(this, offset, 2, true, true); }
    readInt16BE(offset) { return readInteger(this, offset, 2, false, true); }
    readInt32LE(offset) { return readInteger(this, offset, 4, true, true); }
    readInt32BE(offset) { return readInteger(this, offset, 4, false, true); }
    writeUInt8(value, offset) { return writeInteger(this, value, offset, 1, false, false); }
    writeUInt16LE(value, offset) { return writeInteger(this, value, offset, 2, true, false); }
    writeUInt16BE(value, offset) { return writeInteger(this, value, offset, 2, false, false); }
    writeUInt32LE(value, offset) { return writeInteger(this, value, offset, 4, true, false); }
    writeUInt32BE(value, offset) { return writeInteger(this, value, offset, 4, false, false); }
    writeInt8(value, offset) { return writeInteger(this, value, offset, 1, false, true); }
    writeInt16LE(value, offset) { return writeInteger(this, value, offset, 2, true, true); }
    writeInt16BE(value, offset) { return writeInteger(this, value, offset, 2, false, true); }
    writeInt32LE(value, offset) { return writeInteger(this, value, offset, 4, true, true); }
    writeInt32BE(value, offset) { return writeInteger(this, value, offset, 4, false, true); }
  }

  function validateSize(size) {
    if (!Number.isSafeInteger(size) || size < 0 || size > 0x7fffffff) throw new RangeError("The value of size is out of range");
  }
  function compareBytes(a, b) {
    var length = Math.min(a.length, b.length);
    for (var i = 0; i < length; i += 1) if (a[i] !== b[i]) return a[i] < b[i] ? -1 : 1;
    return a.length === b.length ? 0 : (a.length < b.length ? -1 : 1);
  }
  function indexBytes(buffer, value, offset, encoding, reverse) {
    offset = offset === undefined ? (reverse ? buffer.length : 0) : Math.trunc(Number(offset));
    var needle = typeof value === "number" ? Uint8Array.of(value & 0xff) : (typeof value === "string" ? encode(value, encoding) : asBytes(value));
    if (offset < 0) offset = Math.max(buffer.length + offset, 0);
    if (needle.length === 0) return Math.min(offset, buffer.length);
    if (reverse) {
      offset = Math.min(offset, buffer.length - needle.length);
      for (var i = offset; i >= 0; i -= 1) if (compareBytes(buffer.subarray(i, i + needle.length), needle) === 0) return i;
    } else {
      for (var j = offset; j <= buffer.length - needle.length; j += 1) if (compareBytes(buffer.subarray(j, j + needle.length), needle) === 0) return j;
    }
    return -1;
  }
  function swap(buffer, width) {
    if (buffer.length % width !== 0) throw new RangeError("Buffer size must be a multiple of " + width);
    for (var offset = 0; offset < buffer.length; offset += width) {
      for (var i = 0; i < width / 2; i += 1) {
        var temp = buffer[offset + i];
        buffer[offset + i] = buffer[offset + width - i - 1];
        buffer[offset + width - i - 1] = temp;
      }
    }
    return buffer;
  }
  function readInteger(buffer, offset, width, littleEndian, signed) {
    offset = offset === undefined ? 0 : Number(offset);
    if (!Number.isInteger(offset) || offset < 0 || offset + width > buffer.length) throw new RangeError("Offset is outside buffer bounds");
    var view = new DataView(buffer.buffer, buffer.byteOffset + offset, width);
    if (width === 1) return signed ? view.getInt8(0) : view.getUint8(0);
    if (width === 2) return signed ? view.getInt16(0, littleEndian) : view.getUint16(0, littleEndian);
    return signed ? view.getInt32(0, littleEndian) : view.getUint32(0, littleEndian);
  }
  function writeInteger(buffer, value, offset, width, littleEndian, signed) {
    offset = offset === undefined ? 0 : Number(offset);
    if (!Number.isInteger(offset) || offset < 0 || offset + width > buffer.length) throw new RangeError("Offset is outside buffer bounds");
    var bits = width * 8;
    var min = signed ? -(2 ** (bits - 1)) : 0;
    var max = signed ? (2 ** (bits - 1)) - 1 : (2 ** bits) - 1;
    if (!Number.isInteger(value) || value < min || value > max) throw new RangeError("value is outside the range for this integer width");
    var view = new DataView(buffer.buffer, buffer.byteOffset + offset, width);
    if (width === 1) signed ? view.setInt8(0, value) : view.setUint8(0, value);
    else if (width === 2) signed ? view.setInt16(0, value, littleEndian) : view.setUint16(0, value, littleEndian);
    else signed ? view.setInt32(0, value, littleEndian) : view.setUint32(0, value, littleEndian);
    return offset + width;
  }

  NivaBuffer.prototype._isBuffer = true;
  Object.defineProperty(NivaBuffer, "name", { value: "Buffer" });
  NivaBuffer.poolSize = 8192;
  var module = { Buffer: NivaBuffer, SlowBuffer: NivaBuffer, INSPECT_MAX_BYTES: 50, kMaxLength: 0x7fffffff };
  runtime.createBufferModule = function () { return module; };
  runtime.buffer = module;
})(globalThis);
