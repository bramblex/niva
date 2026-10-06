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
      String(metadata?.version || target?.bootstrap?.nivaVersion || "v0.10.0-beta.1").replace(/^v/, "");
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
    let suspendStdio = function (_error: Error) {};
    let resumeStdio = function () {};

    function unavailable(name: string): never {
      throw runtime.bridgeError("process." + name + " is available only in the trusted main window", "ERR_NIVA_PROCESS_UNAVAILABLE");
    }
    process.cwd = function () {
      if (!metadata) return unavailable("cwd");
      return runtime.callSyncAs(target, "process.cwd", "process.currentDir", []);
    };
    process.chdir = function (path: string) {
      if (typeof path !== "string") throw Object.assign(new TypeError('The "directory" argument must be of type string'), { code: "ERR_INVALID_ARG_TYPE" });
      if (!metadata) return unavailable("chdir");
      return runtime.call(target, "process.setCurrentDir", [path]).then(function () { return undefined; });
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
      const outputStates: any[] = [];
      const retiredOutputSessions = new Set<string>();
      function pruneRetiredOutputSessions(retain?: string) {
        const inUse = new Set<string>();
        if (retain) inUse.add(retain);
        for (const state of outputStates) {
          if (state.activeWrite?.sessionId) inUse.add(state.activeWrite.sessionId);
          if (state.recoverable?.sessionId) inUse.add(state.recoverable.sessionId);
        }
        for (const sessionId of retiredOutputSessions) {
          if (!inUse.has(sessionId)) retiredOutputSessions.delete(sessionId);
        }
      }
      function tryRestoreOutput(state: any) {
        const stream = state.stream;
        const failure = state.recoverable;
        if (!failure) return false;
        const writable = stream._writableState;
        const callbacksDrained = !!writable && state.pendingWriteCallbacks === 0
          && writable.destroyed === true && writable.closed === true
          && writable.closeEmitted === true && writable.writing !== true && writable.writecb === null && writable.length === 0
          && Array.isArray(writable.buffered) && writable.buffered.length === 0;
        if (!callbacksDrained) return false;
        if (state.explicitlyDestroyed || writable.ending === true || writable.ended === true || writable.finished === true) {
          state.recoverable = undefined;
          state.restoredSessions.delete(failure.sessionId);
          pruneRetiredOutputSessions();
          return false;
        }
        if (!state.restoredSessions.has(failure.sessionId) || target.bridge?.sessionId === failure.sessionId
          || typeof stream._undestroy !== "function" || writable.errored !== failure.error
          || writable.errorEmitted !== true || writable.pendingcb !== failure.stalePendingCallbacks) return false;

        // readable-stream@4.7.0 drains buffered error callbacks without decrementing pendingcb.
        // Our callback count and empty terminal queue prove this residue cannot represent live callbacks.
        if (writable.pendingcb !== 0) writable.pendingcb = 0;
        // _undestroy resets the public stream lifecycle only after that pinned-vendor residue is normalized.
        stream._undestroy();
        state.recoverable = undefined;
        state.restoredSessions.delete(failure.sessionId);
        pruneRetiredOutputSessions();
        return true;
      }
      const output = (channel: string) => {
        const state: any = {
          channel,
          activeWrite: undefined,
          recoverable: undefined,
          restoredSessions: new Set<string>(),
          pendingWriteCallbacks: 0,
          explicitlyDestroyed: false,
          inUserWriteCallback: false,
          stream: undefined,
        };
        const stream = new Writable({
          write(chunk: any, _encoding: string, callback: (error?: Error | null) => void) {
            const sessionId = String(target.bridge?.sessionId || "");
            const write = { sessionId };
            state.activeWrite = write;
            runtime.call(target, "process.write", [channel, Buffer.from(chunk).toString("base64")]).then(() => {
              if (state.activeWrite === write) state.activeWrite = undefined;
              pruneRetiredOutputSessions();
              callback();
            }, (error: any) => {
              const writeError = runtime.nativeError(error);
              if (writeError?.code === "ERR_NIVA_SESSION_EXPIRED" && retiredOutputSessions.has(sessionId)) {
                const writable = state.stream?._writableState;
                const stalePendingCallbacks = Array.isArray(writable?.buffered)
                  ? Math.max(0, writable.buffered.length - (writable.bufferedIndex || 0)) : 0;
                state.recoverable = { sessionId, error: writeError, stalePendingCallbacks };
              }
              if (state.activeWrite === write) state.activeWrite = undefined;
              pruneRetiredOutputSessions();
              callback(writeError);
            });
          },
        });
        state.stream = stream;
        outputStates.push(state);

        const nativeWrite = stream.write;
        stream.write = function () {
          const args = Array.prototype.slice.call(arguments);
          const callbackIndex = typeof args[1] === "function" ? 1
            : typeof args[2] === "function" ? 2
              : args.length < 2 ? 1 : 2;
          const callback = typeof args[callbackIndex] === "function" ? args[callbackIndex] : undefined;
          let callbackSettled = false;
          args[callbackIndex] = function () {
            if (!callbackSettled) {
              callbackSettled = true;
              state.pendingWriteCallbacks = Math.max(0, state.pendingWriteCallbacks - 1);
            }
            if (callback) {
              const wasInUserWriteCallback = state.inUserWriteCallback;
              state.inUserWriteCallback = true;
              try { return callback.apply(this, arguments); }
              finally {
                state.inUserWriteCallback = wasInUserWriteCallback;
                tryRestoreOutput(state);
              }
            }
            tryRestoreOutput(state);
          };
          state.pendingWriteCallbacks += 1;
          try {
            return nativeWrite.apply(this, args);
          } catch (error) {
            if (!callbackSettled) {
              callbackSettled = true;
              state.pendingWriteCallbacks = Math.max(0, state.pendingWriteCallbacks - 1);
            }
            throw error;
          }
        };

        const nativeDestroy = stream.destroy;
        stream.destroy = function () {
          const error = arguments[0];
          const isWritableFailure = state.recoverable && error === state.recoverable.error
            && !state.inUserWriteCallback && !stream.destroyed && !state.explicitlyDestroyed;
          if (!isWritableFailure) state.explicitlyDestroyed = true;
          return nativeDestroy.apply(this, arguments);
        };
        stream.on("error", function (error: any) {
          if (state.recoverable?.error === error) return;
          if (stream.listenerCount("error") === 1) throw error;
        });
        stream.on("close", function () { tryRestoreOutput(state); });
        return stream;
      };
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
      let inputSuspended = false;
      let inputRequested = false;
      let input: any;
      function releaseInputOwner() {
        if (inputOwner) inputOwner.release();
        inputOwner = undefined;
      }
      function startInput() {
        if (inputCall || inputSuspended || input.destroyed) return;
        const call = runtime.stream(target, "process.stdin", [], {
          onChunk: (chunk: Uint8Array) => input.push(Buffer.from(chunk)),
        });
        inputCall = call;
        if (typeof runtime.registerResource === "function") {
          inputOwner = runtime.registerResource(input, function () { return call.cancel(); }, function () { return call.id; });
        }
        call.promise.then(() => {
          if (inputCall !== call) return;
          inputCall = undefined;
          releaseInputOwner();
          input.push(null);
        }, (error: any) => {
          if (inputCall !== call) return;
          inputCall = undefined;
          releaseInputOwner();
          input.destroy(inputInvalidationError || runtime.nativeError(error));
        });
      }
      input = new Readable({
        read() {
          inputRequested = true;
          startInput();
        },
        destroy(error: Error | null, callback: (error?: Error | null) => void) {
          const call = inputCall;
          inputCall = undefined;
          if (call && !inputInvalidationError) call.cancel();
          releaseInputOwner();
          callback(error);
        },
      });
      process.stdin = input;
      input.fd = 0;
      input.isTTY = stdioIsTTY.stdin === true;
      input.__nivaInvalidate = function (error: Error) {
        if (input.destroyed) return;
        inputSuspended = false;
        inputInvalidationError = error;
        const call = inputCall;
        inputCall = undefined;
        if (call) call.cancel();
        input.destroy(error);
      };
      input.__nivaSuspend = function (error: Error) {
        if (input.destroyed || inputSuspended) return;
        inputSuspended = true;
        inputInvalidationError = error;
        const call = inputCall;
        inputCall = undefined;
        if (call) call.cancel();
        releaseInputOwner();
      };
      input.__nivaResume = function () {
        if (input.destroyed || !inputSuspended) return;
        inputSuspended = false;
        inputInvalidationError = undefined;
        if (inputRequested) startInput();
      };
      suspendStdio = function (error: Error) {
        const sessionId = String(target.bridge?.sessionId || "");
        if (sessionId) retiredOutputSessions.add(sessionId);
        pruneRetiredOutputSessions(sessionId || undefined);
        input.__nivaSuspend(error);
      };
      resumeStdio = function () {
        for (const state of outputStates) {
          state.restoredSessions.clear();
          for (const sessionId of retiredOutputSessions) {
            if (state.activeWrite?.sessionId === sessionId || state.recoverable?.sessionId === sessionId)
              state.restoredSessions.add(sessionId);
          }
        }
        pruneRetiredOutputSessions();
        for (const state of outputStates) tryRestoreOutput(state);
        input.__nivaResume();
      };
    } else {
      process.stdin = null;
      process.stdout = null;
      process.stderr = null;
    }
    Object.defineProperties(process, {
      __nivaSuspendSession: { value: suspendStdio, enumerable: false },
      __nivaResumeSession: { value: resumeStdio, enumerable: false },
    });
    return process;
  }

  runtime.createProcessModule = createProcessModule;
  runtime.process = createProcessModule();
})(globalThis as any);
