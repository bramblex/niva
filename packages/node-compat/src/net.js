import "./runtime/bridge.js";
import "./runtime/vendor.js";
import "./runtime/buffer.js";
import "./runtime/events.js";
import "./runtime/stream.js";
import "./runtime/net.js";

const net = globalThis[Symbol.for("niva.node-compat.runtime")].net;
export const Socket = net.Socket;
export const Server = net.Server;
export const connect = net.connect;
export const createConnection = net.createConnection;
export const createServer = net.createServer;
export const isIP = net.isIP;
export const isIPv4 = net.isIPv4;
export const isIPv6 = net.isIPv6;
export default net;
