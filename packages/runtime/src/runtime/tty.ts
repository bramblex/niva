(function (root: any) {
  "use strict";

  var runtime = root[Symbol.for("niva.node-compat.runtime")];
  if (typeof runtime.createTtyModule === "function") return;

  function createTtyModule(niva?) {
    const target = runtime.resolveNiva(niva);
    const state = target.bootstrap?.process?.stdioIsTTY || {};
    function isatty(fd: number) {
      if (!Number.isInteger(fd) || fd < 0 || fd > 2) return false;
      const name = fd === 0 ? "stdin" : fd === 1 ? "stdout" : "stderr";
      return state[name] === true;
    }
    class ReadStream {
      constructor() { throw runtime.bridgeError("tty.ReadStream is not supported by Niva", "ENOTSUP"); }
    }
    class WriteStream {
      constructor() { throw runtime.bridgeError("tty.WriteStream is not supported by Niva", "ENOTSUP"); }
    }
    return { isatty, ReadStream, WriteStream };
  }

  runtime.createTtyModule = createTtyModule;
})(globalThis);
