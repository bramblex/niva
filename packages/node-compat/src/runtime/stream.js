(function (root) {
  "use strict";

  var runtime = root[Symbol.for("niva.node-compat.runtime")];

  function pipeline() {
    var args = Array.prototype.slice.call(arguments);
    var callback = typeof args[args.length - 1] === "function" ? args.pop() : undefined;
    var streams = args;
    if (streams.length < 2) throw new TypeError("pipeline requires a source and destination");
    streams.forEach(function (stream, index) {
      if (!stream || typeof stream.on !== "function") throw new TypeError("pipeline stream at index " + index + " must provide on()");
    });
    streams.slice(0, -1).forEach(function (stream, index) {
      if (typeof stream.pipe !== "function") throw new TypeError("pipeline source at index " + index + " must provide pipe()");
    });

    var destination = streams[streams.length - 1];
    var resolvePipeline;
    var rejectPipeline;
    var promise = new Promise(function (resolve, reject) { resolvePipeline = resolve; rejectPipeline = reject; });
    var settled = false;
    var onError;

    function cleanup() {
      streams.forEach(function (stream) {
        if (typeof stream.off === "function") stream.off("error", onError);
        else if (typeof stream.removeListener === "function") stream.removeListener("error", onError);
      });
    }

    function succeed() {
      if (settled) return;
      settled = true;
      cleanup();
      resolvePipeline(destination);
    }

    function fail(error) {
      if (settled) return;
      settled = true;
      cleanup();
      streams.forEach(function (stream) {
        if (stream !== destination && typeof stream.destroy === "function") {
          try { stream.destroy(error); } catch (_) {}
        }
      });
      if (destination && typeof destination.destroy === "function") {
        try { destination.destroy(error); } catch (_) {}
      }
      rejectPipeline(error);
    }

    onError = fail;
    streams.forEach(function (stream) { stream.on("error", onError); });

    var completion = destination.result || destination.completion;
    if (completion && typeof completion.then === "function") completion.then(succeed, fail);
    else destination.once("finish", succeed);

    try {
      for (var i = 0; i < streams.length - 1; i += 1) streams[i].pipe(streams[i + 1]);
    } catch (error) {
      fail(error);
    }

    if (callback) {
      promise.then(function () { callback(null); }, function (error) { callback(error); });
      return destination;
    }
    return promise;
  }

  var promises = { pipeline: pipeline };
  var module = { pipeline: pipeline, promises: promises };
  runtime.createStreamModule = function () { return module; };
  runtime.streamModule = module;
})(globalThis);
