(function (root) {
    "use strict";
    var runtime = root[Symbol.for("niva.node-compat.runtime")];
  if (typeof runtime.createProcessModule === "function") return;
    function createProcessModule(niva) {
        var target = niva || root.Niva;
        if (!target || !target.bootstrap || !target.bootstrap.process)
            return undefined;
        var metadata = target.bootstrap.process;
        var process = new runtime.events.EventEmitter();
        Object.assign(process, metadata, { env: Object.assign({}, metadata.env), argv: metadata.argv.slice() });
        process.cwd = function () { return runtime.callSync(target, "process.currentDir", []); };
        process.chdir = function (path) { if (typeof path !== "string") { var error = new TypeError('The "directory" argument must be of type string'); error.code = "ERR_INVALID_ARG_TYPE"; throw error; } return runtime.callSync(target, "process.setCurrentDir", [path]); };
        process.nextTick = function (callback) { if (typeof callback !== "function")
            throw new TypeError("callback must be a function"); var args = Array.prototype.slice.call(arguments, 1); queueMicrotask(function () { callback.apply(null, args); }); };
        process.exitCode = 0;
        process.exit = function (code) { process.exitCode = code === undefined ? process.exitCode : code; process.emit("exit", process.exitCode); runtime.call(target, "process.exit", [process.exitCode]).catch(function (error) { process.emit("error", runtime.nativeError(error)); }); };
        function output(channel) { return new runtime.vendor.stream.Writable({ write: function (chunk, encoding, callback) { runtime.call(target, "process.write", [channel, runtime.vendor.Buffer.from(chunk).toString("base64")]).then(function () { callback(); }, function (error) { callback(runtime.nativeError(error)); }); } }); }
        process.stdout = output("stdout");
        process.stderr = output("stderr");
        var inputCall;
        process.stdin = new runtime.vendor.stream.Readable({ read: function () { if (inputCall)
                return; inputCall = runtime.stream(target, "process.stdin", [], { onChunk: function (chunk) { process.stdin.push(runtime.vendor.Buffer.from(chunk)); } }); inputCall.promise.then(function () { process.stdin.push(null); }, function (error) { process.stdin.destroy(runtime.nativeError(error)); }); }, destroy: function (error, callback) { if (inputCall)
                inputCall.cancel(); callback(error); } });
        return process;
    }
    runtime.createProcessModule = createProcessModule;
    runtime.process = createProcessModule();
})(globalThis);
