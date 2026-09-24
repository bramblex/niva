// Copyright Joyent, Inc. and other Node contributors.
//
// Permission is hereby granted, free of charge, to any person obtaining a
// copy of this software and associated documentation files (the
// "Software"), to deal in the Software without restriction, including
// without limitation the rights to use, copy, modify, merge, publish,
// distribute, sublicense, and/or sell copies of the Software, and to permit
// persons to whom the Software is furnished to do so, subject to the
// following conditions:
//
// The above copyright notice and this permission notice shall be included
// in all copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS
// OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF
// MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN
// NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM,
// DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR
// OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE
// USE OR OTHER DEALINGS IN THE SOFTWARE.
// Adapted from Node.js v22.14.0, commit 5d2feb257bcee090e57900eb51720171a6aa92f3:
// https://github.com/nodejs/node/blob/5d2feb257bcee090e57900eb51720171a6aa92f3/lib/querystring.js
// https://github.com/nodejs/node/blob/5d2feb257bcee090e57900eb51720171a6aa92f3/lib/internal/querystring.js
(function (root) {
  "use strict";

  var runtime = root[Symbol.for("niva.node-compat.runtime")];
  if (typeof runtime.createQuerystringModule === "function") return;

  function hexByte(value) {
    return "%" + (value < 16 ? "0" : "") + value.toString(16).toUpperCase();
  }

  function encodeString(value) {
    var length = value.length;
    if (length === 0) return "";
    var output = "";
    var lastPosition = 0;
    var i = 0;
    outer: for (; i < length; i += 1) {
      var code = value.charCodeAt(i);
      while (code < 0x80) {
        var safe = code >= 65 && code <= 90 || code >= 97 && code <= 122 ||
          code >= 48 && code <= 57 || code === 45 || code === 95 || code === 46 ||
          code === 33 || code === 126 || code === 42 || code === 39 || code === 40 || code === 41;
        if (!safe) {
          if (lastPosition < i) output += value.slice(lastPosition, i);
          lastPosition = i + 1;
          output += hexByte(code);
        }
        if (++i === length) break outer;
        code = value.charCodeAt(i);
      }

      if (lastPosition < i) output += value.slice(lastPosition, i);
      if (code < 0x800) {
        lastPosition = i + 1;
        output += hexByte(0xC0 | (code >> 6)) + hexByte(0x80 | (code & 0x3F));
      } else if (code < 0xD800 || code >= 0xE000) {
        lastPosition = i + 1;
        output += hexByte(0xE0 | (code >> 12)) + hexByte(0x80 | ((code >> 6) & 0x3F)) + hexByte(0x80 | (code & 0x3F));
      } else {
        i += 1;
        if (i >= length) {
          var uriError = new URIError("URI malformed");
          uriError.code = "ERR_INVALID_URI";
          throw uriError;
        }
        var low = value.charCodeAt(i) & 0x3FF;
        lastPosition = i + 1;
        code = 0x10000 + (((code & 0x3FF) << 10) | low);
        output += hexByte(0xF0 | (code >> 18)) + hexByte(0x80 | ((code >> 12) & 0x3F)) +
          hexByte(0x80 | ((code >> 6) & 0x3F)) + hexByte(0x80 | (code & 0x3F));
      }
    }
    if (lastPosition === 0) return value;
    return lastPosition < length ? output + value.slice(lastPosition) : output;
  }

  function escape(value) {
    if (typeof value !== "string") {
      if (typeof value === "object") value = String(value);
      else value += "";
    }
    return encodeString(value);
  }

  function unescapeBuffer(value, decodeSpaces) {
    var input = String(value);
    var output = new Uint8Array(input.length);
    var outputIndex = 0;
    var maxPercentIndex = input.length - 2;
    var hasHex = false;
    for (var index = 0; index < input.length;) {
      var code = input.charCodeAt(index);
      if (code === 43 && decodeSpaces) {
        output[outputIndex++] = 32;
        index += 1;
        continue;
      }
      if (code === 37 && index < maxPercentIndex) {
        var highChar = input.charCodeAt(++index);
        var high = parseInt(String.fromCharCode(highChar), 16);
        if (!(high >= 0 && high <= 15 && /^[0-9a-f]$/i.test(String.fromCharCode(highChar)))) {
          output[outputIndex++] = 37;
          continue;
        }
        code = highChar;
        var lowChar = input.charCodeAt(++index);
        var lowText = String.fromCharCode(lowChar);
        var low = parseInt(lowText, 16);
        if (!(low >= 0 && low <= 15 && /^[0-9a-f]$/i.test(lowText))) {
          output[outputIndex++] = 37;
          index -= 1;
        } else {
          hasHex = true;
          code = high * 16 + low;
        }
      }
      output[outputIndex++] = code;
      index += 1;
    }
    var bytes = hasHex ? output.slice(0, outputIndex) : output.slice(0, outputIndex);
    var bufferModule = runtime.buffer;
    if (bufferModule && bufferModule.Buffer) return bufferModule.Buffer.from(bytes);
    var fallback = bytes;
    Object.defineProperty(fallback, "toString", {
      configurable: true,
      value: function () { return new TextDecoder("utf-8").decode(fallback); },
    });
    return fallback;
  }

  function unescape(value, decodeSpaces) {
    var input = String(value);
    try { return decodeURIComponent(input); }
    catch (_) { return unescapeBuffer(input, decodeSpaces).toString(); }
  }

  function decodePart(value, decoder, custom, isKey) {
    if (isKey && value.length === 0) return "";
    if (!custom && !/%[0-9a-f]{2}/i.test(value)) return value;
    try { return decoder(value); }
    catch (_) { return unescape(value, true); }
  }

  function parse(value, separator, equals, options) {
    var result = Object.create(null);
    if (typeof value !== "string" || value.length === 0) return result;

    separator = separator || "&";
    equals = equals || "=";
    separator = String(separator);
    equals = String(equals);
    options = options || {};

    var maxKeys = 1000;
    if (typeof options.maxKeys === "number") maxKeys = options.maxKeys > 0 ? options.maxKeys : -1;
    var decoder = typeof options.decodeURIComponent === "function" ? options.decodeURIComponent : module.unescape;
    var customDecoder = decoder !== unescape;
    var parts = separator.length === 0 ? [value] : value.split(separator);
    var processed = 0;
    for (var i = 0; i < parts.length && (maxKeys < 0 || processed < maxKeys); i += 1) {
      processed += 1;
      var part = parts[i];
      if (part.length === 0) continue;
      var splitAt = equals.length === 0 ? 0 : part.indexOf(equals);
      var rawKey = splitAt < 0 ? part : part.slice(0, splitAt);
      var rawValue = splitAt < 0 ? "" : part.slice(splitAt + equals.length);
      var plusReplacement = customDecoder ? "%20" : " ";
      rawKey = rawKey.replace(/\+/g, plusReplacement);
      rawValue = rawValue.replace(/\+/g, plusReplacement);
      var key = decodePart(rawKey, decoder, customDecoder, true);
      var item = decodePart(rawValue, decoder, customDecoder, false);
      if (Object.prototype.hasOwnProperty.call(result, key)) {
        if (Array.isArray(result[key])) result[key].push(item);
        else result[key] = [result[key], item];
      } else {
        result[key] = item;
      }
    }
    return result;
  }

  function stringifyPrimitive(value) {
    if (typeof value === "string") return value;
    if (typeof value === "number" && Number.isFinite(value)) return "" + value;
    if (typeof value === "bigint") return "" + value;
    if (typeof value === "boolean") return value ? "true" : "false";
    return "";
  }

  function encodeStringified(value, encoder) {
    if (typeof value === "string") return value.length ? encoder(value) : "";
    if (typeof value === "number" && Number.isFinite(value)) {
      return Math.abs(value) < 1e21 ? "" + value : encoder("" + value);
    }
    if (typeof value === "bigint") return "" + value;
    if (typeof value === "boolean") return value ? "true" : "false";
    return "";
  }

  function stringify(value, separator, equals, options) {
    separator = separator || "&";
    equals = equals || "=";
    options = options || {};
    var encoder = module.escape;
    if (typeof options.encodeURIComponent === "function") encoder = options.encodeURIComponent;
    var customEncoder = encoder !== escape;
    var convert = customEncoder
      ? function (item) { return encoder(stringifyPrimitive(item)); }
      : function (item) { return encodeStringified(item, encoder); };
    if (value === null || typeof value !== "object") return "";

    var keys = Object.keys(value);
    var fields = "";
    for (var i = 0; i < keys.length; i += 1) {
      var key = keys[i];
      var encodedKey = convert(key) + equals;
      var item = value[key];
      if (Array.isArray(item)) {
        if (item.length === 0) continue;
        if (fields) fields += separator;
        for (var j = 0; j < item.length; j += 1) {
          if (j) fields += separator;
          fields += encodedKey + convert(item[j]);
        }
      } else {
        if (fields) fields += separator;
        fields += encodedKey + convert(item);
      }
    }
    return fields;
  }

  var module = {
    unescapeBuffer: unescapeBuffer,
    parse: parse,
    decode: parse,
    stringify: stringify,
    encode: stringify,
    escape: escape,
    unescape: unescape,
  };
  runtime.createQuerystringModule = function () { return module; };
  runtime.querystring = module;
})(globalThis);
