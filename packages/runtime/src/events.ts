
const events = globalThis.Niva.events;

export const EventEmitter = events;
export const once = events.once;
export const on = events.on;
export const defaultMaxListeners = events.defaultMaxListeners;
export const errorMonitor = events.errorMonitor;
export default EventEmitter;
