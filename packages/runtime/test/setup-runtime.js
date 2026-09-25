// Exercise the real bootstrap ordering: Native provides only __niva_* globals,
// so no Niva object exists while the runtime side-effect modules load.
globalThis.__niva_runtime_config = { injectCommonJs: false, injectEsm: false };
await import("../dist/bootstrap.js");
globalThis.Niva.path.setCwd(globalThis.process.cwd());
globalThis[Symbol.for("niva.node-compat.runtime")].path.setCwd(globalThis.process.cwd());
