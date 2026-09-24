(function (root) {
  "use strict";

  var runtime = root[Symbol.for("niva.node-compat.runtime")];
  if (typeof runtime.createAssertModule === "function") return;

  class AssertionError extends Error {
    constructor(options) {
      if (options === null || typeof options !== "object") throw invalidArgType("options", "of type object", options);
      var message = options.generatedMessage === true && options.message !== undefined
        ? String(options.message)
        : options.message == null
          ? makeMessage(options)
          : customMessage(options);
      super(message);
      this.name = "AssertionError";
      this.code = "ERR_ASSERTION";
      this.actual = options.actual;
      this.expected = options.expected;
      this.operator = options.operator || "===";
      this.generatedMessage = options.generatedMessage === undefined ? !options.message : options.generatedMessage;
      if (Error.captureStackTrace) Error.captureStackTrace(this, options.stackStartFn || options.stackStartFunction || AssertionError);
      var assertionError = this;
      Object.defineProperty(this, Symbol.for("nodejs.util.inspect.custom"), {
        __proto__: null,
        configurable: true,
        value: function (_depth, _context, inspect) {
          function preview(value) {
            if (typeof value !== "string") return inspect(value, { customInspect: false, depth: -1 });
            if (value.length > 512) return "'" + value.slice(0, 488).replace(/\\/g, "\\\\").replace(/'/g, "\\'") + "...'";
            var lines = value.split("\n");
            if (lines.length > 11) {
              var chunks = [];
              for (var i = 0; i < 10; i += 1) chunks.push(inspect(lines[i] + "\n", { customInspect: false }));
              return chunks[0] + " +\n" + chunks.slice(1).map(function (chunk) { return "    " + chunk + " +\n"; }).join("") + "    '...'";
            }
            return inspect(value, { customInspect: false });
          }
          var stack = assertionError.stack || "AssertionError: " + assertionError.message;
          stack = stack.replace(/^AssertionError: /, "AssertionError [ERR_ASSERTION]: ");
          var fields = [
            "  generatedMessage: " + inspect(assertionError.generatedMessage),
            "  code: " + inspect(assertionError.code),
            "  actual: " + preview(assertionError.actual),
            "  expected: " + preview(assertionError.expected),
            "  operator: " + inspect(assertionError.operator),
          ];
          return stack + "\n{\n" + fields.join(",\n") + "\n}";
        },
      });
    }
  }

  class Comparison {
    constructor() {}
  }

  function makeMessage(options) {
    return assertionMessage(options.actual, options.expected, options.operator || "===");
  }

  function isErrorValue(value) {
    if (value instanceof Error) return true;
    try { return Object.prototype.toString.call(value) === "[object Error]"; } catch (_) { return false; }
  }

  function assertionInspect(value) {
    return runtime.util.inspect(value, { compact: false, customInspect: false, depth: 1000, maxArrayLength: null, showHidden: false, sorted: true, getters: true });
  }

  function readableOperator(operator) {
    return {
      deepStrictEqual: "Expected values to be strictly deep-equal:",
      strictEqual: "Expected values to be strictly equal:",
      strictEqualObject: 'Expected "actual" to be reference-equal to "expected":',
      deepEqual: "Expected values to be loosely deep-equal:",
      notDeepStrictEqual: 'Expected "actual" not to be strictly deep-equal to:',
      partialDeepStrictEqual: "Expected values to be partially and strictly deep-equal:",
      notStrictEqual: 'Expected "actual" to be strictly unequal to:',
      notStrictEqualObject: 'Expected "actual" not to be reference-equal to "expected":',
      notDeepEqual: 'Expected "actual" not to be loosely deep-equal to:',
      notIdentical: "Values have same structure but are not reference-equal:",
      notDeepEqualUnequal: "Expected values not to be loosely deep-equal:",
    }[operator] || "Expected values to satisfy the assertion:";
  }

  // Assertion message formatting is adapted from Node.js v22.14.0, commit
  // 5d2feb257bcee090e57900eb51720171a6aa92f3:
  // lib/internal/assert/assertion_error.js and myers_diff.js. Copyright Node.js
  // contributors; MIT license is retained at upstream/node-v22.14.0/LICENSE.
  function linesEqual(actual, expected, checkCommaDisparity) {
    return actual === expected || checkCommaDisparity && (actual + "," === expected || actual === expected + ",");
  }

  function myersDiff(actual, expected, checkCommaDisparity) {
    var actualLength = actual.length, expectedLength = expected.length, max = actualLength + expectedLength;
    var vector = new Int32Array(2 * max + 1), trace = [];
    for (var level = 0; level <= max; level += 1) {
      trace.push(vector.slice());
      for (var diagonal = -level; diagonal <= level; diagonal += 2) {
        var offset = diagonal + max, previous = vector[offset - 1], next = vector[offset + 1];
        var x = diagonal === -level || diagonal !== level && previous < next ? next : previous + 1;
        var y = x - diagonal;
        while (x < actualLength && y < expectedLength && linesEqual(actual[x], expected[y], checkCommaDisparity)) { x += 1; y += 1; }
        vector[offset] = x;
        if (x >= actualLength && y >= expectedLength) return backtrackDiff(trace, actual, expected, checkCommaDisparity, max);
      }
    }
    return [];
  }

  function backtrackDiff(trace, actual, expected, checkCommaDisparity, max) {
    var x = actual.length, y = expected.length, result = [];
    for (var level = trace.length - 1; level >= 0; level -= 1) {
      var vector = trace[level], diagonal = x - y, offset = diagonal + max;
      var previousDiagonal = diagonal === -level || diagonal !== level && vector[offset - 1] < vector[offset + 1] ? diagonal + 1 : diagonal - 1;
      var previousX = vector[previousDiagonal + max], previousY = previousX - previousDiagonal;
      while (x > previousX && y > previousY) {
        var actualLine = actual[x - 1];
        result.push({ type: "same", value: !checkCommaDisparity || actualLine.endsWith(",") ? actualLine : expected[y - 1] });
        x -= 1; y -= 1;
      }
      if (level > 0) {
        if (x > previousX) { result.push({ type: "insert", value: actual[x - 1] }); x -= 1; }
        else { result.push({ type: "delete", value: expected[y - 1] }); y -= 1; }
      }
    }
    return result;
  }

  function printMyersDiff(diff, operator) {
    var message = "", skipped = false, unchanged = 0;
    for (var index = diff.length - 1; index >= 0; index -= 1) {
      var row = diff[index], previousType = index < diff.length - 1 ? diff[index + 1].type : null;
      if (previousType === "same" && row.type !== previousType) {
        if (unchanged === 6) message += "  " + diff[index + 1].value + "\n";
        else if (unchanged === 7) message += "  " + diff[index + 2].value + "\n  " + diff[index + 1].value + "\n";
        else if (unchanged >= 8) { message += "...\n  " + diff[index + 1].value + "\n"; skipped = true; }
        unchanged = 0;
      }
      if (row.type === "insert") message += (operator === "partialDeepStrictEqual" ? "+ " : "+ ") + row.value + "\n";
      else if (row.type === "delete") message += "- " + row.value + "\n";
      else { if (unchanged < 5) message += "  " + row.value + "\n"; unchanged += 1; }
    }
    return { message: "\n" + message.replace(/\n+$/, ""), skipped: skipped };
  }

  function isErrorObject(value) {
    return value !== null && typeof value === "object" && (value instanceof Error || Object.prototype.toString.call(value) === "[object Error]");
  }

  function copyError(value) {
    var copy = Object.assign(Object.create(Object.getPrototypeOf(value)), value);
    Object.defineProperty(copy, "message", { value: value.message, configurable: true });
    if (Object.prototype.hasOwnProperty.call(value, "cause")) {
      Object.defineProperty(copy, "cause", { value: value.cause, configurable: true });
    }
    return copy;
  }

  function diffText(actual, expected, operator, customMessage) {
    if (isErrorObject(actual) && isErrorObject(expected) && "stack" in actual && "stack" in expected) {
      actual = copyError(actual); expected = copyError(expected);
    }
    operator = operator === "strictEqual" && (actual !== null && typeof actual === "object" && expected !== null && typeof expected === "object" || typeof actual === "function" && typeof expected === "function") ? "strictEqualObject" : operator;
    var actualText = assertionInspect(actual), expectedText = assertionInspect(expected);
    var actualLines = actualText.split("\n"), expectedLines = expectedText.split("\n");
    var header = "+ actual - expected", body = "", skipped = false;
    var simple = actualLines.length <= 1 && expectedLines.length <= 1 && (actual === null || typeof actual !== "object" || expected === null || typeof expected !== "object");
    if (simple) {
      var length = actualText.length + expectedText.length;
      if (typeof actual === "string") length -= 2;
      if (typeof expected === "string") length -= 2;
      if (length <= 12 && (actual !== 0 || expected !== 0)) { body = actualText + " !== " + expectedText; header = ""; }
      else {
        body = "\n+ " + actualText + "\n- " + expectedText;
        if (typeof actual === "string" && typeof expected === "string" && actualText.length + expectedText.length <= 80) {
          var indicator = -1;
          for (var indicatorIndex = 0; indicatorIndex < Math.min(actualText.length, expectedText.length); indicatorIndex += 1) {
            if (actualText[indicatorIndex] !== expectedText[indicatorIndex]) { if (indicatorIndex >= 3) indicator = indicatorIndex; break; }
          }
          if (indicator >= 0) body += "\n" + " ".repeat(indicator + 2) + "^";
        }
      }
    } else if (actualText === expectedText) {
      operator = "notIdentical";
      header = "";
      if (actualLines.length > 50) { body = actualLines.slice(0, 50).join("\n") + "\n...}"; skipped = true; }
      else body = actualLines.join("\n");
    } else {
      var diff = myersDiff(actualLines, expectedLines, actual !== null && typeof actual === "object");
      var printed = printMyersDiff(diff, operator);
      body = printed.message; skipped = printed.skipped;
    }
    var heading = (customMessage == null ? readableOperator(operator) : String(customMessage)) + "\n" + header;
    var skippedText = skipped ? "\n... Skipped lines" : "";
    return heading + skippedText + "\n" + body + "\n";
  }

  function assertionMessage(actual, expected, operator) {
    if (operator === "strictEqual" || operator === "deepStrictEqual" || operator === "partialDeepStrictEqual") return diffText(actual, expected, operator);
    if (operator === "deepEqual") return "Expected values to be loosely deep-equal:\n\n" + assertionInspect(actual) + "\n\nshould loosely deep-equal\n\n" + assertionInspect(expected);
    if (operator === "notDeepEqual") {
      var actualText = assertionInspect(actual), expectedText = assertionInspect(expected);
      if (actualText === expectedText) return readableOperator(operator) + "\n\n" + actualText.slice(0, 1021) + (actualText.length > 1024 ? "..." : "");
      return "Expected values not to be loosely deep-equal:\n\n" + actualText + "\n\nshould not loosely deep-equal\n\n" + expectedText;
    }
    if (operator === "notStrictEqual" || operator === "notDeepStrictEqual") {
      var notEqualHeader = readableOperator(operator);
      if (operator === "notStrictEqual" && actual !== null && (typeof actual === "object" || typeof actual === "function")) notEqualHeader = readableOperator("notStrictEqualObject");
      var notEqualText = assertionInspect(actual);
      if (notEqualText.length > 1024) notEqualText = notEqualText.slice(0, 509) + "...";
      var notEqualLines = notEqualText.split("\n");
      if (notEqualLines.length > 50) { notEqualLines[46] = "..."; notEqualLines.length = 47; }
      if (notEqualLines.length === 1) return notEqualHeader + (notEqualText.length > 5 ? "\n\n" + notEqualText : " " + notEqualText);
      return notEqualHeader + "\n\n" + notEqualLines.join("\n") + "\n";
    }
    if (operator === "==" || operator === "!=") return assertionInspect(actual) + " " + operator + " " + assertionInspect(expected);
    return "Expected values to satisfy the assertion:";
  }

  function customMessage(options) {
    if (["strictEqual", "deepStrictEqual", "partialDeepStrictEqual"].includes(options.operator)) return diffText(options.actual, options.expected, options.operator, options.message);
    return String(options.message);
  }

  function fail(actual, expected, message, operator, stackStartFn) {
    if (isErrorValue(message)) throw message;
    throw new AssertionError({ actual: actual, expected: expected, message: message, operator: operator, stackStartFn: stackStartFn || fail });
  }

  function generatedFailure(actual, expected, message, operator, stackStartFn) {
    throw new AssertionError({ actual: actual, expected: expected, message: message, operator: operator, stackStartFn: stackStartFn, generatedMessage: true });
  }

  function assertFail(actual, expected, message, operator, stackStartFn) {
    var argc = arguments.length;
    var internalMessage = false;
    if (actual === null || actual === undefined) {
      if (argc <= 1) {
        internalMessage = true;
        message = "Failed";
      }
    }
    if (!internalMessage && argc === 1) {
      message = actual;
      actual = undefined;
    } else if (!internalMessage && argc === 2) {
      operator = "!=";
    }
    if (isErrorValue(message)) throw message;
    var error = new AssertionError({
      actual: actual,
      expected: expected,
      message: message,
      operator: operator === undefined ? "fail" : operator,
      stackStartFn: stackStartFn || assertFail,
      generatedMessage: internalMessage,
    });
    throw error;
  }

  function ok(value, message) {
    return assertValue(value, message, ok, arguments.length);
  }

  function assertValue(value, message, stackStartFn, argumentCount) {
    if (argumentCount === 0) generatedFailure(undefined, true, "No value argument passed to `assert.ok()`", "==", stackStartFn);
    if (!value) {
      if (isErrorValue(message)) throw message;
      var expression;
      var sourceApi = runtime.source;
      if (sourceApi && typeof sourceApi.getAssertionExpression === "function") {
        try {
          expression = sourceApi.getAssertionExpression(stackStartFn);
        } catch (_) {}
      }
      if (message == null) {
        if (typeof expression === "string" && expression.length) generatedFailure(value, true, "The expression evaluated to a falsy value:\n\n  " + expression + "\n", "==", stackStartFn);
        generatedFailure(value, true, undefined, "==", stackStartFn);
      }
      fail(value, true, message, "==", stackStartFn);
    }
  }

  function strictEqual(actual, expected, message) {
    requireActualExpected(arguments);
    if (!Object.is(actual, expected)) fail(actual, expected, message, "strictEqual", strictEqual);
  }

  function notStrictEqual(actual, expected, message) {
    requireActualExpected(arguments);
    if (Object.is(actual, expected)) fail(actual, expected, message, "notStrictEqual", notStrictEqual);
  }

  function deepStrictEqual(actual, expected, message) {
    requireActualExpected(arguments);
    if (!runtime.util.isDeepStrictEqual(actual, expected)) fail(actual, expected, message, "deepStrictEqual", deepStrictEqual);
  }

  function notDeepStrictEqual(actual, expected, message) {
    requireActualExpected(arguments);
    if (runtime.util.isDeepStrictEqual(actual, expected)) fail(actual, expected, message, "notDeepStrictEqual", notDeepStrictEqual);
  }

  function partialDeepStrictEqual(actual, expected, message) {
    requireActualExpected(arguments);
    if (root.process && typeof root.process.emitWarning === "function") {
      root.process.emitWarning("assert.partialDeepStrictEqual is an experimental feature and might change at any time", "ExperimentalWarning", "EXPERIMENTAL_FEATURE");
    }
    if (!runtime.util.isPartialDeepStrictEqual(actual, expected)) fail(actual, expected, message, "partialDeepStrictEqual", partialDeepStrictEqual);
  }

  function looseDeepEqual(actual, expected, message) {
    requireActualExpected(arguments);
    if (!runtime.util.isDeepEqual(actual, expected)) fail(actual, expected, message, "deepEqual", looseDeepEqual);
  }

  function notLooseDeepEqual(actual, expected, message) {
    requireActualExpected(arguments);
    if (runtime.util.isDeepEqual(actual, expected)) fail(actual, expected, message, "notDeepEqual", notLooseDeepEqual);
  }

  var NO_EXCEPTION = {};

  function received(value) {
    if (value === null) return "Received null";
    if (value === undefined) return "Received undefined";
    if (typeof value === "string") return "Received type string ('" + value.replace(/\\/g, "\\\\").replace(/'/g, "\\'") + "')";
    if (typeof value === "number" || typeof value === "boolean") return "Received type " + typeof value + " (" + String(value) + ")";
    if (typeof value === "bigint") return "Received type bigint (" + String(value) + "n)";
    if (typeof value === "symbol") return "Received type symbol (" + String(value) + ")";
    if (typeof value === "function") return "Received function" + (value.name ? " " + value.name : "");
    if (typeof value === "object") {
      var name;
      try { name = value.constructor && value.constructor.name; } catch (_) {}
      return name ? "Received an instance of " + name : "Received an object";
    }
    return "Received type " + typeof value;
  }

  function invalidArgType(argument, expected, value) {
    var error = new TypeError('The "' + argument + '" argument must be ' + expected + '. ' + received(value));
    error.code = "ERR_INVALID_ARG_TYPE";
    return error;
  }

  function invalidStringArgument(value) {
    var receivedValue;
    if (value === null) receivedValue = "Received null";
    else if (value === undefined) receivedValue = "Received undefined";
    else receivedValue = "Received type " + typeof value + " (" + runtime.util.inspect(value) + ")";
    var error = new TypeError('The "string" argument must be of type string. ' + receivedValue);
    error.code = "ERR_INVALID_ARG_TYPE";
    return error;
  }

  function requireActualExpected(args) {
    if (args.length < 2) {
      var missing = new TypeError('The "actual" and "expected" arguments must be specified');
      missing.code = "ERR_MISSING_ARGS";
      throw missing;
    }
  }

  function invalidReturnValue(expected, argument, value) {
    var returned;
    if (value === undefined) returned = "undefined";
    else if (value === null) returned = "null";
    else if (typeof value === "function") returned = "a function";
    else if (typeof value === "object") {
      var constructorName;
      try { constructorName = value.constructor && value.constructor.name; } catch (_) {}
      returned = constructorName ? "an instance of " + constructorName : "an object";
    } else returned = "type " + typeof value;
    var error = new TypeError("Expected instance of " + expected + ' to be returned from the "' + argument + '" function but got ' + returned + ".");
    error.code = "ERR_INVALID_RETURN_VALUE";
    return error;
  }

  function invalidExpectedError(value) {
    var error = new TypeError('The "error" argument must be of type function or an instance of Error, RegExp, or Object. ' + received(value));
    error.code = "ERR_INVALID_ARG_TYPE";
    return error;
  }

  function isErrorConstructor(expected) {
    return typeof expected === "function" && (expected === Error || expected.prototype instanceof Error || Object.prototype.isPrototypeOf.call(Error, expected));
  }

  function validationFailure(actual, expected, result, operator, stackStartFn) {
    var name = expected.name ? '"' + expected.name + '" ' : "";
    var message = "The " + name + "validation function is expected to return \"true\". Received " + runtime.util.inspect(result);
    if (actual instanceof Error) message += "\n\nCaught error:\n\n" + String(actual);
    throw new AssertionError({ actual: actual, expected: expected, message: message, operator: operator, stackStartFn: stackStartFn, generatedMessage: true });
  }

  function expectedObjectFailure(actual, expected, stackStartFn) {
    var compared = actual;
    var expectedCompared = expected;
    if (isErrorValue(actual)) {
      var keys = Object.keys(expected);
      if (expected instanceof Error) keys.push("name", "message");
      compared = new Comparison();
      expectedCompared = new Comparison();
      keys.forEach(function (key) {
        var actualValue = actual[key];
        var expectedValue = expected[key];
        if (key in actual) compared[key] = actualValue;
        if (key in expected) {
          expectedCompared[key] = expectedValue instanceof RegExp && typeof actualValue === "string" && expectedValue.test(actualValue)
            ? actualValue
          : expectedValue;
        }
      });
      var comparedError = new AssertionError({ actual: compared, expected: expectedCompared, operator: "deepStrictEqual", stackStartFn: stackStartFn });
      comparedError.operator = stackStartFn && stackStartFn.name || "throws";
      throw comparedError;
    }
    var message = diffText(actual, expected, "deepStrictEqual");
    throw new AssertionError({ actual: actual, expected: expected, message: message, operator: stackStartFn && stackStartFn.name || "throws", stackStartFn: stackStartFn, generatedMessage: true });
  }

  function expectedObjectFailureWithMessage(actual, expected, message, stackStartFn) {
    var customDiff = diffText(actual, expected, "deepStrictEqual", message);
    throw new AssertionError({ actual: actual, expected: expected, message: customDiff, operator: "throws", stackStartFn: stackStartFn, generatedMessage: false });
  }

  function errorTypeValidationFailure(actual, expected, operator, stackStartFn) {
    var expectedName = expected.name || "Error";
    var message = 'The error is expected to be an instance of "' + expectedName + '". Received ';
    if (isErrorValue(actual)) {
      var actualName = actual && actual.constructor && actual.constructor.name || actual && actual.name || "Error";
      if (actualName === expectedName) message += "an error with identical name but a different prototype.";
      else message += '"' + actualName + '"';
      if (actual && actual.message) message += "\n\nError message:\n\n" + actual.message;
    } else {
      message += '"' + runtime.util.inspect(actual, {depth: -1}) + '"';
    }
    throw new AssertionError({ actual: actual, expected: expected, message: message, operator: operator, stackStartFn: stackStartFn, generatedMessage: true });
  }

  function matchesExpected(error, expected, result) {
    if (!expected) return true;
    if (expected instanceof RegExp) return expected.test(String(error));
    if (typeof expected === "function") {
      if (expected.prototype !== undefined && error instanceof expected) return true;
      if (isErrorConstructor(expected)) {
        return false;
      }
      var returned = expected(error);
      if (result) result.value = returned;
      return returned === true;
    }
    if (expected && typeof expected === "object") {
      var keys = Object.keys(expected);
      if (expected instanceof Error) keys.push("name", "message");
      if (keys.length === 0) {
        var emptyError = new TypeError("The argument 'error' may not be an empty object. Received {}");
        emptyError.code = "ERR_INVALID_ARG_VALUE";
        throw emptyError;
      }
      var objectMatch = keys.every(function (key) {
        var expectedValue = expected[key];
        if (error === null || error === undefined || !(key in Object(error))) return false;
        var actualValue = error && error[key];
        if (expectedValue instanceof RegExp && typeof actualValue === "string") return expectedValue.test(actualValue);
        return runtime.util.isDeepStrictEqual(actualValue, expectedValue);
      });
      return objectMatch;
    }
    var invalidExpected = new TypeError('The "error" argument must be of type function or an instance of Error, RegExp, or Object. Received type ' + typeof expected + " (" + runtime.util.inspect(expected) + ")");
    invalidExpected.code = "ERR_INVALID_ARG_TYPE";
    throw invalidExpected;
  }

  function getActual(fn) {
    if (typeof fn !== "function") throw invalidArgType("fn", "of type function", fn);
    try { fn(); } catch (error) { return error; }
    return NO_EXCEPTION;
  }

  function throws(block, expected, message) {
    var expectedWasMessage = typeof expected === "string" && message === undefined;
    if (expectedWasMessage) { message = expected; expected = undefined; }
    else if (typeof expected === "string" || typeof expected === "number" || typeof expected === "boolean" || typeof expected === "symbol" || typeof expected === "bigint") throw invalidExpectedError(expected);
    var thrown = getActual(block);
    if (thrown === NO_EXCEPTION) {
      var missingDetails = expected && expected.name ? " (" + expected.name + ")" : "";
      missingDetails += message ? ": " + message : ".";
      if (message === undefined) generatedFailure(undefined, expected, "Missing expected exception" + missingDetails, "throws", throws);
      fail(undefined, expected, "Missing expected exception" + missingDetails, "throws", throws);
    }
    if (expectedWasMessage && (thrown === message || isErrorValue(thrown) && thrown.message === message)) {
      var ambiguousValue = isErrorValue(thrown) ? thrown.message : thrown;
      var ambiguous = new TypeError('The "error/message" argument is ambiguous. The ' + (isErrorValue(thrown) ? "error message" : "error") + " " + JSON.stringify(String(ambiguousValue)) + " is identical to the message.");
      ambiguous.code = "ERR_AMBIGUOUS_ARGUMENT";
      throw ambiguous;
    }
    var validation = {};
    if (!matchesExpected(thrown, expected, validation)) {
      if (isErrorConstructor(expected)) errorTypeValidationFailure(thrown, expected, "throws", throws);
      if (expected instanceof RegExp) {
        var inputDescription = typeof thrown === "symbol" ? runtime.util.inspect(String(thrown)) : runtime.util.inspect(thrown);
        var regexMessage = "The input did not match the regular expression " + runtime.util.inspect(expected) + ". Input:\n\n" + inputDescription + "\n";
        generatedFailure(thrown, expected, regexMessage, "throws", throws);
      }
      if (typeof expected === "function" && !isErrorConstructor(expected) && message === undefined) {
        validationFailure(thrown, expected, validation.value, "throws", throws);
      }
      if (expected && typeof expected === "object" && message === undefined) expectedObjectFailure(thrown, expected, throws);
      if (expected && typeof expected === "object" && message !== undefined && !isErrorValue(thrown)) expectedObjectFailureWithMessage(thrown, expected, message, throws);
      if (message === undefined) generatedFailure(thrown, expected, "", "throws", throws);
      fail(thrown, expected, message, "throws", throws);
    }
  }

  function doesNotThrow(block, expected, message) {
    if (typeof expected === "string" && message === undefined) { message = expected; expected = undefined; }
    if (expected !== undefined && typeof expected !== "function" && !(expected instanceof RegExp)) {
      var expectedTypeError = new TypeError('The "expected" argument must be of type function or an instance of RegExp. ' + received(expected));
      expectedTypeError.code = "ERR_INVALID_ARG_TYPE";
      throw expectedTypeError;
    }
    var thrown = getActual(block);
    if (thrown === NO_EXCEPTION) return;
    if (!expected || matchesExpected(thrown, expected)) {
      fail(thrown, expected, "Got unwanted exception" + (message ? ": " + message : ".") + "\nActual message: \"" + (thrown && thrown.message) + "\"", "doesNotThrow", doesNotThrow);
    }
    throw thrown;
  }

  function isPromiseLike(value) {
    return value instanceof Promise || value !== null && typeof value === "object" &&
      typeof value.then === "function" && typeof value.catch === "function";
  }

  async function waitForActual(promiseOrFn) {
    var promise;
    if (typeof promiseOrFn === "function") {
      promise = promiseOrFn();
      if (!isPromiseLike(promise)) throw invalidReturnValue("Promise", "promiseFn", promise);
    } else if (isPromiseLike(promiseOrFn)) {
      promise = promiseOrFn;
    } else {
      throw invalidArgType("promiseFn", "of type function or an instance of Promise", promiseOrFn);
    }
    try { await promise; } catch (error) { return error; }
    return NO_EXCEPTION;
  }

  async function rejects(promiseOrFn, expected, message) {
    if (typeof expected === "string" && message === undefined) { message = expected; expected = undefined; }
    var actual = await waitForActual(promiseOrFn);
    if (actual === NO_EXCEPTION) {
      var details = expected && expected.name ? " (" + expected.name + ")" : "";
      details += message ? ": " + message : ".";
      fail(undefined, expected, "Missing expected rejection" + details, "rejects", rejects);
    }
    var validation = {};
    if (!matchesExpected(actual, expected, validation)) {
      if (isErrorConstructor(expected)) errorTypeValidationFailure(actual, expected, "rejects", rejects);
      if (expected instanceof RegExp) {
        var rejectionDescription = typeof actual === "symbol" ? runtime.util.inspect(String(actual)) : runtime.util.inspect(actual);
        var rejectionMessage = "The input did not match the regular expression " + runtime.util.inspect(expected) + ". Input:\n\n" + rejectionDescription + "\n";
        generatedFailure(actual, expected, rejectionMessage, "rejects", rejects);
      }
      if (typeof expected === "function" && !isErrorConstructor(expected) && message === undefined) {
        validationFailure(actual, expected, validation.value, "rejects", rejects);
      }
      if (expected && typeof expected === "object" && message === undefined) expectedObjectFailure(actual, expected, rejects);
      if (message === undefined) generatedFailure(actual, expected, "", "rejects", rejects);
      fail(actual, expected, message, "rejects", rejects);
    }
  }

  async function doesNotReject(promiseOrFn, expected, message) {
    if (typeof expected === "string" && message === undefined) { message = expected; expected = undefined; }
    var actual = await waitForActual(promiseOrFn);
    if (actual === NO_EXCEPTION) return;
    if (!expected || matchesExpected(actual, expected)) {
      fail(actual, expected, "Got unwanted rejection" + (message ? ": " + message : ".") + "\nActual message: \"" + (actual && actual.message) + "\"", "doesNotReject", doesNotReject);
    }
    throw actual;
  }

  function ifError(value) {
    if (value !== null && value !== undefined) throw value;
  }

  function match(actual, regexp, message) {
    if (!(regexp instanceof RegExp)) throw invalidArgType("regexp", "an instance of RegExp", regexp);
    if (typeof actual !== "string") {
      var matchTypeError = invalidStringArgument(actual);
      generatedFailure(actual, regexp, matchTypeError.message, "match", match);
    }
    if (!regexp.test(actual)) {
      if (!message) generatedFailure(actual, regexp, "The input did not match the regular expression " + runtime.util.inspect(regexp) + ". Input:\n\n" + runtime.util.inspect(actual) + "\n", "match", match);
      fail(actual, regexp, message, "match", match);
    }
  }

  function doesNotMatch(actual, regexp, message) {
    if (!(regexp instanceof RegExp)) throw invalidArgType("regexp", "an instance of RegExp", regexp);
    if (typeof actual !== "string") {
      var doesNotMatchTypeError = invalidStringArgument(actual);
      generatedFailure(actual, regexp, doesNotMatchTypeError.message, "doesNotMatch", doesNotMatch);
    }
    if (regexp.test(actual)) {
      if (!message) generatedFailure(actual, regexp, "The input was expected to not match the regular expression " + runtime.util.inspect(regexp) + ". Input:\n\n" + runtime.util.inspect(actual) + "\n", "doesNotMatch", doesNotMatch);
      fail(actual, regexp, message, "doesNotMatch", doesNotMatch);
    }
  }

  function assert(value, message) { return assertValue(value, message, assert, arguments.length); }
  function strictAssert(value, message) { return assertValue(value, message, strictAssert, arguments.length); }

  function attachCommon(target) {
    target.AssertionError = AssertionError;
    target.fail = assertFail;
    target.ok = ok;
    target.strictEqual = strictEqual;
    target.notStrictEqual = notStrictEqual;
    target.deepStrictEqual = deepStrictEqual;
    target.notDeepStrictEqual = notDeepStrictEqual;
    target.partialDeepStrictEqual = partialDeepStrictEqual;
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
    requireActualExpected(arguments);
    if (actual != expected && !(typeof actual === "number" && Number.isNaN(actual) && typeof expected === "number" && Number.isNaN(expected))) fail(actual, expected, message, "==", assert.equal);
  };
  assert.notEqual = function (actual, expected, message) {
    requireActualExpected(arguments);
    if (actual == expected || typeof actual === "number" && Number.isNaN(actual) && typeof expected === "number" && Number.isNaN(expected)) fail(actual, expected, message, "!=", assert.notEqual);
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
