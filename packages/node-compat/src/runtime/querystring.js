(function (root) {
  "use strict";

  var runtime = root[Symbol.for("niva.node-compat.runtime")];

  function decode(value, decoder, replacePlus) {
    var encoded = replacePlus ? String(value).replace(/\+/g, " ") : String(value);
    try { return (decoder || decodeURIComponent)(encoded); }
    catch (_) {
      if (decoder && decoder !== decodeURIComponent) return encoded;
      var bytes = [];
      var chars = Array.from(encoded);
      for (var i = 0; i < chars.length; i += 1) {
        if (chars[i] === "%" && /^[0-9a-f]{2}$/i.test(chars[i + 1] + chars[i + 2])) {
          bytes.push(parseInt(chars[i + 1] + chars[i + 2], 16));
          i += 2;
        } else {
          var encoded = new TextEncoder().encode(chars[i]);
          for (var j = 0; j < encoded.length; j += 1) bytes.push(encoded[j]);
        }
      }
      return new TextDecoder("utf-8").decode(Uint8Array.from(bytes));
    }
  }

  function parse(value, separator, equals, options) {
    var input = String(value);
    if (input.length === 0) return Object.create(null);
    separator = separator === undefined ? "&" : String(separator);
    equals = equals === undefined ? "=" : String(equals);
    options = options || {};
    var maxKeys = options.maxKeys === undefined ? 1000 : Number(options.maxKeys);
    var decoder = typeof options.decodeURIComponent === "function" ? options.decodeURIComponent : decodeURIComponent;
    var result = Object.create(null);
    var parts = input.split(separator);
    if (maxKeys > 0 && parts.length > maxKeys) parts.length = maxKeys;
    parts.forEach(function (part) {
      if (!part) return;
      var index = equals ? part.indexOf(equals) : -1;
      var key = index < 0 ? part : part.slice(0, index);
      var item = index < 0 ? "" : part.slice(index + equals.length);
      key = decode(key, decoder, true);
      item = decode(item, decoder, true);
      if (Object.prototype.hasOwnProperty.call(result, key)) {
        if (Array.isArray(result[key])) result[key].push(item);
        else result[key] = [result[key], item];
      } else {
        result[key] = item;
      }
    });
    return result;
  }

  function stringify(value, separator, equals, options) {
    if (value === null || typeof value !== "object") return "";
    separator = separator === undefined ? "&" : String(separator);
    equals = equals === undefined ? "=" : String(equals);
    options = options || {};
    var encoder = typeof options.encodeURIComponent === "function" ? options.encodeURIComponent : encodeURIComponent;
    var output = [];
    Object.keys(value).forEach(function (key) {
      var entries = Array.isArray(value[key]) ? value[key] : [value[key]];
      entries.forEach(function (item) {
        var itemString = item === null || item === undefined ? "" : String(item);
        output.push(encoder(String(key)) + equals + encoder(itemString));
      });
    });
    return output.join(separator);
  }

  var module = {
    parse: parse,
    decode: parse,
    stringify: stringify,
    encode: stringify,
    escape: encodeURIComponent,
    unescape: function (value) { return decode(value); },
  };
  runtime.createQuerystringModule = function () { return module; };
  runtime.querystring = module;
})(globalThis);
