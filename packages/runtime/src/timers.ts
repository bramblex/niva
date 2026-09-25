
const timers = globalThis.Niva.timers;
export const setTimeout = timers.setTimeout;
export const clearTimeout = timers.clearTimeout;
export const setInterval = timers.setInterval;
export const clearInterval = timers.clearInterval;
export const setImmediate = timers.setImmediate;
export const clearImmediate = timers.clearImmediate;
export const Timeout = timers.Timeout;
export const Immediate = timers.Immediate;
export default timers;
