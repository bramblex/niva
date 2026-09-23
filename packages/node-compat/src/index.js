import "./runtime/bridge.js";
import "./runtime/path.js";
import "./runtime/events.js";
import "./runtime/querystring.js";
import "./runtime/buffer.js";
import "./runtime/util.js";
import "./runtime/os.js";
import "./runtime/fs.js";
import "./runtime/child_process.js";
import "./runtime/url.js";
import "./runtime/crypto.js";
import "./runtime/zlib.js";
import "./runtime/assert.js";
import "./runtime/http.js";
import "./runtime/stream.js";
import "./runtime/registration.js";

export { default as path } from "./path.js";
export { default as os } from "./os.js";
export { default as fs } from "./fs.js";
export { default as child_process } from "./child_process.js";
export { default as events } from "./events.js";
export { default as util } from "./util.js";
export { default as querystring } from "./querystring.js";
export { default as buffer } from "./buffer.js";
export { default as url } from "./url.js";
export { default as crypto } from "./crypto.js";
export { default as zlib } from "./zlib.js";
export { default as http } from "./http.js";
export { default as https } from "./https.js";
export { default as assert } from "./assert.js";
export { default as stream } from "./stream.js";

export function registerNodeCompat(niva = globalThis.Niva) {
  return globalThis[Symbol.for("niva.node-compat.runtime")].registerNodeCompat(niva);
}
