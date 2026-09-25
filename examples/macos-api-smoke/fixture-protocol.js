(function (root) {
  "use strict";

  var process = root.process;
  if (!process || !process.stdin || !process.stdout) {
    throw new Error("The main Niva fixture process streams are unavailable");
  }

  var listeners = Object.create(null);
  var decoder = new TextDecoder("utf-8");
  var input = "";

  function write(frame) {
    return new Promise(function (resolve, reject) {
      process.stdout.write(JSON.stringify(Object.assign({
        protocol: "niva-fixture",
        version: 1,
      }, frame)) + "\n", function (error) {
        if (error) reject(error);
        else resolve();
      });
    });
  }

  function dispatch(line) {
    var frame;
    try {
      frame = JSON.parse(line);
    } catch (error) {
      void write({ event: "protocol-error", message: String(error) });
      return;
    }
    if (!frame || frame.protocol !== "niva-fixture" || frame.version !== 1 || frame.event !== "command" || typeof frame.name !== "string") return;
    (listeners[frame.name] || []).slice().forEach(function (handler) {
      Promise.resolve().then(function () { return handler(frame.data); }).catch(function (error) {
        void write({ event: "message", name: "fixture-error", data: { command: frame.name, message: String(error && error.stack || error) } });
      });
    });
  }

  process.stdin.on("data", function (chunk) {
    input += decoder.decode(chunk, { stream: true });
    var newline;
    while ((newline = input.indexOf("\n")) >= 0) {
      var line = input.slice(0, newline).replace(/\r$/, "");
      input = input.slice(newline + 1);
      if (line) dispatch(line);
    }
  });

  process.stdin.on("end", function () {
    input += decoder.decode();
    if (input.trim()) dispatch(input.trim());
  });
  listeners["fixture-exit"] = [function (code) { process.exit(Number.isInteger(code) ? code : 0); }];

  root.NivaFixture = {
    ready: function (name, data) {
      return write({ event: "ready", name: name, data: data });
    },
    send: function (name, data) {
      return write({ event: "message", name: name, data: data });
    },
    onCommand: function (name, handler) {
      (listeners[name] || (listeners[name] = [])).push(handler);
    },
    exit: function (code) {
      process.exit(code === undefined ? 0 : code);
    },
  };

  if (root.Niva && typeof root.Niva.addEventListener === "function") {
    root.Niva.addEventListener("window.message", function (_eventName, payload) {
      if (!payload || typeof payload.message !== "string") return;
      try {
        var frame = JSON.parse(payload.message);
        if (frame && frame.protocol === "niva-fixture" && frame.version === 1 && frame.event === "message") {
          void write({ event: "message", name: frame.name, data: frame.data });
        }
      } catch (_) {}
    });
  }
})(globalThis);
