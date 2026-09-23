import "./runtime/bridge.js";
import "./runtime/buffer.js";
import "./runtime/zlib.js";

const zlib = globalThis[Symbol.for("niva.node-compat.runtime")].zlib;

export const gzip = zlib.gzip;
export const gunzip = zlib.gunzip;
export default zlib;
