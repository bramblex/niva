(function (root) {
    "use strict";
    var runtime = root[Symbol.for("niva.node-compat.runtime")];
  if (typeof runtime.createOsModule === "function") return;
    function createOsModule(niva) {
        function boot() { var target = runtime.resolveNiva(niva); if (!target.bootstrap || !target.bootstrap.os)
            throw runtime.bridgeError("OS startup metadata unavailable", "ENOTSUP"); return target.bootstrap.os; }
        var module = {};
        ["arch", "platform", "homedir", "tmpdir", "hostname", "release", "totalmem", "type", "version"].forEach(function (name) { module[name] = function () { var value = boot()[name]; if (value === undefined)
            throw runtime.bridgeError("OS information unavailable: " + name, "ENOTSUP"); return value; }; });
        ["cpus", "freemem", "networkInterfaces", "uptime"].forEach(function (name) { module[name] = function () { return runtime.callSync(niva, "os." + name, []); }; });
        module.userInfo = function (opts) { var value = boot().userInfo; if (!value)
            throw runtime.bridgeError("User information unavailable", "ENOTSUP"); value = Object.assign({}, value); if (opts && opts.encoding === "buffer")
            ["username", "homedir", "shell"].forEach(function (name) { if (value[name] !== null)
                value[name] = runtime.vendor.Buffer.from(value[name]); }); return value; };
        Object.defineProperties(module, {
            EOL: { configurable: true, enumerable: true, get: function () { return boot().EOL; } },
            devNull: { enumerable: true, get: function () { return boot().platform === "win32" ? "\\\\.\\NUL" : "/dev/null"; } },
        });
        return module;
    }
    runtime.createOsModule = createOsModule;
    runtime.os = createOsModule();
})(globalThis);
