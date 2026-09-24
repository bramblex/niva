(function (root) {
    "use strict";
    var runtime = root[Symbol.for("niva.node-compat.runtime")];
  if (typeof runtime.registerNodeCompat === "function") return;
    function createModules(niva) {
        if (niva === root.Niva) {
            return {
                path: runtime.path,
                events: runtime.events,
                util: runtime.util,
                querystring: runtime.querystring,
                buffer: runtime.buffer,
                os: runtime.os,
                fs: runtime.fs,
                child_process: runtime.child_process,
                url: runtime.url,
                crypto: runtime.crypto,
                get zlib() { return runtime.zlib; },
                http: runtime.http,
                https: runtime.https,
                assert: runtime.assert,
                stream: runtime.streamModule,
                process: runtime.process,
                string_decoder: runtime.stringDecoder,
                timers: runtime.timers,
                net: runtime.net,
                tls: runtime.tls,
                dgram: runtime.dgram,
                dns: runtime.dns,
            };
        }
        var path = runtime.createPathModule();
        return {
            path: path,
            events: runtime.createEventsModule(),
            util: runtime.createUtilModule(),
            querystring: runtime.createQuerystringModule(),
            buffer: runtime.createBufferModule(),
            os: runtime.createOsModule(niva),
            fs: runtime.createFsModule(niva),
            child_process: runtime.createChildProcessModule(niva),
            url: runtime.createUrlModule(),
            crypto: runtime.createCryptoModule(),
            get zlib() { return runtime.createZlibModule(); },
            http: runtime.createHttpModule("http", niva),
            https: runtime.createHttpModule("https", niva),
            assert: runtime.createAssertModule(),
            stream: runtime.createStreamModule(),
            process: runtime.createProcessModule ? runtime.createProcessModule(niva) : undefined,
            string_decoder: runtime.createStringDecoderModule(),
            timers: runtime.createTimersModule(),
            net: runtime.createNetModule(niva),
            tls: runtime.createTlsModule(niva),
            dgram: runtime.createDgramModule(niva),
            dns: runtime.createDnsModule(niva),
        };
    }
    function registerNodeCompat(niva, selectedModules) {
        var target = runtime.resolveNiva(niva);
        if (typeof target.registerModule !== "function") {
            throw new Error("Niva.registerModule is unavailable; load initialize_script.js before node-compat");
        }
        if (typeof target.bridgeVersion !== "number" || target.bridgeVersion < 1) {
            throw new Error("node-compat requires Niva bridge version 1 or newer");
        }
        var modules = createModules(target);
        var moduleNames = ["path", "events", "util", "querystring", "buffer", "os", "fs", "child_process", "url", "crypto", "zlib", "http", "https", "assert", "stream", "process", "string_decoder", "timers", "net", "tls", "dgram", "dns"];
        var configured = selectedModules;
        if (configured === undefined && Array.isArray(root.__niva_node_compat_modules))
            configured = root.__niva_node_compat_modules;
        if (configured !== undefined && !Array.isArray(configured))
            throw new TypeError("node-compat modules must be an array of module names");
        var selected = configured === undefined ? moduleNames : moduleNames.filter(function (name) { return configured.indexOf(name) >= 0; });
        selected = selected.filter(function (name) { return name === "zlib" ? typeof runtime.createZlibModule === "function" : modules[name] !== undefined; });
        selected.forEach(function (name) {
            if (name === "zlib") {
                target.registerModuleFactory(name, runtime.createZlibModule);
                target.registerModuleFactory("node:" + name, runtime.createZlibModule);
                return;
            }
            target.registerModule(name, modules[name]);
            target.registerModule("node:" + name, modules[name]);
        });
        if (selected.indexOf("fs") >= 0) {
            target.registerModule("fs/promises", modules.fs.promises);
            target.registerModule("node:fs/promises", modules.fs.promises);
        }
        if (selected.indexOf("assert") >= 0) {
            target.registerModule("assert/strict", modules.assert.strict);
            target.registerModule("node:assert/strict", modules.assert.strict);
        }
        if (selected.indexOf("stream") >= 0) {
            target.registerModule("stream/promises", modules.stream.promises);
            target.registerModule("node:stream/promises", modules.stream.promises);
        }
        ["dns", "timers"].forEach(function (name) {
            if (selected.indexOf(name) >= 0 && modules[name].promises) {
                target.registerModule(name + "/promises", modules[name].promises);
                target.registerModule("node:" + name + "/promises", modules[name].promises);
            }
        });
        if (selected.indexOf("buffer") >= 0 && typeof root.Buffer === "undefined")
            root.Buffer = modules.buffer.Buffer;
        if (selected.indexOf("process") >= 0)
            root.process = modules.process;
        return Promise.resolve(modules);
    }
    runtime.createModules = createModules;
    runtime.registerNodeCompat = registerNodeCompat;
})(globalThis);
