import "./runtime/bridge.js";
import "./runtime/vendor.js";
import "./runtime/buffer.js";
import "./runtime/events.js";
import "./runtime/dgram.js";

const dgram = globalThis[Symbol.for("niva.node-compat.runtime")].dgram;
export const Socket = dgram.Socket;
export const createSocket = dgram.createSocket;
export default dgram;
