import "./runtime/bridge.js";
import "./runtime/path.js";
import "./runtime/buffer.js";
import "./runtime/http.js";

const http = globalThis[Symbol.for("niva.node-compat.runtime")].http;

export const request = http.request;
export const get = http.get;
export const post = http.post;
export default http;
