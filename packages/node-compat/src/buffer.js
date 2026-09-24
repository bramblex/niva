import "./runtime/bridge.js";
import "./runtime/vendor.js";
import "./runtime/buffer.js";

const buffer = globalThis[Symbol.for("niva.node-compat.runtime")].buffer;

export const Buffer = buffer.Buffer;
export const SlowBuffer = buffer.SlowBuffer;
export const INSPECT_MAX_BYTES = buffer.INSPECT_MAX_BYTES;
export const kMaxLength = buffer.kMaxLength;
export default buffer;
