(function (root: any) {
  "use strict";
  const runtime = root[Symbol.for("niva.node-compat.runtime")];
  if (!runtime || typeof runtime.installCallSiteCompat === "function") return;

  function installCallSiteCompat() {
  const ErrorConstructor: any = root.Error;
  if (!ErrorConstructor) return;

  const nativeCapture = typeof ErrorConstructor.captureStackTrace === "function"
    ? ErrorConstructor.captureStackTrace.bind(ErrorConstructor)
    : null;

  function restoresPrepareStackTrace(descriptor: PropertyDescriptor | undefined) {
    if (descriptor) Object.defineProperty(ErrorConstructor, "prepareStackTrace", descriptor);
    else delete ErrorConstructor.prepareStackTrace;
  }

  function hasV8CallSites() {
    if (!nativeCapture) return false;
    const descriptor = Object.getOwnPropertyDescriptor(ErrorConstructor, "prepareStackTrace");
    let stack: unknown;
    try {
      Object.defineProperty(ErrorConstructor, "prepareStackTrace", {
        configurable: true,
        writable: true,
        value: (_error: Error, callSites: unknown[]) => callSites,
      });
      const probe: any = {};
      nativeCapture(probe);
      stack = probe.stack;
      return Array.isArray(stack);
    } catch (_) {
      return false;
    } finally {
      restoresPrepareStackTrace(descriptor);
    }
  }

  if (hasV8CallSites()) return;

  function parseLocation(value: string) {
    const match = /^(.*):(\d+):(\d+)$/.exec(value);
    if (!match) return { filename: undefined, line: undefined, column: undefined };
    return { filename: match[1], line: Number(match[2]), column: Number(match[3]) };
  }

  function parseFrame(line: string) {
    const v8 = /^\s*at (.*)$/.exec(line);
    if (v8) {
      const body = v8[1];
      const open = body.lastIndexOf(" (");
      const named = open >= 0 && body.endsWith(")");
      const functionName = named ? body.slice(0, open) : undefined;
      const location = named ? body.slice(open + 2, -1) : body;
      return { ...parseLocation(location), functionName, isEval: /^eval at /.test(location), evalOrigin: undefined };
    }
    const at = line.indexOf("@");
    if (at >= 0) {
      const functionName = line.slice(0, at) || undefined;
      const location = parseLocation(line.slice(at + 1));
      return { ...location, functionName, isEval: false, evalOrigin: undefined };
    }
    return { filename: undefined, line: undefined, column: undefined, functionName: undefined, isEval: false, evalOrigin: undefined };
  }

  function callSite(frame: any) {
    const site: any = {
      getThis() { return undefined; },
      getTypeName() { return undefined; },
      getFunction() { return undefined; },
      getFunctionName() { return frame.functionName; },
      getMethodName() {
        if (!frame.functionName) return undefined;
        const separator = Math.max(frame.functionName.lastIndexOf("."), frame.functionName.lastIndexOf("]"));
        return frame.functionName.slice(separator + 1).replace(/^['"]|['"]$/g, "");
      },
      getFileName() { return frame.filename; },
      getScriptNameOrSourceURL() { return frame.filename; },
      getLineNumber() { return frame.line; },
      getColumnNumber() { return frame.column; },
      getEvalOrigin() { return frame.evalOrigin; },
      isToplevel() { return !frame.functionName; },
      isEval() { return frame.isEval; },
      isNative() { return false; },
      isConstructor() { return false; },
      isAsync() { return false; },
      isPromiseAll() { return false; },
      getPromiseIndex() { return undefined; },
      toString() {
        const location = frame.filename || "<anonymous>";
        const suffix = frame.line === undefined ? location : location + ":" + frame.line + ":" + frame.column;
        return frame.functionName ? "    at " + frame.functionName + " (" + suffix + ")" : "    at " + suffix;
      },
    };
    return site;
  }

  function compatibleCaptureStackTrace(target: any, constructorOpt?: Function) {
    if (target === null || (typeof target !== "object" && typeof target !== "function")) {
      throw new TypeError("The first argument must be an object");
    }
    if (nativeCapture) nativeCapture(target, constructorOpt);
    else target.stack = new ErrorConstructor().stack;

    const prepare = ErrorConstructor.prepareStackTrace;
    if (typeof prepare !== "function") return;
    const stack = typeof target.stack === "string" ? target.stack : "";
    const frames = stack.split("\n").slice(1).map(parseFrame)
      .filter((frame: any) => frame.functionName !== "compatibleCaptureStackTrace");
    if (constructorOpt && constructorOpt.name) {
      const constructorFrame = frames.findIndex((frame: any) => frame.functionName === constructorOpt.name);
      if (constructorFrame >= 0) frames.splice(0, constructorFrame + 1);
    }
    const callSites = frames.map(callSite);
    const formatted = prepare(target, callSites);
    try {
      Object.defineProperty(target, "stack", { configurable: true, writable: true, value: formatted });
    } catch (_) {
      target.stack = formatted;
    }
  }

  try {
    Object.defineProperty(ErrorConstructor, "captureStackTrace", {
      configurable: true,
      writable: true,
      value: compatibleCaptureStackTrace,
    });
  } catch (_) {
    // Engines with a non-configurable captureStackTrace retain their native API.
  }
  }

  runtime.installCallSiteCompat = installCallSiteCompat;
})(globalThis as any);
