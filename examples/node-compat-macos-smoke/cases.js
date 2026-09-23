/* Browser integration cases for a real Niva WebView and native bridge. */
(function (root) {
  "use strict";

  var modules = [
    "path", "os", "fs", "child_process", "events", "util", "querystring",
    "buffer", "url", "crypto", "zlib", "http", "https", "assert", "stream",
  ];
  var checks = [];

  function add(id, methods, run) {
    checks.push({ id: id, methods: methods, run: run });
  }

  function assert(condition, message) {
    if (!condition) throw new Error(message);
  }

  function equal(actual, expected, message) {
    if (actual !== expected) {
      throw new Error(message + ": got " + String(actual) + ", expected " + String(expected));
    }
  }

  function deepEqual(actual, expected, message) {
    var normalize = function (value) {
      if (value && typeof value === "object") {
        if (Array.isArray(value)) return value.map(normalize);
        var out = {};
        Object.keys(value).sort().forEach(function (key) { out[key] = normalize(value[key]); });
        return out;
      }
      return value;
    };
    equal(JSON.stringify(normalize(actual)), JSON.stringify(normalize(expected)), message);
  }

  function expectThrow(operation, predicate, message) {
    var thrown = null;
    try { operation(); } catch (error) { thrown = error; }
    assert(thrown, message + ": expected an exception");
    if (predicate) assert(predicate(thrown), message + ": unexpected exception " + thrown);
    return thrown;
  }

  function makeSink(EventEmitter) {
    var emitter = new EventEmitter();
    var chunks = [];
    return {
      chunks: chunks,
      on: function (name, listener) { emitter.on(name, listener); return this; },
      once: function (name, listener) { emitter.once(name, listener); return this; },
      off: function (name, listener) { emitter.off(name, listener); return this; },
      removeListener: function (name, listener) { emitter.removeListener(name, listener); return this; },
      write: function (chunk) { chunks.push(typeof chunk === "string" ? chunk : chunk.toString("utf8")); return true; },
      end: function () { emitter.emit("finish"); },
      destroy: function (error) { if (error) emitter.emit("error", error); },
    };
  }

  add("classic registration and aliases", [
    "Niva.require", "Niva.import", "global.require", "path", "node:path", "fs", "node:fs",
    "alias:fs/promises", "alias:node:fs/promises", "alias:assert/strict", "alias:node:assert/strict",
    "alias:stream/promises", "alias:node:stream/promises", "importmap:fs",
  ], async function () {
    await root.NivaNodeCompatReady;
    assert(root.__nodeCompatImportMapReady === true, "the native import map must resolve the bare fs specifier");
    equal(root.require("path"), root.Niva.require("path"), "global require alias");
    equal(root.Niva.require("node:path"), root.Niva.require("path"), "node:path alias");
    equal(root.Niva.require("node:fs"), root.Niva.require("fs"), "node:fs alias");
    equal(root.Niva.require("node:fs/promises"), root.Niva.require("fs/promises"), "fs/promises alias");
    equal(root.Niva.require("node:assert/strict"), root.Niva.require("assert/strict"), "assert/strict alias");
    equal(root.Niva.require("node:stream/promises"), root.Niva.require("stream/promises"), "stream/promises alias");
    var importedPath = await root.Niva.import("path");
    equal(importedPath.join("a", "b"), "a/b", "Niva.import uses the selected ESM adapter");
    assert(typeof (await root.Niva.import("fs/promises")).readFile === "function", "dynamic fs/promises adapter");
  });

  add("path", [
    "path.join", "path.resolve", "path.basename", "path.dirname", "path.extname",
    "path.normalize", "path.relative", "path.parse", "path.format", "path.isAbsolute",
    "path.toNamespacedPath", "path.matchesGlob", "path.sep", "path.delimiter",
    "path.posix", "path.win32", "path.posix.join", "path.win32.join",
  ], async function (env) {
    var path = env.path;
    equal(path.join("alpha", "..", "beta", "note.txt"), "beta/note.txt", "path.join");
    assert(path.isAbsolute(path.resolve(".")), "path.resolve should return an absolute working directory");
    equal(path.basename("/alpha/beta.txt", ".txt"), "beta", "path.basename");
    equal(path.dirname("/alpha/beta.txt"), "/alpha", "path.dirname");
    equal(path.extname("/alpha/beta.txt"), ".txt", "path.extname");
    equal(path.normalize("/alpha/./beta/../gamma/"), "/alpha/gamma/", "path.normalize");
    equal(path.relative("/alpha/beta", "/alpha/gamma/file"), "../gamma/file", "path.relative");
    deepEqual(path.parse("/alpha/beta.txt"), { root: "/", dir: "/alpha", base: "beta.txt", ext: ".txt", name: "beta" }, "path.parse");
    equal(path.format({ dir: "/alpha", name: "beta", ext: ".txt" }), "/alpha/beta.txt", "path.format");
    equal(path.isAbsolute("/alpha"), true, "path.isAbsolute");
    equal(path.toNamespacedPath("/alpha/beta"), "/alpha/beta", "path.toNamespacedPath on POSIX");
    equal(path.matchesGlob("src/runtime/fs.js", "src/**/*.js"), true, "path.matchesGlob recursive pattern");
    equal(path.matchesGlob("src/runtime/fs.js", "src/runtime/f?.js"), true, "path.matchesGlob question mark");
    equal(path.matchesGlob("src/runtime/fs.js", "src/runtime/[fg]s.js"), true, "path.matchesGlob character class");
    equal(path.sep, "/", "path.sep");
    equal(path.delimiter, ":", "path.delimiter");
    equal(path.posix.join("a", "b"), "a/b", "path.posix");
    equal(path.win32.join("a", "b"), "a\\b", "path.win32");
    equal(path.win32.normalize("C:\\alpha\\..\\beta\\"), "C:\\beta\\", "path.win32 normalize");
  });

  add("os", [
    "os.platform", "os.arch", "os.homedir", "os.tmpdir", "os.EOL", "os.sep", "os.delimiter",
  ], async function (env) {
    equal(await env.os.platform(), "darwin", "os.platform");
    assert(["arm64", "x64"].includes(await env.os.arch()), "os.arch should identify the running Mac architecture");
    assert((await env.os.homedir()).startsWith("/"), "os.homedir should be an absolute path");
    env.tempRoot = await env.os.tmpdir();
    assert(env.tempRoot.startsWith("/"), "os.tmpdir should be an absolute path");
    equal(env.os.EOL, "\n", "os.EOL");
    equal(env.os.sep, "/", "os.sep");
    equal(env.os.delimiter, ":", "os.delimiter");
  });

  add("fs and fs/promises", [
    "fs.readFile", "fs.writeFile", "fs.appendFile", "fs.mkdir", "fs.readdir", "fs.stat",
    "fs.access", "fs.rename", "fs.rm", "fs.cp", "fs.copyFile", "fs/promises",
  ], async function (env) {
    var fs = env.fs;
    var fsp = env.fsp;
    var path = env.path;
    equal(fsp, fs.promises, "fs/promises default export");
    var rootPath = path.join(env.tempRoot, "niva-node-compat-" + env.nonce);
    var sourceDir = path.join(rootPath, "source");
    var source = path.join(sourceDir, "sample.txt");
    var copied = path.join(rootPath, "copy.txt");
    var copiedDir = path.join(rootPath, "copy-tree");
    var moved = path.join(rootPath, "renamed.txt");
    await fs.mkdir(sourceDir, { recursive: true });
    await fs.writeFile(source, "alpha", "utf8");
    await fs.appendFile(source, "-beta", "utf8");
    equal(await fsp.readFile(source, "utf8"), "alpha-beta", "fs/promises.readFile and appendFile");
    var raw = await fs.readFile(source, null);
    assert(env.Buffer.isBuffer(raw), "binary fs.readFile should return a Buffer");
    equal(raw.toString("utf8"), "alpha-beta", "binary fs.readFile content");
    var stats = await fs.stat(source);
    assert(stats.isFile() && !stats.isDirectory() && stats.size === 10, "fs.stat file type and byte size");
    await fs.access(source, fs.constants.F_OK);
    var entries = await fs.readdir(sourceDir, { withFileTypes: true });
    assert(entries.some(function (entry) { return entry.name === "sample.txt" && entry.isFile(); }), "fs.readdir with Dirent");
    await fs.copyFile(source, copied);
    equal(await fs.readFile(copied, "utf8"), "alpha-beta", "fs.copyFile");
    await fs.cp(sourceDir, copiedDir, { recursive: true });
    equal(await fs.readFile(path.join(copiedDir, "sample.txt"), "utf8"), "alpha-beta", "fs.cp recursive directory");
    await fsp.rename(copied, moved);
    equal(await fs.readFile(moved, "utf8"), "alpha-beta", "fs/promises.rename");
    await fsp.rm(moved);
    equal(await fs.access(moved).then(function () { return false; }, function () { return true; }), true, "fs/promises.rm");
    await fs.rm(rootPath, { recursive: true });
    equal(await fs.access(rootPath).then(function () { return false; }, function () { return true; }), true, "fs.rm recursive cleanup");
  });

  add("child_process", [
    "child_process.spawn", "child_process.exec", "spawn.stdin.write", "spawn.stdin.end",
    "spawn.stdout", "spawn.stderr", "exec.callback",
  ], async function (env) {
    var cp = env.childProcess;
    var stage = "cat stdin/stdout";
    try {
    var child = cp.spawn("/bin/cat", [], { encoding: "utf8" });
    var output = "";
    child.stdout.setEncoding("utf8").on("data", function (chunk) { output += chunk; });
    assert(child.stdin.write("stdin-echo"), "spawn stdin.write did not queue a bridge chunk");
    child.stdin.end();
    var completed = await child.completion;
    equal(completed.status, 0, "spawn exit status");
    equal(output, "stdin-echo", "spawn stdin to stdout");
    stage = "stderr child";
    var stderrChild = cp.spawn("/bin/sh", ["-c", "printf stderr-check >&2"], { encoding: "utf8" });
    var stderr = "";
    stderrChild.stderr.on("data", function (chunk) { stderr += chunk; });
    await stderrChild.completion;
    equal(stderr, "stderr-check", "spawn stderr");
    stage = "exec callback";
    var callbackResult = await new Promise(function (resolve, reject) {
      var execChild = cp.exec("printf exec-check", function (error, stdout, stderrText) {
        if (error) reject(error);
        else resolve({ stdout: stdout, stderr: stderrText });
      });
      execChild.completion.catch(reject);
    });
    deepEqual(callbackResult, { stdout: "exec-check", stderr: "" }, "exec callback output");
    } catch (error) { throw new Error(`${stage}: ${error?.message || error}`); }
  });

  add("events", [
    "events.EventEmitter", "events.on", "events.addListener", "events.once", "events.prependListener",
    "events.prependOnceListener", "events.off", "events.removeListener", "events.removeAllListeners",
    "events.emit", "events.listeners", "events.rawListeners", "events.listenerCount", "events.eventNames",
  ], async function (env) {
    var EventEmitter = env.events.EventEmitter;
    var emitter = new EventEmitter();
    var calls = [];
    var original = function (value) { calls.push("on:" + value); };
    emitter.on("data", original);
    emitter.addListener("data", function (value) { calls.push("add:" + value); });
    emitter.prependListener("data", function (value) { calls.push("first:" + value); });
    emitter.once("data", function (value) { calls.push("once:" + value); });
    emitter.prependOnceListener("data", function (value) { calls.push("first-once:" + value); });
    equal(emitter.listenerCount("data"), 5, "events.listenerCount");
    equal(emitter.eventNames()[0], "data", "events.eventNames");
    equal(emitter.listeners("data").length, 5, "events.listeners");
    equal(emitter.rawListeners("data").length, 5, "events.rawListeners");
    emitter.emit("data", 7);
    deepEqual(calls, ["first-once:7", "first:7", "on:7", "add:7", "once:7"], "events emit order");
    var receiver = false;
    emitter.on("receiver", function () { receiver = this === emitter; });
    emitter.emit("receiver");
    equal(receiver, true, "events listener this binding");
    var unhandled = new Error("unhandled event error");
    expectThrow(function () { emitter.emit("error", unhandled); }, function (error) { return error === unhandled; }, "events unhandled error behavior");
    emitter.off("data", original);
    emitter.removeListener("data", emitter.listeners("data")[0]);
    emitter.removeAllListeners("data");
    equal(emitter.listenerCount("data"), 0, "events listener removal");
  });

  add("util", [
    "util.format", "util.inspect", "util.promisify", "util.callbackify", "util.isDeepStrictEqual", "util.deprecate",
  ], async function (env) {
    var util = env.util;
    equal(util.format("%s:%d:%%", "item", 2), "item:2:%", "util.format");
    equal(util.format("%s %d %i %f %j %c %%", "x", 2.5, "3x", "1.5", { a: 1 }, "style"), "x 2.5 3 1.5 {\"a\":1}  %", "util.format specifiers");
    assert(util.format("%o", { item: 2 }).includes("item: 2"), "util.format object specifier");
    assert(util.format("%O", { item: 2 }).includes("item: 2"), "util.format inspected-object specifier");
    assert(util.inspect({ item: 2 }).includes("item"), "util.inspect output");
    assert(util.inspect({ value: "x" }, { colors: true }).includes("\u001b["), "util.inspect colors option");
    var promisified = util.promisify(function (value, callback) { callback(null, value + 1); });
    equal(await promisified(4), 5, "util.promisify");
    var callbackified = util.callbackify(function (value) { return Promise.resolve(value + 2); });
    equal(await new Promise(function (resolve, reject) {
      callbackified(4, function (error, value) { if (error) reject(error); else resolve(value); });
    }), 6, "util.callbackify");
    equal(util.isDeepStrictEqual({ a: [1] }, { a: [1] }), true, "util.isDeepStrictEqual");
    var warningCount = 0;
    var previousWarn = root.console.warn;
    root.console.warn = function () { warningCount += 1; };
    try {
      var deprecated = util.deprecate(function (value) { return value; }, "deprecated integration function");
      equal(deprecated(1), 1, "util.deprecate wrapped return value");
      equal(deprecated(2), 2, "util.deprecate repeated return value");
    } finally {
      root.console.warn = previousWarn;
    }
    equal(warningCount, 1, "util.deprecate warns once");
  });

  add("querystring", [
    "querystring.parse", "querystring.stringify", "querystring.escape", "querystring.unescape",
    "querystring.decode", "querystring.encode",
  ], async function (env) {
    var qs = env.querystring;
    var parsed = qs.parse("a=one+two&x=1&x=2");
    equal(parsed.a, "one two", "querystring.parse plus decoding");
    deepEqual(parsed.x, ["1", "2"], "querystring.parse repeated keys");
    equal(Object.getPrototypeOf(parsed), null, "querystring.parse null prototype");
    equal(qs.stringify({ a: "one two", x: [1, 2] }), "a=one%20two&x=1&x=2", "querystring.stringify");
    equal(qs.escape("a b"), "a%20b", "querystring.escape");
    equal(qs.unescape("a%20b"), "a b", "querystring.unescape");
    equal(qs.decode("a~b:1", "~", ":").b, "1", "querystring.decode custom separators");
    equal(qs.encode({ a: "b c" }, "~", ":"), "a:b%20c", "querystring.encode custom separators");
    equal(Object.keys(qs.parse("a=1&b=2", "&", "=", { maxKeys: 1 })).length, 1, "querystring maxKeys");
    equal(qs.parse("a+b=%41", "&", "=", { decodeURIComponent: function (value) { return "decoded:" + value; } })["decoded:a b"], "decoded:%41", "querystring custom decoder");
    equal(qs.stringify({ "a b": "x/y" }, "&", "=", { encodeURIComponent: function (value) { return "[" + value + "]"; } }), "[a b]=[x/y]", "querystring custom encoder");
  });

  add("buffer", [
    "buffer.Buffer.from", "buffer.Buffer.alloc", "buffer.Buffer.allocUnsafe", "buffer.Buffer.concat",
    "buffer.Buffer.byteLength", "buffer.Buffer.isBuffer", "buffer.Buffer.isEncoding", "buffer.toString",
    "buffer.write", "buffer.fill", "buffer.equals", "buffer.compare", "buffer.copy", "buffer.indexOf",
    "buffer.lastIndexOf", "buffer.includes", "buffer.swap16", "buffer.swap32", "buffer.swap64",
    "buffer.toJSON", "buffer.readUInt8", "buffer.readInt8", "buffer.readUInt16LE", "buffer.readUInt16BE",
    "buffer.readInt16LE", "buffer.readInt16BE", "buffer.readUInt32LE", "buffer.readUInt32BE",
    "buffer.readInt32LE", "buffer.readInt32BE", "buffer.writeUInt8", "buffer.writeInt8",
    "buffer.writeUInt16LE", "buffer.writeUInt16BE", "buffer.writeInt16LE", "buffer.writeInt16BE",
    "buffer.writeUInt32LE", "buffer.writeUInt32BE", "buffer.writeInt32LE", "buffer.writeInt32BE",
  ], async function (env) {
    var Buffer = env.Buffer;
    var a = Buffer.from("café", "utf8");
    assert(Buffer.isBuffer(a), "Buffer.isBuffer");
    equal(a.toString("utf8"), "café", "Buffer.toString");
    equal(Buffer.from("niva", "utf8").toString("base64"), "bml2YQ==", "Buffer base64 encoding");
    equal(Buffer.from("bml2YQ==", "base64").toString("utf8"), "niva", "Buffer base64 decoding");
    equal(Buffer.from("🌍", "utf16le").toString("utf16le"), "🌍", "Buffer UTF-16LE encoding");
    equal(Buffer.from([0xe9]).toString("latin1"), "é", "Buffer latin1 encoding");
    equal(Buffer.from([0xe9]).toString("binary"), "é", "Buffer binary encoding");
    equal(Buffer.byteLength("café", "utf8"), 5, "Buffer.byteLength");
    assert(Buffer.isEncoding("base64url") && !Buffer.isEncoding("rot13"), "Buffer.isEncoding");
    var alloc = Buffer.alloc(4, 0x2a);
    equal(alloc.toString("hex"), "2a2a2a2a", "Buffer.alloc and fill");
    var unsafe = Buffer.allocUnsafe(2);
    unsafe.fill(0x61);
    equal(unsafe.toString("ascii"), "aa", "Buffer.allocUnsafe and fill");
    var joined = Buffer.concat([Buffer.from("a"), Buffer.from("b")]);
    equal(joined.toString(), "ab", "Buffer.concat");
    var written = Buffer.alloc(5);
    equal(written.write("abc", 1, 3, "utf8"), 3, "Buffer.write");
    deepEqual(Array.from(written), [0, 97, 98, 99, 0], "Buffer.write bytes");
    assert(Buffer.from([1, 2, 3]).equals(Buffer.from([1, 2, 3])), "Buffer.equals");
    equal(Buffer.compare(Buffer.from([1]), Buffer.from([2])), -1, "Buffer.compare");
    var copied = Buffer.alloc(2);
    equal(Buffer.from([5, 6, 7]).copy(copied, 0, 1), 2, "Buffer.copy");
    deepEqual(Array.from(copied), [6, 7], "Buffer.copy bytes");
    equal(Buffer.from("ababa").indexOf("ba"), 1, "Buffer.indexOf");
    equal(Buffer.from("ababa").lastIndexOf("ba"), 3, "Buffer.lastIndexOf");
    assert(Buffer.from("ababa").includes("bab"), "Buffer.includes");
    equal(Buffer.from([1, 2]).swap16().toString("hex"), "0201", "Buffer.swap16");
    equal(Buffer.from([1, 2, 3, 4]).swap32().toString("hex"), "04030201", "Buffer.swap32");
    equal(Buffer.from([1, 2, 3, 4, 5, 6, 7, 8]).swap64().toString("hex"), "0807060504030201", "Buffer.swap64");
    deepEqual(Buffer.from([1, 2]).toJSON(), { type: "Buffer", data: [1, 2] }, "Buffer.toJSON");
    var numeric = Buffer.alloc(28);
    numeric.writeUInt8(255, 0); numeric.writeInt8(-2, 1);
    numeric.writeUInt16LE(0x1234, 2); numeric.writeUInt16BE(0x2345, 4);
    numeric.writeInt16LE(-123, 6); numeric.writeInt16BE(-234, 8);
    numeric.writeUInt32LE(0x12345678, 10); numeric.writeUInt32BE(0x23456789, 14);
    numeric.writeInt32LE(-123456, 18); numeric.writeInt32BE(-234567, 22);
    equal(numeric.readUInt8(0), 255, "Buffer.readUInt8"); equal(numeric.readInt8(1), -2, "Buffer.readInt8");
    equal(numeric.readUInt16LE(2), 0x1234, "Buffer.readUInt16LE"); equal(numeric.readUInt16BE(4), 0x2345, "Buffer.readUInt16BE");
    equal(numeric.readInt16LE(6), -123, "Buffer.readInt16LE"); equal(numeric.readInt16BE(8), -234, "Buffer.readInt16BE");
    equal(numeric.readUInt32LE(10), 0x12345678, "Buffer.readUInt32LE"); equal(numeric.readUInt32BE(14), 0x23456789, "Buffer.readUInt32BE");
    equal(numeric.readInt32LE(18), -123456, "Buffer.readInt32LE"); equal(numeric.readInt32BE(22), -234567, "Buffer.readInt32BE");
  });

  add("url", [
    "url.URL", "url.URLSearchParams", "url.fileURLToPath", "url.pathToFileURL",
  ], async function (env) {
    var url = env.url;
    var targetPath = env.path.join(env.tempRoot, "url space #.txt");
    var fileUrl = url.pathToFileURL(targetPath);
    equal(fileUrl.protocol, "file:", "url.pathToFileURL protocol");
    equal(url.fileURLToPath(fileUrl), targetPath, "url.fileURLToPath round trip");
    expectThrow(function () { url.fileURLToPath("file:///tmp/a%2Fb"); }, function (error) { return error.code === "ERR_INVALID_FILE_URL_PATH"; }, "url.fileURLToPath rejects encoded separators");
    expectThrow(function () { url.fileURLToPath("file://example.invalid/tmp/a"); }, function (error) { return error.code === "ERR_INVALID_FILE_URL_HOST"; }, "url.fileURLToPath rejects remote POSIX host");
    var parsed = new url.URL("https://example.invalid/path?x=1");
    equal(parsed.hostname, "example.invalid", "url.URL");
    var query = new url.URLSearchParams("x=1&x=2");
    deepEqual(query.getAll("x"), ["1", "2"], "url.URLSearchParams");
  });

  add("crypto", [
    "crypto.randomBytes", "crypto.randomUUID", "crypto.createHash",
  ], async function (env) {
    var crypto = env.crypto;
    var bytes = await crypto.randomBytes(16);
    assert(env.Buffer.isBuffer(bytes) && bytes.length === 16, "crypto.randomBytes Buffer length");
    assert(/^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/i.test(await crypto.randomUUID()), "crypto.randomUUID");
    var digests = {
      sha1: "a9993e364706816aba3e25717850c26c9cd0d89d",
      sha256: "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad",
      sha384: "cb00753f45a35e8bb5a03d699ac65007272c32ab0eded1631a8b605a43ff5bed8086072ba1e7cc2358baeca134c825a7",
      sha512: "ddaf35a193617abacc417349ae20413112e6fa4e89a97ea20a9eeee64b55d39a2192992a274fc1a836ba3c23a3feebbd454d4423643ce80e2a9ac94fa54ca49f",
    };
    for (var algorithm of Object.keys(digests)) {
      var digest = await crypto.createHash(algorithm.toUpperCase().replace("SHA", "SHA-")).update("a").update("bc").digest("hex");
      equal(digest, digests[algorithm], "crypto.createHash " + algorithm);
    }
  });

  add("zlib", ["zlib.gzip", "zlib.gunzip"], async function (env) {
    var compressed = await env.zlib.gzip("niva-node-compat-gzip");
    assert(env.Buffer.isBuffer(compressed) && compressed.length > 0, "zlib.gzip returns compressed Buffer");
    equal((await env.zlib.gunzip(compressed)).toString("utf8"), "niva-node-compat-gzip", "zlib.gunzip round trip");
  });

  add("http", [
    "http.request", "http.get", "http.post", "ClientRequest.response", "ClientRequest.result",
    "ClientRequest.completion", "ClientRequest.on(response)",
    "ClientRequest.setHeader", "ClientRequest.getHeader", "ClientRequest.getHeaders", "ClientRequest.hasHeader",
    "ClientRequest.removeHeader", "ClientRequest.write", "ClientRequest.end", "IncomingMessage.statusCode",
    "IncomingMessage.headers", "IncomingMessage.setEncoding", "IncomingMessage.text", "IncomingMessage.json", "IncomingMessage.arrayBuffer",
    "IncomingMessage.buffer", "IncomingMessage.data", "IncomingMessage.end", "IncomingMessage.pipe",
  ], async function (env) {
    var stage = "GET";
    try {
    var baseUrl = await root.Niva.api.webview.baseUrl();
    var parsed = new root.URL(baseUrl);
    assert(parsed.hostname === "127.0.0.1" || parsed.hostname === "localhost", "debug base URL should be loopback");
    var requestUrl = baseUrl.endsWith("/") ? baseUrl : baseUrl + "/";
    var http = env.http;
    var dataChunks = [];
    var ended = false;
    var getRequest = http.get(requestUrl);
    getRequest.on("response", function (message) {
      equal(message.statusCode, 200, "http.get status");
      equal(message.setEncoding("utf8"), message, "IncomingMessage.setEncoding");
      message.on("data", function (chunk) { dataChunks.push(chunk); });
      message.once("end", function () { ended = true; });
    });
    var getResponse = await getRequest.result;
    equal(await getRequest.response, getResponse, "ClientRequest.response");
    equal(getRequest.completion, getRequest.result, "ClientRequest.completion alias");
    equal(getResponse.statusCode, 200, "http.get result response");
    assert((await getResponse.text()).includes("NodeCompat macOS integration smoke"), "http.get text response");
    equal(ended, true, "IncomingMessage end event");
    assert(dataChunks.length > 0, "IncomingMessage data event");
    var json = await getResponse.json().catch(function () { return null; });
    equal(json, null, "html response is not JSON");
    var bytes = await getResponse.arrayBuffer();
    assert(bytes.byteLength > 0, "IncomingMessage.arrayBuffer");
    assert((await getResponse.buffer()).length > 0, "IncomingMessage.buffer");

    var request = http.request({
      hostname: parsed.hostname,
      port: Number(parsed.port),
      path: "/",
      method: "POST",
      headers: { "x-niva-smoke": "first" },
    });
    stage = "POST request";
    equal(request.method, "POST", "ClientRequest method");
    request.setHeader("x-niva-smoke", "second");
    equal(request.getHeader("X-Niva-Smoke"), "second", "ClientRequest.getHeader");
    assert(request.hasHeader("x-niva-smoke"), "ClientRequest.hasHeader");
    assert(request.getHeaders()["x-niva-smoke"] === "second", "ClientRequest.getHeaders");
    request.removeHeader("x-niva-smoke");
    equal(request.hasHeader("x-niva-smoke"), false, "ClientRequest.removeHeader");
    request.write("buffered");
    var requestResult = await request.end().result;
    equal(requestResult.statusCode, 405, "http.request status");
    equal(await requestResult.text(), "GET only", "http.request.write body is accepted by native bridge");
    var sink = makeSink(env.EventEmitter);
    await env.stream.pipeline(requestResult, sink);
    equal(sink.chunks.join(""), "GET only", "IncomingMessage.pipe through stream.pipeline");

    var post = http.post(requestUrl, "disposable-post-body");
    stage = "POST convenience";
    var postResult = await post.result;
    equal(postResult.statusCode, 405, "http.post sees the local static server's GET-only status");
    } catch (error) { throw new Error(`${stage}: ${error?.message || error}`); }
  });

  add("https wrapper validation", ["https.request", "https.get", "https.post"], async function (env) {
    function rejectsWrongProtocol(operation) {
      return Promise.resolve().then(operation).then(function () {
        throw new Error("expected https wrapper to reject an http URL");
      }, function (error) {
        equal(error.code, "ERR_INVALID_PROTOCOL", "https rejects wrong scheme before networking");
      });
    }
    await rejectsWrongProtocol(function () { return env.https.request("http://example.invalid/"); });
    await rejectsWrongProtocol(function () { return env.https.get("http://example.invalid/"); });
    await rejectsWrongProtocol(function () { return env.https.post("http://example.invalid/", "body"); });
  });

  add("stream", [
    "stream.pipeline", "stream/promises.pipeline", "stream.pipeline.callback",
  ], async function (env) {
    var source = env.childProcess.spawn("/usr/bin/printf", ["pipeline-ok"], { encoding: "utf8" });
    var destination = makeSink(env.EventEmitter);
    equal(await env.stream.promises.pipeline(source.stdout, destination), destination, "stream/promises.pipeline destination");
    await source.completion;
    equal(destination.chunks.join(""), "pipeline-ok", "stream/promises.pipeline data");
    var second = env.childProcess.spawn("/usr/bin/printf", ["callback-ok"], { encoding: "utf8" });
    var secondDestination = makeSink(env.EventEmitter);
    var callbackResult = await new Promise(function (resolve, reject) {
      var returned = env.stream.pipeline(second.stdout, secondDestination, function (error) {
        if (error) reject(error); else resolve(returned);
      });
      equal(returned, secondDestination, "stream.pipeline callback return");
    });
    equal(callbackResult, secondDestination, "stream.pipeline callback completion");
    await second.completion;
    equal(secondDestination.chunks.join(""), "callback-ok", "stream.pipeline callback data");
  });

  add("assert and assert/strict", [
    "assert.callable", "assert.ok", "assert.fail", "assert.equal", "assert.notEqual",
    "assert.strictEqual", "assert.notStrictEqual", "assert.deepEqual", "assert.notDeepEqual",
    "assert.deepStrictEqual", "assert.notDeepStrictEqual", "assert.throws", "assert.doesNotThrow",
    "assert.rejects", "assert.doesNotReject", "assert.ifError", "assert.match", "assert.doesNotMatch",
    "assert.AssertionError", "assert.strict", "assert/strict.default", "assert/strict.strictEqual",
  ], async function (env) {
    var a = env.assert;
    var strict = env.assertStrict;
    a(true, "callable assertion"); a.ok(true); a.equal(1, "1"); a.notEqual(1, 2);
    a.strictEqual(1, 1); a.notStrictEqual(1, "1"); a.deepEqual({ x: 1 }, { x: "1" });
    a.notDeepEqual({ x: 1 }, { x: 2 }); a.deepStrictEqual([1], [1]); a.notDeepStrictEqual([1], [2]);
    a.doesNotThrow(function () {});
    expectThrow(function () { a.fail("expected failure"); }, function (error) { return error instanceof a.AssertionError; }, "assert.fail");
    a.throws(function () { throw new TypeError("expected"); }, TypeError);
    expectThrow(function () { a.throws(function () {}, TypeError); }, null, "assert.throws missing error");
    expectThrow(function () { strict.strictEqual(1, "1"); }, function (error) { return error instanceof a.AssertionError; }, "assert/strict.strictEqual");
    equal(await a.rejects(Promise.reject(new TypeError("expected")), TypeError).then(function () { return true; }), true, "assert.rejects");
    await a.doesNotReject(Promise.resolve("ok"));
    a.ifError(null);
    a.match("niva-compat", /compat/);
    a.doesNotMatch("niva-compat", /missing/);
    equal(typeof strict, "function", "assert/strict default export");
  });

  var catalog = checks.map(function (item) { return { id: item.id, methods: item.methods.slice() }; });

  async function run() {
    await root.NivaNodeCompatReady;
    var env = {
      nonce: String(Date.now()) + "-" + Math.random().toString(16).slice(2),
      path: root.Niva.require("path"),
      os: root.Niva.require("os"),
      fs: root.Niva.require("fs"),
      fsp: root.Niva.require("fs/promises"),
      childProcess: root.Niva.require("child_process"),
      events: root.Niva.require("events"),
      util: root.Niva.require("util"),
      querystring: root.Niva.require("querystring"),
      Buffer: root.Niva.require("buffer").Buffer,
      url: root.Niva.require("url"),
      crypto: root.Niva.require("crypto"),
      zlib: root.Niva.require("zlib"),
      http: root.Niva.require("http"),
      https: root.Niva.require("https"),
      assert: root.Niva.require("assert"),
      assertStrict: root.Niva.require("assert/strict"),
      stream: root.Niva.require("stream"),
      EventEmitter: root.Niva.require("events").EventEmitter,
      tempRoot: null,
    };
    var passed = [];
    for (var i = 0; i < checks.length; i += 1) {
      var item = checks[i];
      try {
        await item.run(env);
        passed.push.apply(passed, item.methods);
      } catch (error) {
        error.nodeCompatCase = item.id;
        error.nodeCompatPassedMethods = passed.slice();
        throw error;
      }
    }
    return { modules: modules.slice(), cases: catalog, passedMethods: passed };
  }

  root.NivaNodeCompatMacCases = { modules: modules, catalog: catalog, run: run };

  root.addEventListener("load", function () {
    var status = root.document.getElementById("status");
    run().then(function (report) {
      status.textContent = "PASS — " + report.passedMethods.length + " NodeCompat methods and aliases";
      return root.Niva.api.host.send("nodecompat-macos-result", report);
    }).then(function () {
      root.setTimeout(function () { root.Niva.api.process.exit().catch(function () {}); }, 250);
    }).catch(function (error) {
      status.textContent = "FAIL — " + String(error.nodeCompatCase || "startup");
      var failure = {
        case: error.nodeCompatCase || "startup",
        message: String(error && error.message || error).slice(0, 1200),
        passedMethods: error.nodeCompatPassedMethods || [],
      };
      root.Niva.api.host.send("nodecompat-macos-failure", failure).catch(function () {}).then(function () {
        root.setTimeout(function () { root.Niva.api.process.exit().catch(function () {}); }, 250);
      });
    });
  }, { once: true });
})(globalThis);
