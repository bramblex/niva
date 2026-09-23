(function (root) {
  "use strict";

  var runtime = root[Symbol.for("niva.node-compat.runtime")];

  class AssertionError extends Error {
    constructor(options) {
      options = options || {};
      var message = options.message || makeMessage(options);
      super(message);
      this.name = "AssertionError";
      this.code = "ERR_ASSERTION";
      this.actual = options.actual;
      this.expected = options.expected;
      this.operator = options.operator || "===";
      this.generatedMessage = !options.message;
      if (Error.captureStackTrace) Error.captureStackTrace(this, options.stackStartFn || AssertionError);
    }
  }

  function makeMessage(options) {
    var inspect = runtime.util.inspect;
    return "Expected " + inspect(options.actual) + " " + (options.operator || "===") + " " + inspect(options.expected);
  }

  function fail(actual, expected, message, operator, stackStartFn) {
    throw new AssertionError({ actual: actual, expected: expected, message: message, operator: operator, stackStartFn: stackStartFn || fail });
  }

  function ok(value, message) {
    if (!value) fail(value, true, message, "==", ok);
  }

  function strictEqual(actual, expected, message) {
    if (!Object.is(actual, expected)) fail(actual, expected, message, "strictEqual", strictEqual);
  }

  function notStrictEqual(actual, expected, message) {
    if (Object.is(actual, expected)) fail(actual, expected, message, "notStrictEqual", notStrictEqual);
  }

  function deepStrictEqual(actual, expected, message) {
    if (!runtime.util.isDeepStrictEqual(actual, expected)) fail(actual, expected, message, "deepStrictEqual", deepStrictEqual);
  }

  function notDeepStrictEqual(actual, expected, message) {
    if (runtime.util.isDeepStrictEqual(actual, expected)) fail(actual, expected, message, "notDeepStrictEqual", notDeepStrictEqual);
  }

  function looseDeepEqual(actual, expected, message) {
    if (!isDeepEqual(actual, expected, [], [])) fail(actual, expected, message, "deepEqual", looseDeepEqual);
  }

  function notLooseDeepEqual(actual, expected, message) {
    if (isDeepEqual(actual, expected, [], [])) fail(actual, expected, message, "notDeepEqual", notLooseDeepEqual);
  }

  function isDeepEqual(actual, expected, seenActual, seenExpected) {
    if (Object.is(actual, expected) || actual == expected) return true;
    if (actual === null || expected === null || typeof actual !== "object" || typeof expected !== "object") return false;
    var seenIndex = seenActual.indexOf(actual);
    if (seenIndex >= 0) return seenExpected[seenIndex] === expected;
    if (Array.isArray(actual) !== Array.isArray(expected) || (Array.isArray(actual) && actual.length !== expected.length)) return false;
    if (Object.prototype.toString.call(actual) !== Object.prototype.toString.call(expected)) return false;
    seenActual = seenActual.concat([actual]);
    seenExpected = seenExpected.concat([expected]);
    if (actual instanceof Date) return actual.getTime() === expected.getTime();
    if (actual instanceof RegExp) return actual.source === expected.source && actual.flags === expected.flags;
    if (actual instanceof Map || actual instanceof Set) return runtime.util.isDeepStrictEqual(actual, expected);
    if (ArrayBuffer.isView(actual)) {
      if (!ArrayBuffer.isView(expected) || actual.constructor !== expected.constructor || actual.byteLength !== expected.byteLength) return false;
      var actualBytes = new Uint8Array(actual.buffer, actual.byteOffset, actual.byteLength);
      var expectedBytes = new Uint8Array(expected.buffer, expected.byteOffset, expected.byteLength);
      for (var byteIndex = 0; byteIndex < actualBytes.length; byteIndex += 1) if (actualBytes[byteIndex] !== expectedBytes[byteIndex]) return false;
      return true;
    }
    var actualKeys = Object.keys(actual);
    var expectedKeys = Object.keys(expected);
    if (actualKeys.length !== expectedKeys.length) return false;
    for (var i = 0; i < actualKeys.length; i += 1) {
      var key = actualKeys[i];
      if (!Object.prototype.hasOwnProperty.call(expected, key) || !isDeepEqual(actual[key], expected[key], seenActual, seenExpected)) return false;
    }
    return true;
  }

  function matchesExpected(error, expected) {
    if (!expected) return true;
    if (expected instanceof RegExp) return expected.test(String(error && error.message));
    if (typeof expected === "function") {
      if (expected.prototype instanceof Error || expected === Error || expected === TypeError || expected === RangeError) return error instanceof expected;
      return !!expected(error);
    }
    if (expected && typeof expected === "object") {
      return Object.keys(expected).every(function (key) {
        var expectedValue = expected[key];
        var actualValue = error && error[key];
        if (expectedValue instanceof RegExp) return expectedValue.test(String(actualValue));
        if (typeof expectedValue === "function") return actualValue instanceof expectedValue;
        return Object.is(actualValue, expectedValue);
      });
    }
    throw new TypeError("The expected argument must be a RegExp, Error constructor, predicate, or object");
  }

  function throws(block, expected, message) {
    if (typeof block !== "function") throw new TypeError("block must be a function");
    if (typeof expected === "string" && message === undefined) { message = expected; expected = undefined; }
    var thrown;
    try { block(); } catch (error) { thrown = error; }
    if (!thrown) fail(undefined, expected || Error, message || "Missing expected exception", "throws", throws);
    if (!matchesExpected(thrown, expected)) fail(thrown, expected, message || "The error did not match the expected assertion", "throws", throws);
    return thrown;
  }

  function doesNotThrow(block, expected, message) {
    if (typeof block !== "function") throw new TypeError("block must be a function");
    if (typeof expected === "string" && message === undefined) { message = expected; expected = undefined; }
    try { return block(); }
    catch (error) {
      if (!expected || matchesExpected(error, expected)) fail(error, undefined, message || "Got unwanted exception", "doesNotThrow", doesNotThrow);
      throw error;
    }
  }

  function rejects(promiseOrFn, expected, message) {
    if (typeof expected === "string" && message === undefined) { message = expected; expected = undefined; }
    var promise;
    try { promise = typeof promiseOrFn === "function" ? promiseOrFn() : promiseOrFn; }
    catch (error) { promise = Promise.reject(error); }
    return Promise.resolve(promise).then(function () {
      fail(undefined, expected || Error, message || "Missing expected rejection", "rejects", rejects);
    }, function (error) {
      if (!matchesExpected(error, expected)) fail(error, expected, message || "The rejection did not match the expected assertion", "rejects", rejects);
      return error;
    });
  }

  function doesNotReject(promiseOrFn, expected, message) {
    if (typeof expected === "string" && message === undefined) { message = expected; expected = undefined; }
    var promise;
    try { promise = typeof promiseOrFn === "function" ? promiseOrFn() : promiseOrFn; }
    catch (error) { promise = Promise.reject(error); }
    return Promise.resolve(promise).catch(function (error) {
      if (!expected || matchesExpected(error, expected)) fail(error, undefined, message || "Got unwanted rejection", "doesNotReject", doesNotReject);
      throw error;
    });
  }

  function ifError(value) {
    if (value !== null && value !== undefined) throw value;
  }

  function match(actual, regexp, message) {
    if (!(regexp instanceof RegExp)) throw new TypeError("regexp must be a RegExp");
    if (typeof actual !== "string" || !regexp.test(actual)) fail(actual, regexp, message, "match", match);
  }

  function doesNotMatch(actual, regexp, message) {
    if (!(regexp instanceof RegExp)) throw new TypeError("regexp must be a RegExp");
    if (typeof actual !== "string" || regexp.test(actual)) fail(actual, regexp, message, "doesNotMatch", doesNotMatch);
  }

  function assert(value, message) { return ok(value, message); }
  function strictAssert(value, message) { return ok(value, message); }

  function attachCommon(target) {
    target.AssertionError = AssertionError;
    target.fail = fail;
    target.ok = ok;
    target.strictEqual = strictEqual;
    target.notStrictEqual = notStrictEqual;
    target.deepStrictEqual = deepStrictEqual;
    target.notDeepStrictEqual = notDeepStrictEqual;
    target.throws = throws;
    target.doesNotThrow = doesNotThrow;
    target.rejects = rejects;
    target.doesNotReject = doesNotReject;
    target.ifError = ifError;
    target.match = match;
    target.doesNotMatch = doesNotMatch;
  }

  attachCommon(assert);
  assert.equal = function (actual, expected, message) {
    if (actual != expected) fail(actual, expected, message, "==", assert.equal);
  };
  assert.notEqual = function (actual, expected, message) {
    if (actual == expected) fail(actual, expected, message, "!=", assert.notEqual);
  };
  assert.deepEqual = looseDeepEqual;
  assert.notDeepEqual = notLooseDeepEqual;

  attachCommon(strictAssert);
  strictAssert.equal = strictEqual;
  strictAssert.notEqual = notStrictEqual;
  strictAssert.deepEqual = deepStrictEqual;
  strictAssert.notDeepEqual = notDeepStrictEqual;
  strictAssert.strict = strictAssert;
  assert.strict = strictAssert;

  var module = assert;
  module.AssertionError = AssertionError;
  runtime.createAssertModule = function () { return module; };
  runtime.assert = module;
  runtime.strictAssert = strictAssert;
})(globalThis);
