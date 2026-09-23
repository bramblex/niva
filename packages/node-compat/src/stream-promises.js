import "./runtime/bridge.js";
import "./runtime/stream.js";

const promises = globalThis[Symbol.for("niva.node-compat.runtime")].streamModule.promises;

export const pipeline = promises.pipeline;
export default promises;
