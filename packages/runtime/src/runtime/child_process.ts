(function (root: any) {
    "use strict";
    var runtime = root[Symbol.for("niva.node-compat.runtime")], Buffer = runtime.vendor.Buffer, Readable = runtime.vendor.stream.Readable, Writable = runtime.vendor.stream.Writable;
  if (typeof runtime.createChildProcessModule === "function") return;
    function checkStdio(value) { if (value === undefined || value === "pipe")
        return; if (Array.isArray(value) && value.length === 3 && value.every(item => item === "pipe"))
        return; throw runtime.bridgeError("Only piped child stdio is supported", "ENOTSUP"); }
    function shellInvocation(command, shell) { var windows = runtime.path.sep === "\\", file = typeof shell === "string" ? shell : windows ? "cmd.exe" : "/bin/sh"; return { command: file, args: windows ? ["/d", "/s", "/c", command] : ["-c", command] }; }
    var commonErrnoNames = { 1: "EPERM", 2: "ENOENT", 3: "ESRCH", 4: "EINTR", 5: "EIO", 6: "ENXIO", 7: "E2BIG", 8: "ENOEXEC", 9: "EBADF", 10: "ECHILD", 12: "ENOMEM", 13: "EACCES", 14: "EFAULT", 16: "EBUSY", 17: "EEXIST", 18: "EXDEV", 19: "ENODEV", 20: "ENOTDIR", 21: "EISDIR", 22: "EINVAL", 23: "ENFILE", 24: "EMFILE", 25: "ENOTTY", 27: "EFBIG", 28: "ENOSPC", 29: "ESPIPE", 30: "EROFS", 31: "EMLINK", 32: "EPIPE", 33: "EDOM", 34: "ERANGE" };
    var linuxErrnoNames = { 11: "EAGAIN", 35: "EDEADLK", 36: "ENAMETOOLONG", 37: "ENOLCK", 38: "ENOSYS", 39: "ENOTEMPTY", 40: "ELOOP", 98: "EADDRINUSE", 99: "EADDRNOTAVAIL", 100: "ENETDOWN", 101: "ENETUNREACH", 102: "ENETRESET", 103: "ECONNABORTED", 104: "ECONNRESET", 105: "ENOBUFS", 106: "EISCONN", 107: "ENOTCONN", 108: "ESHUTDOWN", 110: "ETIMEDOUT", 111: "ECONNREFUSED", 112: "EHOSTDOWN", 113: "EHOSTUNREACH", 114: "EALREADY", 115: "EINPROGRESS" };
    var darwinErrnoNames = { 11: "EDEADLK", 35: "EAGAIN", 36: "EINPROGRESS", 37: "EALREADY", 38: "ENOTSOCK", 39: "EDESTADDRREQ", 40: "EMSGSIZE", 41: "EPROTOTYPE", 42: "ENOPROTOOPT", 43: "EPROTONOSUPPORT", 44: "ESOCKTNOSUPPORT", 45: "EOPNOTSUPP", 46: "EPFNOSUPPORT", 47: "EAFNOSUPPORT", 48: "EADDRINUSE", 49: "EADDRNOTAVAIL", 50: "ENETDOWN", 51: "ENETUNREACH", 52: "ENETRESET", 53: "ECONNABORTED", 54: "ECONNRESET", 55: "ENOBUFS", 56: "EISCONN", 57: "ENOTCONN", 60: "ETIMEDOUT", 61: "ECONNREFUSED", 64: "EHOSTDOWN", 65: "EHOSTUNREACH", 66: "ENOTEMPTY", 70: "ESTALE", 78: "ENOSYS" };
    // libuv assigns its own error numbers on Windows; they are not the host
    // errno values used by POSIX builds. These values follow Node 22's pinned
    // deps/uv/include/uv/errno.h and UV_ERRNO_MAP.
    var windowsUvErrorNames = { 3000: "EAI_ADDRFAMILY", 3001: "EAI_AGAIN", 3002: "EAI_BADFLAGS", 3003: "EAI_CANCELED", 3004: "EAI_FAIL", 3005: "EAI_FAMILY", 3006: "EAI_MEMORY", 3007: "EAI_NODATA", 3008: "EAI_NONAME", 3009: "EAI_OVERFLOW", 3010: "EAI_SERVICE", 3011: "EAI_SOCKTYPE", 3013: "EAI_BADHINTS", 3014: "EAI_PROTOCOL", 4023: "EUNATCH", 4024: "ENODATA", 4025: "ESOCKTNOSUPPORT", 4026: "EOVERFLOW", 4027: "EILSEQ", 4028: "EFTYPE", 4029: "ENOTTY", 4030: "EREMOTEIO", 4031: "EHOSTDOWN", 4032: "EMLINK", 4033: "ENXIO", 4034: "ERANGE", 4035: "ENOPROTOOPT", 4036: "EFBIG", 4037: "EXDEV", 4038: "ETXTBSY", 4039: "ETIMEDOUT", 4040: "ESRCH", 4041: "ESPIPE", 4042: "ESHUTDOWN", 4043: "EROFS", 4044: "EPROTOTYPE", 4045: "EPROTONOSUPPORT", 4046: "EPROTO", 4047: "EPIPE", 4048: "EPERM", 4049: "ENOTSUP", 4050: "ENOTSOCK", 4051: "ENOTEMPTY", 4052: "ENOTDIR", 4053: "ENOTCONN", 4054: "ENOSYS", 4055: "ENOSPC", 4056: "ENONET", 4057: "ENOMEM", 4058: "ENOENT", 4059: "ENODEV", 4060: "ENOBUFS", 4061: "ENFILE", 4062: "ENETUNREACH", 4063: "ENETDOWN", 4064: "ENAMETOOLONG", 4065: "EMSGSIZE", 4066: "EMFILE", 4067: "ELOOP", 4068: "EISDIR", 4069: "EISCONN", 4070: "EIO", 4071: "EINVAL", 4072: "EINTR", 4073: "EHOSTUNREACH", 4074: "EFAULT", 4075: "EEXIST", 4076: "EDESTADDRREQ", 4077: "ECONNRESET", 4078: "ECONNREFUSED", 4079: "ECONNABORTED", 4080: "ECHARSET", 4081: "ECANCELED", 4082: "EBUSY", 4083: "EBADF", 4084: "EALREADY", 4088: "EAGAIN", 4089: "EAFNOSUPPORT", 4090: "EADDRNOTAVAIL", 4091: "EADDRINUSE", 4092: "EACCES", 4093: "E2BIG", 4094: "UNKNOWN", 4095: "EOF" };
    function systemErrorName(code, niva) {
        var platform = "";
        try { platform = runtime.resolveNiva(niva).bootstrap.os.platform; } catch (_) { platform = root.process && root.process.platform || ""; }
        if (platform === "win32") return windowsUvErrorNames[Math.abs(code)] || "Unknown system error " + code;
        var names = platform === "darwin" ? darwinErrnoNames : linuxErrnoNames;
        return commonErrnoNames[Math.abs(code)] || names[Math.abs(code)] || "Unknown system error " + code;
    }
    function abortError(signal) {
        var error = new Error("The operation was aborted");
        error.name = "AbortError";
        error.code = "ABORT_ERR";
        if (signal && signal.reason !== undefined) error.cause = signal.reason;
        return error;
    }
    function createWeakChildHandlers(childRef, Buffer, sendSignal, pendingKill) {
        return {
            onChunk: function (bytes, isStderr) {
                var child = childRef.deref();
                if (child && bytes.length) (isStderr ? child.stderr : child.stdout).push(Buffer.from(bytes));
            },
            onEvent: function (name, data) {
                var child = childRef.deref();
                if (!child) return;
                if (name === "spawn") {
                    child.pid = data.pid;
                    if (pendingKill.value !== undefined) sendSignal(pendingKill.value);
                    child.emit("spawn");
                } else child.emit(name, data);
            },
        };
    }
    function createChildProcessModule(niva?) {
        function textOptions(options) {
            if (options === undefined) options = {};
            if (options === null || typeof options !== "object" || Array.isArray(options)) throw runtime.bridgeError("options must be an object", "ERR_INVALID_ARG_TYPE");
            var allowed = ["cwd", "env", "timeoutMs", "maxOutputBytes"];
            var unsupported = Object.keys(options).find(function (name) { return !allowed.includes(name); });
            if (unsupported) throw runtime.bridgeError("process text options do not support " + unsupported, "ERR_NIVA_IPC_OPTION_UNSUPPORTED");
            var timeoutMs = options.timeoutMs;
            var maxOutputBytes = options.maxOutputBytes;
            if (timeoutMs !== undefined && (!Number.isInteger(timeoutMs) || timeoutMs < 1 || timeoutMs > 30000))
                throw runtime.bridgeError("timeoutMs must be between 1 and 30000", "ERR_OUT_OF_RANGE");
            if (maxOutputBytes !== undefined && (!Number.isInteger(maxOutputBytes) || maxOutputBytes < 1 || maxOutputBytes > 65536))
                throw runtime.bridgeError("maxOutputBytes must be between 1 and 65536", "ERR_OUT_OF_RANGE");
            var result: Record<string, any> = {};
            ["cwd", "env"].forEach(function (name) { if (options[name] !== undefined) result[name] = options[name]; });
            if (timeoutMs !== undefined) result.timeoutMs = timeoutMs;
            if (maxOutputBytes !== undefined) result.maxOutputBytes = maxOutputBytes;
            return result;
        }
        function textResult(method, args) {
            return runtime.call(niva, method, args).then(function (result: any) {
                if (!result || typeof result.stdout !== "string" || typeof result.stderr !== "string" || !(result.status === null || typeof result.status === "number"))
                    throw runtime.bridgeError("Invalid Native text process response", "ERR_NIVA_IPC_RESPONSE");
                return { stdout: result.stdout, stderr: result.stderr, status: result.status, ...(result.signal === undefined ? {} : { signal: result.signal }) };
            }, function (error) { throw runtime.nativeError(error); });
        }
        function execText(command, options) {
            if (typeof command !== "string" || !command) return Promise.reject(runtime.bridgeError("command must be a non-empty string", "ERR_INVALID_ARG_TYPE"));
            return textResult("process.execText", [command, textOptions(options)]);
        }
        function execFileText(file, args, options) {
            if (typeof args === "function") return Promise.reject(runtime.bridgeError("execFileText does not support callbacks", "ERR_INVALID_ARG_TYPE"));
            if (!Array.isArray(args)) { options = args || options; args = []; }
            if (typeof file !== "string" || !file) return Promise.reject(runtime.bridgeError("file must be a non-empty string", "ERR_INVALID_ARG_TYPE"));
            if (args.some(function (arg) { return typeof arg !== "string" || arg.includes("\0"); })) return Promise.reject(runtime.bridgeError("args must be strings without null bytes", "ERR_INVALID_ARG_VALUE"));
            return textResult("process.execFileText", [file, args, textOptions(options)]);
        }
        function spawn(command, args, options) {
            if (typeof command !== "string" || !command)
                throw new TypeError("command must be a non-empty string");
            if (!Array.isArray(args)) {
                options = args || {};
                args = [];
            }
            options = options || {};
            checkStdio(options.stdio);
            if (options.signal !== undefined && (options.signal === null || typeof options.signal.aborted !== "boolean" || typeof options.signal.addEventListener !== "function" || typeof options.signal.removeEventListener !== "function"))
                throw Object.assign(new TypeError('The "options.signal" property must be an instance of AbortSignal'), {code: "ERR_INVALID_ARG_TYPE"});
            args = args.map(String);
            if (args.some(arg => arg.includes("\0"))) throw Object.assign(new TypeError("args must not contain null bytes"), {code: "ERR_INVALID_ARG_VALUE"});
            var invocation = options.shell ? shellInvocation([command].concat(args).join(" "), options.shell) : { command: command, args: args };
            var preAborted = !!(options.signal && options.signal.aborted);
            var child = new runtime.events.EventEmitter();
            Object.assign(child, { pid: undefined, killed: false, connected: false, exitCode: null, signalCode: null, spawnfile: invocation.command, spawnargs: [invocation.command].concat(invocation.args) });
            var call, owner, pendingKill = { value: undefined }, settled = false, callCancelled = false, resourceInvalidated = false, timer, signalListener;
            var childRef = typeof WeakRef === "function" ? new WeakRef(child) : null;
            var childFallback = childRef ? undefined : child;
            child.stdout = new Readable({ read: function () { } });
            child.stderr = new Readable({ read: function () { } });
            child.stdin = new Writable({ write: function (chunk, encoding, callback) { try {
                    if (!call || call.id === undefined) { callback(); return; }
                    if (!runtime.streamSend(niva, call.id, chunk, false))
                        throw runtime.bridgeError("Child input bridge is full", "ENOBUFS");
                    callback();
                }
                catch (error) {
                    callback(error);
                } }, final: function (callback) { try {
                    if (!call || call.id === undefined) { callback(); return; }
                    runtime.streamSend(niva, call.id, new Uint8Array(0), true);
                    callback();
                }
                catch (error) {
                    callback(error);
                } } });
            child.stdio = [child.stdin, child.stdout, child.stderr];
            function sendSignal(signal) {
                if (!call || call.id === undefined) return;
                runtime.streamRelated(niva, call, "process.signal", [call.id, String(signal)]).promise.catch(function (error) {
                    var target = childRef ? childRef.deref() : childFallback;
                    if (target && !target._abortError) target.emit("error", runtime.nativeError(error));
                });
            }
            child.kill = function (signal) { signal = signal === undefined ? "SIGTERM" : signal; if (settled)
                return false; if (!["SIGTERM", "SIGKILL", "SIGINT", "SIGHUP", 0, "0"].includes(signal))
                throw new TypeError("Unsupported signal"); if (!call || call.id === undefined)
                return false; child.killed = signal !== 0 && signal !== "0"; if (child.pid)
                sendSignal(signal);
            else
                pendingKill.value = signal; return true; };
            function cancelCall() { if (settled || callCancelled) return false; callCancelled = true; return call.cancel(); }
            child.cancel = function () { return cancelCall(); };
            child.ref = child.unref = function () { return child; };
            function cleanup(target) { if (settled) return; settled = true; if (timer)
                clearTimeout(timer); if (options.signal && signalListener)
                options.signal.removeEventListener("abort", signalListener); target.stdout.push(null); target.stderr.push(null); if (owner) owner.release(); }
            function complete(target, result) {
                cleanup(target);
                if (typeof result === "number") {
                    target.pid = result;
                    return { pid: result, status: null, detached: true };
                }
                target.exitCode = result.status;
                target.signalCode = ({ 1: "SIGHUP", 2: "SIGINT", 9: "SIGKILL", 15: "SIGTERM" })[result.signal] || null;
                target.emit("exit", target.exitCode, target.signalCode);
                if (target._abortError) {
                    try { target.emit("error", target._abortError); } finally { target.emit("close", target.exitCode, target.signalCode); }
                    throw target._abortError;
                }
                target.emit("close", target.exitCode, target.signalCode);
                return { status: target.exitCode, signal: target.signalCode };
            }
            function fail(target, error) {
                cleanup(target);
                error = target._abortError || runtime.nativeError(error);
                try { target.emit("error", error); } finally { target.emit("close", null, null); }
                throw error;
            }
            if (preAborted) {
                child._abortError = abortError(options.signal);
                call = { id: undefined, promise: Promise.reject(child._abortError), cancel: function () { return false; } };
            }
            else {
                try {
                    call = runtime.stream(niva, "process.execStream", [invocation.command, invocation.args, { currentDir: options.cwd, env: options.env, detached: !!options.detached }], {
                        ...(childRef ? createWeakChildHandlers(childRef, Buffer, sendSignal, pendingKill) : {
                            onChunk: function (bytes, isStderr) { if (bytes.length) (isStderr ? child.stderr : child.stdout).push(Buffer.from(bytes)); },
                            onEvent: function (name, data) { if (name === "spawn") { child.pid = data.pid; if (pendingKill.value !== undefined) sendSignal(pendingKill.value); child.emit("spawn"); } else child.emit(name, data); },
                        }),
                    });
                }
                catch (error) {
                    call = { id: undefined, promise: Promise.reject(error), cancel: function () { return false; } };
                }
            }
            if (call.id !== undefined && typeof runtime.registerResource === "function") {
                owner = runtime.registerResource(child, function () { return call.cancel(); }, call.id);
                child.__nivaInvalidate = function (error) {
                    if (settled || resourceInvalidated) return;
                    resourceInvalidated = true;
                    this._abortError = this._abortError || error;
                    this.stdin.destroy(error);
                    cancelCall();
                };
            }
            if (childRef) {
                child.completion = call.promise.then(function (result) { var target = childRef.deref(); return target ? complete(target, result) : result; }, function (error) { var target = childRef.deref(); if (!target) throw error; return fail(target, error); });
            } else {
                child.completion = call.promise.then(function (result) { return complete(child, result); }, function (error) { return fail(child, error); });
            }
            child.completion.catch(function () { });
            if (options.timeout > 0)
                timer = childRef ? setTimeout(function () { var target = childRef.deref(); if (target) target.kill(options.killSignal); }, options.timeout) : setTimeout(() => child.kill(options.killSignal), options.timeout);
            if (options.signal && !preAborted) {
                signalListener = childRef ? function () { var target = childRef.deref(); if (target && !settled) { target._abortError = target._abortError || abortError(options.signal); target.kill(options.killSignal); } } : function () { if (!settled) { child._abortError = child._abortError || abortError(options.signal); child.kill(options.killSignal); } };
                options.signal.addEventListener("abort", signalListener, { once: true });
            }
            return child;
        }
        function execFile(file, args, options, callback) {
            if (typeof args === "function") {
                callback = args;
                args = [];
                options = {};
            }
            else if (!Array.isArray(args)) {
                callback = options;
                options = args;
                args = [];
            }
            if (typeof options === "function") {
                callback = options;
                options = {};
            }
            options = options || {};
            var child = spawn(file, args, options), out = [], err = [], outSize = 0, errSize = 0, overflow = false, max = options.maxBuffer === undefined ? 1024 * 1024 : options.maxBuffer;
            var command = [file].concat(args || []).join(" "), closeInfo, childError;
            function collect(target, isErr) { return function (chunk) { if (overflow)
                return; var size = isErr ? (errSize += chunk.length) : (outSize += chunk.length); if (size > max) {
                overflow = true;
                child.kill();
                return;
            } target.push(Buffer.from(chunk)); }; }
            child.stdout.on("data", collect(out, false));
            child.stderr.on("data", collect(err, true));
            var closed: Promise<any> = new Promise(resolve => child.once("close", function (code, signal) {
                closeInfo = { status: code, signal: signal };
                resolve(closeInfo);
            }));
            var drained = Promise.all([child.stdout, child.stderr].map(stream => new Promise(resolve => stream.once("end", resolve))));
            child.on("error", function (error) { childError = error; });
            child.stdin.end();
            function formattedOutput() {
                var stdout = Buffer.concat(out), stderr = Buffer.concat(err);
                if (options.encoding !== null && options.encoding !== "buffer") {
                    stdout = stdout.toString(options.encoding || "utf8");
                    stderr = stderr.toString(options.encoding || "utf8");
                }
                return { stdout: stdout, stderr: stderr };
            }
            function completionError(exit, output) {
                var status = exit.status;
                var code = typeof status === "number" && status < 0 ? systemErrorName(status, niva) : status;
                var error = runtime.bridgeError("Command failed: " + command, overflow ? "ERR_CHILD_PROCESS_STDIO_MAXBUFFER" : code);
                error.stdout = output.stdout;
                error.stderr = output.stderr;
                error.status = status;
                error.killed = child.killed;
                error.signal = exit.signal;
                error.cmd = command;
                return error;
            }
            function attachOutput(error, output) {
                if (error.stdout === undefined) error.stdout = output.stdout;
                if (error.stderr === undefined) error.stderr = output.stderr;
                if (error.killed === undefined) error.killed = child.killed;
                if (error.cmd === undefined) error.cmd = command;
                if (error.name !== "AbortError" && error.signal === undefined && closeInfo) error.signal = closeInfo.signal;
                return error;
            }
            child.result = closed.then(async function (exit) {
                await drained;
                var output = formattedOutput();
                if (child._abortError) throw attachOutput(child._abortError, output);
                if (childError) throw attachOutput(runtime.nativeError(childError), output);
                if (overflow || exit.status !== 0 || exit.signal) throw completionError(exit, output);
                return output;
            });
            child.result.catch(function () { });
            if (typeof callback === "function")
                child.result.then(value => callback(null, value.stdout, value.stderr), error => callback(error, error.stdout, error.stderr));
            return child;
        }
        function exec(command, options, callback) { if (typeof options === "function") {
            callback = options;
            options = {};
        } options = options || {}; var invocation = shellInvocation(command, options.shell); return execFile(invocation.command, invocation.args, Object.assign({}, options, { shell: false }), callback); }
        function spawnSyncWithApiName(command, args, options, apiName) {
            if (!Array.isArray(args)) {
                options = args || {};
                args = [];
            }
            options = options || {};
            checkStdio(options.stdio);
            var invocation = options.shell ? shellInvocation(command + (args.length ? " " + args.join(" ") : ""), options.shell) : { command: command, args: args };
            var payload = Object.assign({}, options);
            if (options.input !== undefined)
                payload.input = runtime.vendor.Buffer.from(options.input, options.encoding || "utf8").toString("base64");
            var result;
            try {
                result = runtime.callSyncAs(niva, apiName, "process.spawnSync", [invocation.command, invocation.args, payload]);
            }
            catch (error) {
                return { pid: 0, status: null, signal: null, output: null, stdout: null, stderr: null, error: runtime.nativeError(error) };
            }
            var stdout = runtime.vendor.Buffer.from(result.stdout, "base64"), stderr = runtime.vendor.Buffer.from(result.stderr, "base64");
            if (options.encoding && options.encoding !== "buffer") {
                stdout = stdout.toString(options.encoding);
                stderr = stderr.toString(options.encoding);
            }
            result.stdout = stdout;
            result.stderr = stderr;
            result.output = [null, stdout, stderr];
            if (result.signal)
                result.signal = ({ 2: "SIGINT", 9: "SIGKILL", 15: "SIGTERM" })[result.signal] || "SIG" + result.signal;
            if (result.error)
                result.error = runtime.bridgeError("spawnSync " + result.error, result.error);
            else
                delete result.error;
            return result;
        }
        function spawnSync(command, args, options) {
            return spawnSyncWithApiName(command, args, options, "spawnSync");
        }
        function execFileSyncWithApiName(file, args, options, apiName) {
            if (!Array.isArray(args)) {
                options = args || {};
                args = [];
            }
            var result = spawnSyncWithApiName(file, args, options, apiName);
            if (result.error || result.status !== 0)
                throw Object.assign(result.error || new Error("Command failed: " + file), result);
            return result.stdout;
        }
        function execFileSync(file, args, options) {
            return execFileSyncWithApiName(file, args, options, "execFileSync");
        }
        function execSync(command, options) { return execFileSyncWithApiName(command, [], Object.assign({}, options, { shell: options && options.shell || true }), "execSync"); }
        return { spawn, exec, execFile, spawnSync, execFileSync, execSync, execText, execFileText };
    }
    runtime.createChildProcessModule = createChildProcessModule;
    runtime.child_process = createChildProcessModule();
})(globalThis);
