import "./runtime/bridge.js";
import "./runtime/vendor.js";
import "./runtime/stream.js";

const stream = globalThis[Symbol.for("niva.node-compat.runtime")].streamModule;

export const Stream = stream.Stream;
export const Readable = stream.Readable;
export const Writable = stream.Writable;
export const Duplex = stream.Duplex;
export const Transform = stream.Transform;
export const PassThrough = stream.PassThrough;
export const pipeline = stream.pipeline;
export const finished = stream.finished;
export const promises = stream.promises;
export const addAbortSignal = stream.addAbortSignal;
export const compose = stream.compose;
export const destroy = stream.destroy;
export const from = stream.from;
export const fromWeb = stream.fromWeb;
export const toWeb = stream.toWeb;
export const wrap = stream.wrap;
export const isDisturbed = stream.isDisturbed;
export const isErrored = stream.isErrored;
export const isReadable = stream.isReadable;
export default stream;
