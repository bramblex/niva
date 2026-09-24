import "./runtime/bridge.js";
import "./runtime/events.js";

const events = globalThis[Symbol.for("niva.node-compat.runtime")].events;

export const EventEmitter = events;
export const once = events.once;
export const on = events.on;
export const defaultMaxListeners = events.defaultMaxListeners;
export const errorMonitor = events.errorMonitor;
export default EventEmitter;
