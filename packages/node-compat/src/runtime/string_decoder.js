(function (root) {
  "use strict";

  var runtime = root[Symbol.for("niva.node-compat.runtime")];
  if (typeof runtime.createStringDecoderModule === "function") return;
  var VendorStringDecoder = runtime.vendor && runtime.vendor.StringDecoder;
  var Buffer = runtime.buffer && runtime.buffer.Buffer;
  if (typeof VendorStringDecoder !== "function" || typeof Buffer !== "function") {
    throw new Error("Load the Niva browser Buffer and vendor adapters before string_decoder.");
  }

  function codedTypeError(code, message) {
    var error = new TypeError(message);
    error.code = code;
    return error;
  }

  function received(value) {
    if (value === null) return "Received null";
    if (value === undefined) return "Received undefined";
    if (typeof value === "function") return "Received function" + (value.name ? " " + value.name : "");
    if (typeof value === "object") {
      var name;
      try { name = value.constructor && value.constructor.name; } catch (_) {}
      if (name) return "Received an instance of " + name;
    }
    return "Received type " + typeof value + (typeof value === "string" ? " ('" + value + "')" : " (" + String(value) + ")");
  }

  function normalizeError(value) {
    return codedTypeError("ERR_UNKNOWN_ENCODING", "Unknown encoding: " + String(value));
  }

  function isBase64Url(encoding) {
    return typeof encoding === "string" && encoding.toLowerCase() === "base64url";
  }

  function asBase64Url(text) {
    return text.replace(/\+/g, "-").replace(/\//g, "_").replace(/=+$/g, "");
  }

  function validBuffer(value) {
    return Buffer.isBuffer(value) || ArrayBuffer.isView(value);
  }

  function validateBuffer(value) {
    if (!validBuffer(value)) {
      throw codedTypeError("ERR_INVALID_ARG_TYPE", "The \"buf\" argument must be an instance of Buffer, TypedArray, or DataView. " + received(value));
    }
  }

  function validateThis(value) {
    if (!value || value._nivaStringDecoder !== true) {
      throw codedTypeError("ERR_INVALID_THIS", "Value of \"this\" must be of type StringDecoder");
    }
  }

  function pendingUtf8(bytes) {
    var end = bytes.length - 1;
    if (end < 0) return null;
    var first = end;
    while (first >= 0 && (bytes[first] & 0xc0) === 0x80 && end - first < 3) first--;
    if (first < 0) return null;

    var lead = bytes[first];
    var total;
    if ((lead >> 5) === 0x06) total = 2;
    else if ((lead >> 4) === 0x0e) total = 3;
    else if ((lead >> 3) === 0x1e) total = 4;
    else return null;

    var present = end - first + 1;
    if (present >= total) return null;
    for (var i = first + 1; i <= end; i++) {
      if ((bytes[i] & 0xc0) !== 0x80) return null;
    }
    return { first: first, total: total, present: present };
  }

  function makeByteView(value) {
    if (value instanceof ArrayBuffer) return new Uint8Array(value);
    if (ArrayBuffer.isView(value)) return new Uint8Array(value.buffer, value.byteOffset, value.byteLength);
    return value;
  }

  function utf8Write(value) {
    validateThis(this);
    validateBuffer(value);
    if (value.byteLength > 0x1fffffe8) {
      var sizeError = new RangeError("Cannot create a string longer than 0x1fffffe8 characters");
      sizeError.code = "ERR_STRING_TOO_LONG";
      throw sizeError;
    }
    if (value.byteLength === 0) return "";
    var input = makeByteView(value);
    var prior = this._nivaPending;
    var bytes;
    if (prior && prior.length) {
      bytes = new Uint8Array(prior.length + input.length);
      bytes.set(prior, 0);
      bytes.set(input, prior.length);
    } else {
      bytes = input;
    }
    var pending = pendingUtf8(bytes);
    var completeEnd = pending ? pending.first : bytes.length;
    var result = this._nivaTextDecoder.decode(bytes.subarray(0, completeEnd), { stream: true });
    this._nivaPending = pending ? bytes.slice(pending.first) : new Uint8Array(0);
    this.lastNeed = pending ? pending.total - pending.present : 0;
    this.lastTotal = pending ? pending.total : 0;
    if (pending) {
      for (var i = 0; i < 4; i++) this.lastChar[i] = i < pending.present ? bytes[pending.first + i] : 0;
    }
    return result;
  }

  function utf8End(value) {
    validateThis(this);
    var result = "";
    if (value !== undefined && value !== null && value.byteLength !== 0) result = utf8Write.call(this, value);
    if (this._nivaPending.length) result += this._nivaTextDecoder.decode(this._nivaPending);
    else result += this._nivaTextDecoder.decode();
    this._nivaPending = new Uint8Array(0);
    this.lastNeed = 0;
    this.lastTotal = 0;
    return result;
  }

  function decoderWrite(value) {
    validateThis(this);
    validateBuffer(value);
    return this._nivaWriteImpl(value);
  }

  function decoderEnd(value) {
    validateThis(this);
    if (value !== undefined && value !== null) validateBuffer(value);
    var result = this._nivaEndImpl(value);
    this.lastNeed = 0;
    this.lastTotal = 0;
    return result;
  }

  function StringDecoder(encoding) {
    if (this == null) return new StringDecoder(encoding);

    var base64url = isBase64Url(encoding);
    try {
      VendorStringDecoder.call(this, base64url ? "base64" : encoding);
    } catch (error) {
      if (error && /Unknown encoding:/.test(error.message)) throw normalizeError(encoding);
      throw error;
    }

    this._nivaStringDecoder = true;
    var ownWrite = Object.prototype.hasOwnProperty.call(this, "write") ? this.write : VendorStringDecoder.prototype.write;
    var ownEnd = Object.prototype.hasOwnProperty.call(this, "end") ? this.end : VendorStringDecoder.prototype.end;
    this._nivaWriteImpl = function (value) {
      var result = ownWrite.call(this, value);
      return base64url ? asBase64Url(result) : result;
    };
    this._nivaEndImpl = function (value) {
      var result = ownEnd.call(this, value);
      return base64url ? asBase64Url(result) : result;
    };
    delete this.write;
    delete this.end;

    if (base64url) this.encoding = "base64url";
    if (this.encoding === "utf8") {
      if (typeof root.TextDecoder !== "function") throw new Error("TextDecoder is required for UTF-8 StringDecoder support.");
      this._nivaTextDecoder = new root.TextDecoder("utf-8", { ignoreBOM: true });
      this.lastNeed = 0;
      this.lastTotal = 0;
      this.lastChar = Buffer.alloc(4, 0);
      this._nivaPending = new Uint8Array(0);
      this._nivaWriteImpl = utf8Write;
      this._nivaEndImpl = utf8End;
    }
  }

  StringDecoder.prototype = Object.create(VendorStringDecoder.prototype);
  Object.defineProperty(StringDecoder.prototype, "constructor", { value: StringDecoder, writable: true, configurable: true });
  StringDecoder.prototype.write = decoderWrite;
  StringDecoder.prototype.end = decoderEnd;

  var module = { StringDecoder: StringDecoder };
  runtime.createStringDecoderModule = function () { return module; };
  runtime.stringDecoder = module;
})(globalThis);
