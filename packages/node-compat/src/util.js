import "./runtime/bridge.js";
import "./runtime/vendor.js";
import "./runtime/buffer.js";
import "./runtime/util.js";

const util = globalThis[Symbol.for("niva.node-compat.runtime")].util;

export const format = util.format;
export const formatWithOptions = util.formatWithOptions;
export const inspect = util.inspect;
export const promisify = util.promisify;
export const callbackify = util.callbackify;
export const isDeepStrictEqual = util.isDeepStrictEqual;
export const deprecate = util.deprecate;
export default util;
