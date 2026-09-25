(function (root: any) {
    "use strict";
    var runtime = root[Symbol.for("niva.node-compat.runtime")], Buffer = runtime.vendor.Buffer;
    var Readable = runtime.vendor.stream.Readable, Writable = runtime.vendor.stream.Writable, Parser = runtime.vendor.HTTPParser;
    var token = /^[!#$%&'*+.^_`|~0-9A-Za-z-]+$/;
    var STATUS_CODES = { 100: "Continue", 101: "Switching Protocols", 200: "OK", 201: "Created", 202: "Accepted", 204: "No Content", 206: "Partial Content", 301: "Moved Permanently", 302: "Found", 304: "Not Modified", 307: "Temporary Redirect", 308: "Permanent Redirect", 400: "Bad Request", 401: "Unauthorized", 403: "Forbidden", 404: "Not Found", 405: "Method Not Allowed", 408: "Request Timeout", 413: "Payload Too Large", 418: "I'm a Teapot", 429: "Too Many Requests", 500: "Internal Server Error", 502: "Bad Gateway", 503: "Service Unavailable" };
    function fail(message, code?) { return runtime.bridgeError(message, code || "HPE_INVALID_HEADER_TOKEN"); }
    function header(name, value) { if (!token.test(name))
        throw fail("Invalid HTTP header name", "ERR_INVALID_HTTP_TOKEN"); value = String(value); if (/[^\t\x20-\x7e\x80-\xff]/.test(value))
        throw fail("Invalid HTTP header value", "ERR_INVALID_CHAR"); return value; }
    function headers(raw) {
        var result = Object.create(null), seen = Object.create(null);
        for (var i = 0; i < raw.length; i += 2) {
            var name = raw[i].toLowerCase(), value = header(raw[i], raw[i + 1]);
            seen[name] = (seen[name] || 0) + 1;
            if (name === "set-cookie")
                (result[name] || (result[name] = [])).push(value);
            else
                result[name] = result[name] === undefined ? value : result[name] + (name === "cookie" ? "; " : ", ") + value;
        }
        if (seen["content-length"] > 1 || (seen["content-length"] && seen["transfer-encoding"]))
            throw fail("Ambiguous HTTP framing", "HPE_UNEXPECTED_CONTENT_LENGTH");
        if (result["content-length"] !== undefined && !/^\d+$/.test(result["content-length"]))
            throw fail("Invalid Content-Length", "HPE_INVALID_CONTENT_LENGTH");
        if (result["transfer-encoding"] !== undefined && result["transfer-encoding"].toLowerCase() !== "chunked")
            throw fail("Unsupported Transfer-Encoding", "HPE_INVALID_TRANSFER_ENCODING");
        return result;
    }
    class IncomingMessage extends Readable {
        constructor(socket) { super(); this.socket = this.connection = socket; this.headers = {}; this.rawHeaders = []; this.trailers = {}; this.rawTrailers = []; this.complete = false; this.aborted = false; }
        _read() { this.socket.resume(); }
        _destroy(error, callback) { if (!this.complete)
            this.socket.destroy(error); callback(error); }
        setTimeout(timeout, callback) { this.socket.setTimeout(timeout, callback); return this; }
    }
    class OutgoingMessage extends Writable {
        constructor(socket, options) { super(options); this.socket = this.connection = socket; this.headersSent = false; this._headers = Object.create(null); this._chunked = false; this._sent = 0; this._expected = null; this._trailers = []; }
        setHeader(name, value) { if (this.headersSent)
            throw fail("Headers already sent", "ERR_HTTP_HEADERS_SENT"); var values = Array.isArray(value) ? value.map(function (item) { return header(name, item); }) : header(name, value); this._headers[name.toLowerCase()] = { name: name, value: values }; return this; }
        getHeader(name) { return this._headers[String(name).toLowerCase()]?.value; }
        getHeaders() { var result = Object.create(null); Object.keys(this._headers).forEach(name => result[name] = this._headers[name].value); return result; }
        getHeaderNames() { return Object.keys(this._headers); }
        hasHeader(name) { return this.getHeader(name) !== undefined; }
        removeHeader(name) { if (this.headersSent)
            throw fail("Headers already sent", "ERR_HTTP_HEADERS_SENT"); delete this._headers[String(name).toLowerCase()]; }
        addTrailers(values) { Object.keys(values).forEach(name => { this._trailers.push(name + ": " + header(name, values[name])); }); }
        flushHeaders() { if (this.headersSent)
            return; this._sendHeaders(); }
        _sendHeaders() {
            if (!this.hasHeader("connection"))
                this.setHeader("Connection", "close");
            if (this._noBody) {
                this._chunked = false;
            }
            else if (this.hasHeader("content-length")) {
                var length = String(this.getHeader("content-length"));
                if (!/^\d+$/.test(length))
                    throw fail("Invalid Content-Length");
                this._expected = Number(length);
                if (!Number.isSafeInteger(this._expected))
                    throw fail("Content-Length out of range");
                if (this.hasHeader("transfer-encoding"))
                    throw fail("Ambiguous HTTP framing");
            }
            else {
                if (this.hasHeader("transfer-encoding") && String(this.getHeader("transfer-encoding")).toLowerCase() !== "chunked")
                    throw fail("Unsupported Transfer-Encoding");
                this.setHeader("Transfer-Encoding", "chunked");
                this._chunked = true;
            }
            var lines = [this._startLine()];
            Object.keys(this._headers).forEach(name => { var entry = this._headers[name]; (Array.isArray(entry.value) ? entry.value : [entry.value]).forEach(value => lines.push(entry.name + ": " + value)); });
            this.headersSent = true;
            this.socket.write(Buffer.from(lines.join("\r\n") + "\r\n\r\n", "latin1"));
        }
        _write(chunk, encoding, callback) { try {
            this.flushHeaders();
            if (this._noBody || chunk.length === 0) {
                callback();
                return;
            }
            this._sent += chunk.length;
            if (this._expected !== null && this._sent > this._expected)
                throw fail("Body exceeds Content-Length", "ERR_HTTP_CONTENT_LENGTH_MISMATCH");
            var bytes = this._chunked ? Buffer.concat([Buffer.from(chunk.length.toString(16) + "\r\n"), chunk, Buffer.from("\r\n")]) : chunk;
            this.socket.write(bytes, callback);
        }
        catch (error) {
            callback(error);
        } }
        _final(callback) { try {
            this.flushHeaders();
            if (!this._noBody && this._expected !== null && this._sent !== this._expected)
                throw fail("Body does not match Content-Length", "ERR_HTTP_CONTENT_LENGTH_MISMATCH");
            if (this._chunked)
                this.socket.write(Buffer.from("0\r\n" + this._trailers.join("\r\n") + (this._trailers.length ? "\r\n" : "") + "\r\n"), callback);
            else
                callback();
        }
        catch (error) {
            callback(error);
        } }
        _destroy(error, callback) { if (error)
            this.socket.destroy(error); callback(error); }
        setTimeout(timeout, callback) { this.socket.setTimeout(timeout, callback); return this; }
    }
    class ServerResponse extends OutgoingMessage {
        constructor(request) { super(request.socket, { autoDestroy: false }); this.req = request; this.statusCode = 200; this.statusMessage = undefined; this.sendDate = true; this.once("finish", () => this.socket.end()); }
        _startLine() { if (!Number.isInteger(this.statusCode) || this.statusCode < 100 || this.statusCode > 999)
            throw new RangeError("Invalid statusCode"); return "HTTP/1.1 " + this.statusCode + " " + header("status", this.statusMessage || STATUS_CODES[this.statusCode] || "unknown"); }
        _sendHeaders() { this._noBody = this.req.method === "HEAD" || this.statusCode === 204 || this.statusCode === 304 || this.statusCode < 200; if (this.sendDate && !this.hasHeader("date"))
            this.setHeader("Date", new Date().toUTCString()); super._sendHeaders(); }
        writeHead(status, message, values) { if (typeof message !== "string") {
            values = message;
            message = undefined;
        } if (!Number.isInteger(status) || status < 100 || status > 999)
            throw new RangeError("Invalid statusCode"); this.statusCode = status; if (message !== undefined)
            this.statusMessage = header("status", message); if (values)
            Object.keys(values).forEach(name => this.setHeader(name, values[name])); this.flushHeaders(); return this; }
        writeContinue(callback?) { this.socket.write(Buffer.from("HTTP/1.1 100 Continue\r\n\r\n"), callback); }
    }
    function parseSocket(socket, type, onMessage, onError, headMethod = false) {
        var parser = new Parser(type), message, initial = Buffer.alloc(0), validated = false, count = 0;
        parser.maxHeaderSize = 16384;
        parser[Parser.kOnHeadersComplete] = function (info) {
            var values = headers(info.headers);
            if (info.upgrade)
                throw fail("HTTP upgrade is not implemented", "HPE_UNSUPPORTED_UPGRADE");
            if (type === Parser.REQUEST && ++count > 1)
                throw fail("Pipelining is unavailable on a closing connection");
            message = new IncomingMessage(socket);
            message.headers = values;
            message.rawHeaders = info.headers.slice();
            message.httpVersion = info.versionMajor + "." + info.versionMinor;
            message.httpVersionMajor = info.versionMajor;
            message.httpVersionMinor = info.versionMinor;
            if (type === Parser.REQUEST) {
                message.method = Parser.methods[info.method];
                message.url = info.url;
            }
            else {
                message.statusCode = info.statusCode;
                message.statusMessage = info.statusMessage;
            }
            if (type === Parser.RESPONSE && info.statusCode < 200) {
                message = null;
                return true;
            }
            onMessage(message);
            return headMethod || info.statusCode === 204 || info.statusCode === 304;
        };
        parser[Parser.kOnBody] = function (chunk, offset, length) { if (message && !message.push(Buffer.from(chunk.subarray(offset, offset + length))))
            socket.pause(); };
        parser[Parser.kOnHeaders] = function (raw) { if (message) {
            message.rawTrailers = raw.slice();
            message.trailers = headers(raw);
        } };
        parser[Parser.kOnMessageComplete] = function () { if (message) {
            message.complete = true;
            message.push(null);
            message = null;
        } };
        socket.on("data", function (chunk) {
            try {
                if (!validated) {
                    initial = Buffer.concat([initial, chunk]);
                    var end = initial.indexOf("\r\n\r\n");
                    if (end < 0) {
                        if (initial.length > 16384)
                            throw fail("HTTP headers too large", "HPE_HEADER_OVERFLOW");
                        return;
                    }
                    if (end > 16384)
                        throw fail("HTTP headers too large", "HPE_HEADER_OVERFLOW");
                    var lines = initial.subarray(0, end).toString("latin1").split("\r\n");
                    var first = lines.shift();
                    if (type === Parser.REQUEST ? !/^[A-Z-]+ [^\x00-\x20]+ HTTP\/1\.[01]$/.test(first) : !/^HTTP\/1\.[01] \d{3}(?: [^\r\n]*)?$/.test(first))
                        throw fail("Invalid HTTP start line");
                    lines.forEach(function (line) { var index = line.indexOf(":"); if (index <= 0 || !token.test(line.slice(0, index)))
                        throw fail("Malformed HTTP header"); header(line.slice(0, index), line.slice(index + 1)); });
                    chunk = initial;
                    initial = null;
                    validated = true;
                }
                var result = parser.execute(chunk);
                if (result instanceof Error)
                    throw result;
            }
            catch (error) {
                onError(error);
            }
        });
        socket.on("end", function () { try {
            var result = parser.finish();
            if (result instanceof Error)
                throw result;
        }
        catch (error) {
            onError(error);
        } });
        socket.on("close", function () { if (message && !message.complete) {
            message.aborted = true;
            message.emit("aborted");
            message.destroy(fail("Premature HTTP close", "ECONNRESET"));
        } });
    }
    function createHttpModule(protocol, niva?) {
        function transport() { return protocol === "https" ? runtime.createTlsModule(niva) : runtime.createNetModule(niva); }
        function requestText(input, options) {
            var opts;
            if (typeof input === "string" || input instanceof URL) {
                var parsedUrl = new URL(String(input));
                opts = Object.assign({}, options || {}, { url: parsedUrl.href });
            }
            else if (input && typeof input === "object") {
                opts = Object.assign({}, input, options || {});
            }
            else {
                return Promise.reject(runtime.bridgeError("requestText requires a URL or options object", "ERR_INVALID_ARG_TYPE"));
            }
            var url;
            try { url = new URL(opts.url); }
            catch (_) { return Promise.reject(runtime.bridgeError("requestText requires an absolute URL", "ERR_INVALID_URL")); }
            if (url.protocol !== protocol + ":") return Promise.reject(runtime.bridgeError("Unexpected protocol", "ERR_INVALID_PROTOCOL"));
            if (opts.body !== undefined && typeof opts.body !== "string") return Promise.reject(runtime.bridgeError("requestText only supports UTF-8 string bodies", "ERR_NIVA_IPC_BINARY_UNSUPPORTED"));
            var allowed = ["url", "method", "headers", "body", "timeout", "maxResponseBytes"];
            var unsupported = Object.keys(opts).find(function (name) { return !allowed.includes(name); });
            if (unsupported) return Promise.reject(runtime.bridgeError("requestText does not support option " + unsupported, "ERR_NIVA_IPC_OPTION_UNSUPPORTED"));
            var timeout = opts.timeout;
            var maxResponseBytes = opts.maxResponseBytes;
            if (timeout !== undefined && (!Number.isInteger(timeout) || timeout < 1 || timeout > 30000)) return Promise.reject(runtime.bridgeError("timeout must be between 1 and 30000", "ERR_OUT_OF_RANGE"));
            if (maxResponseBytes !== undefined && (!Number.isInteger(maxResponseBytes) || maxResponseBytes < 1 || maxResponseBytes > 1024 * 1024)) return Promise.reject(runtime.bridgeError("maxResponseBytes must be between 1 and 1048576", "ERR_OUT_OF_RANGE"));
            var requestOptions: Record<string, any> = {
                url: url.href,
                method: String(opts.method || "GET").toUpperCase(),
                headers: opts.headers || {},
                body: opts.body,
            };
            if (timeout !== undefined) requestOptions.timeout = timeout;
            if (maxResponseBytes !== undefined) requestOptions.maxResponseBytes = maxResponseBytes;
            return runtime.call(niva, "http.requestText", [requestOptions]).then(function (result) {
                if (!result || typeof result !== "object" || typeof result.statusCode !== "number" || typeof result.body !== "string") throw runtime.bridgeError("Invalid Native requestText response", "ERR_NIVA_IPC_RESPONSE");
                return { statusCode: result.statusCode, statusMessage: result.statusMessage || "", headers: result.headers || {}, body: result.body };
            }, function (error) { throw runtime.nativeError(error); });
        }
        class ClientRequest extends OutgoingMessage {
            constructor(input, options, callback) {
                if (typeof options === "function") {
                    callback = options;
                    options = {};
                }
                var opts = Object.assign({}, typeof input === "object" && !(input instanceof URL) ? input : {}, options);
                if (opts.protocol && opts.protocol !== protocol + ":")
                    throw fail("Unexpected protocol", "ERR_INVALID_PROTOCOL");
                var url = typeof input === "string" || input instanceof URL ? new URL(input) : null;
                if (url && url.protocol !== protocol + ":")
                    throw fail("Unexpected protocol", "ERR_INVALID_PROTOCOL");
                var authority = typeof input === "string" ? /^[a-z]+:\/\/([^/?#]*)/i.exec(input) : null;
                if (url && (url.username || url.password || authority && authority[1].includes("@")))
                    throw fail("URL credentials are unsupported", "ERR_INVALID_URL");
                var hostname = opts.hostname || (url && url.hostname) || opts.host || "localhost";
                hostname = hostname.replace(/^\[|\]$/g, "");
                var port = Number(opts.port || (url && url.port) || (protocol === "https" ? 443 : 80));
                if (!Number.isInteger(port) || port < 1 || port > 65535)
                    throw new RangeError("Invalid port");
                if (opts.createConnection || opts.socketPath)
                    throw fail("Custom HTTP connections are unsupported", "ENOTSUP");
                var method = String(opts.method || "GET").toUpperCase();
                if (!token.test(method)) throw fail("Invalid HTTP method", "ERR_INVALID_HTTP_TOKEN");
                var requestPath = opts.path || (url ? url.pathname + url.search : "/");
                if (typeof requestPath !== "string" || /[^\x21-\x7e\x80-\xff]/.test(requestPath)) throw fail("Invalid request path", "ERR_UNESCAPED_CHARACTERS");
                if (opts.headers && (typeof opts.headers !== "object" || Array.isArray(opts.headers))) throw fail("HTTP headers must be an object", "ENOTSUP");
                Object.keys(opts.headers || {}).forEach(name => {
                    var values = Array.isArray(opts.headers[name]) ? opts.headers[name] : [opts.headers[name]];
                    values.forEach(value => header(name, value));
                });
                var socketOptions = Object.assign({}, opts, { host: hostname, port: port });
                delete socketOptions.path;
                delete socketOptions.method;
                delete socketOptions.headers;
                var socket = transport().connect(socketOptions);
                super(socket, { autoDestroy: false });
                this.method = method;
                this.path = requestPath;
                this.host = hostname;
                this.protocol = protocol + ":";
                this.aborted = false;
                Object.keys(opts.headers || {}).forEach(name => this.setHeader(name, opts.headers[name]));
                if (!this.hasHeader("host"))
                    this.setHeader("Host", (hostname.indexOf(":") >= 0 ? "[" + hostname + "]" : hostname) + ((protocol === "https" ? 443 : 80) !== port ? ":" + port : ""));
                if (callback)
                    this.once("response", callback);
                parseSocket(socket, Parser.RESPONSE, response => { this.res = response; this.emit("response", response); }, error => this.destroy(error), this.method === "HEAD");
                socket.on("error", error => this.destroy(error));
                socket.on("close", () => { if (!this.res && !this.destroyed)
                    this.destroy(fail("socket hang up", "ECONNRESET")); this.emit("close"); });
                queueMicrotask(() => this.emit("socket", socket));
                if (opts.timeout)
                    socket.setTimeout(opts.timeout, () => this.emit("timeout"));
            }
            _startLine() { return this.method + " " + this.path + " HTTP/1.1"; }
            abort() { this.aborted = true; this.emit("abort"); this.destroy(); }
            _destroy(error, callback) { this.socket.destroy(); callback(error); }
        }
        function request(input, options, callback) { return new ClientRequest(input, options, callback); }
        function get(input, options, callback) { var req = request(input, options, callback); req.end(); return req; }
        function createServer(options, listener) {
            if (typeof options === "function") {
                listener = options;
                options = {};
            }
            options = options || {};
            var server = transport().createServer(options, function (socket) {
                socket.on("error", function (error) { if (server.listenerCount("clientError"))
                    server.emit("clientError", error, socket); });
                parseSocket(socket, Parser.REQUEST, function (req) { var res = new ServerResponse(req); if (req.headers.expect && req.headers.expect.toLowerCase() === "100-continue")
                    res.writeContinue(); server.emit("request", req, res); }, function (error) { if (server.listenerCount("clientError"))
                    server.emit("clientError", error, socket);
                else
                    socket.destroy(); });
            });
            if (listener)
                server.on("request", listener);
            return server;
        }
        return { request: request, get: get, requestText: requestText, createServer: createServer, IncomingMessage: IncomingMessage, ServerResponse: ServerResponse, ClientRequest: ClientRequest, METHODS: Parser.methods.slice(), STATUS_CODES: STATUS_CODES };
    }
    runtime.createHttpModule = createHttpModule;
    runtime.http = createHttpModule("http");
    runtime.https = createHttpModule("https");
})(globalThis);
