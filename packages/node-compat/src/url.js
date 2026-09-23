import "./runtime/bridge.js";
import "./runtime/path.js";
import "./runtime/url.js";

const url = globalThis[Symbol.for("niva.node-compat.runtime")].url;

export const URL = url.URL;
export const URLSearchParams = url.URLSearchParams;
export const fileURLToPath = url.fileURLToPath;
export const pathToFileURL = url.pathToFileURL;
export default url;
