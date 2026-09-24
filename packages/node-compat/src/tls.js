import "./runtime/bridge.js";
import "./runtime/vendor.js";
import "./runtime/buffer.js";
import "./runtime/events.js";
import "./runtime/stream.js";
import "./runtime/net.js";
import "./runtime/tls.js";

const tls = globalThis[Symbol.for("niva.node-compat.runtime")].tls;
export const TLSSocket = tls.TLSSocket;
export const Server = tls.Server;
export const connect = tls.connect;
export const createServer = tls.createServer;
export default tls;
