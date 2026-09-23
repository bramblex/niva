/**
 * Browser module wrappers and runtime files needed by each selected builtin.
 * Keep this list aligned with packages/node-compat/src and crates/niva's
 * NodeCompat module allowlist. Paths are relative to __niva_compat/.
 */
export const NODE_COMPAT_MODULE_FILES = Object.freeze({
  path: ["src/path.js", "src/runtime/bridge.js", "src/runtime/path.js"],
  os: ["src/os.js", "src/runtime/bridge.js", "src/runtime/path.js", "src/runtime/os.js"],
  fs: ["src/fs.js", "src/fs-promises.js", "src/runtime/bridge.js", "src/runtime/path.js", "src/runtime/buffer.js", "src/runtime/fs.js"],
  child_process: ["src/child_process.js", "src/runtime/bridge.js", "src/runtime/path.js", "src/runtime/buffer.js", "src/runtime/child_process.js"],
  events: ["src/events.js", "src/runtime/bridge.js", "src/runtime/events.js"],
  util: ["src/util.js", "src/runtime/bridge.js", "src/runtime/buffer.js", "src/runtime/util.js"],
  querystring: ["src/querystring.js", "src/runtime/bridge.js", "src/runtime/querystring.js"],
  buffer: ["src/buffer.js", "src/runtime/bridge.js", "src/runtime/buffer.js"],
  url: ["src/url.js", "src/runtime/bridge.js", "src/runtime/path.js", "src/runtime/url.js"],
  crypto: ["src/crypto.js", "src/runtime/bridge.js", "src/runtime/buffer.js", "src/runtime/crypto.js"],
  zlib: ["src/zlib.js", "src/runtime/bridge.js", "src/runtime/buffer.js", "src/runtime/zlib.js"],
  http: ["src/http.js", "src/runtime/bridge.js", "src/runtime/path.js", "src/runtime/buffer.js", "src/runtime/http.js"],
  https: ["src/https.js", "src/runtime/bridge.js", "src/runtime/path.js", "src/runtime/buffer.js", "src/runtime/http.js"],
  assert: ["src/assert.js", "src/assert-strict.js", "src/runtime/bridge.js", "src/runtime/buffer.js", "src/runtime/util.js", "src/runtime/assert.js"],
  stream: ["src/stream.js", "src/stream-promises.js", "src/runtime/bridge.js", "src/runtime/stream.js"],
});

const AVAILABLE_MODULES = Object.keys(NODE_COMPAT_MODULE_FILES);

/**
 * Resolve a project's nodeCompat configuration to the exact files to package.
 * The classic script and selected ESM wrappers with their runtime import
 * closure are included whenever enabled. `importmap` controls server-side
 * static map injection, not the assets used by Niva.import().
 *
 * @param {boolean | {modules?: string[], importmap?: boolean} | null | undefined} option
 */
export function resolveNodeCompatAssets(option) {
  if (option == null || option === false) {
    return { enabled: false, modules: [], importmap: false, files: [] };
  }

  if (option !== true && (typeof option !== "object" || Array.isArray(option))) {
    throw new TypeError("nodeCompat must be a boolean or an object");
  }

  const configuredModules = option === true ? undefined : option.modules;
  if (configuredModules !== undefined && !Array.isArray(configuredModules)) {
    throw new TypeError("nodeCompat.modules must be an array of module names");
  }

  const modules = configuredModules === undefined
    ? AVAILABLE_MODULES
    : [...new Set(configuredModules)];

  for (const moduleName of modules) {
    if (
      typeof moduleName !== "string" ||
      !Object.prototype.hasOwnProperty.call(NODE_COMPAT_MODULE_FILES, moduleName)
    ) {
      throw new TypeError(`Unknown nodeCompat module: ${String(moduleName)}`);
    }
  }

  const importmap = option === true || option.importmap === undefined
    ? true
    : option.importmap;
  if (typeof importmap !== "boolean") {
    throw new TypeError("nodeCompat.importmap must be a boolean");
  }

  const files = new Set(["node-compat.js"]);
  for (const moduleName of modules) {
    for (const file of NODE_COMPAT_MODULE_FILES[moduleName]) files.add(file);
  }

  return {
    enabled: true,
    modules: [...modules],
    importmap,
    files: ["node-compat.js", ...[...files].filter((file) => file !== "node-compat.js").sort()],
  };
}
