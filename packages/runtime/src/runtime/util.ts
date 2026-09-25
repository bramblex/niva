(function (root: any) {
  "use strict";

  var runtime = root[Symbol.for("niva.node-compat.runtime")];
  if (typeof runtime.createUtilModule === "function") return;
  var inspectColors = { string: 32, number: 33, bigint: 33, boolean: 33, undefined: 90, null: 1, special: 32, name: 34 };

  function paint(value, kind, colors) {
    if (!colors || typeof value !== "string") return value;
    var code = inspectColors[kind];
    return code ? "\u001b[" + code + "m" + value + (kind === "null" ? "\u001b[22m" : "\u001b[39m") : value;
  }

  function inspect(value, options?) {
    options = typeof options === "number" ? { depth: options } : (options || {});
    options = Object.assign({}, inspect.defaultOptions, options);
    var depth = options.depth;
    if (depth === null) depth = Infinity;
    var colors = !!options.colors;
    var maxArrayLength = options.maxArrayLength === undefined ? 100 : options.maxArrayLength;
    var maxStringLength = options.maxStringLength === undefined ? 10000 : options.maxStringLength;
    var breakLength = options.breakLength === undefined ? 80 : options.breakLength;
    var compact = options.compact === undefined ? true : options.compact;
    var showHidden = !!options.showHidden;
    var customInspect = options.customInspect !== false;
    var getterMode = options.getters || false;
    var customInspectSymbol = Symbol.for("nodejs.util.inspect.custom");
    var seen = [];
    var referenceCounts = new Map();
    var traversed = new Set();
    var referenceIds = new Map();
    var printedReferences = new Set();
    var nextReferenceId = 1;

    function countReferences(input) {
      if (input === null || typeof input !== "object" && typeof input !== "function") return;
      referenceCounts.set(input, (referenceCounts.get(input) || 0) + 1);
      if (traversed.has(input)) return;
      traversed.add(input);
      var children = [];
      try {
        if (input instanceof Map) {
          input.forEach(function (item, key) { children.push(key, item); });
        } else if (input instanceof Set) {
          input.forEach(function (item) { children.push(item); });
        }
        (showHidden ? Object.getOwnPropertyNames(input) : Object.keys(input)).forEach(function (key) {
          var descriptor = Object.getOwnPropertyDescriptor(input, key);
          if (!descriptor) return;
          if (Object.prototype.hasOwnProperty.call(descriptor, "value")) children.push(descriptor.value);
          else if (getterMode && typeof descriptor.get === "function") children.push(descriptor.get.call(input));
        });
        if (Object.getOwnPropertySymbols) Object.getOwnPropertySymbols(input).forEach(function (key) {
          var descriptor = Object.getOwnPropertyDescriptor(input, key);
          if (!descriptor || !showHidden && !descriptor.enumerable) return;
          if (Object.prototype.hasOwnProperty.call(descriptor, "value")) children.push(descriptor.value);
          else if (getterMode && typeof descriptor.get === "function") children.push(descriptor.get.call(input));
        });
      } catch (_) {}
      children.forEach(countReferences);
    }
    countReferences(value);

    function getReferenceId(input) {
      if (!referenceIds.has(input)) referenceIds.set(input, nextReferenceId++);
      return referenceIds.get(input);
    }

    function constructorName(input) {
      try {
        var prototype = Object.getPrototypeOf(input);
        var descriptor = prototype && Object.getOwnPropertyDescriptor(prototype, "constructor");
        var constructor = descriptor && descriptor.value;
        return typeof constructor === "function" && constructor.name ? constructor.name : "";
      } catch (_) { return ""; }
    }

    function sortDisplay(input, visited = []) {
      if (typeof input === "string") return quote(input);
      if (typeof input === "symbol") return input.toString();
      if (input === null) return "null";
      if (typeof input !== "object") return String(input);
      visited = visited || [];
      if (visited.indexOf(input) >= 0) return "[Circular]";
      visited.push(input);
      if (input instanceof Date) { var dateSort = Date.prototype.toISOString.call(input); visited.pop(); return dateSort; }
      if (input instanceof RegExp) { var regexpSort = RegExp.prototype.toString.call(input); visited.pop(); return regexpSort; }
      if (typeof URL !== "undefined" && input instanceof URL) { var urlSort = input.href; visited.pop(); return urlSort; }
      var sortedValue;
      if (Array.isArray(input)) {
        sortedValue = "[" + Array.prototype.map.call(input, function (item) { return sortDisplay(item, visited); }).join(",") + "]";
      } else if (input instanceof Set) {
        sortedValue = "Set(" + input.size + "){" + Array.from(input.values()).map(function (item) { return sortDisplay(item, visited); }).sort().join(",") + "}";
      } else if (input instanceof Map) {
        sortedValue = "Map(" + input.size + "){" + Array.from(input.entries()).map(function (entry) { return sortDisplay(entry[0], visited) + ":" + sortDisplay(entry[1], visited); }).sort().join(",") + "}";
      } else {
        var keys = Object.keys(input).sort();
        var objectName = constructorName(input);
        sortedValue = (objectName && objectName !== "Object" ? objectName + " " : "") + "{" + keys.map(function (key) { return key + ":" + sortDisplay(input[key], visited); }).join(",") + "}";
      }
      visited.pop();
      return sortedValue;
    }

    function ownEntries(input, remaining, level) {
      var keys = enumerableKeys(input);
      sortKeys(keys);
      return keys.map(function (key) { return formatProperty(input, key, remaining, level); });
    }

    function withProperties(prefix, input, remaining, level, entries) {
      if (!entries.length) return prefix + "{}";
      var flat = "{ " + entries.join(", ") + " }";
      if (compact !== false && prefix.length + flat.length <= breakLength) return prefix + flat;
      var indent = "  ".repeat(level + 1);
      return prefix + "{\n" + indent + entries.join(",\n" + indent) + "\n" + "  ".repeat(level) + "}";
    }

    function instancePrefix(input) {
      var name = constructorName(input);
      if (Object.getPrototypeOf(input) === null) return "[Object: null prototype] ";
      if (name && name !== "Object") return name + " ";
      return "";
    }

    function quote(input, level = 0) {
      var text = input
        .replace(/\\/g, "\\\\")
        .replace(/\n/g, "\\n")
        .replace(/\r/g, "\\r")
        .replace(/\t/g, "\\t")
        .replace(/\x08/g, "\\b")
        .replace(/\f/g, "\\f")
        .replace(/\v/g, "\\v");
      if (maxStringLength !== null && maxStringLength >= 0 && text.length > maxStringLength) {
        text = text.slice(0, maxStringLength) + "... " + (text.length - maxStringLength) + " more characters";
      }
      function wrap(part) {
        var delimiter = part.indexOf("'") < 0 ? "'" : part.indexOf('"') < 0 ? '"' : "`";
        var quoted = delimiter === "`" ? part.replace(/`/g, "\\`") : part.replace(new RegExp(delimiter, "g"), "\\" + delimiter);
        return delimiter + quoted + delimiter;
      }
      if (text.length + 2 > breakLength && text.indexOf("\\n") >= 0) {
        var chunks = text.split("\\n");
        var endsWithLineBreak = chunks.length > 1 && chunks[chunks.length - 1] === "";
        if (endsWithLineBreak) chunks.pop();
        if (chunks.length > 1 && chunks.every(function (chunk) { return chunk.length + 4 <= breakLength; })) {
          var continuationIndent = "  ".repeat((level || 0) + 1);
          return chunks.map(function (chunk, index) {
            return wrap(chunk + (index < chunks.length - 1 || endsWithLineBreak ? "\\n" : ""));
          }).join(" +\n" + continuationIndent);
        }
      }
      return wrap(text);
    }

    function joinEntries(entries, open, close, level) {
      if (!entries.length) return open + close;
      var flat = open + " " + entries.join(", ") + " " + close;
      var lineLimit = breakLength;
      if (compact !== false && flat.length <= lineLimit) return flat;
      var indent = "  ".repeat(level + 1);
      return open + "\n" + indent + entries.join(",\n" + indent) + "\n" + "  ".repeat(level) + close;
    }

    function enumerableKeys(input) {
      var keys: any[] = showHidden ? Object.getOwnPropertyNames(input) : Object.keys(input);
      if (Object.getOwnPropertySymbols) {
        Object.getOwnPropertySymbols(input).forEach(function (symbol) {
          var descriptor = Object.getOwnPropertyDescriptor(input, symbol);
          if (descriptor && (showHidden || descriptor.enumerable)) keys.push(symbol);
        });
      }
      return keys;
    }

    function sortKeys(keys: any[]) {
      if (!options.sorted) return keys;
      var comparator = typeof options.sorted === "function" ? options.sorted : function (a, b) {
        var aSymbol = typeof a === "symbol", bSymbol = typeof b === "symbol";
        if (aSymbol !== bSymbol) return aSymbol ? -1 : 1;
        return String(a).localeCompare(String(b));
      };
      return keys.sort(comparator);
    }

    function visit(input, remaining, level = 0) {
      level = level || 0;
      if (input === null) return paint("null", "null", colors);
      if (input === undefined) return paint("undefined", "undefined", colors);
      var type = typeof input;
      if (type === "string") return paint(quote(input, level), "string", colors);
      if (type === "number") {
        var numberText = Object.is(input, -0) ? "-0" : String(input);
        if (options.numericSeparator && Number.isFinite(input) && Number.isInteger(input)) numberText = numberText.replace(/\B(?=(\d{3})+(?!\d))/g, "_");
        return paint(numberText, "number", colors);
      }
      if (type === "bigint") {
        var bigintText = String(input);
        if (options.numericSeparator) bigintText = bigintText.replace(/\B(?=(\d{3})+(?!\d))/g, "_");
        return paint(bigintText + "n", "bigint", colors);
      }
      if (type === "boolean") return paint(String(input), "boolean", colors);
      if (type === "symbol") return paint(input.toString(), "special", colors);
      if (type === "function") {
        if (seen.indexOf(input) >= 0) return referenceCounts.get(input) > 1 ? "[Circular *" + getReferenceId(input) + "]" : "[Circular]";
        var functionReferencePrefix = "";
        if (referenceCounts.get(input) > 1) {
          var functionReferenceId = getReferenceId(input);
          if (printedReferences.has(input)) return "[Ref *" + functionReferenceId + "]";
          printedReferences.add(input);
          functionReferencePrefix = "<ref *" + functionReferenceId + "> ";
        }
        var functionLabel = input.name ? "[Function: " + input.name + "]" : "[Function (anonymous)]";
        var functionKeys = enumerableKeys(input).filter(function (key) { return key !== "caller" && key !== "arguments"; });
        sortKeys(functionKeys);
        if (!functionKeys.length) return functionReferencePrefix + functionLabel;
        seen.push(input);
        var functionEntries = functionKeys.map(function (key) { return formatProperty(input, key, remaining - 1, level + 1); });
        seen.pop();
        return functionReferencePrefix + withProperties(functionLabel + " ", input, remaining, level, functionEntries);
      }
      if (seen.indexOf(input) >= 0) {
        return referenceCounts.get(input) > 1 ? "[Circular *" + getReferenceId(input) + "]" : "[Circular]";
      }
      var usesReference = referenceCounts.get(input) > 1;
      var referencePrefix = "";
      if (usesReference) {
        var referenceId = getReferenceId(input);
        if (printedReferences.has(input)) return "[Ref *" + referenceId + "]";
        printedReferences.add(input);
        referencePrefix = "<ref *" + referenceId + "> ";
      }
      if (customInspect && input && typeof input[customInspectSymbol] === "function") {
        try {
          var custom = input[customInspectSymbol](remaining, options, inspect);
          if (custom !== input) return typeof custom === "string" ? custom : visit(custom, remaining, level);
        } catch (error) {
          return "[Thrown: " + error.message + "]";
        }
      }
      if (remaining < 0) {
          if (Array.isArray(input)) return "[Array]";
          if (input instanceof Map) return "[Map]";
          if (input instanceof Set) return "[Set]";
        return "[Object]";
      }
      if (input instanceof Date) {
        var dateTime;
        var hasDateValue = true;
        try { dateTime = Date.prototype.getTime.call(input); } catch (_) { hasDateValue = false; }
        if (hasDateValue) {
          var dateText = Number.isNaN(dateTime) ? "Invalid Date" : Date.prototype.toISOString.call(input);
          var dateName = constructorName(input);
          if (dateName && dateName !== "Date") dateText = dateName + " " + dateText;
          var dateEntries = ownEntries(input, remaining - 1, level + 1);
          return referencePrefix + (dateEntries.length ? withProperties(dateText + " ", input, remaining - 1, level, dateEntries) : dateText);
        }
      }
      var regexpText;
      try {
        Object.getOwnPropertyDescriptor(RegExp.prototype, "source").get.call(input);
        regexpText = RegExp.prototype.toString.call(input);
      } catch (_) { regexpText = null; }
      if (regexpText !== null) {
        var regexpName = constructorName(input);
        if (regexpName && regexpName !== "RegExp") regexpText = regexpName + " " + regexpText;
        var regexpEntries = ownEntries(input, remaining - 1, level + 1);
        return referencePrefix + (regexpEntries.length ? withProperties(regexpText + " ", input, remaining - 1, level, regexpEntries) : regexpText);
      }
      if (typeof URL !== "undefined" && input instanceof URL) {
        var urlEntries = ownEntries(input, remaining - 1, level + 1);
        return referencePrefix + (urlEntries.length ? withProperties(input.href + " ", input, remaining - 1, level, urlEntries) : input.href);
      }
      if (typeof URLSearchParams !== "undefined" && input instanceof URLSearchParams) return "URLSearchParams {}";
      if (isInspectableError(input)) {
        var errorText = input.name + (input.message ? ": " + input.message : "");
        var errorEntries = ownEntries(input, remaining - 1, level + 1);
        if (Object.prototype.hasOwnProperty.call(input, "stack")) return referencePrefix + (input.stack || errorText);
        return referencePrefix + (errorEntries.length ? withProperties("[" + errorText + "] ", input, remaining - 1, level, errorEntries) : "[" + errorText + "]");
      }
      var bufferApi = runtime.buffer && runtime.buffer.Buffer;
      var bufferObject = bufferApi && bufferApi.isBuffer(input);
      if (bufferObject && ArrayBuffer.isView(input)) {
        var bufferEntries = Array.prototype.map.call(input, function (byte) { return String(byte); });
        ownEntries(input, remaining - 1, level + 1).forEach(function (entry) {
          if (!/^\s*['"]?(?:0|[1-9][0-9]*)['"]?:/.test(entry)) bufferEntries.push(entry);
        });
        return referencePrefix + "Buffer(" + (input as any).length + ") [Uint8Array] " + joinEntries(bufferEntries, "[", "]", level);
      }
      if (ArrayBuffer.isView(input) && !(input instanceof DataView)) {
        var typedEntries = Array.prototype.map.call(input, function (entry) { return visit(entry, remaining - 1, level + 1); });
        ownEntries(input, remaining - 1, level + 1).forEach(function (entry) {
          if (!/^\s*['"]?(?:0|[1-9][0-9]*)['"]?:/.test(entry)) typedEntries.push(entry);
        });
        return referencePrefix + (constructorName(input) || "TypedArray") + "(" + (input as any).length + ") " + joinEntries(typedEntries, "[", "]", level);
      }
      if (input instanceof DataView) return referencePrefix + "DataView { }";
      var inputTag = objectTag(input);
      if (inputTag === "[object ArrayBuffer]" || inputTag === "[object SharedArrayBuffer]") {
        var bufferContents = new Uint8Array(input);
        var contents = Array.prototype.map.call(bufferContents, function (byte) { return byte.toString(16).padStart(2, "0"); }).join(" ");
        return referencePrefix + constructorName(input) + " { [Uint8Contents]: <" + contents + ">, byteLength: " + bufferContents.length + " }";
      }
      var boxed = boxedPrimitive(input);
      if (boxed) {
        var boxedLabel = "[" + boxed.kind + ": " + visit(boxed.value, remaining - 1, level + 1) + "]";
        var boxedProperties = ownEntries(input, remaining - 1, level + 1).filter(function (entry) {
          return boxed.kind !== "String" || !/^\s*['"]?\d+['"]?:/.test(entry);
        });
        return referencePrefix + (boxedProperties.length ? withProperties(boxedLabel + " ", input, remaining - 1, level, boxedProperties) : boxedLabel);
      }
      seen.push(input);
      var output;
      if (Array.isArray(input)) {
        var arrayEntries = [];
        var shownLength = maxArrayLength === null ? input.length : Math.min(input.length, Math.max(0, maxArrayLength));
        for (var ai = 0; ai < shownLength; ai += 1) {
          if (Object.prototype.hasOwnProperty.call(input, ai)) arrayEntries.push(visit(input[ai], remaining - 1, level + 1));
          else if (arrayEntries.length && /^<\d+ empty items?>$/.test(arrayEntries[arrayEntries.length - 1])) arrayEntries[arrayEntries.length - 1] = "<" + (Number(arrayEntries[arrayEntries.length - 1].match(/\d+/)[0]) + 1) + " empty items>";
          else arrayEntries.push("<1 empty item>");
        }
        if (shownLength < input.length) {
          var omitted = input.length - shownLength;
          arrayEntries.push("... " + omitted + " more item" + (omitted === 1 ? "" : "s"));
        }
        var arrayProps = enumerableKeys(input).filter(function (key) {
          if (key === "length") return showHidden;
          if (typeof key !== "string" || !/^(0|[1-9][0-9]*)$/.test(key)) return true;
          return Number(key) >= 0xffffffff;
        });
        sortKeys(arrayProps).forEach(function (key) { arrayEntries.push(formatProperty(input, key, remaining - 1, level + 1)); });
        var arrayName = constructorName(input);
        var arrayPrefix = arrayName && arrayName !== "Array" ? arrayName + "(" + input.length + ") " : "";
        output = arrayPrefix + joinEntries(arrayEntries, "[", "]", level);
      } else if (input instanceof Map) {
        var mapValues = Array.from(input.entries());
        if (options.sorted) mapValues.sort(function (a, b) { var aText = sortDisplay(a[0]), bText = sortDisplay(b[0]); return aText < bText ? -1 : aText > bText ? 1 : 0; });
        var mapEntries = mapValues.slice(0, maxArrayLength === null ? input.size : Math.max(0, maxArrayLength)).map(function (entry) {
          return visit(entry[0], remaining - 1, level + 1) + " => " + visit(entry[1], remaining - 1, level + 1);
        });
        if (mapEntries.length < input.size) {
          var mapOmitted = input.size - mapEntries.length;
          mapEntries.push("... " + mapOmitted + " more item" + (mapOmitted === 1 ? "" : "s"));
        }
        mapEntries = mapEntries.concat(ownEntries(input, remaining - 1, level + 1));
        output = "Map(" + input.size + ") " + joinEntries(mapEntries, "{", "}", level);
      } else if (input instanceof Set) {
        var setValues = Array.from(input.values());
        if (options.sorted) setValues.sort(function (a, b) { var aText = sortDisplay(a), bText = sortDisplay(b); return aText < bText ? -1 : aText > bText ? 1 : 0; });
        var setEntries = setValues.slice(0, maxArrayLength === null ? input.size : Math.max(0, maxArrayLength)).map(function (item) {
          return visit(item, remaining - 1, level + 1);
        });
        if (setEntries.length < input.size) {
          var setOmitted = input.size - setEntries.length;
          setEntries.push("... " + setOmitted + " more item" + (setOmitted === 1 ? "" : "s"));
        }
        setEntries = setEntries.concat(ownEntries(input, remaining - 1, level + 1));
        output = "Set(" + input.size + ") " + joinEntries(setEntries, "{", "}", level);
      } else {
        var entries = ownEntries(input, remaining - 1, level + 1);
        var prefix = instancePrefix(input);
        if (Object.prototype.toString.call(input) === "[object Arguments]") prefix = "[Arguments] ";
        if (bufferObject && !ArrayBuffer.isView(input)) prefix = constructorName(input) + " [Uint8Array] ";
        if (Object.prototype.toString.call(input) === "[object KeyObject]") prefix = constructorName(input) + " [KeyObject] ";
        output = withProperties(prefix, input, remaining - 1, level, entries);
      }
      seen.pop();
      return referencePrefix + output;
    }

    function formatProperty(input, key: any, remaining, level) {
      var descriptor = Object.getOwnPropertyDescriptor(input, key);
      var label = typeof key === "symbol" ? "[" + key.toString() + "]"
        : (/^[A-Za-z_$][\w$]*$/.test(key) ? key : quote(key));
      var field;
      if (descriptor && !Object.prototype.hasOwnProperty.call(descriptor, "value")) {
        var hasGetter = typeof descriptor.get === "function";
        var hasSetter = typeof descriptor.set === "function";
        if (getterMode && hasGetter && (getterMode === true || getterMode === "get" || getterMode === "get+set" && hasSetter)) {
          try { field = "[Getter: " + visit(descriptor.get.call(input), remaining, level) + "]"; }
          catch (error) { field = "[Thrown: " + error.message + "]"; }
        } else {
          field = hasGetter && hasSetter ? "[Getter/Setter]" : hasGetter ? "[Getter]" : "[Setter]";
        }
      } else {
        try { field = visit(input[key], remaining, level); }
        catch (error) { field = "[Thrown: " + error.message + "]"; }
      }
      if (showHidden && descriptor && !descriptor.enumerable && typeof key === "string") label = "[" + key + "]";
      return paint(label, "name", colors) + ": " + field;
    }

    return visit(value, depth);
  }

  function boxedPrimitive(input) {
    var candidates = [
      ["Number", Number.prototype], ["String", String.prototype], ["Boolean", Boolean.prototype],
      ["BigInt", typeof BigInt === "function" ? BigInt.prototype : null], ["Symbol", typeof Symbol === "function" ? Symbol.prototype : null],
    ];
    for (var i = 0; i < candidates.length; i += 1) {
      if (!candidates[i][1]) continue;
      try { return { kind: candidates[i][0], value: candidates[i][1].valueOf.call(input) }; } catch (_) {}
    }
    return null;
  }

  function isInspectableError(input) {
    if (input instanceof Error) return true;
    try {
      if (Object.prototype.toString.call(input) === "[object Error]") return true;
      return Error.prototype.isPrototypeOf(input);
    } catch (_) { return false; }
  }
  inspect.defaultOptions = {
    showHidden: false,
    depth: 2,
    colors: false,
    customInspect: true,
    showProxy: false,
    maxArrayLength: 100,
    maxStringLength: 10000,
    breakLength: 80,
    compact: 3,
    sorted: false,
    getters: false,
    numericSeparator: false,
  };

  function addNumericSeparators(value) {
    if (!inspect.defaultOptions.numericSeparator) return value;
    return value.replace(/\B(?=(\d{3})+(?!\d))/g, "_");
  }

  function formatNumber(value) {
    if (typeof value === "bigint") return addNumericSeparators(String(value)) + "n";
    var number = Number(value);
    var text = Object.is(number, -0) ? "-0" : String(number);
    if (Number.isInteger(number) && Number.isFinite(number) && Math.abs(number) < 1e21) return addNumericSeparators(text);
    return text;
  }

  function format(first) {
    if (arguments.length === 0) return "";
    var args = Array.prototype.slice.call(arguments, 1);
    if (typeof first !== "string") {
      if (first instanceof Error && args.length === 0) return first.stack || "[" + first.name + (first.message ? ": " + first.message : "") + "]";
      return [first].concat(args).map(function (value, index) {
        return index > 0 && typeof value === "string" ? value : inspect(value);
      }).join(" ");
    }
    var index = 0;
    var output = first.replace(/%[sdifjoOc%]/g, function (token) {
      if (token === "%%") return "%";
      if (index >= args.length) return token;
      var value = args[index++];
      if (token === "%c") return "";
      switch (token) {
        case "%s":
          if (typeof value === "bigint") return formatNumber(value);
          if (typeof value === "number") return formatNumber(value);
          if (value !== null && typeof value === "object") {
            if (typeof value[Symbol.toPrimitive] === "function") return String(value);
            var toString = value.toString;
            if (typeof toString === "function" && toString !== Object.prototype.toString && toString !== Array.prototype.toString) {
              try { return String(value); } catch (_) {}
            }
            return inspect(value, { depth: 0, colors: false });
          }
          return String(value);
        case "%d":
          if (typeof value === "bigint") return formatNumber(value);
          try {
            var numeric = Number(value);
            return formatNumber(numeric);
          } catch (_) { return "NaN"; }
        case "%i":
          if (typeof value === "bigint") return formatNumber(value);
          try { return formatNumber(parseInt(value, 10)); } catch (_) { return "NaN"; }
        case "%f":
          try {
            var floatValue = parseFloat(value);
            return Object.is(floatValue, -0) ? "-0" : String(floatValue);
          } catch (_) { return "NaN"; }
        case "%j":
          try { return JSON.stringify(value); }
          catch (error) {
            if (error && /converting circular structure|circular reference|cyclic object value/i.test(error.message)) return "[Circular]";
            throw error;
          }
        case "%o": return inspect(value, { depth: 4, showHidden: true, showProxy: true });
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

  function formatWithOptions(options) {
    if (options === null || typeof options !== "object" || Array.isArray(options)) {
      var invalidOptions = new TypeError('The "inspectOptions" argument must be of type object. ' + receivedArgument(options));
      invalidOptions.code = "ERR_INVALID_ARG_TYPE";
      throw invalidOptions;
    }
    var previous = inspect.defaultOptions;
    inspect.defaultOptions = Object.assign({}, previous, options || {});
    try { return format.apply(null, Array.prototype.slice.call(arguments, 1)); }
    finally { inspect.defaultOptions = previous; }
  }

  function processTick(callback, receiver, args) {
    args = args || [];
    var scheduled = receiver === undefined ? callback : callback.bind(receiver);
    if (root.process && typeof root.process.nextTick === "function") {
      root.process.nextTick.apply(root.process, [scheduled].concat(args));
      return;
    }
    var invoke = function () { return scheduled.apply(undefined, args); };
    if (typeof queueMicrotask === "function") queueMicrotask(invoke);
    else Promise.resolve().then(invoke);
  }

  function callbackifyOnRejected(reason, callback, receiver) {
    var error = reason;
    if (!reason) {
      error = new Error("Promise was rejected with falsy value");
      error.reason = reason;
      error.code = "ERR_FALSY_VALUE_REJECTION";
      if (Error.captureStackTrace) Error.captureStackTrace(error, callbackifyOnRejected);
    }
    callback.call(receiver, error);
  }

  function receivedArgument(value) {
    if (value === null) return "Received null";
    if (value === undefined) return "Received undefined";
    if (typeof value === "string") return "Received type string ('" + value.replace(/\\/g, "\\\\").replace(/'/g, "\\'") + "')";
    if (typeof value === "number" || typeof value === "boolean") return "Received type " + typeof value + " (" + String(value) + ")";
    if (typeof value === "bigint") return "Received type bigint (" + String(value) + "n)";
    if (typeof value === "symbol") return "Received type symbol (" + String(value) + ")";
    if (typeof value === "function") return "Received function" + (value.name ? " " + value.name : "");
    if (typeof value === "object") {
      var constructorName;
      try { constructorName = value.constructor && value.constructor.name; } catch (_) {}
      return constructorName ? "Received an instance of " + constructorName : "Received an object";
    }
    return "Received type " + typeof value;
  }

  function invalidFunctionArgument(argument, value) {
    var error = new TypeError('The "' + argument + '" argument must be of type function. ' + receivedArgument(value));
    error.code = "ERR_INVALID_ARG_TYPE";
    return error;
  }

  function promisify(fn) {
    if (typeof fn !== "function") throw invalidFunctionArgument("original", fn);
    var custom = fn[promisify.custom];
    if (custom) {
      if (typeof custom !== "function") throw invalidFunctionArgument("util.promisify.custom", custom);
      Object.defineProperty(custom, promisify.custom, { value: custom, configurable: true });
      return custom;
    }
    var argumentNames = fn[promisify.customArgs];
    function wrapped() {
      var receiver = this;
      var args = Array.prototype.slice.call(arguments);
      return new Promise(function (resolve, reject) {
        args.push(function (error) {
          if (error) reject(error);
          else {
            var values = Array.prototype.slice.call(arguments, 1);
            if (Array.isArray(argumentNames) && values.length > 1) {
              var result = {};
              for (var valueIndex = 0; valueIndex < values.length; valueIndex += 1) {
                result[argumentNames[valueIndex]] = values[valueIndex];
              }
              resolve(result);
            } else {
              resolve(values[0]);
            }
          }
        });
        try {
          var returned = fn.apply(receiver, args);
          if (returned instanceof Promise && root.process && typeof root.process.emitWarning === "function") {
            root.process.emitWarning("Calling promisify on a function that returns a Promise is likely a mistake.", "DeprecationWarning", "DEP0174");
          }
        }
        catch (error) { reject(error); }
      });
    }
    Object.defineProperty(wrapped, "name", { value: "promisified " + (fn.name || "function"), configurable: true });
    Object.defineProperty(wrapped, "length", { value: Math.max(0, fn.length - 1), configurable: true });
    Object.defineProperty(wrapped, promisify.custom, { value: wrapped, configurable: true });
    Object.setPrototypeOf(wrapped, Object.getPrototypeOf(fn));
    return wrapped;
  }
  promisify.custom = Symbol.for("nodejs.util.promisify.custom");
  promisify.customArgs = Symbol.for("nodejs.util.promisify.customArgs");

  function callbackify(fn) {
    if (typeof fn !== "function") throw invalidFunctionArgument("original", fn);
    function wrapped() {
      var receiver = this;
      var args = Array.prototype.slice.call(arguments);
      var callback = args.pop();
      if (typeof callback !== "function") {
        var callbackError = new TypeError("The last argument must be of type function. " + receivedArgument(callback));
        callbackError.code = "ERR_INVALID_ARG_TYPE";
        throw callbackError;
      }
      var result;
      try { result = fn.apply(receiver, args); }
      catch (error) { processTick(callback, receiver, [error]); return; }
      Promise.resolve(result).then(function (value) {
        processTick(callback, receiver, [null, value]);
      }, function (reason) { processTick(callbackifyOnRejected, undefined, [reason, callback, receiver]); });
    }
    Object.defineProperty(wrapped, "name", { value: (fn.name || "") + "Callbackified", configurable: true });
    Object.defineProperty(wrapped, "length", { value: fn.length + 1, configurable: true });
    return wrapped;
  }

  function enumerableKeys(value) {
    var keys: any[] = Object.keys(value);
    if (Object.getOwnPropertySymbols) {
      Object.getOwnPropertySymbols(value).forEach(function (symbol) {
        var descriptor = Object.getOwnPropertyDescriptor(value, symbol);
        if (descriptor && descriptor.enumerable) keys.push(symbol);
      });
    }
    return keys;
  }

  // Deep comparison follows the Node v22.14.0 observable contracts while
  // keeping all byte access and object inspection in browser JavaScript.
  function isDeepStrictEqual(left, right) { return compareValues(left, right, true, [], []); }
  function isDeepEqual(left, right) { return compareValues(left, right, false, [], []); }

  // Partial branch comparison follows Node.js v22.14.0 lib/assert.js at commit
  // 5d2feb257bcee090e57900eb51720171a6aa92f3. Copyright Node.js contributors;
  // MIT license is retained at upstream/node-v22.14.0/LICENSE.
  function isPartialDeepStrictEqual(actual, expected) {
    return comparePartialBranch(actual, expected, new WeakSet());
  }

  function comparePartialBranch(actual, expected, comparedActual) {
    if (actual === expected) return true;
    var actualMap = mapSize(actual), expectedMap = mapSize(expected);
    if (actualMap.matched || expectedMap.matched) {
      if (!actualMap.matched || !expectedMap.matched || expectedMap.value > actualMap.value) return false;
      var mapEntries = Array.from(Map.prototype.entries.call(expected));
      for (var mapIndex = 0; mapIndex < mapEntries.length; mapIndex += 1) {
        var key = mapEntries[mapIndex][0];
        if (!Map.prototype.has.call(actual, key) || !comparePartialBranch(Map.prototype.get.call(actual, key), mapEntries[mapIndex][1], comparedActual)) return false;
      }
      return true;
    }

    if (ArrayBuffer.isView(actual) || ArrayBuffer.isView(expected) ||
        objectTag(actual) === "[object ArrayBuffer]" || objectTag(expected) === "[object ArrayBuffer]" ||
        objectTag(actual) === "[object SharedArrayBuffer]" || objectTag(expected) === "[object SharedArrayBuffer]") {
      return comparePartialBytes(actual, expected);
    }

    var bufferApi = runtime.buffer && runtime.buffer.Buffer;
    var cryptoApi = runtime.crypto;
    var keyA = cryptoApi && typeof cryptoApi.isNivaKeyObject === "function" && cryptoApi.isNivaKeyObject(actual);
    var keyB = cryptoApi && typeof cryptoApi.isNivaKeyObject === "function" && cryptoApi.isNivaKeyObject(expected);
    var cryptoKeyA = webCryptoKeyMetadata(actual);
    var cryptoKeyB = webCryptoKeyMetadata(expected);
    if (keyA || keyB || cryptoKeyA || cryptoKeyB ||
        bufferApi && (bufferApi.isBuffer(actual) || bufferApi.isBuffer(expected)) ||
        actual instanceof WeakMap || expected instanceof WeakMap ||
        actual instanceof WeakSet || expected instanceof WeakSet ||
        typeof URL !== "undefined" && (actual instanceof URL || expected instanceof URL) ||
        typeof URLSearchParams !== "undefined" && (actual instanceof URLSearchParams || expected instanceof URLSearchParams)) {
      return isDeepStrictEqual(actual, expected);
    }

    var actualSet = setSize(actual), expectedSet = setSize(expected);
    if (actualSet.matched || expectedSet.matched) {
      if (!actualSet.matched || !expectedSet.matched || expectedSet.value > actualSet.value) return false;
      return !!findUnorderedMatch(Array.from(Set.prototype.values.call(expected)), Array.from(Set.prototype.values.call(actual)), 0, [], true, [], [], false);
    }

    if (Array.isArray(actual) && Array.isArray(expected)) {
      if (expected.length > actual.length) return false;
      return !!findUnorderedMatch(Array.from(expected), Array.from(actual), 0, [], true, [], [], false);
    }

    if (actual === null || expected === null || typeof actual !== "object" || typeof expected !== "object" ||
        isErrorValue(actual) || isErrorValue(expected) || actual instanceof Date || expected instanceof Date ||
        actual instanceof RegExp || expected instanceof RegExp) return isDeepStrictEqual(actual, expected);

    if (comparedActual.has(actual)) return true;
    comparedActual.add(actual);
    var keys = Reflect.ownKeys(expected);
    for (var keyIndex = 0; keyIndex < keys.length; keyIndex += 1) {
      var key = keys[keyIndex];
      if (!(key in actual) || !comparePartialBranch(actual[key], expected[key], comparedActual)) return false;
    }
    return true;
  }

  function comparePartialBytes(actual, expected) {
    var actualArrayBuffer = objectTag(actual) === "[object ArrayBuffer]" || objectTag(actual) === "[object SharedArrayBuffer]";
    var expectedArrayBuffer = objectTag(expected) === "[object ArrayBuffer]" || objectTag(expected) === "[object SharedArrayBuffer]";
    if (actualArrayBuffer || expectedArrayBuffer) {
      if (!actualArrayBuffer || !expectedArrayBuffer || objectTag(actual) !== objectTag(expected)) return false;
      var actualBytes = bufferView(actual), expectedBytes = bufferView(expected);
      if (!actualBytes || !expectedBytes || expectedBytes.length > actualBytes.length) return false;
      for (var arrayBufferIndex = 0; arrayBufferIndex < expectedBytes.length; arrayBufferIndex += 1) if (!Object.is(actualBytes[arrayBufferIndex], expectedBytes[arrayBufferIndex])) return false;
      return true;
    }
    if (!ArrayBuffer.isView(actual) || !ArrayBuffer.isView(expected)) return false;
    var actualDataView = objectTag(actual) === "[object DataView]", expectedDataView = objectTag(expected) === "[object DataView]";
    if (actualDataView || expectedDataView) {
      if (!actualDataView || !expectedDataView) return false;
      var actualData = viewBytes(actual), expectedData = viewBytes(expected);
      if (!actualData || !expectedData || expectedData.length > actualData.length) return false;
      for (var dataViewIndex = 0; dataViewIndex < expectedData.length; dataViewIndex += 1) if (!Object.is(actualData[dataViewIndex], expectedData[dataViewIndex])) return false;
      return true;
    }
    if (typedArrayName(actual) !== typedArrayName(expected) || (expected as any).length > (actual as any).length) return false;
    for (var typedIndex = 0; typedIndex < (expected as any).length; typedIndex += 1) if (!Object.is((actual as any)[typedIndex], (expected as any)[typedIndex])) return false;
    return true;
  }

  function objectTag(value) { return Object.prototype.toString.call(value); }
  function getGetter(proto, key, value) {
    var descriptor = Object.getOwnPropertyDescriptor(proto, key);
    return descriptor && descriptor.get ? descriptor.get.call(value) : undefined;
  }
  function dateValue(value) {
    try { return { matched: true, value: Date.prototype.getTime.call(value) }; } catch (_) { return { matched: false }; }
  }
  function regexpValue(value) {
    try {
      var flags = "";
      var names = [["hasIndices", "d"], ["global", "g"], ["ignoreCase", "i"], ["multiline", "m"], ["dotAll", "s"], ["unicode", "u"], ["unicodeSets", "v"], ["sticky", "y"]];
      for (var i = 0; i < names.length; i += 1) if (getGetter(RegExp.prototype, names[i][0], value)) flags += names[i][1];
      return { matched: true, source: getGetter(RegExp.prototype, "source", value), flags: flags, lastIndex: value.lastIndex };
    } catch (_) { return { matched: false }; }
  }
  function boxedValue(value) {
    var candidates = [["Number", Number.prototype], ["String", String.prototype], ["Boolean", Boolean.prototype],
      ["BigInt", typeof BigInt === "function" ? BigInt.prototype : null], ["Symbol", typeof Symbol === "function" ? Symbol.prototype : null]];
    for (var i = 0; i < candidates.length; i += 1) {
      if (!candidates[i][1]) continue;
      try { return { kind: candidates[i][0], value: candidates[i][1].valueOf.call(value) }; } catch (_) {}
    }
    return null;
  }
  function isErrorValue(value) {
    if (value instanceof Error) return true;
    try { return objectTag(value) === "[object Error]"; } catch (_) { return false; }
  }
  function webCryptoKeyMetadata(value) {
    if (typeof CryptoKey === "function") {
      try {
        var prototype = CryptoKey.prototype;
        var typeGetter = Object.getOwnPropertyDescriptor(prototype, "type").get;
        var extractableGetter = Object.getOwnPropertyDescriptor(prototype, "extractable").get;
        var algorithmGetter = Object.getOwnPropertyDescriptor(prototype, "algorithm").get;
        var usagesGetter = Object.getOwnPropertyDescriptor(prototype, "usages").get;
        return {
          type: typeGetter.call(value),
          extractable: extractableGetter.call(value),
          algorithm: algorithmGetter.call(value),
          usages: usagesGetter.call(value),
        };
      } catch (_) { return null; }
    }
    // Without the public CryptoKey constructor there is no standard brand
    // getter. Treat a matching tag conservatively as opaque: compare metadata,
    // then report distinct instances as unequal because key bytes are hidden.
    try {
      if (objectTag(value) === "[object CryptoKey]") return {type:value.type, extractable:value.extractable, algorithm:value.algorithm, usages:value.usages, opaqueTagFallback:true};
    } catch (_) {}
    return null;
  }
  function viewBytes(value) {
    try {
      if (objectTag(value) === "[object DataView]") {
        return new Uint8Array(getGetter(DataView.prototype, "buffer", value), getGetter(DataView.prototype, "byteOffset", value), getGetter(DataView.prototype, "byteLength", value));
      }
      var typedArrayPrototype = Object.getPrototypeOf(Uint8Array.prototype);
      return new Uint8Array(getGetter(typedArrayPrototype, "buffer", value), getGetter(typedArrayPrototype, "byteOffset", value), getGetter(typedArrayPrototype, "byteLength", value));
    } catch (_) { return null; }
  }
  function typedArrayName(value) {
    try {
      var typedArrayPrototype = Object.getPrototypeOf(Uint8Array.prototype);
      return Object.getOwnPropertyDescriptor(typedArrayPrototype, Symbol.toStringTag).get.call(value);
    } catch (_) { return null; }
  }
  function bufferView(value) { try { return new Uint8Array(value); } catch (_) { return null; } }
  function copySeen(a, b) { return [a.slice(), b.slice()]; }
  function adoptSeen(a, b, copy) {
    a.length = 0; b.length = 0;
    Array.prototype.push.apply(a, copy.a); Array.prototype.push.apply(b, copy.b);
  }
  function mapSize(value) {
    try { return { matched: true, value: Object.getOwnPropertyDescriptor(Map.prototype, "size").get.call(value) }; }
    catch (_) { return { matched: false }; }
  }
  function setSize(value) {
    try { return { matched: true, value: Object.getOwnPropertyDescriptor(Set.prototype, "size").get.call(value) }; }
    catch (_) { return { matched: false }; }
  }
  function findUnorderedMatch(left, right, index, used, strict, seenA, seenB, mapMode) {
    if (index === left.length) return { a: seenA, b: seenB };
    for (var j = 0; j < right.length; j += 1) {
      if (used[j]) continue;
      var copy = copySeen(seenA, seenB);
      var matched = mapMode
        ? compareValues(left[index][0], right[j][0], strict, copy[0], copy[1]) && compareValues(left[index][1], right[j][1], strict, copy[0], copy[1])
        : compareValues(left[index], right[j], strict, copy[0], copy[1]);
      if (!matched) continue;
      used[j] = true;
      var result = findUnorderedMatch(left, right, index + 1, used, strict, copy[0], copy[1], mapMode);
      if (result) return result;
      used[j] = false;
    }
    return null;
  }

  function compareValues(a, b, strict, seenA, seenB) {
    if (strict) {
      if (Object.is(a, b)) return true;
    } else {
      if (a === b || a !== a && b !== b || a == null && b == null) return true;
      var aPrimitive = a === null || a === undefined || typeof a !== "object" && typeof a !== "function";
      var bPrimitive = b === null || b === undefined || typeof b !== "object" && typeof b !== "function";
      if (aPrimitive && bPrimitive) return a == b;
    }
    if (a === null || b === null || typeof a !== "object" || typeof b !== "object") {
      if (!strict && a != null && b != null && typeof a !== "object" && typeof b !== "object") return a == b;
      return false;
    }
    if (strict && Object.getPrototypeOf(a) !== Object.getPrototypeOf(b)) return false;
    var tagA = objectTag(a), tagB = objectTag(b);
    if (tagA !== tagB) return false;
    for (var seenIndex = 0; seenIndex < seenA.length; seenIndex += 1) {
      if (seenA[seenIndex] === a && seenB[seenIndex] === b) return true;
    }
    seenA.push(a); seenB.push(b);

    var keyApi = runtime.crypto;
    if (keyApi && typeof keyApi.isNivaKeyObject === "function") {
      var aIsKey = keyApi.isNivaKeyObject(a), bIsKey = keyApi.isNivaKeyObject(b);
      if (aIsKey || bIsKey) return aIsKey && bIsKey && typeof keyApi.compareNivaKeyObjects === "function" && keyApi.compareNivaKeyObjects(a, b) === true;
    }

    var cryptoKeyA = webCryptoKeyMetadata(a), cryptoKeyB = webCryptoKeyMetadata(b);
    if (cryptoKeyA || cryptoKeyB) {
      if (!cryptoKeyA || !cryptoKeyB || cryptoKeyA.type !== cryptoKeyB.type || cryptoKeyA.extractable !== cryptoKeyB.extractable ||
          !compareValues(cryptoKeyA.algorithm, cryptoKeyB.algorithm, strict, seenA, seenB) ||
          !compareValues(cryptoKeyA.usages, cryptoKeyB.usages, strict, seenA, seenB)) return false;
      // Non-extractable material is intentionally opaque to browser JavaScript.
      // Equal metadata cannot prove that distinct CryptoKeys contain equal bytes.
      return false;
    }

    var dateA = dateValue(a), dateB = dateValue(b);
    if (dateA.matched || dateB.matched) {
      if (!dateA.matched || !dateB.matched || dateA.value !== dateB.value) return false;
    }
    var regexpA = regexpValue(a), regexpB = regexpValue(b);
    if (regexpA.matched || regexpB.matched) {
      if (!regexpA.matched || !regexpB.matched || regexpA.source !== regexpB.source || regexpA.flags !== regexpB.flags || regexpA.lastIndex !== regexpB.lastIndex) return false;
    }
    var boxedA = boxedValue(a), boxedB = boxedValue(b);
    if (boxedA || boxedB) {
      if (!boxedA || !boxedB || boxedA.kind !== boxedB.kind || (strict ? !Object.is(boxedA.value, boxedB.value) : boxedA.value != boxedB.value)) return false;
    }
    var errorA = isErrorValue(a), errorB = isErrorValue(b);
    if (errorA || errorB) {
      if (!errorA || !errorB) return false;
      var errorKeys = ["message", "name"];
      for (var ei = 0; ei < errorKeys.length; ei += 1) {
        var errorKey = errorKeys[ei];
        var aEnumerable = Object.prototype.propertyIsEnumerable.call(a, errorKey);
        var bEnumerable = Object.prototype.propertyIsEnumerable.call(b, errorKey);
        if (aEnumerable !== bEnumerable || !aEnumerable && a[errorKey] !== b[errorKey]) return false;
      }
      var nestedErrorFields = ["cause", "errors"];
      for (var nestedIndex = 0; nestedIndex < nestedErrorFields.length; nestedIndex += 1) {
        var nestedKey = nestedErrorFields[nestedIndex];
        var hasA = Object.prototype.hasOwnProperty.call(a, nestedKey), hasB = Object.prototype.hasOwnProperty.call(b, nestedKey);
        if (hasA !== hasB || hasA && !compareValues(a[nestedKey], b[nestedKey], strict, seenA, seenB)) return false;
      }
    }

    var bufferApi = runtime.buffer && runtime.buffer.Buffer;
    if (bufferApi && (bufferApi.isBuffer(a) || bufferApi.isBuffer(b))) {
      var aIsBuffer = bufferApi.isBuffer(a), bIsBuffer = bufferApi.isBuffer(b);
      if (strict && (!aIsBuffer || !bIsBuffer) || !ArrayBuffer.isView(a) || !ArrayBuffer.isView(b)) return false;
      var bufferBytesA = viewBytes(a), bufferBytesB = viewBytes(b);
      if (!bufferBytesA || !bufferBytesB || !bytesEqual(bufferBytesA, bufferBytesB)) return false;
    }
    if (ArrayBuffer.isView(a) || ArrayBuffer.isView(b)) {
      if (!ArrayBuffer.isView(a) || !ArrayBuffer.isView(b)) return false;
      var bytesA = viewBytes(a), bytesB = viewBytes(b);
      if (!bytesA || !bytesB || typedArrayName(a) !== typedArrayName(b) || !bytesEqual(bytesA, bytesB)) return false;
    }
    if (tagA === "[object ArrayBuffer]" || tagA === "[object SharedArrayBuffer]") {
      var arrayBytesA = bufferView(a), arrayBytesB = bufferView(b);
      if (!arrayBytesA || !arrayBytesB || !bytesEqual(arrayBytesA, arrayBytesB)) return false;
    }

    var setA = setSize(a), setB = setSize(b);
    if (setA.matched || setB.matched) {
      if (!setA.matched || !setB.matched || setA.value !== setB.value) return false;
      var setValuesA = Array.from(Set.prototype.values.call(a)), setValuesB = Array.from(Set.prototype.values.call(b));
      var setMatch = findUnorderedMatch(setValuesA, setValuesB, 0, [], strict, seenA, seenB, false);
      if (!setMatch) return false;
      adoptSeen(seenA, seenB, setMatch);
    }
    var mapA = mapSize(a), mapB = mapSize(b);
    if (mapA.matched || mapB.matched) {
      if (!mapA.matched || !mapB.matched || mapA.value !== mapB.value) return false;
      var entriesA = Array.from(Map.prototype.entries.call(a)), entriesB = Array.from(Map.prototype.entries.call(b));
      var mapMatch = findUnorderedMatch(entriesA, entriesB, 0, [], strict, seenA, seenB, true);
      if (!mapMatch) return false;
      adoptSeen(seenA, seenB, mapMatch);
    }
    if (tagA === "[object WeakMap]" || tagA === "[object WeakSet]" || tagA === "[object Promise]" || tagA === "[object Generator]" || tagA === "[object AsyncGenerator]") return false;
    if (typeof URL !== "undefined" && (a instanceof URL || b instanceof URL)) {
      if (!(a instanceof URL) || !(b instanceof URL) || a.href !== b.href) return false;
    }
    if (typeof URLSearchParams !== "undefined" && (a instanceof URLSearchParams || b instanceof URLSearchParams)) {
      if (!(a instanceof URLSearchParams) || !(b instanceof URLSearchParams) || a.toString() !== b.toString()) return false;
    }
    if (Array.isArray(a) !== Array.isArray(b) || Array.isArray(a) && a.length !== b.length) return false;

    var keysA = strict ? enumerableKeys(a) : Object.keys(a);
    var keysB = strict ? enumerableKeys(b) : Object.keys(b);
    if (keysA.length !== keysB.length) return false;
    for (var keyIndex = 0; keyIndex < keysA.length; keyIndex += 1) {
      var key = keysA[keyIndex];
      if (!Object.prototype.propertyIsEnumerable.call(b, key) || !compareValues(a[key], b[key], strict, seenA, seenB)) return false;
    }
    return true;
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
      var processFlags = root.process;
      var shouldWarn = code ? !deprecationCodes[code] : !warned;
      if (shouldWarn && !deprecate.noDeprecation && !(processFlags && processFlags.noDeprecation)) {
        if (code) deprecationCodes[code] = true;
        else warned = true;
        var warning = new Error(message || "This function is deprecated.");
        warning.name = "DeprecationWarning";
        if (code) warning.code = code;
        if (processFlags && processFlags.throwDeprecation) throw warning;
        var emitWarning = function () {
          if (processFlags && typeof processFlags.emit === "function") {
            processFlags.emit("warning", warning);
          } else if (typeof console !== "undefined") {
            if (processFlags && processFlags.traceDeprecation && typeof console.trace === "function") console.trace(warning);
            else if (typeof console.warn === "function") console.warn(warning);
          }
        };
        if (typeof queueMicrotask === "function") queueMicrotask(emitWarning);
        else {
          Promise.resolve().then(emitWarning);
        }
      }
      if (new.target) return Reflect.construct(fn, Array.prototype.slice.call(arguments), new.target === deprecated ? fn : new.target);
      return fn.apply(this, arguments);
    }
    Object.defineProperty(deprecated, "name", { value: fn.name, configurable: true });
    if (fn.prototype) deprecated.prototype = fn.prototype;
    return deprecated;
  }
  var deprecationCodes = Object.create(null);
  deprecate.noDeprecation = false;
  deprecate.throwDeprecation = false;
  deprecate.traceDeprecation = false;

  function inherits(ctor, superCtor) {
    if (typeof ctor !== "function") throw invalidFunctionArgument("ctor", ctor);
    if (typeof superCtor !== "function") throw invalidFunctionArgument("superCtor", superCtor);
    Object.defineProperty(ctor, "super_", { value: superCtor, writable: true, configurable: true });
    Object.setPrototypeOf(ctor.prototype, superCtor.prototype);
  }

  var module = {
    format: format,
    formatWithOptions: formatWithOptions,
    inspect: inspect,
    promisify: promisify,
    callbackify: callbackify,
    isDeepStrictEqual: isDeepStrictEqual,
    isDeepEqual: isDeepEqual,
    isPartialDeepStrictEqual: isPartialDeepStrictEqual,
    deprecate: deprecate,
    inherits: inherits,
    customPromisifyArgs: promisify.customArgs,
  };
  runtime.createUtilModule = function () { return module; };
  runtime.util = module;
})(globalThis);
