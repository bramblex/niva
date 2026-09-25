(function (root: any) {
  "use strict";

  var runtime = root[Symbol.for("niva.node-compat.runtime")];
  if (typeof runtime.createStreamModule === "function") return;
  var vendor = runtime.vendor;
  var stream = vendor && vendor.stream;
  if (!stream || typeof stream.pipeline !== "function") {
    throw new Error("Load the Niva browser vendor bundle before the stream adapter.");
  }

  // Node's `require('stream')` is itself the Stream constructor. Attach the
  // browser stream implementation's public statics to that exact function so
  // `stream instanceof require('stream')` and named imports share identity.
  var module = stream.Stream;
  Object.assign(module, {
    Stream: module,
    Readable: stream.Readable,
    Writable: stream.Writable,
    Duplex: stream.Duplex,
    Transform: stream.Transform,
    PassThrough: stream.PassThrough,
    pipeline: stream.pipeline,
    finished: stream.finished,
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
  });
  runtime.createStreamModule = function () { return module; };
  runtime.streamModule = module;
})(globalThis);
