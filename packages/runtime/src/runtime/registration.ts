(function (root: any) {
  "use strict";
  const runtime = root[Symbol.for("niva.node-compat.runtime")];
  if (typeof runtime.registerNodeCompat === "function") return;

  function createModules(niva: any) {
    const modules: any = {
      cluster: createClusterModule(niva),
      path: runtime.createPathModule ? runtime.createPathModule() : runtime.path,
      events: runtime.createEventsModule ? runtime.createEventsModule() : runtime.events,
      util: runtime.createUtilModule ? runtime.createUtilModule() : runtime.util,
      querystring: runtime.createQuerystringModule ? runtime.createQuerystringModule() : runtime.querystring,
      buffer: runtime.createBufferModule ? runtime.createBufferModule() : runtime.buffer,
      os: runtime.createOsModule ? runtime.createOsModule(niva) : runtime.os,
      fs: runtime.createFsModule ? runtime.createFsModule(niva) : runtime.fs,
      child_process: runtime.createChildProcessModule ? runtime.createChildProcessModule(niva) : runtime.child_process,
      url: runtime.createUrlModule ? runtime.createUrlModule() : runtime.url,
      crypto: runtime.createCryptoModule ? runtime.createCryptoModule() : runtime.crypto,
      http: runtime.createHttpModule ? runtime.createHttpModule("http", niva) : runtime.http,
      https: runtime.createHttpModule ? runtime.createHttpModule("https", niva) : runtime.https,
      assert: runtime.createAssertModule ? runtime.createAssertModule() : runtime.assert,
      stream: runtime.createStreamModule ? runtime.createStreamModule() : runtime.streamModule,
      process: runtime.createProcessModule ? runtime.createProcessModule(niva) : runtime.process,
      tty: runtime.createTtyModule ? runtime.createTtyModule(niva) : runtime.tty,
      string_decoder: runtime.createStringDecoderModule ? runtime.createStringDecoderModule() : runtime.stringDecoder,
      timers: runtime.createTimersModule ? runtime.createTimersModule() : runtime.timers,
      net: runtime.createNetModule ? runtime.createNetModule(niva) : runtime.net,
      tls: runtime.createTlsModule ? runtime.createTlsModule(niva) : runtime.tls,
      dgram: runtime.createDgramModule ? runtime.createDgramModule(niva) : runtime.dgram,
      dns: runtime.createDnsModule ? runtime.createDnsModule(niva) : runtime.dns,
    };
    // zlib snapshots Buffer.kMaxLength when it is first loaded. Keep this
    // property lazy so `require('buffer').kMaxLength = ...` before the first
    // `require('zlib')` has the same effect as Node, while every entry point
    // still observes one cached product module.
    Object.defineProperty(modules, "zlib", {
      configurable: false,
      enumerable: true,
      get() {
        return runtime.createZlibModule ? runtime.createZlibModule() : runtime.zlib;
      },
    });
    // Node's legacy `constants` builtin aliases filesystem constants. Keep the
    // same object so `require('constants')` and `require('fs').constants` stay
    // in sync, including when the adapter is created for an isolated Niva.
    modules.constants = modules.fs?.constants;
    return modules;
  }

  function createClusterModule(niva: any) {
    // Niva runs each backend in one main process. Model Node's primary-process
    // state for libraries that only inspect cluster.worker/isMaster (for
    // example shortid), and fail explicitly if code asks Niva to fork workers.
    const cluster: any = new runtime.events.EventEmitter();
    const unsupported = function () {
      throw runtime.bridgeError("Node cluster workers are unavailable in Niva", "ERR_NIVA_CLUSTER_UNAVAILABLE");
    };
    Object.assign(cluster, {
      isPrimary: true,
      isMaster: true,
      isWorker: false,
      worker: undefined,
      workers: {},
      schedulingPolicy: 2,
      SCHED_NONE: 1,
      SCHED_RR: 2,
      settings: {},
      setupPrimary: unsupported,
      setupMaster: unsupported,
      fork: unsupported,
      disconnect(callback?: Function) {
        if (callback !== undefined) {
          if (typeof callback !== "function") throw new TypeError("callback must be a function");
          queueMicrotask(() => callback());
        }
        return cluster;
      },
    });
    return cluster;
  }

  function registerNodeCompat(niva: any) {
    const target = runtime.resolveNiva(niva);
    if (typeof target.__registerModule !== "function") throw new Error("Niva module registry is unavailable");
    if (typeof target.bridgeVersion !== "number" || target.bridgeVersion < 1) throw new Error("@niva/runtime requires bridge version 1 or newer");
    const modules = createModules(target);
    for (const name of Object.keys(modules)) {
      if (name === "zlib" && typeof target.__registerModuleFactory === "function") {
        Object.defineProperty(target, name, {
          configurable: true,
          enumerable: true,
          get() { return modules.zlib; },
        });
        target.__registerModuleFactory(name, () => modules.zlib);
        target.__registerModuleFactory("node:" + name, () => modules.zlib);
        continue;
      }
      const value = modules[name];
      if (value === undefined) continue;
      target[name] = value;
      target.__registerModule(name, value);
      target.__registerModule("node:" + name, value);
    }
    const aliases: Record<string, any> = {
      "fs/promises": modules.fs?.promises,
      "assert/strict": modules.assert?.strict,
      "stream/promises": modules.stream?.promises,
      "dns/promises": modules.dns?.promises,
      "timers/promises": modules.timers?.promises,
    };
    for (const [name, value] of Object.entries(aliases)) {
      if (value === undefined) continue;
      target.__registerModule(name, value);
      target.__registerModule("node:" + name, value);
    }
    return modules;
  }

  runtime.createModules = createModules;
  runtime.registerNodeCompat = registerNodeCompat;
})(globalThis as any);
