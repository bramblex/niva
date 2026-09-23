import "./runtime/bridge.js";
import "./runtime/stream.js";

const stream = globalThis[Symbol.for("niva.node-compat.runtime")].streamModule;

export const pipeline = stream.pipeline;
export const promises = stream.promises;
export default stream;
