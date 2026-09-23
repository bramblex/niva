import "./runtime/bridge.js";
import "./runtime/path.js";
import "./runtime/buffer.js";
import "./runtime/http.js";

const https = globalThis[Symbol.for("niva.node-compat.runtime")].https;

export const request = https.request;
export const get = https.get;
export const post = https.post;
export default https;
