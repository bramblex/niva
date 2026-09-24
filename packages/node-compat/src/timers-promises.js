import "./runtime/bridge.js";
import "./runtime/timers.js";

const promises = globalThis[Symbol.for("niva.node-compat.runtime")].timers.promises;
export const setTimeout = promises.setTimeout;
export const setImmediate = promises.setImmediate;
export const setInterval = promises.setInterval;
export const scheduler = promises.scheduler;
export default promises;
