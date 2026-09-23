import "./runtime/bridge.js";
import "./runtime/querystring.js";

const querystring = globalThis[Symbol.for("niva.node-compat.runtime")].querystring;

export const parse = querystring.parse;
export const decode = querystring.decode;
export const stringify = querystring.stringify;
export const encode = querystring.encode;
export const escape = querystring.escape;
export const unescape = querystring.unescape;
export default querystring;
