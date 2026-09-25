(function (root: any) {
  "use strict";

  var runtime = root[Symbol.for("niva.node-compat.runtime")];
  if (typeof runtime.createBufferModule === "function") return;
  var vendor = runtime.vendor;
  if (!vendor || typeof vendor.Buffer !== "function") {
    throw new Error("Load the Niva browser vendor bundle before the Buffer adapter.");
  }

  var Buffer = vendor.Buffer;
  // Keep the public constructor identity stable after vendor minification.
  Object.defineProperty(Buffer, "name", {__proto__: null, value: "Buffer", configurable: true});
  var maxLength = Number.MAX_SAFE_INTEGER;
  var maxStringLength = 0x1fffffe8;
  var nativeIsEncoding = Buffer.isEncoding;
  var nativeFrom = Buffer.from;
  var nativeByteLength = Buffer.byteLength;
  var nativeCompare = Buffer.compare;
  var nativeConcat = Buffer.concat;
  var nativeAlloc = Buffer.alloc;
  var nativeWrite = Buffer.prototype.write;
  var nativeToString = Buffer.prototype.toString;
  var nativeFill = Buffer.prototype.fill;
  var nativeCompareInstance = Buffer.prototype.compare;
  var nativeCopy = Buffer.prototype.copy;
  var nativeIndexOf = Buffer.prototype.indexOf;
  var nativeLastIndexOf = Buffer.prototype.lastIndexOf;
  var fromDepth = 0;
  var textDecoder = typeof root.TextDecoder === "function" ? new root.TextDecoder("utf-8", { ignoreBOM: true }) : null;
  var typedArrayLengthGetter = Object.getOwnPropertyDescriptor(Object.getPrototypeOf(Uint8Array.prototype), "length").get;

  function actualLength(buffer) {
    return typedArrayLengthGetter.call(buffer);
  }

  function isArrayBuffer(value) {
    return value instanceof ArrayBuffer || Object.prototype.toString.call(value) === "[object ArrayBuffer]";
  }

  function isSharedArrayBuffer(value) {
    return (typeof SharedArrayBuffer === "function" && value instanceof SharedArrayBuffer) ||
      Object.prototype.toString.call(value) === "[object SharedArrayBuffer]";
  }

  function isUint8Array(value) {
    return ArrayBuffer.isView(value) && Object.prototype.toString.call(value) === "[object Uint8Array]";
  }

  function isBase64Url(encoding) {
    return typeof encoding === "string" && encoding.toLowerCase() === "base64url";
  }

  function asBase64(encoding) {
    return isBase64Url(encoding) ? "base64" : encoding;
  }

  function valueDescription(value) {
    if (value === null) return "Received null";
    if (value === undefined) return "Received undefined";
    var type = typeof value;
    if (type === "function") return "Received function " + (value.name || "");
    if (type === "string") {
      var inspected = "'" + value.replace(/\\/g, "\\\\").replace(/'/g, "\\'").replace(/\n/g, "\\n").replace(/\r/g, "\\r").replace(/\t/g, "\\t") + "'";
      if (inspected.length > 28) inspected = inspected.slice(0, 25) + "...";
      return "Received type string (" + inspected + ")";
    }
    if (type === "number") return "Received type number (" + String(value) + ")";
    if (type === "boolean") return "Received type boolean (" + String(value) + ")";
    if (type === "bigint") return "Received type bigint (" + String(value) + "n)";
    if (type === "symbol") return "Received type symbol (" + String(value) + ")";
    if (type === "object") {
      var constructorName;
      try { constructorName = value.constructor && value.constructor.name; } catch (_) {}
      if (constructorName) return "Received an instance of " + constructorName;
      try { if (Object.getPrototypeOf(value) === null) return "Received [Object: null prototype] {}"; } catch (_) {}
      return "Received an object";
    }
    return "Received type " + type;
  }

  function typeError(argument, expectation, value, prefix?, description?) {
    var error = new TypeError((prefix || "The \"" + argument + "\" argument must be ") + expectation + ". " + (description || valueDescription(value)));
    error.code = "ERR_INVALID_ARG_TYPE";
    return error;
  }

  function codedError(name, code, message) {
    var error = new name(message);
    error.code = code;
    return error;
  }

  function outOfRange(argument, value, min, max) {
    return codedError(RangeError, "ERR_OUT_OF_RANGE",
      "The value of \"" + argument + "\" is out of range. It must be >= " + min + " && <= " + max + ". Received " + String(value));
  }

  Buffer.kMaxLength = maxLength;
  Buffer.isEncoding = function (encoding) {
    return isBase64Url(encoding) || (typeof encoding === "string" && nativeIsEncoding.call(Buffer, encoding));
  };

  Buffer.alloc = function (size, fill, encoding) {
    if (typeof fill === "string" && encoding !== undefined && typeof encoding !== "string") {
      throw typeError("encoding", "of type string", encoding);
    }
    return nativeAlloc.call(Buffer, size, fill, asBase64(encoding));
  };

  Buffer.from = function (value, encodingOrOffset, length) {
    var encoding = asBase64(encodingOrOffset);
    fromDepth++;
    try {
      if (value === null || value === undefined || typeof value === "number" || typeof value === "boolean" ||
          typeof value === "symbol" || typeof value === "bigint" || typeof value === "function") {
        if (fromDepth > 1) throw new TypeError("Invalid Buffer.from input");
        throw typeError("first", "of type string or an instance of Buffer, ArrayBuffer, or Array or an Array-like Object", value,
          "The first argument must be ");
      }
      return nativeFrom.call(Buffer, value, encoding, length);
    } catch (error) {
      if (fromDepth === 1 && error instanceof TypeError && error.code === undefined &&
          !(typeof value === "string" && typeof encoding === "string" && !Buffer.isEncoding(encoding))) {
        throw typeError("first", "of type string or an instance of Buffer, ArrayBuffer, or Array or an Array-like Object", value,
          "The first argument must be ");
      }
      if (fromDepth === 1 && error instanceof TypeError && error.message.indexOf("Unknown encoding:") === 0) {
        error.code = "ERR_UNKNOWN_ENCODING";
      }
      if (error instanceof RangeError && /outside of buffer bounds/.test(error.message)) {
        error.code = "ERR_BUFFER_OUT_OF_BOUNDS";
      }
      throw error;
    } finally {
      fromDepth--;
    }
  };

  Buffer.byteLength = function (value, encoding) {
    if (typeof value !== "string" && !Buffer.isBuffer(value) && !ArrayBuffer.isView(value) &&
        !isArrayBuffer(value) && !isSharedArrayBuffer(value)) {
      throw typeError("string", "of type string or an instance of Buffer or ArrayBuffer", value);
    }
    return nativeByteLength.call(Buffer, value, asBase64(encoding));
  };

  Buffer.compare = function (left, right) {
    if (!isUint8Array(left)) {
      throw typeError("buf1", "an instance of Buffer or Uint8Array", left);
    }
    if (!isUint8Array(right)) {
      throw typeError("buf2", "an instance of Buffer or Uint8Array", right);
    }
    if (!Buffer.isBuffer(left)) left = Buffer.from(left);
    if (!Buffer.isBuffer(right)) right = Buffer.from(right);
    return nativeCompare.call(Buffer, left, right);
  };

  Buffer.concat = function (list, length) {
    if (!Array.isArray(list)) {
      throw typeError("list", "an instance of Array", list);
    }
    for (var i = 0; i < list.length; i++) {
      if (!isUint8Array(list[i])) {
        throw typeError("list[" + i + "]", "an instance of Buffer or Uint8Array", list[i]);
      }
    }
    return nativeConcat.call(Buffer, list, length);
  };

  Buffer.prototype.toString = function (encoding, start, end) {
    if (isBase64Url(encoding)) {
      return nativeToString.call(this, "base64", start, end).replace(/\+/g, "-").replace(/\//g, "_").replace(/=+$/g, "");
    }
    if (encoding !== undefined && String(encoding).toLowerCase() !== "utf8" && String(encoding).toLowerCase() !== "utf-8") {
      return nativeToString.call(this, encoding, start, end);
    }
    if (!textDecoder) return nativeToString.call(this, encoding, start, end);
    var length = actualLength(this);
    if (length === 0) return "";
    if (start === undefined || start < 0) start = 0;
    if (start > length) return "";
    if (end === undefined || end > length) end = length;
    if (end <= 0) return "";
    end >>>= 0;
    start >>>= 0;
    if (end <= start) return "";
    return textDecoder.decode(new Uint8Array(this.buffer, this.byteOffset + start, end - start));
  };
  Buffer.prototype.toLocaleString = Buffer.prototype.toString;

  Buffer.prototype.write = function (value, offset, length, encoding) {
    if (typeof value !== "string") {
      throw typeError("string", "of type string", value, "The \"string\" argument must be ");
    }

    var argc = arguments.length;
    var callArgs = [value, offset, length, encoding].slice(0, argc);
    if (typeof offset === "string") {
      if (argc > 2) {
        throw typeError("offset", "of type number", offset);
      }
      offset = asBase64(offset);
      callArgs[1] = offset;
    }
    if (typeof length === "string" && encoding === undefined) {
      length = asBase64(length);
      callArgs[2] = length;
    }
    if (isBase64Url(encoding)) {
      encoding = "base64";
      callArgs[3] = encoding;
    }

    var numericOffset = offset === undefined || typeof offset === "string" ? 0 : Number(offset);
    if (!Number.isFinite(numericOffset) || !Number.isInteger(numericOffset)) {
      throw typeError("offset", "of type number", offset);
    }
    if (numericOffset < 0 || numericOffset > this.length) {
      throw outOfRange("offset", offset, 0, this.length);
    }
    if (typeof length === "number" && (length < 0 || !Number.isInteger(length))) {
      throw outOfRange("length", length, 0, this.length - numericOffset);
    }

    try {
      return nativeWrite.apply(this, callArgs);
    } catch (error) {
      if (error instanceof RangeError && error.code === undefined) {
        error.code = "ERR_OUT_OF_RANGE";
        if (error.message === "Attempt to write outside buffer bounds") {
          error.message = "The value of \"offset\" is out of range. It must be >= 0 && <= " + this.length + ". Received " + String(offset);
        }
      }
      if (error instanceof TypeError && error.message.indexOf("Unknown encoding:") === 0) error.code = "ERR_UNKNOWN_ENCODING";
      throw error;
    }
  };

  Buffer.prototype.fill = function (value, start, end, encoding) {
    if (typeof start === "string") {
      encoding = start;
      start = 0;
      end = this.length;
    } else if (typeof end === "string") {
      encoding = end;
      end = this.length;
    }
    if (encoding !== undefined && typeof encoding !== "string") {
      throw typeError("encoding", "of type string", encoding);
    }
    if (isBase64Url(encoding)) encoding = "base64";
    if (Array.isArray(value) && value.length === 0) value = 0;

    var length = actualLength(this);
    if (this.length !== length) {
      throw codedError(RangeError, "ERR_BUFFER_OUT_OF_BOUNDS", "Attempt to access memory outside buffer bounds");
    }
    var startValue = start === undefined ? 0 : start;
    var endValue = end === undefined ? length : end;
    if (typeof startValue !== "number" || Number.isNaN(startValue)) {
      throw typeError("start", "of type number", startValue);
    }
    if (typeof endValue !== "number" || Number.isNaN(endValue)) {
      throw typeError("end", "of type number", endValue);
    }
    if (startValue < 0 || startValue > length) throw outOfRange("start", startValue, 0, length);
    if (endValue < 0 || endValue > length) throw outOfRange("end", endValue, 0, length);

    try {
      return nativeFill.call(this, value, startValue, endValue, encoding);
    } catch (error) {
      if (error instanceof TypeError && error.message.indexOf("Unknown encoding:") === 0) error.code = "ERR_UNKNOWN_ENCODING";
      if (error instanceof TypeError && error.message.indexOf("The value ") === 0) error.code = "ERR_INVALID_ARG_VALUE";
      throw error;
    }
  };

  Buffer.prototype.compare = function (target) {
    if (!isUint8Array(target)) {
      throw typeError("target", "an instance of Buffer or Uint8Array", target);
    }
    if (!Buffer.isBuffer(target)) target = Buffer.from(target);
    var args = Array.prototype.slice.call(arguments);
    args[0] = target;
    try { return nativeCompareInstance.apply(this, args); }
    catch (error) { throw error; }
  };

  Buffer.prototype.copy = function () {
    var target = arguments[0];
    if (!isUint8Array(target)) throw typeError("target", "an instance of Buffer or Uint8Array", target);
    if (!Buffer.isBuffer(target)) {
      var args = Array.prototype.slice.call(arguments);
      args[0] = Buffer.from(target.buffer, target.byteOffset, target.byteLength);
      return nativeCopy.apply(this, args);
    }
    try { return nativeCopy.apply(this, arguments); }
    catch (error) {
      if (error instanceof RangeError && !error.code) error.code = "ERR_OUT_OF_RANGE";
      throw error;
    }
  };

  function isEmptyNeedle(value, encoding) {
    if (typeof value === "string") return Buffer.byteLength(value, encoding) === 0;
    if (ArrayBuffer.isView(value)) return value.byteLength === 0;
    if (value instanceof ArrayBuffer) return value.byteLength === 0;
    return false;
  }

  function normalizeOffset(buffer, offset, reverse) {
    if (typeof offset === "string") offset = undefined;
    if (offset === undefined) return reverse ? buffer.length : 0;
    offset = Number(offset);
    if (Number.isNaN(offset)) offset = 0;
    offset = Math.trunc(offset);
    if (offset < 0) offset = Math.max(buffer.length + offset, 0);
    return Math.min(offset, buffer.length);
  }

  Buffer.prototype.indexOf = function indexOf(value, byteOffset, encoding) {
    if (!Buffer.isBuffer(this)) {
      throw typeError("buffer", "an instance of Buffer, TypedArray, or DataView", this);
    }
    if (typeof value !== "number" && typeof value !== "string" && !Buffer.isBuffer(value) && !isUint8Array(value)) {
      throw typeError("value", "one of type number or string or an instance of Buffer or Uint8Array", value,
        "The \"value\" argument must be ");
    }
    if (typeof byteOffset === "string") byteOffset = asBase64(byteOffset);
    if (isBase64Url(encoding)) encoding = "base64";
    if (isEmptyNeedle(value, typeof byteOffset === "string" ? byteOffset : encoding)) return normalizeOffset(this, byteOffset, false);
    var haystack = this;
    var searchEncoding = typeof byteOffset === "string" ? byteOffset : encoding;
    if (typeof searchEncoding === "string" && /^(ucs2|ucs-2|utf16le|utf-16le)$/i.test(searchEncoding) && (actualLength(this) & 1)) {
      haystack = this.subarray(0, actualLength(this) - 1);
    }
    if (isUint8Array(value) && !Buffer.isBuffer(value)) value = Buffer.from(value);
    return nativeIndexOf.call(haystack, value, byteOffset, encoding);
  };
  Buffer.prototype.lastIndexOf = function lastIndexOf(value, byteOffset, encoding) {
    if (!Buffer.isBuffer(this)) {
      if (this && Object.getPrototypeOf(this) === Buffer.prototype.lastIndexOf.prototype) {
        throw typeError("buffer", "an instance of Buffer, TypedArray, or DataView", this,
          "The \"buffer\" argument must be ", "Received an instance of lastIndexOf");
      }
      throw typeError("buffer", "an instance of Buffer, TypedArray, or DataView", this);
    }
    if (typeof value !== "number" && typeof value !== "string" && !Buffer.isBuffer(value) && !isUint8Array(value)) {
      throw typeError("value", "one of type number or string or an instance of Buffer or Uint8Array", value,
        "The \"value\" argument must be ");
    }
    if (typeof byteOffset === "string") byteOffset = asBase64(byteOffset);
    if (isBase64Url(encoding)) encoding = "base64";
    if (isEmptyNeedle(value, typeof byteOffset === "string" ? byteOffset : encoding)) return normalizeOffset(this, byteOffset, true);
    var haystack = this;
    var searchEncoding = typeof byteOffset === "string" ? byteOffset : encoding;
    if (typeof searchEncoding === "string" && /^(ucs2|ucs-2|utf16le|utf-16le)$/i.test(searchEncoding) && (actualLength(this) & 1)) {
      haystack = this.subarray(0, actualLength(this) - 1);
    }
    if (isUint8Array(value) && !Buffer.isBuffer(value)) value = Buffer.from(value);
    return nativeLastIndexOf.call(haystack, value, byteOffset, encoding);
  };

  Buffer.prototype.equals = function (value) {
    if (!Buffer.isBuffer(value) && !isUint8Array(value)) throw new TypeError("Argument must be a Buffer");
    if (!Buffer.isBuffer(value)) value = Buffer.from(value);
    return Buffer.compare(this, value) === 0;
  };

  Buffer.copyBytesFrom = function (view, offset, length) {
    if (!ArrayBuffer.isView(view) || Object.prototype.toString.call(view) === "[object DataView]") {
      throw typeError("view", "an instance of TypedArray", view);
    }
    var typedView = view as ArrayBufferView & { length: number; BYTES_PER_ELEMENT: number };
    var elementCount = typedView.length;
    var bytesPerElement = typedView.BYTES_PER_ELEMENT;
    if (elementCount === 0) return Buffer.alloc(0);
    if (offset !== undefined) {
      if (typeof offset !== "number") throw typeError("offset", "of type number", offset);
      if (!Number.isInteger(offset) || offset < 0) throw outOfRange("offset", offset, 0, elementCount - 1);
      if (offset >= elementCount) return Buffer.alloc(0);
    } else {
      offset = 0;
    }
    if (length !== undefined) {
      if (typeof length !== "number") throw typeError("length", "of type number", length);
      if (!Number.isInteger(length) || length < 0) throw outOfRange("length", length, 0, elementCount - offset);
    } else {
      length = elementCount - offset;
    }
    var end = Math.min(elementCount, offset + length);
    var byteView = new Uint8Array(view.buffer, view.byteOffset + offset * bytesPerElement, (end - offset) * bytesPerElement);
    return Buffer.from(byteView);
  };

  ["writeFloatLE", "writeFloatBE", "writeDoubleLE", "writeDoubleBE", "writeInt8", "writeUInt8",
    "writeInt16LE", "writeInt16BE", "writeUInt16LE", "writeUInt16BE", "writeInt32LE", "writeInt32BE",
    "writeUInt32LE", "writeUInt32BE", "writeBigInt64LE", "writeBigInt64BE", "writeBigUInt64LE", "writeBigUInt64BE",
    "writeIntLE", "writeIntBE", "writeUIntLE", "writeUIntBE"].forEach(function (name) {
    var method = Buffer.prototype[name];
    if (typeof method !== "function") return;
    Buffer.prototype[name] = function () {
      try { return method.apply(this, arguments); }
      catch (error) {
        if (error instanceof RangeError && !error.code) error.code = "ERR_OUT_OF_RANGE";
        throw error;
      }
    };
  });

  ["asciiWrite", "latin1Write", "utf8Write"].forEach(function (name) {
    var encoding = name === "asciiWrite" ? "ascii" : name === "latin1Write" ? "latin1" : "utf8";
    Buffer.prototype[name] = function (value, offset, length) {
      if (typeof length === "number" && length < 0) {
        throw codedError(RangeError, "ERR_BUFFER_OUT_OF_BOUNDS", "Attempt to access memory outside buffer bounds");
      }
      return Buffer.prototype.write.call(this, value, offset, length, encoding);
    };
  });

  var module = {
    Buffer: Buffer,
    SlowBuffer: Buffer,
    INSPECT_MAX_BYTES: 50,
    kMaxLength: maxLength,
    constants: { MAX_LENGTH: maxLength, MAX_STRING_LENGTH: maxStringLength },
  };

  runtime.bufferBinding = {
    fill: function (buffer, value, start, end, encoding) {
      if (!Buffer.isBuffer(buffer)) throw typeError("buffer", "an instance of Buffer", buffer);
      if (typeof start !== "number" || Number.isNaN(start)) throw typeError("start", "of type number", start);
      if (typeof end !== "number" || Number.isNaN(end)) throw typeError("end", "of type number", end);
      if (start < 0 || start > buffer.length) throw outOfRange("start", start, 0, buffer.length);
      if (end < 0 || end > buffer.length) throw outOfRange("end", end, 0, buffer.length);
      return Buffer.prototype.fill.call(buffer, value, start, end, encoding);
    },
  };

  runtime.createBufferModule = function () { return module; };
  runtime.buffer = module;
})(globalThis);
