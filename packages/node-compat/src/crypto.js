import "./runtime/bridge.js";
import "./runtime/buffer.js";
import "./runtime/crypto.js";

const crypto = globalThis[Symbol.for("niva.node-compat.runtime")].crypto;

export const randomUUID = crypto.randomUUID;
export const randomBytes = crypto.randomBytes;
export const createHash = crypto.createHash;
export const getHashes = crypto.getHashes;
export default crypto;
