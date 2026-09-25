(function (root: any) {
  "use strict";

  var runtime = root[Symbol.for("niva.node-compat.runtime")];
  if (typeof runtime.createUrlModule === "function") return;
  var NativeURL = root.URL;
  var NativeURLSearchParams = root.URLSearchParams;

  function useWindows(options) {
    return options && typeof options.windows === "boolean" ? options.windows : runtime.path.sep === "\\";
  }

  function describeInput(value) {
    if (value === null) return "Received null";
    if (value === undefined) return "Received undefined";
    var type = typeof value;
    if (type === "string") return "Received type string ('" + value.replace(/\\/g, "\\\\").replace(/'/g, "\\'") + "')";
    if (type === "number" || type === "boolean") return "Received type " + type + " (" + String(value) + ")";
    if (type === "bigint") return "Received type bigint (" + String(value) + "n)";
    if (type === "symbol") return "Received type symbol (" + String(value) + ")";
    if (type === "function") return "Received function" + (value.name ? " " + value.name : "");
    if (type === "object") {
      var constructorName;
      try { constructorName = value.constructor && value.constructor.name; } catch (_) {}
      return constructorName ? "Received an instance of " + constructorName : "Received an object";
    }
    return "Received type " + type;
  }

  function invalidPathType(value, acceptsUrl) {
    var types = acceptsUrl ? "of type string or an instance of URL" : "of type string";
    var error = new TypeError('The "path" argument must be ' + types + ". " + describeInput(value));
    error.code = "ERR_INVALID_ARG_TYPE";
    return error;
  }

  function encodeFileUrlPath(path) {
    var output = "";
    function isSafe(code) {
      return code >= 65 && code <= 90 || code >= 97 && code <= 122 || code >= 48 && code <= 57 ||
        code === 33 || code === 36 || code === 38 || code === 39 || code === 40 || code === 41 ||
        code === 42 || code === 43 || code === 44 || code === 45 || code === 46 || code === 47 ||
        code === 58 || code === 59 || code === 61 || code === 64 || code === 95;
    }
    function appendByte(byte) {
      output += "%" + (byte < 16 ? "0" : "") + byte.toString(16).toUpperCase();
    }
    for (var index = 0; index < path.length; index += 1) {
      var code = path.charCodeAt(index);
      if (code < 0x80) {
        if (isSafe(code)) output += String.fromCharCode(code);
        else appendByte(code);
        continue;
      }
      if (code >= 0xD800 && code <= 0xDBFF) {
        var next = index + 1 < path.length ? path.charCodeAt(index + 1) : 0;
        if (next >= 0xDC00 && next <= 0xDFFF) {
          code = 0x10000 + ((code - 0xD800) << 10) + (next - 0xDC00);
          index += 1;
        } else {
          code = 0xFFFD;
        }
      } else if (code >= 0xDC00 && code <= 0xDFFF) {
        code = 0xFFFD;
      }
      if (code < 0x800) {
        appendByte(0xC0 | (code >> 6));
        appendByte(0x80 | (code & 0x3F));
      } else if (code < 0x10000) {
        appendByte(0xE0 | (code >> 12));
        appendByte(0x80 | ((code >> 6) & 0x3F));
        appendByte(0x80 | (code & 0x3F));
      } else {
        appendByte(0xF0 | (code >> 18));
        appendByte(0x80 | ((code >> 12) & 0x3F));
        appendByte(0x80 | ((code >> 6) & 0x3F));
        appendByte(0x80 | (code & 0x3F));
      }
    }
    return output;
  }

  function fileURLToPath(input, options) {
    if (typeof input !== "string" && !(input instanceof NativeURL)) throw invalidPathType(input, true);
    var url;
    try { url = input instanceof NativeURL ? input : new NativeURL(input); }
    catch (error) { throw runtime.bridgeError("The URL must be valid", "ERR_INVALID_URL", { cause: error }); }
    if (url.protocol !== "file:") throw runtime.bridgeError("The URL must use the file: scheme", "ERR_INVALID_URL_SCHEME");
    var windows = useWindows(options);
    if (windows ? /%2f|%5c/i.test(url.pathname) : /%2f/i.test(url.pathname)) {
      throw runtime.bridgeError("Encoded path separators are not allowed in file URLs", "ERR_INVALID_FILE_URL_PATH");
    }
    var decoded;
    try { decoded = decodeURIComponent(url.pathname); }
    catch (error) { throw runtime.bridgeError("The file URL pathname contains invalid percent encoding", "ERR_INVALID_FILE_URL_PATH", { cause: error }); }
    if (url.hostname && url.hostname.toLowerCase() !== "localhost") {
      if (!windows) throw runtime.bridgeError("File URL host must be empty or localhost on this platform", "ERR_INVALID_FILE_URL_HOST");
      return "\\\\" + url.hostname + decoded.replace(/\//g, "\\");
    }
    if (windows) {
      if (!/^\/[A-Za-z]:\//.test(decoded)) {
        throw runtime.bridgeError("File URL path must be absolute", "ERR_INVALID_FILE_URL_PATH");
      }
      decoded = decoded.slice(1);
      return decoded.replace(/\//g, "\\");
    }
    if (!decoded.startsWith("/")) throw runtime.bridgeError("File URL path must be absolute", "ERR_INVALID_FILE_URL_PATH");
    return decoded || "/";
  }

  function pathToFileURL(input, options) {
    if (typeof input !== "string") throw invalidPathType(input, false);
    var windows = useWindows(options);
    var absolute = (windows ? runtime.path.win32 : runtime.path.posix).resolve(input);
    var trailingSeparator = windows ? /[\\/]$/.test(input) : input.endsWith("/");
    var pathSeparator = windows ? "\\" : "/";
    if (trailingSeparator && !absolute.endsWith(pathSeparator)) absolute += pathSeparator;
    var url;
    if (windows) {
      var normalized = absolute.replace(/\\/g, "/");
      if (/^\/\/\?\/UNC\//i.test(normalized)) normalized = "//" + normalized.slice(8);
      else if (/^\/\/[?.]\/[A-Za-z]:\//.test(normalized)) normalized = normalized.slice(4);
      var unc = /^\/\/([^/]+)(\/.*)?$/.exec(normalized);
      if (unc) {
        url = new NativeURL("file://" + unc[1] + "/");
        url.pathname = encodeFileUrlPath(unc[2] || "/");
      } else {
        if (/^[A-Za-z]:/.test(normalized)) normalized = "/" + normalized;
        else if (!normalized.startsWith("/")) normalized = "/" + normalized;
        url = new NativeURL("file:///");
        url.pathname = encodeFileUrlPath(normalized);
      }
    } else {
      url = new NativeURL("file:///");
      url.pathname = encodeFileUrlPath(absolute);
    }
    return url;
  }

  var module = {
    parse: runtime.vendor.legacyUrl.parse,
    format: runtime.vendor.legacyUrl.format,
    resolve: runtime.vendor.legacyUrl.resolve,
    resolveObject: runtime.vendor.legacyUrl.resolveObject,
    Url: runtime.vendor.legacyUrl.Url,
    URL: NativeURL,
    URLSearchParams: NativeURLSearchParams,
    fileURLToPath: fileURLToPath,
    pathToFileURL: pathToFileURL,
  };
  runtime.createUrlModule = function () { return module; };
  runtime.url = module;
})(globalThis);
