import "./runtime/bridge.js";
import "./runtime/vendor.js";
import "./runtime/buffer.js";
import "./runtime/string_decoder.js";

const stringDecoder = globalThis[Symbol.for("niva.node-compat.runtime")].stringDecoder;
export const StringDecoder = stringDecoder.StringDecoder;
export default stringDecoder;
