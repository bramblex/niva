import "./runtime/bridge.js";
import "./runtime/vendor.js";
import "./runtime/buffer.js";
import "./runtime/crypto.js";

const crypto = globalThis[Symbol.for("niva.node-compat.runtime")].crypto;

export const randomUUID = crypto.randomUUID;
export const randomBytes = crypto.randomBytes;
export const createHash = crypto.createHash;
export const createHmac = crypto.createHmac;
export const pbkdf2 = crypto.pbkdf2;
export const pbkdf2Sync = crypto.pbkdf2Sync;
export const scrypt = crypto.scrypt;
export const timingSafeEqual = crypto.timingSafeEqual;
export const getHashes = crypto.getHashes;
export const createSecretKey = crypto.createSecretKey;
export const KeyObject = crypto.KeyObject;
export default crypto;
