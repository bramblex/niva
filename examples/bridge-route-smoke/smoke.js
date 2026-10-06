(() => {
  "use strict";
  const status = document.getElementById("status");
  const query = new URLSearchParams(location.search);
  const mode = query.get("mode") || "ws";
  const reportUrl = query.get("report");
  const bytes = [0, 1, 2, 127, 128, 254, 255, 78, 105, 118, 97];
  const record = window.__bridgeRouteRecord;

  function toHex(value) {
    return Array.from(value, item => item.toString(16).padStart(2, "0")).join("");
  }
  function equalBytes(left, right) {
    return left.length === right.length && left.every((value, index) => value === right[index]);
  }
  function assert(condition, message) {
    if (!condition) throw new Error(message);
  }
  async function runSameOriginFrame() {
    // Before the child is created, this parent reply set can only contain the
    // top frame's own settled session. Later routed child replies are observed
    // here too, so the post-navigation parent set is not an owner identity.
    const parentOwnSessions = new Set(window.__bridgeRouteSessionValues());
    const iframe = document.createElement("iframe");
    const result = await new Promise((resolve, reject) => {
      const timer = setTimeout(() => reject(new Error("same-origin iframe unary call timed out")), 10000);
      Object.defineProperty(window, "__bridgeRouteReceiveFrameResult", {
        configurable: true,
        writable: true,
        value: value => {
          clearTimeout(timer);
          resolve(value);
        }
      });
      iframe.src = "frame.html";
      iframe.onerror = () => {
        clearTimeout(timer);
        reject(new Error("same-origin iframe fixture failed to load"));
      };
      document.body.appendChild(iframe);
    });
    try {
      assert(result && result.frame === iframe.contentWindow, "iframe result did not come from the created child frame");
      assert(result.ok === true, `same-origin iframe Niva.window.title failed: ${result && result.message || "unknown error"}`);
      assert(typeof result.title === "string", "same-origin iframe unary call returned a non-string title");
      assert(Number.isInteger(result.sessionCount) && result.sessionCount > 0,
        "same-origin iframe did not observe its own Native IPC reply session");
      const parentUrl = new URL(location.href);
      const childUrl = new URL(result.href);
      assert(childUrl.protocol === parentUrl.protocol && childUrl.host === parentUrl.host && childUrl.pathname === "/frame.html",
        "child frame did not load from the parent's exact origin and fixture path");
      assert(result.sessionSetAvailable === true
        && typeof iframe.contentWindow.__bridgeRouteSessionValues === "function",
      "same-origin iframe session observer is unavailable");
      const childSessions = iframe.contentWindow.__bridgeRouteSessionValues();
      assert(parentOwnSessions.size > 0, "top-level page had no settled IPC session before creating the child");
      assert(childSessions.length > 0 && childSessions.some(sessionId => !parentOwnSessions.has(sessionId)),
        "same-origin iframe did not use a session distinct from the parent's pre-child session");
      const childEvents = Array.isArray(result.events) ? result.events : [];
      window.__bridgeRouteImportEvents(childEvents, "same-origin-child");
      assert(childEvents.some(event => event.kind === "ipc-reply"), "same-origin iframe did not observe its own eval reply");
      const parentTitle = await Niva.window.title();
      assert(typeof parentTitle === "string", "top-level unary call did not settle after the child-frame reply");
      record("post-iframe-unary-complete", { method: "window.title" });
      record("iframe-unary-complete", { sameOrigin: true, independentSession: true,
        parentSessionCount: parentOwnSessions.size, childSessionCount: result.sessionCount,
        parentSessionSetAvailable: typeof window.__bridgeRouteSessionValues === "function",
        childSessionSetAvailable: typeof iframe.contentWindow.__bridgeRouteSessionValues === "function" });
      return { sameOrigin: true, independentSession: true, titleType: typeof result.title,
        parentSessionCount: parentOwnSessions.size, childSessionCount: result.sessionCount,
        parentSessionSetAvailable: typeof window.__bridgeRouteSessionValues === "function",
        childSessionSetAvailable: typeof iframe.contentWindow.__bridgeRouteSessionValues === "function" };
    } finally {
      iframe.remove();
      delete window.__bridgeRouteReceiveFrameResult;
    }
  }
  function settleChild(child, payload) {
    return new Promise((resolve, reject) => {
      const stdout = [];
      const stderr = [];
      let settled = false;
      const finish = (error, result) => {
        if (settled) return;
        settled = true;
        clearTimeout(timer);
        if (error) reject(error); else resolve(result);
      };
      const timer = setTimeout(() => {
        try { child.kill(); } catch (_) {}
        finish(new Error("child_process stream timed out"));
      }, 15000);
      child.stdout.on("data", chunk => stdout.push(...chunk));
      child.stderr.on("data", chunk => stderr.push(...chunk));
      child.once("error", error => finish(error));
      child.once("close", (code, signal) => {
        if (code !== 0) return finish(new Error(`child_process exit ${code} (${signal || "no signal"}); stderrBytes=${stderr.length}`));
        finish(null, { stdout, stderr, code });
      });
      child.stdin.end(Uint8Array.from(payload));
    });
  }

  async function run() {
    assert(window.Niva && Niva.bridge, "Niva.bridge was not installed");
    assert(typeof window.ipc?.postMessage === "function", "window.ipc.postMessage is unavailable");
    assert(typeof window.__niva_ipc_reply === "function", "__niva_ipc_reply is unavailable");
    const runtimeConfig = Niva.runtimeConfig || {};
    const rawRuntimeConfig = window.__niva_runtime_config && typeof window.__niva_runtime_config === "object"
      ? window.__niva_runtime_config : {};
    let bridgeTrustedLocal = null;
    try {
      bridgeTrustedLocal = typeof Niva.bridge.isTrustedLocal === "function"
        ? Niva.bridge.isTrustedLocal() === true : null;
    } catch (_) {}
    record("runtime-surfaces", {
      runtimeConfigType: typeof Niva.runtimeConfig,
      runtimeConfig: {
        injectCommonJs: runtimeConfig.injectCommonJs === true,
        injectEsm: runtimeConfig.injectEsm === true,
        trustedLocal: runtimeConfig.trustedLocal === true
      },
      rawRuntimeConfig: {
        exists: !!window.__niva_runtime_config,
        injectCommonJs: rawRuntimeConfig.injectCommonJs === true,
        injectEsm: rawRuntimeConfig.injectEsm === true
      },
      hasBridgeToken: !!window.__niva_token,
      bridgeTrustedLocal,
      observedSessionCount: window.__bridgeRouteSessionCount(),
      sessionSetAvailable: typeof window.__bridgeRouteSessionValues === "function",
      globalRequire: typeof window.require,
      nivaProcess: typeof Niva.process,
      processCwd: typeof Niva.process?.cwd,
      nivaChildProcess: typeof Niva.child_process,
      childProcessSpawn: typeof Niva.child_process?.spawn,
      nivaFs: typeof Niva.fs
    });
    assert(runtimeConfig.injectCommonJs === true, "Niva.runtimeConfig.injectCommonJs is not true");
    assert(runtimeConfig.trustedLocal === true && Niva.bridge.isTrustedLocal() === true,
      "trusted local runtime marker is not active");
    assert(typeof window.require === "function", "global CommonJS require is unavailable");

    const title = await Niva.window.title();
    assert(typeof title === "string", `window.title returned ${typeof title}, expected string`);
    record("unary-complete", { method: "window.title" });
    record("top-session-observed", {
      sessionSetAvailable: typeof window.__bridgeRouteSessionValues === "function",
      sessionCount: window.__bridgeRouteSessionCount()
    });
    const iframeResult = await runSameOriginFrame();

    let cwdObserved = false;
    let syncXhr = false;
    if (mode === "ws") {
      assert(typeof Niva.process?.cwd === "function", "Niva.process.cwd() compatibility export is unavailable");
      const nodeProcess = require("node:process");
      const cwd = nodeProcess.cwd();
      assert(typeof cwd === "string" && cwd.length > 0, "Node process.cwd() returned an empty path");
      assert(Niva.process.cwd() === cwd, "Niva.process.cwd() disagreed with the Node-compatible process module");
      cwdObserved = true;
      syncXhr = window.__bridgeRouteEvents.some(event => event.kind === "xhr-open" && event.async === false);
      assert(syncXhr, "Node process.cwd() did not use synchronous XHR compatibility path");
    }

    const filePath = window.__bridgeRouteFixture.filePath;
    assert(typeof filePath === "string" && filePath.length > 0, "fixture did not provide a binary file path");
    await Niva.fs.promises.writeFile(filePath, Uint8Array.from(bytes));
    const fileBytes = await Niva.fs.promises.readFile(filePath);
    assert(equalBytes(Array.from(fileBytes), bytes), `Niva.fs binary readback mismatch: ${toHex(Array.from(fileBytes))}`);
    await Niva.fs.promises.unlink(filePath);
    record("file-io-complete", { bytes: fileBytes.length, hex: toHex(Array.from(fileBytes)) });

    const largePath = `${filePath}.large`;
    const largeExpected = new Uint8Array(1024 * 1024);
    let checksum = 2166136261;
    for (let index = 0; index < largeExpected.length; index += 1) {
      const value = (index * 31 + (index >> 8)) & 255;
      largeExpected[index] = value;
      checksum = Math.imul(checksum ^ value, 16777619) >>> 0;
    }
    const handle = await Niva.fs.promises.open(largePath, "w+");
    try {
      for (let offset = 0; offset < largeExpected.length;) {
        const count = Math.min(65536, largeExpected.length - offset);
        const written = await handle.write(largeExpected, offset, count, offset);
        assert(written.bytesWritten > 0, `FileHandle.write made no progress at ${offset}`);
        offset += written.bytesWritten;
      }
      const stats = await handle.stat();
      assert(stats.size === largeExpected.length, `FileHandle.stat size ${stats.size} did not match ${largeExpected.length}`);
      const largeActual = new Uint8Array(largeExpected.length);
      for (let offset = 0; offset < largeActual.length;) {
        const count = Math.min(65536, largeActual.length - offset);
        const read = await handle.read(largeActual, offset, count, offset);
        assert(read.bytesRead > 0, `FileHandle.read ended early at ${offset}`);
        offset += read.bytesRead;
      }
      assert(equalBytes(largeActual, largeExpected), "FileHandle 1 MiB binary readback differed from written bytes");
    } finally {
      await handle.close();
      await Niva.fs.promises.unlink(largePath);
    }
    record("filehandle-large-complete", { bytes: largeExpected.length, checksum: checksum.toString(16).padStart(8, "0") });

    assert(typeof Niva.child_process?.spawn === "function", "Niva.child_process.spawn() compatibility export is unavailable");
    const nodeChildProcess = require("node:child_process");
    const child = nodeChildProcess.spawn(window.__bridgeRouteFixture.python, ["-c", "import sys; data=sys.stdin.buffer.read(); sys.stdout.buffer.write(data); sys.stdout.buffer.flush()"], { stdio: ["pipe", "pipe", "pipe"] });
    const result = await settleChild(child, bytes);
    assert(equalBytes(result.stdout, bytes), `child_process binary roundtrip mismatch: ${toHex(result.stdout)}`);
    const publicChild = Niva.child_process.spawn(window.__bridgeRouteFixture.python, ["-c", "import sys; data=sys.stdin.buffer.read(); sys.stdout.buffer.write(data); sys.stdout.buffer.flush()"], { stdio: ["pipe", "pipe", "pipe"] });
    const publicResult = await settleChild(publicChild, bytes);
    assert(equalBytes(publicResult.stdout, bytes), `Niva.child_process binary roundtrip mismatch: ${toHex(publicResult.stdout)}`);
    record("stream-complete", { route: mode, bytes: result.stdout.length, hex: toHex(result.stdout),
      nivaApiBytes: publicResult.stdout.length, nivaApiHex: toHex(publicResult.stdout) });

    const events = window.__bridgeRouteEvents.slice();
    const instrumentationErrors = events.filter(event => event.kind === "instrumentation-error");
    assert(instrumentationErrors.length === 0, `route instrumentation failed: ${JSON.stringify(instrumentationErrors)}`);
    const report = {
      ok: true,
      mode,
      origin: location.origin,
      title,
      iframe: iframeResult,
      cwdObserved,
      syncXhr,
      inputHex: toHex(bytes),
      outputHex: toHex(result.stdout),
      nivaApiHex: toHex(publicResult.stdout),
      fileHex: toHex(Array.from(fileBytes)),
      fileHandleBytes: 1024 * 1024,
      fileHandleChecksum: checksum.toString(16).padStart(8, "0"),
      events
    };
    const response = await fetch(reportUrl, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(report)
    });
    assert(response.ok, `report POST failed with HTTP ${response.status}`);
    status.textContent = `PASS: ${mode}`;
  }

  run().catch(async error => {
    const errorMessage = String(error && error.message || error);
    const report = {
      ok: false,
      mode,
      origin: location.origin,
      error: errorMessage,
      stack: String(error && error.stack || ""),
      events: window.__bridgeRouteEvents || []
    };
    status.textContent = `FAIL: ${report.error}`;
    try {
      await fetch(reportUrl, { method: "POST", headers: { "Content-Type": "application/json" }, body: JSON.stringify(report) });
    } catch (_) {}
  });
})();
