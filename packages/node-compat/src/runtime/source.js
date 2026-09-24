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
// https://github.com/nodejs/node/blob/5d2feb257bcee090e57900eb51720171a6aa92f3/lib/internal/assert/utils.js
(function (root) {
  "use strict";
  var runtime = root[Symbol.for("niva.node-compat.runtime")];
  if (runtime.source) return;
  var sources = new Map();
  var MAX_SOURCE = 512 * 1024;
  var MAX_CACHED = 8;

  function normalizeName(name) {
    if (name.startsWith("file://")) {
      try {
        var url = new URL(name);
        name = decodeURIComponent(url.pathname);
        if (/^\/[A-Za-z]:\//.test(name)) name = name.slice(1);
        if (url.hostname && url.hostname !== "localhost") name = "//" + url.hostname + name;
      } catch (_) {}
    }
    return name.replace(/\\/g, "/");
  }

  function registerSource(name, text) {
    if (typeof name !== "string" || typeof text !== "string") throw new TypeError("Source name and text must be strings");
    name = normalizeName(name);
    if (!sources.has(name) && sources.size >= MAX_CACHED) sources.delete(sources.keys().next().value);
    sources.set(name, text.length <= MAX_SOURCE ? text : undefined);
  }

  function readSource(name) {
    var key = normalizeName(name);
    if (sources.has(key)) return sources.get(key);
    if (!root.location || typeof root.XMLHttpRequest !== "function") return undefined;
    try {
      var base = new URL(root.location.href);
      var url = new URL(name, base);
      // Diagnostics may read this page's own scripts, not arbitrary network/filesystem paths.
      if (!["http:", "https:", "niva:"].includes(url.protocol) || url.protocol !== base.protocol || url.host !== base.host || url.username || url.password) return undefined;
      var request = new root.XMLHttpRequest();
      request.open("GET", url.href, false);
      request.send();
      if (request.status !== 200 && !(request.status === 0 && url.protocol === "niva:")) return undefined;
      registerSource(name, request.responseText);
      return sources.get(key);
    } catch (_) { return undefined; }
  }

  function parseStackFrame(line) {
    var location, name;
    var v8 = /^\s*at (.*)$/.exec(line);
    if (v8) {
      var body = v8[1];
      var open = body.lastIndexOf(" (");
      name = open >= 0 ? body.slice(0, open) : "";
      location = open >= 0 && body.endsWith(")") ? body.slice(open + 2, -1) : body;
    } else {
      var at = line.indexOf("@");
      if (at < 0) return undefined;
      name = line.slice(0, at);
      location = line.slice(at + 1);
    }
    var match = /^(.*):(\d+):(\d+)$/.exec(location);
    if (!match) return undefined;
    return {name: name, filename: match[1], line: Number(match[2]), column: Number(match[3])};
  }

  function callerFrame(fn) {
    var error, captured = false, stack;
    var descriptor = Object.getOwnPropertyDescriptor(Error, "stackTraceLimit");
    if (descriptor) Object.setPrototypeOf(descriptor, null);
    var changed = !!descriptor && descriptor.writable;
    try {
      if (changed) Error.stackTraceLimit = 20;
      if (typeof Error.captureStackTrace === "function") {
        error = {};
        Error.captureStackTrace(error, fn);
        captured = true;
      } else error = new Error();
      stack = error.stack;
    } finally {
      if (changed) Object.defineProperty(Error, "stackTraceLimit", descriptor);
    }
    if (typeof stack !== "string") return undefined;
    var frames = stack.split("\n").map(parseStackFrame).filter(Boolean);
    var isRuntimeFrame = function (frame) { return /\/(?:__niva_compat\/|runtime\/(?:source|assert)\.js)/.test(frame.filename); };
    if (captured && frames.length && !isRuntimeFrame(frames[0])) return frames[0];
    // JavaScriptCore may tail-call-eliminate the assertion wrapper. Its
    // captureStackTrace then has no matching constructor frame; recover from
    // the ordinary stack and skip our separately loaded runtime script.
    if (captured) {
      try {
        if (changed) Error.stackTraceLimit = 20;
        frames = String(new Error().stack || "").split("\n").map(parseStackFrame).filter(Boolean);
      } finally {
        if (changed) Object.defineProperty(Error, "stackTraceLimit", descriptor);
      }
      return frames.find(function (frame) {
        return !isRuntimeFrame(frame);
      });
    }
    var functionName = fn && fn.name;
    for (var i = 0; i < frames.length - 1; i++) {
      if (frames[i].name.split(".").pop() === functionName) return frames[i + 1];
    }
    return undefined;
  }

  // Adapted from Node v22.14.0 lib/internal/assert/utils.js (MIT; upstream LICENSE
  // is retained under upstream/node-v22.14.0). Uses the same Acorn expression
  // selection and indentation rules, with browser source/stack acquisition.
  function expressionAt(code, column) {
    var parser = runtime.vendor.acorn;
    var walk = runtime.vendor.acornWalk;
    var options = {ecmaVersion: "latest"};
    for (var token of parser.tokenizer(code, options)) {
      if (token.start > column) break;
      try {
        var expression = parser.parseExpressionAt(code, token.start, options);
        var found = walk.findNodeAround(expression, column, "CallExpression");
        if (!found || found.node.end < column) continue;
        var text = code.slice(found.node.start, found.node.end).replace(/[\x00-\x08\x0b\x0c\x0e-\x1f]/g, function (char) {
          return char === "\b" ? "\\b" : char === "\f" ? "\\f" : "\\u" + char.charCodeAt(0).toString(16).padStart(4, "0");
        });
        var lines = text.replace(/\r\n/g, "\n").split("\n");
        return lines.map(function (line, index) {
          if (!index) return line;
          var pos = 0;
          while (pos < found.node.start && (line[pos] === " " || line[pos] === "\t")) pos++;
          return "  " + line.slice(pos);
        }).join("\n");
      } catch (_) {}
    }
    return undefined;
  }

  function getAssertionExpression(fn) {
    try {
      var frame = callerFrame(fn);
      if (!frame || frame.line < 1 || frame.column < 1) return undefined;
      var source = readSource(frame.filename);
      if (source === undefined) return undefined;
      var offset = 0;
      for (var line = 1; line < frame.line; line++) {
        var next = source.indexOf("\n", offset);
        if (next < 0) return undefined;
        offset = next + 1;
      }
      var column = frame.column - 1;
      return expressionAt(source.slice(offset, offset + column + 2500), column);
    } catch (_) { return undefined; }
  }

  runtime.source = {registerSource: registerSource, getAssertionExpression: getAssertionExpression};
})(globalThis);
