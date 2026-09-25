(function (root: any) {
  "use strict";
  const runtime = root[Symbol.for("niva.node-compat.runtime")];
  if (typeof runtime.createProcessModule === "function") return;

  const NODE_COMPAT_VERSION = "22.14.0";
  const NODE_COMPAT_PROCESS_VERSION = "v" + NODE_COMPAT_VERSION;

  function createProcessModule(niva?: any) {
    const target = niva || root.Niva;
    const metadata = target?.bootstrap?.process || null;
    const process: any = new runtime.events.EventEmitter();
    const nivaVersion = metadata?.versions?.niva ||
      String(metadata?.version || target?.bootstrap?.nivaVersion || "v0.9.9").replace(/^v/, "");
    const versions = Object.assign({}, metadata?.versions || {}, {
      niva: nivaVersion,
      node: NODE_COMPAT_VERSION,
      nodeCompat: NODE_COMPAT_VERSION,
    });
    Object.assign(process, metadata || {}, {
      __nivaAvailable: !!metadata,
      env: Object.assign({}, metadata?.env || {}),
      argv: Array.isArray(metadata?.argv) ? metadata.argv.slice() : [],
      version: NODE_COMPAT_PROCESS_VERSION,
      versions,
      exitCode: 0,
    });

    function unavailable(name: string): never {
      throw runtime.bridgeError("process." + name + " is available only in the trusted main window", "ERR_NIVA_PROCESS_UNAVAILABLE");
    }
    process.cwd = function () {
      if (!metadata) return unavailable("cwd");
      return runtime.callSync(target, "process.currentDir", []);
    };
    process.chdir = function (path: string) {
      if (typeof path !== "string") throw Object.assign(new TypeError('The "directory" argument must be of type string'), { code: "ERR_INVALID_ARG_TYPE" });
      if (!metadata) return unavailable("chdir");
      return runtime.callSync(target, "process.setCurrentDir", [path]);
    };
    process.nextTick = function (callback: Function) {
      if (typeof callback !== "function") throw new TypeError("callback must be a function");
      const args = Array.prototype.slice.call(arguments, 1);
      queueMicrotask(() => callback.apply(null, args));
    };
    function nowNs(): bigint {
      if (!root.performance || typeof root.performance.now !== "function")
        throw runtime.bridgeError("A monotonic performance clock is unavailable", "ERR_METHOD_NOT_IMPLEMENTED");
      return BigInt(Math.floor(root.performance.now() * 1_000_000));
    }
    const hrtime: any = function (previous?: [number, number]): [number, number] {
      let elapsed = nowNs();
      if (previous !== undefined) {
        if (!Array.isArray(previous) || previous.length !== 2 || !Number.isInteger(previous[0]) || !Number.isInteger(previous[1]) || previous[0] < 0 || previous[1] < 0 || previous[1] >= 1_000_000_000)
          throw Object.assign(new TypeError('The "time" argument must be an array of two non-negative integers'), { code: "ERR_INVALID_ARG_TYPE" });
        elapsed -= BigInt(previous[0]) * 1_000_000_000n + BigInt(previous[1]);
      }
      let seconds = elapsed / 1_000_000_000n;
      let nanoseconds = elapsed % 1_000_000_000n;
      if (nanoseconds < 0n) { seconds -= 1n; nanoseconds += 1_000_000_000n; }
      return [Number(seconds), Number(nanoseconds)];
    };
    hrtime.bigint = function () { return nowNs(); };
    process.hrtime = hrtime;
    process.exit = function (code?: number) {
      if (!metadata) return unavailable("exit");
      process.exitCode = code === undefined ? process.exitCode : code;
      process.emit("exit", process.exitCode);
      runtime.call(target, "process.exit", [process.exitCode]).catch((error: any) => process.emit("error", runtime.nativeError(error)));
    };

    if (metadata) {
      const Writable = runtime.vendor.stream.Writable;
      const Readable = runtime.vendor.stream.Readable;
      const Buffer = runtime.vendor.Buffer;
      const output = (channel: string) => new Writable({
        write(chunk: any, _encoding: string, callback: (error?: Error | null) => void) {
          runtime.call(target, "process.write", [channel, Buffer.from(chunk).toString("base64")]).then(() => callback(), (error: any) => callback(runtime.nativeError(error)));
        },
      });
      process.stdout = output("stdout");
      process.stderr = output("stderr");
      const stdioIsTTY = metadata?.stdioIsTTY || {};
      process.stdout.fd = 1;
      process.stdout.isTTY = stdioIsTTY.stdout === true;
      process.stderr.fd = 2;
      process.stderr.isTTY = stdioIsTTY.stderr === true;
      let inputCall: any;
      let inputOwner: any;
      let inputInvalidationError: Error | undefined;
      const input = new Readable({
        read() {
          if (inputCall) return;
          inputCall = runtime.stream(target, "process.stdin", [], { onChunk: (chunk: Uint8Array) => input.push(Buffer.from(chunk)) });
          inputCall.promise.then(() => { if (inputOwner) inputOwner.release(); input.push(null); }, (error: any) => {
            if (inputOwner) inputOwner.release();
            input.destroy(inputInvalidationError || runtime.nativeError(error));
          });
        },
        destroy(error: Error | null, callback: (error?: Error | null) => void) {
          if (inputCall) { if (!inputInvalidationError) inputCall.cancel(); inputCall = undefined; }
          if (inputOwner) inputOwner.release();
          callback(error);
        },
      });
      process.stdin = input;
      input.fd = 0;
      input.isTTY = stdioIsTTY.stdin === true;
      input.__nivaInvalidate = function (error: Error) {
        if (input.destroyed) return;
        inputInvalidationError = error;
        if (inputCall) inputCall.cancel();
        input.destroy(error);
        if (inputOwner) inputOwner.release();
      };
      if (typeof runtime.registerResource === "function") {
        inputOwner = runtime.registerResource(input, function () { return inputCall && inputCall.cancel(); });
      }
    } else {
      process.stdin = null;
      process.stdout = null;
      process.stderr = null;
    }
    return process;
  }

  runtime.createProcessModule = createProcessModule;
  runtime.process = createProcessModule();
})(globalThis as any);
