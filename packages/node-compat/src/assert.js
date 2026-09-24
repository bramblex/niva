import "./runtime/bridge.js";
import "./runtime/vendor.js";
import "./runtime/buffer.js";
import "./runtime/util.js";
import "./runtime/source.js";
import "./runtime/assert.js";

const assert = globalThis[Symbol.for("niva.node-compat.runtime")].assert;

export const AssertionError = assert.AssertionError;
export const fail = assert.fail;
export const ok = assert.ok;
export const equal = assert.equal;
export const notEqual = assert.notEqual;
export const strictEqual = assert.strictEqual;
export const notStrictEqual = assert.notStrictEqual;
export const deepEqual = assert.deepEqual;
export const notDeepEqual = assert.notDeepEqual;
export const deepStrictEqual = assert.deepStrictEqual;
export const notDeepStrictEqual = assert.notDeepStrictEqual;
export const throws = assert.throws;
export const doesNotThrow = assert.doesNotThrow;
export const rejects = assert.rejects;
export const doesNotReject = assert.doesNotReject;
export const ifError = assert.ifError;
export const match = assert.match;
export const doesNotMatch = assert.doesNotMatch;
export const strict = assert.strict;
export default assert;
