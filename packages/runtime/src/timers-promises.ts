
const promises = globalThis.Niva.timers.promises;
export const setTimeout = promises.setTimeout;
export const setImmediate = promises.setImmediate;
export const setInterval = promises.setInterval;
export const scheduler = promises.scheduler;
export default promises;
