(function (root) {
  "use strict";

  var runtime = root[Symbol.for("niva.node-compat.runtime")];
  var reasonPhrases = {
    200: "OK", 201: "Created", 202: "Accepted", 204: "No Content", 301: "Moved Permanently",
    302: "Found", 304: "Not Modified", 400: "Bad Request", 401: "Unauthorized", 403: "Forbidden",
    404: "Not Found", 405: "Method Not Allowed", 408: "Request Timeout", 409: "Conflict",
    418: "I'm a teapot", 429: "Too Many Requests", 500: "Internal Server Error", 501: "Not Implemented",
    502: "Bad Gateway", 503: "Service Unavailable", 504: "Gateway Timeout",
  };

  function byteData(value, encoding) {
    if (typeof value === "string") return new TextEncoder().encode(value);
    if (value instanceof ArrayBuffer) return new Uint8Array(value);
    if (ArrayBuffer.isView(value)) return new Uint8Array(value.buffer, value.byteOffset, value.byteLength).slice();
    throw new TypeError("HTTP request body chunks must be strings or byte arrays");
  }

  function bodyText(value) {
    if (typeof value === "string") return value;
    var bytes = byteData(value);
    try { return new TextDecoder("utf-8", { fatal: true }).decode(bytes); }
    catch (_) { throw runtime.bridgeError("The Niva HTTP bridge accepts UTF-8 text request bodies only", "ERR_HTTP_BODY_ENCODING"); }
  }

  function normalizeHeaders(headers) {
    if (!headers) return null;
    var normalized = Object.create(null);
    var entries;
    if (typeof Headers !== "undefined" && headers instanceof Headers) entries = Array.from(headers.entries());
    else if (Array.isArray(headers)) entries = headers;
    else if (typeof headers === "object") entries = Object.keys(headers).map(function (key) { return [key, headers[key]]; });
    else throw new TypeError("headers must be an object, Headers, or a list of name/value pairs");
    entries.forEach(function (entry) {
      if (!entry || entry.length < 2) throw new TypeError("header entries must contain a name and value");
      var name = String(entry[0]).toLowerCase();
      var value = entry[1];
      if (Array.isArray(value)) value = value.join(", ");
      if (value === undefined || value === null) return;
      if (!name || /[^!#$%&'*+.^_`|~0-9a-z-]/i.test(name)) throw new TypeError("Invalid HTTP header name: " + name);
      normalized[name] = String(value);
      if (/[\r\n]/.test(normalized[name])) throw new TypeError("Invalid HTTP header value for " + name);
    });
    return normalized;
  }

  function buildTarget(protocol, input, options) {
    var inputUrl = typeof input === "string" || (typeof URL !== "undefined" && input instanceof URL);
    var target;
    if (inputUrl) {
      target = input instanceof URL ? new URL(input.href) : new URL(input);
      options = options || {};
    } else {
      options = input || {};
      if (typeof options.url === "string" || (typeof URL !== "undefined" && options.url instanceof URL)) {
        target = options.url instanceof URL ? new URL(options.url.href) : new URL(options.url);
      } else {
        var hostname = options.hostname || options.host;
        if (typeof hostname !== "string" || !hostname) throw new TypeError("options.hostname or options.host is required");
        if (hostname.indexOf(":") >= 0 && hostname.charAt(0) !== "[") hostname = "[" + hostname + "]";
        var port = options.port === undefined || options.port === null || options.port === "" ? "" : ":" + String(options.port);
        var pathname = options.path || "/";
        if (pathname.charAt(0) !== "/") pathname = "/" + pathname;
        target = new URL(protocol + "://" + hostname + port + pathname);
      }
    }
    if (target.protocol !== protocol + ":") {
      throw runtime.bridgeError("Protocol " + target.protocol + " is not supported by the " + protocol + " module", "ERR_INVALID_PROTOCOL");
    }
    if (target.username || target.password) throw runtime.bridgeError("URL credentials are unsupported; set an Authorization header instead", "ENOTSUP");
    if (inputUrl && options.path !== undefined) {
      var pathValue = String(options.path);
      target.pathname = pathValue.split("?")[0] || "/";
      target.search = pathValue.indexOf("?") >= 0 ? pathValue.slice(pathValue.indexOf("?")) : "";
    }
    target.hash = "";
    return { url: target.href, options: options };
  }

  function makeIncomingMessage(head, method, requestUrl, cancelRequest) {
    var message = runtime.createEmitter();
    var resolveBody;
    var rejectBody;
    var bodyPromise = new Promise(function (resolve, reject) { resolveBody = resolve; rejectBody = reject; });
    bodyPromise.catch(function () {});
    var headers = Object.create(null);
    Object.keys(head.headers || {}).forEach(function (name) { headers[name.toLowerCase()] = String(head.headers[name]); });
    var rawHeaders = [];
    Object.keys(headers).forEach(function (name) { rawHeaders.push(name, headers[name]); });
    var encoding = null;

    message.statusCode = Number(head.status);
    message.statusMessage = reasonPhrases[message.statusCode] || "";
    message.headers = headers;
    message.headersDistinct = Object.create(null);
    Object.keys(headers).forEach(function (name) { message.headersDistinct[name] = [headers[name]]; });
    message.rawHeaders = rawHeaders;
    message.httpVersion = "1.1";
    message.httpVersionMajor = 1;
    message.httpVersionMinor = 1;
    message.method = method;
    message.url = requestUrl;
    message.complete = false;
    message.aborted = false;
    message.readable = true;
    message.body = null;
    message.bodyPromise = bodyPromise;

    message.setEncoding = function (value) {
      if (value !== "utf8" && value !== "utf-8" && value !== "latin1" && value !== "base64" && value !== "hex") {
        throw new TypeError("HTTP response streams support utf8, latin1, base64, and hex encodings");
      }
      encoding = value;
      return message;
    };
    message.text = function () { return bodyPromise.then(function (body) { return body.toString("utf8"); }); };
    message.json = function () { return message.text().then(function (text) { return JSON.parse(text); }); };
    message.arrayBuffer = function () {
      return bodyPromise.then(function (body) { return body.buffer.slice(body.byteOffset, body.byteOffset + body.byteLength); });
    };
    message.buffer = function () { return bodyPromise; };
    message.pipe = function (destination) {
      bodyPromise.then(function (body) {
        if (body.length) destination.write(encoding ? body.toString(encoding) : body);
        if (typeof destination.end === "function") destination.end();
      }, function (error) {
        if (typeof destination.destroy === "function") destination.destroy(error);
      });
      return destination;
    };
    message.destroy = function (error) {
      if (message.complete) return message;
      var converted = error || runtime.bridgeError("The HTTP response was aborted", "ABORT_ERR");
      if (typeof cancelRequest === "function") cancelRequest(converted);
      else message._fail(converted);
      return message;
    };
    message._finish = function (body) {
      if (message.complete) return;
      message.body = body;
      message.complete = true;
      if (body.length) message.emit("data", encoding ? body.toString(encoding) : body);
      message.emit("end");
      message.readable = false;
      message.emit("close");
      resolveBody(body);
    };
    message._fail = function (error) {
      if (message.aborted || message.complete) return;
      message.aborted = true;
      message.readable = false;
      message.emit("aborted");
      message.emit("error", error);
      message.emit("close");
      rejectBody(error);
    };
    return message;
  }

  function parseRequestArgs(protocol, defaultMethod, args) {
    var input = args[0];
    var options;
    var callback;
    if (typeof input === "string" || (typeof URL !== "undefined" && input instanceof URL)) {
      options = args[1] && typeof args[1] === "object" ? args[1] : {};
      callback = typeof args[1] === "function" ? args[1] : args[2];
    } else {
      options = input || {};
      callback = args[1];
    }
    if (typeof callback !== "function") callback = undefined;
    var built = buildTarget(protocol, input, options);
    options = built.options;
    ["agent", "signal", "timeout", "lookup", "createConnection", "socketPath", "localAddress", "auth", "proxy"].forEach(function (key) {
      if (options[key] !== undefined) throw runtime.bridgeError("http option is unsupported by Niva: " + key, "ENOTSUP");
    });
    var method = String(options.method || defaultMethod).toUpperCase();
    if (!/^[A-Z!#$%&'*+.^_`|~-]+$/.test(method)) throw new TypeError("Invalid HTTP method");
    var body = options.body === undefined ? undefined : bodyText(options.body);
    return {
      callback: callback,
      options: {
        method: method,
        url: built.url,
        headers: normalizeHeaders(options.headers),
        body: body,
      },
    };
  }

  function createHttpModule(protocol, niva) {
    function request() {
      var parsed = parseRequestArgs(protocol, "GET", arguments);
      var callback = parsed.callback;
      var options = parsed.options;
      var emitter = runtime.createEmitter();
      var requestBody = options.body === undefined ? [] : [options.body];
      var hasBody = options.body !== undefined;
      var ended = false;
      var bridgeCall = null;
      var responseMessage = null;
      var failRequest = null;
      var requestDone = false;
      var headerSettled = false;
      var resolveHeaders;
      var rejectHeaders;
      var resolveResult;
      var rejectResult;
      var headerPromise = new Promise(function (resolve, reject) { resolveHeaders = resolve; rejectHeaders = reject; });
      var resultPromise = new Promise(function (resolve, reject) { resolveResult = resolve; rejectResult = reject; });
      headerPromise.catch(function () {});
      resultPromise.catch(function () {});

      var clientRequest = emitter;
      Object.assign(clientRequest, {
        method: options.method,
        path: new URL(options.url).pathname + new URL(options.url).search,
        host: new URL(options.url).host,
        protocol: protocol + ":",
        headers: Object.assign(Object.create(null), options.headers || {}),
        writable: true,
        finished: false,
        response: headerPromise,
        result: resultPromise,
        completion: resultPromise,
        setHeader: function (name, value) {
          if (ended) throw runtime.bridgeError("Cannot set headers after request end", "ERR_HTTP_HEADERS_SENT");
          var key = String(name).toLowerCase();
          normalizeHeaders([[key, value]]);
          clientRequest.headers[key] = Array.isArray(value) ? value.join(", ") : String(value);
          return clientRequest;
        },
        getHeader: function (name) { return clientRequest.headers[String(name).toLowerCase()]; },
        getHeaders: function () { return Object.assign(Object.create(null), clientRequest.headers); },
        hasHeader: function (name) { return Object.prototype.hasOwnProperty.call(clientRequest.headers, String(name).toLowerCase()); },
        removeHeader: function (name) { delete clientRequest.headers[String(name).toLowerCase()]; },
        write: function (data, encoding, callback) {
          if (typeof encoding === "function") callback = encoding;
          else if (encoding !== undefined && String(encoding).toLowerCase() !== "utf8" && String(encoding).toLowerCase() !== "utf-8") {
            throw runtime.bridgeError("The Niva HTTP bridge accepts UTF-8 request strings only", "ERR_HTTP_BODY_ENCODING");
          }
          if (ended) throw runtime.bridgeError("write after end", "ERR_STREAM_WRITE_AFTER_END");
          requestBody.push(bodyText(data));
          hasBody = true;
          if (callback) Promise.resolve().then(callback);
          return true;
        },
        end: function (data, encoding, callback) {
          if (typeof data === "function") { callback = data; data = undefined; }
          else if (typeof encoding === "function") callback = encoding;
          else if (encoding !== undefined && String(encoding).toLowerCase() !== "utf8" && String(encoding).toLowerCase() !== "utf-8") {
            throw runtime.bridgeError("The Niva HTTP bridge accepts UTF-8 request strings only", "ERR_HTTP_BODY_ENCODING");
          }
          if (ended) {
            if (callback) Promise.resolve().then(callback);
            return clientRequest;
          }
          if (data !== undefined) {
            requestBody.push(bodyText(data));
            hasBody = true;
          }
          ended = true;
          clientRequest.finished = true;
          clientRequest.writable = false;
          emitter.emit("finish");
          if (callback) Promise.resolve().then(callback);
          var requestOptions = {
            method: options.method,
            url: options.url,
            headers: normalizeHeaders(clientRequest.headers),
          };
          if (requestOptions.headers && Object.keys(requestOptions.headers).length === 0) requestOptions.headers = null;
          if (hasBody) requestOptions.body = requestBody.join("");
          var bodyChunks = [];
          var bodyChain = Promise.resolve();
          function fail(error) {
            if (requestDone) return;
            requestDone = true;
            var converted = runtime.nativeError(error);
            if (bridgeCall && typeof bridgeCall.cancel === "function") bridgeCall.cancel();
            if (!headerSettled) { headerSettled = true; rejectHeaders(converted); }
            if (responseMessage) {
              responseMessage._fail(converted);
              rejectResult(converted);
            } else {
              rejectResult(converted);
            }
            emitter.emit("error", converted);
            emitter.emit("close");
          }
          failRequest = fail;
          try {
            bridgeCall = runtime.stream(niva, "http.requestStream", [requestOptions], {
              onEvent: function (name, data) {
                if (name !== "head" || responseMessage) return;
                responseMessage = makeIncomingMessage(data, options.method, new URL(options.url).pathname + new URL(options.url).search, function (error) {
                  if (bridgeCall && typeof bridgeCall.cancel === "function") bridgeCall.cancel();
                  if (failRequest) failRequest(error);
                });
                headerSettled = true;
                resolveHeaders(responseMessage);
                emitter.emit("response", responseMessage);
              },
              onBlob: function (blob) {
                bodyChain = bodyChain.then(function () {
                  var bufferPromise = blob && typeof blob.arrayBuffer === "function"
                    ? blob.arrayBuffer()
                    : Promise.resolve(blob instanceof Uint8Array ? blob.buffer.slice(blob.byteOffset, blob.byteOffset + blob.byteLength) : blob);
                  return Promise.resolve(bufferPromise).then(function (buffer) { bodyChunks.push(runtime.buffer.Buffer.from(buffer)); });
                });
              },
            });
          } catch (error) {
            Promise.resolve().then(function () { fail(error); });
            return clientRequest;
          }
          Promise.resolve(bridgeCall.promise).then(function (terminal) {
            return bodyChain.then(function () {
              if (!responseMessage) throw runtime.bridgeError("Niva HTTP response did not include a head event", "ERR_HTTP_INVALID_RESPONSE");
              var body = runtime.buffer.Buffer.concat(bodyChunks);
              if (terminal && terminal.status !== undefined && Number(terminal.status) !== responseMessage.statusCode) {
                throw runtime.bridgeError("Niva HTTP terminal status did not match its response head", "ERR_HTTP_INVALID_RESPONSE");
              }
              responseMessage._finish(body);
              requestDone = true;
              resolveResult(responseMessage);
              emitter.emit("close");
            });
          }).catch(fail);
          return clientRequest;
        },
        abort: function () {
          var error = runtime.bridgeError("The HTTP request was aborted", "ABORT_ERR");
          if (failRequest) failRequest(error);
          else {
            ended = true;
            clientRequest.finished = true;
            clientRequest.writable = false;
            headerSettled = true;
            rejectHeaders(error);
            rejectResult(error);
            emitter.emit("abort");
            emitter.emit("error", error);
            emitter.emit("close");
          }
          return clientRequest;
        },
        destroy: function (error) {
          var converted = runtime.nativeError(error || runtime.bridgeError("The HTTP request was destroyed", "ABORT_ERR"));
          if (failRequest) failRequest(converted);
          else {
            ended = true;
            clientRequest.finished = true;
            clientRequest.writable = false;
            headerSettled = true;
            rejectHeaders(converted);
            rejectResult(converted);
            emitter.emit("error", converted);
            emitter.emit("close");
          }
          return clientRequest;
        },
        setTimeout: function () { throw runtime.bridgeError("HTTP request timeouts are not supported by the Niva bridge", "ENOTSUP"); },
      });
      if (callback) emitter.once("response", callback);
      return clientRequest;
    }

    function get() {
      var req = request.apply(null, arguments);
      req.end();
      return req;
    }

    function post(input, body, options, callback) {
      if (typeof body === "function") { callback = body; body = undefined; options = {}; }
      else if (body && typeof body === "object" && !(body instanceof ArrayBuffer) && !ArrayBuffer.isView(body)) {
        callback = typeof options === "function" ? options : callback;
        options = body;
        body = undefined;
      }
      else if (typeof options === "function") { callback = options; options = {}; }
      if (body === null) body = undefined;
      options = Object.assign({}, options || {}, { method: "POST" });
      var requestInput = input;
      if (input && typeof input === "object" && !(typeof URL !== "undefined" && input instanceof URL)) {
        requestInput = Object.assign({}, input, options);
        return request(requestInput, callback).end(body);
      }
      var req = request(requestInput, options, callback);
      req.end(body);
      return req;
    }

    return { request: request, get: get, post: post };
  }

  runtime.createHttpModule = createHttpModule;
  runtime.http = createHttpModule("http");
  runtime.https = createHttpModule("https");
})(globalThis);
