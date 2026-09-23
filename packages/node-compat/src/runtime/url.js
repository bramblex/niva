(function (root) {
  "use strict";

  var runtime = root[Symbol.for("niva.node-compat.runtime")];
  var NativeURL = root.URL;
  var NativeURLSearchParams = root.URLSearchParams;

  function fileURLToPath(input) {
    var url;
    try { url = input instanceof NativeURL ? input : new NativeURL(input); }
    catch (error) { throw runtime.bridgeError("The URL must be valid", "ERR_INVALID_URL", { cause: error }); }
    if (url.protocol !== "file:") throw runtime.bridgeError("The URL must use the file: scheme", "ERR_INVALID_URL_SCHEME");
    var windows = runtime.path.sep === "\\";
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
      if (/^\/[A-Za-z]:\//.test(decoded)) decoded = decoded.slice(1);
      return decoded.replace(/\//g, "\\");
    }
    return decoded || "/";
  }

  function pathToFileURL(input) {
    if (typeof input !== "string") throw new TypeError("path must be a string");
    var windows = runtime.path.sep === "\\";
    var absolute = runtime.path.resolve(input);
    var url;
    if (windows) {
      var normalized = absolute.replace(/\\/g, "/");
      var unc = /^\/\/([^/]+)(\/.*)?$/.exec(normalized);
      if (unc) {
        url = new NativeURL("file://" + unc[1] + "/");
        url.pathname = (unc[2] || "/").replace(/%/g, "%25");
      } else {
        if (/^[A-Za-z]:/.test(normalized)) normalized = "/" + normalized;
        else if (!normalized.startsWith("/")) normalized = "/" + normalized;
        url = new NativeURL("file:///");
        url.pathname = normalized.replace(/%/g, "%25");
      }
    } else {
      url = new NativeURL("file:///");
      url.pathname = absolute.replace(/%/g, "%25");
    }
    return url;
  }

  var module = {
    URL: NativeURL,
    URLSearchParams: NativeURLSearchParams,
    fileURLToPath: fileURLToPath,
    pathToFileURL: pathToFileURL,
  };
  runtime.createUrlModule = function () { return module; };
  runtime.url = module;
})(globalThis);
