(function (root) {
  "use strict";

  var runtime = root[Symbol.for("niva.node-compat.runtime")];

  function bytesOf(value) {
    if (value instanceof Uint8Array) return new Uint8Array(value.buffer, value.byteOffset, value.byteLength);
    if (value instanceof ArrayBuffer) return new Uint8Array(value);
    if (ArrayBuffer.isView(value)) return new Uint8Array(value.buffer, value.byteOffset, value.byteLength);
    if (typeof value === "string") return new TextEncoder().encode(value);
    throw new TypeError("stdin data must be a string, Buffer, Uint8Array, or ArrayBuffer");
  }

  function concatenate(chunks) {
    var length = chunks.reduce(function (total, chunk) { return total + chunk.length; }, 0);
    var output = new Uint8Array(length);
    var offset = 0;
    chunks.forEach(function (chunk) { output.set(chunk, offset); offset += chunk.length; });
    return output;
  }

  function decode(chunks, encoding) {
    if (encoding === null) return concatenate(chunks);
    var decoder;
    try { decoder = new TextDecoder(encoding || "utf-8"); }
    catch (_) { throw new TypeError("Unsupported output encoding: " + encoding); }
    return decoder.decode(concatenate(chunks));
  }

  function createOutputStream(encoding) {
    var emitter = runtime.createEmitter();
    var chunks = [];
    var ended = false;
    var decoder = encoding === null ? null : new TextDecoder("utf-8");
    var readable = {
      readable: true,
      on: function (event, listener) { emitter.on(event, listener); return readable; },
      addListener: function (event, listener) { return readable.on(event, listener); },
      once: function (event, listener) { emitter.once(event, listener); return readable; },
      off: function (event, listener) { emitter.off(event, listener); return readable; },
      removeListener: function (event, listener) { return readable.off(event, listener); },
      removeAllListeners: function (event) { emitter.removeAllListeners(event); return readable; },
      listeners: function (event) { return emitter.listeners(event); },
      listenerCount: function (event) { return emitter.listenerCount(event); },
      setEncoding: function (value) {
        if (value !== null && value !== "utf8" && value !== "utf-8") throw new TypeError("Only UTF-8 and binary child output are supported");
        encoding = value === null ? null : "utf8";
        decoder = encoding === null ? null : new TextDecoder("utf-8");
        return readable;
      },
      pipe: function (destination) {
        readable.on("data", function (chunk) { destination.write(chunk); });
        readable.once("end", function () { if (typeof destination.end === "function") destination.end(); });
        return destination;
      },
      _push: function (bytes) {
        var chunk = encoding === null
          ? (runtime.buffer ? runtime.buffer.Buffer.from(bytes) : new Uint8Array(bytes))
          : decoder.decode(bytes, { stream: true });
        if (chunk.length === 0) return;
        if (emitter.listenerCount("data") > 0) emitter.emit("data", chunk);
        else chunks.push(chunk);
      },
      _finish: function () {
        if (ended) return;
        ended = true;
        if (decoder) {
          var tail = decoder.decode();
          if (tail) {
            if (emitter.listenerCount("data") > 0) emitter.emit("data", tail);
            else chunks.push(tail);
          }
        }
        if (chunks.length) chunks.splice(0).forEach(function (chunk) { emitter.emit("data", chunk); });
        emitter.emit("end");
        emitter.emit("close");
      },
    };
    return readable;
  }

  function createStdin(niva, streamRef, isDetached) {
    var emitter = runtime.createEmitter();
    var ended = false;
    var stdin = {
      writable: true,
      destroyed: false,
      on: function (event, listener) { emitter.on(event, listener); return stdin; },
      addListener: function (event, listener) { return stdin.on(event, listener); },
      once: function (event, listener) { emitter.once(event, listener); return stdin; },
      off: function (event, listener) { emitter.off(event, listener); return stdin; },
      removeListener: function (event, listener) { return stdin.off(event, listener); },
      end: function (chunk, encoding, callback) {
        if (typeof chunk === "function") { callback = chunk; chunk = undefined; }
        else if (typeof encoding === "function") { callback = encoding; }
        if (ended) { if (callback) callback(); return stdin; }
        ended = true;
        if (!isDetached && streamRef.current) {
          try {
            var bytes = chunk === undefined ? new Uint8Array(0) : bytesOf(chunk);
            niva.streamSend(streamRef.current.id, bytes, true);
          } catch (error) {
            emitter.emit("error", runtime.nativeError(error));
          }
        }
        stdin.writable = false;
        if (callback) Promise.resolve().then(callback);
        emitter.emit("finish");
        return stdin;
      },
      write: function (chunk, encoding, callback) {
        if (typeof encoding === "function") callback = encoding;
        if (ended || isDetached || !streamRef.current) return false;
        try {
          var ok = niva.streamSend(streamRef.current.id, bytesOf(chunk), false);
          if (callback) Promise.resolve().then(callback);
          return ok;
        } catch (error) {
          var converted = runtime.nativeError(error);
          emitter.emit("error", converted);
          if (callback) Promise.resolve().then(function () { callback(converted); });
          return false;
        }
      },
      destroy: function (error) {
        stdin.destroyed = true;
        stdin.writable = false;
        if (error) emitter.emit("error", error);
        if (streamRef.current && typeof streamRef.current.cancel === "function") streamRef.current.cancel();
        return stdin;
      },
    };
    return stdin;
  }

  function checkStdio(value) {
    if (value === undefined || value === "pipe") return;
    if (Array.isArray(value) && value.length === 3 && value.every(function (entry) { return entry === "pipe"; })) return;
    throw runtime.bridgeError("Niva child_process supports piped stdio only", "ENOTSUP");
  }

  function shellInvocation(command, shell) {
    var windows = runtime.path && runtime.path.sep === "\\";
    if (shell === false) return { command: command, args: [] };
    var executable = typeof shell === "string" ? shell : (windows ? "cmd.exe" : "/bin/sh");
    return windows
      ? { command: executable, args: ["/d", "/s", "/c", command] }
      : { command: executable, args: ["-c", command] };
  }

  function createChildProcessModule(niva) {
    function spawn(command, args, options) {
      if (typeof command !== "string" || command.length === 0) throw new TypeError("command must be a non-empty string");
      if (args === undefined || args === null) args = [];
      if (!Array.isArray(args) || args.some(function (value) { return typeof value !== "string"; })) {
        throw new TypeError("args must be an array of strings");
      }
      options = options || {};
      checkStdio(options.stdio);
      if (options.signal) throw runtime.bridgeError("AbortSignal is not supported by the Niva process bridge", "ENOTSUP");
      if (options.timeout) throw runtime.bridgeError("Process timeouts are not supported by the Niva process bridge", "ENOTSUP");
      var encoding = options.encoding === undefined ? null : options.encoding;
      if (encoding !== null && encoding !== "utf8" && encoding !== "utf-8") {
        throw new TypeError("Only UTF-8 and binary child output are supported");
      }
      var invocation = options.shell ? shellInvocation(args.length ? command : command, options.shell) : { command: command, args: args };
      if (options.shell && args.length > 0) throw new TypeError("shell: true with an args array is not supported");
      var execOptions = {};
      if (options.cwd !== undefined) execOptions.currentDir = options.cwd;
      else if (options.currentDir !== undefined) execOptions.currentDir = options.currentDir;
      if (options.env !== undefined) execOptions.env = options.env;
      if (options.detached !== undefined) execOptions.detached = !!options.detached;

      var events = runtime.createEmitter();
      var streamRef = { current: null };
      var detached = !!options.detached;
      var child = {
        stdin: null,
        stdout: createOutputStream(encoding),
        stderr: createOutputStream(encoding),
        stdio: null,
        pid: undefined,
        killed: false,
        connected: false,
        exitCode: null,
        signalCode: null,
        spawnargs: [invocation.command].concat(invocation.args),
        spawnfile: invocation.command,
        on: function (event, listener) { events.on(event, listener); return child; },
        addListener: function (event, listener) { return child.on(event, listener); },
        once: function (event, listener) { events.once(event, listener); return child; },
        off: function (event, listener) { events.off(event, listener); return child; },
        removeListener: function (event, listener) { events.off(event, listener); return child; },
        removeAllListeners: function (event) { events.removeAllListeners(event); return child; },
        listeners: function (event) { return events.listeners(event); },
        listenerCount: function (event) { return events.listenerCount(event); },
        emit: function () { return events.emit.apply(events, arguments); },
        kill: function () {
          // Per-process kill is not wired to the native handler. Use cancel()
          // to stop an attached child through the bridge call instead.
          return false;
        },
        cancel: function () {
          if (!streamRef.current || typeof streamRef.current.cancel !== "function") return false;
          streamRef.current.cancel();
          return true;
        },
      };
      child.stdin = createStdin(niva, streamRef, detached);
      child.stdio = [child.stdin, child.stdout, child.stderr];

      var byteChain = Promise.resolve();
      var stdoutChunks = [];
      var stderrChunks = [];
      var requestError = null;
      var st;
      try {
        st = runtime.stream(niva, "process.execStream", [invocation.command, invocation.args, Object.keys(execOptions).length ? execOptions : null], {
        onBlob: function (blob, isStderr) {
          byteChain = byteChain.then(function () {
              var arrayBuffer = blob && typeof blob.arrayBuffer === "function"
                ? blob.arrayBuffer()
                : Promise.resolve(blob instanceof Uint8Array ? blob.buffer.slice(blob.byteOffset, blob.byteOffset + blob.byteLength) : blob);
              return Promise.resolve(arrayBuffer).then(function (buffer) {
                var bytes = new Uint8Array(buffer);
                if (isStderr) {
                  stderrChunks.push(bytes);
                  child.stderr._push(bytes);
                } else {
                  stdoutChunks.push(bytes);
                  child.stdout._push(bytes);
                }
              });
            });
          },
          onEvent: function (name, data) { events.emit(name, data); },
        });
        streamRef.current = st;
      } catch (error) {
        requestError = runtime.nativeError(error);
      }

      var completion = new Promise(function (resolve, reject) {
        Promise.resolve().then(function () {
          if (!requestError) events.emit("spawn");
        });
        if (requestError) {
          Promise.resolve().then(function () {
            child.stdout._finish();
            child.stderr._finish();
            events.emit("error", requestError);
            events.emit("close", null, null);
            reject(requestError);
          });
          return;
        }
        Promise.resolve(st.promise).then(function (result) {
          return byteChain.then(function () {
            if (typeof result === "number") {
              child.pid = result;
              child.stdout._finish();
              child.stderr._finish();
              resolve({ pid: result, status: null, detached: true });
              return;
            }
            var status = result && result.status;
            child.exitCode = status == null ? null : status;
            child.stdout._finish();
            child.stderr._finish();
            events.emit("exit", child.exitCode, null);
            events.emit("close", child.exitCode, null);
            resolve({ status: child.exitCode, detached: false });
          });
        }, function (error) {
          var converted = runtime.nativeError(error);
          byteChain.then(function () {
            child.stdout._finish();
            child.stderr._finish();
            events.emit("error", converted);
            events.emit("close", null, null);
            reject(converted);
          }, reject);
        }).catch(function (error) {
          var converted = runtime.nativeError(error);
          child.stdout._finish();
          child.stderr._finish();
          events.emit("error", converted);
          events.emit("close", null, null);
          reject(converted);
        });
      });
      completion.catch(function () {});
      child.completion = completion;
      child._stdoutChunks = stdoutChunks;
      child._stderrChunks = stderrChunks;
      child._stream = streamRef;
      return child;
    }

    function exec(command, options, callback) {
      if (typeof options === "function") { callback = options; options = {}; }
      options = options || {};
      if (options.detached) throw runtime.bridgeError("child_process.exec does not support detached processes", "ENOTSUP");
      if (options.timeout) throw runtime.bridgeError("Process timeouts are not supported by the Niva process bridge", "ENOTSUP");
      if (options.maxBuffer !== undefined) throw runtime.bridgeError("maxBuffer is not supported by the Niva process bridge", "ENOTSUP");
      var encoding = options.encoding === undefined ? "utf8" : options.encoding;
      if (encoding !== null && encoding !== "utf8" && encoding !== "utf-8") throw new TypeError("Only UTF-8 and binary exec output are supported");
      var invocation = shellInvocation(command, options.shell === undefined ? true : options.shell);
      var child = spawn(invocation.command, invocation.args, {
        cwd: options.cwd,
        env: options.env,
        stdio: options.stdio,
        encoding: encoding,
      });
      // exec is non-interactive by default; close the bridge's piped stdin so
      // commands that read until EOF can finish. spawn leaves it caller-owned.
      child.stdin.end();
      var out = [];
      var err = [];
      child.stdout.on("data", function (chunk) { out.push(chunk instanceof Uint8Array ? new Uint8Array(chunk) : chunk); });
      child.stderr.on("data", function (chunk) { err.push(chunk instanceof Uint8Array ? new Uint8Array(chunk) : chunk); });

      function joined(chunks) {
        if (encoding !== null) return chunks.join("");
        return runtime.buffer.Buffer.from(concatenate(chunks));
      }
      function execError(status, stdout, stderr) {
        var error = runtime.bridgeError("Command failed with exit code " + status, status == null ? "NIVA_BRIDGE_ERROR" : status);
        error.status = status;
        error.code = status;
        error.stdout = stdout;
        error.stderr = stderr;
        return error;
      }

      var result = child.completion.then(function (exit) {
        var stdout = joined(out);
        var stderr = joined(err);
        if (exit.status !== 0) throw execError(exit.status, stdout, stderr);
        return { status: exit.status, stdout: stdout, stderr: stderr };
      }, function (error) {
        error.stdout = joined(out);
        error.stderr = joined(err);
        throw error;
      });
      result.catch(function () {});
      child.result = result;
      child.completion = result;
      if (typeof callback === "function") {
        result.then(function (value) { callback(null, value.stdout, value.stderr); }, function (error) { callback(error, error.stdout, error.stderr); });
      }
      return child;
    }

    return { spawn: spawn, exec: exec };
  }

  runtime.createChildProcessModule = createChildProcessModule;
  runtime.child_process = createChildProcessModule();
})(globalThis);
