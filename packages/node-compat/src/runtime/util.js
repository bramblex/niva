(function (root) {
  "use strict";

  var runtime = root[Symbol.for("niva.node-compat.runtime")];
  var inspectColors = { string: 32, number: 33, bigint: 33, boolean: 33, undefined: 90, null: 1, special: 36, name: 34 };

  function paint(value, kind, colors) {
    if (!colors || typeof value !== "string") return value;
    var code = inspectColors[kind];
    return code ? "\u001b[" + code + "m" + value + "\u001b[39m" : value;
  }

  function inspect(value, options) {
    options = options || {};
    var depth = options.depth === undefined ? 2 : options.depth;
    var colors = !!options.colors;
    var seen = [];

    function visit(input, remaining) {
      if (input === null) return paint("null", "null", colors);
      if (input === undefined) return paint("undefined", "undefined", colors);
      var type = typeof input;
      if (type === "string") return paint("'" + input.replace(/\\/g, "\\\\").replace(/'/g, "\\'").replace(/\n/g, "\\n").replace(/\r/g, "\\r") + "'", "string", colors);
      if (type === "number") return paint(Object.is(input, -0) ? "-0" : String(input), "number", colors);
      if (type === "bigint") return paint(String(input) + "n", "bigint", colors);
      if (type === "boolean") return paint(String(input), "boolean", colors);
      if (type === "symbol") return paint(input.toString(), "special", colors);
      if (type === "function") return "[Function" + (input.name ? ": " + input.name : "") + "]";
      if (seen.indexOf(input) >= 0) return "[Circular]";
      if (remaining < 0) {
        if (Array.isArray(input)) return "[Array]";
        if (input instanceof Map) return "[Map]";
        if (input instanceof Set) return "[Set]";
        return "[Object]";
      }
      if (input instanceof Date) return Number.isNaN(input.getTime()) ? "Invalid Date" : input.toISOString();
      if (input instanceof RegExp) return input.toString();
      if (input instanceof Error) return input.name + (input.message ? ": " + input.message : "");
      var bufferApi = runtime.buffer && runtime.buffer.Buffer;
      if (bufferApi && bufferApi.isBuffer(input)) {
        return "<Buffer " + Array.prototype.map.call(input, function (byte) { return byte.toString(16).padStart(2, "0"); }).join(" ") + ">";
      }
      if (ArrayBuffer.isView(input) && !(input instanceof DataView)) {
        return "<" + input.constructor.name + " " + Array.prototype.join.call(input, ", ") + ">";
      }
      if (input instanceof ArrayBuffer) return "<ArrayBuffer " + input.byteLength + ">";

      seen.push(input);
      var output;
      if (Array.isArray(input)) {
        output = input.length
          ? "[ " + input.map(function (item) { return visit(item, remaining - 1); }).join(", ") + " ]"
          : "[]";
      } else if (input instanceof Map) {
        output = "Map(" + input.size + ") { " + Array.from(input.entries()).map(function (entry) {
          return visit(entry[0], remaining - 1) + " => " + visit(entry[1], remaining - 1);
        }).join(", ") + " }";
      } else if (input instanceof Set) {
        output = "Set(" + input.size + ") { " + Array.from(input.values()).map(function (item) { return visit(item, remaining - 1); }).join(", ") + " }";
      } else {
        var keys = Object.keys(input);
        if (Object.getOwnPropertySymbols) {
          Object.getOwnPropertySymbols(input).forEach(function (symbol) {
            var descriptor = Object.getOwnPropertyDescriptor(input, symbol);
            if (descriptor && descriptor.enumerable) keys.push(symbol);
          });
        }
        var entries = keys.map(function (key) {
          var label = typeof key === "symbol" ? "[" + key.toString() + "]" : (/^[A-Za-z_$][\w$]*$/.test(key) ? key : visit(key, remaining - 1));
          var field;
          try { field = visit(input[key], remaining - 1); }
          catch (error) { field = "[Thrown: " + error.message + "]"; }
          return paint(label, "name", colors) + ": " + field;
        });
        output = "{" + (entries.length ? " " + entries.join(", ") + " " : "") + "}";
      }
      seen.pop();
      return output;
    }

    return visit(value, depth);
  }

  function format(first) {
    var args = Array.prototype.slice.call(arguments, 1);
    if (typeof first !== "string") return [first].concat(args).map(function (value) { return inspect(value); }).join(" ");
    var index = 0;
    var output = first.replace(/%[sdifjoOc%]/g, function (token) {
      if (token === "%%") return "%";
      if (index >= args.length) return token;
      var value = args[index++];
      if (token === "%c") return "";
      switch (token) {
        case "%s": return typeof value === "object" && value !== null ? inspect(value, { depth: 0 }) : String(value);
        case "%d": return typeof value === "bigint" ? String(value) : String(Number(value));
        case "%i": return String(parseInt(value, 10));
        case "%f": return String(parseFloat(value));
        case "%j":
          try { return JSON.stringify(value); }
          catch (_) { return "[Circular]"; }
        case "%o": return inspect(value, { depth: 4 });
        case "%O": return inspect(value, { depth: 2 });
        default: return token;
      }
    });
    while (index < args.length) {
      var extra = args[index++];
      output += " " + (typeof extra === "string" ? extra : inspect(extra));
    }
    return output;
  }

  function processTick(callback) {
    if (typeof queueMicrotask === "function") queueMicrotask(callback);
    else Promise.resolve().then(callback);
  }

  function promisify(fn) {
    if (typeof fn !== "function") throw new TypeError("The \"original\" argument must be of type Function");
    if (fn[promisify.custom]) {
      if (typeof fn[promisify.custom] !== "function") throw new TypeError("util.promisify.custom must be a function");
      return fn[promisify.custom];
    }
    function wrapped() {
      var receiver = this;
      var args = Array.prototype.slice.call(arguments);
      return new Promise(function (resolve, reject) {
        args.push(function (error, value) {
          if (error) reject(error);
          else resolve(value);
        });
        try { fn.apply(receiver, args); }
        catch (error) { reject(error); }
      });
    }
    Object.defineProperty(wrapped, "name", { value: "promisified " + (fn.name || "function"), configurable: true });
    return wrapped;
  }
  promisify.custom = Symbol.for("nodejs.util.promisify.custom");
  promisify.customArgs = Symbol.for("nodejs.util.promisify.customArgs");

  function callbackify(fn) {
    if (typeof fn !== "function") throw new TypeError("The \"original\" argument must be of type Function");
    function wrapped() {
      var receiver = this;
      var args = Array.prototype.slice.call(arguments);
      var callback = args.pop();
      if (typeof callback !== "function") throw new TypeError("The last argument must be of type Function");
      var result;
      try { result = fn.apply(receiver, args); }
      catch (error) { processTick(function () { callback(error); }); return; }
      Promise.resolve(result).then(function (value) {
        processTick(function () { callback(null, value); });
      }, function (reason) {
        var error = reason;
        if (!reason) {
          error = new Error("Promise was rejected with a falsy value");
          error.reason = reason;
          error.code = "ERR_FALSY_VALUE_REJECTION";
        }
        processTick(function () { callback(error); });
      });
    }
    Object.defineProperty(wrapped, "name", { value: "callbackified " + (fn.name || "function"), configurable: true });
    return wrapped;
  }

  function enumerableKeys(value) {
    var keys = Object.keys(value);
    if (Object.getOwnPropertySymbols) {
      Object.getOwnPropertySymbols(value).forEach(function (symbol) {
        var descriptor = Object.getOwnPropertyDescriptor(value, symbol);
        if (descriptor && descriptor.enumerable) keys.push(symbol);
      });
    }
    return keys;
  }

  function isDeepStrictEqual(left, right) {
    function equal(a, b, seenA, seenB) {
      if (Object.is(a, b)) return true;
      if (a === null || b === null || typeof a !== "object" || typeof b !== "object") return false;
      if (Object.getPrototypeOf(a) !== Object.getPrototypeOf(b)) return false;
      var index = seenA.indexOf(a);
      if (index >= 0) return seenB[index] === b;
      seenA = seenA.concat([a]);
      seenB = seenB.concat([b]);

      if (Array.isArray(a) && a.length !== b.length) return false;
      if (a instanceof Date) return a.getTime() === b.getTime();
      if (a instanceof RegExp) return a.source === b.source && a.flags === b.flags;
      if (typeof URL !== "undefined" && a instanceof URL) return b instanceof URL && a.href === b.href;
      if (a instanceof Number || a instanceof String || a instanceof Boolean) return a.valueOf() === b.valueOf();
      var bufferApi = runtime.buffer && runtime.buffer.Buffer;
      if (bufferApi && bufferApi.isBuffer(a)) return bufferApi.isBuffer(b) && bytesEqual(a, b);
      if (ArrayBuffer.isView(a)) {
        if (!ArrayBuffer.isView(b) || a.constructor !== b.constructor || a.byteLength !== b.byteLength) return false;
        return bytesEqual(new Uint8Array(a.buffer, a.byteOffset, a.byteLength), new Uint8Array(b.buffer, b.byteOffset, b.byteLength));
      }
      if (a instanceof ArrayBuffer) return b instanceof ArrayBuffer && bytesEqual(new Uint8Array(a), new Uint8Array(b));
      if (a instanceof Map) {
        if (!(b instanceof Map) || a.size !== b.size) return false;
        var rightEntries = Array.from(b.entries());
        for (var mi = 0; mi < a.size; mi += 1) {
          var leftEntry = Array.from(a.entries())[mi];
          var found = -1;
          for (var mj = 0; mj < rightEntries.length; mj += 1) {
            if (equal(leftEntry[0], rightEntries[mj][0], seenA, seenB) && equal(leftEntry[1], rightEntries[mj][1], seenA, seenB)) { found = mj; break; }
          }
          if (found < 0) return false;
          rightEntries.splice(found, 1);
        }
        return true;
      }
      if (a instanceof Set) {
        if (!(b instanceof Set) || a.size !== b.size) return false;
        var rightValues = Array.from(b.values());
        var leftValues = Array.from(a.values());
        for (var si = 0; si < leftValues.length; si += 1) {
          var match = -1;
          for (var sj = 0; sj < rightValues.length; sj += 1) {
            if (equal(leftValues[si], rightValues[sj], seenA, seenB)) { match = sj; break; }
          }
          if (match < 0) return false;
          rightValues.splice(match, 1);
        }
        return true;
      }
      if (a instanceof Error && (a.name !== b.name || a.message !== b.message)) return false;
      var keysA = enumerableKeys(a);
      var keysB = enumerableKeys(b);
      if (keysA.length !== keysB.length) return false;
      for (var i = 0; i < keysA.length; i += 1) {
        var key = keysA[i];
        if (!Object.prototype.propertyIsEnumerable.call(b, key) || !equal(a[key], b[key], seenA, seenB)) return false;
      }
      return true;
    }

    return equal(left, right, [], []);
  }

  function bytesEqual(a, b) {
    if (a.length !== b.length) return false;
    for (var i = 0; i < a.length; i += 1) if (a[i] !== b[i]) return false;
    return true;
  }

  function deprecate(fn, message, code) {
    if (typeof fn !== "function") throw new TypeError("The \"fn\" argument must be of type Function");
    var warned = false;
    function deprecated() {
      if (!warned && !deprecate.noDeprecation) {
        warned = true;
        var warning = new Error(message || "This function is deprecated.");
        warning.name = "DeprecationWarning";
        if (code) warning.code = code;
        if (typeof console !== "undefined" && typeof console.warn === "function") console.warn(warning);
      }
      return fn.apply(this, arguments);
    }
    Object.defineProperty(deprecated, "name", { value: fn.name, configurable: true });
    return deprecated;
  }
  deprecate.noDeprecation = false;

  var module = {
    format: format,
    inspect: inspect,
    promisify: promisify,
    callbackify: callbackify,
    isDeepStrictEqual: isDeepStrictEqual,
    deprecate: deprecate,
  };
  runtime.createUtilModule = function () { return module; };
  runtime.util = module;
})(globalThis);
