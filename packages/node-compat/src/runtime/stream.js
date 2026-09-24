(function (root) {
  "use strict";

  var runtime = root[Symbol.for("niva.node-compat.runtime")];
  if (typeof runtime.createStreamModule === "function") return;
  var vendor = runtime.vendor;
  var stream = vendor && vendor.stream;
  if (!stream || typeof stream.pipeline !== "function") {
    throw new Error("Load the Niva browser vendor bundle before the stream adapter.");
  }

  var module = {
    Stream: stream.Stream,
    Readable: stream.Readable,
    Writable: stream.Writable,
    Duplex: stream.Duplex,
    Transform: stream.Transform,
    PassThrough: stream.PassThrough,
    pipeline: stream.pipeline,
    finished: stream.finished,
    promises: stream.promises,
    addAbortSignal: stream.addAbortSignal,
    compose: stream.compose,
    destroy: stream.destroy,
    from: stream.from,
    fromWeb: stream.fromWeb,
    toWeb: stream.toWeb,
    wrap: stream.wrap,
    isDisturbed: stream.isDisturbed,
    isErrored: stream.isErrored,
    isReadable: stream.isReadable,
  };
  runtime.createStreamModule = function () { return module; };
  runtime.streamModule = module;
})(globalThis);
