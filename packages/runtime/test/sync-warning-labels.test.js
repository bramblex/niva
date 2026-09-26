import "./setup-runtime.js";
import test from "node:test";
import assert from "node:assert/strict";

const runtime = globalThis[Symbol.for("niva.node-compat.runtime")];
const callSyncApi = Symbol.for("niva.internal.bridge.callSyncApi");

function makeNiva(syncResult) {
  const calls = [];
  const warnings = [];
  const warned = new Set();
  const bridge = {
    callSync(method, args) {
      calls.push([method, args]);
      return typeof syncResult === "function" ? syncResult(method, args) : syncResult;
    },
  };
  bridge[callSyncApi] = function (apiName, method, args) {
    if (!warned.has(apiName)) {
      warned.add(apiName);
      warnings.push(apiName);
    }
    return bridge.callSync(method, args);
  };
  return { niva: { bridge }, calls, warnings };
}

test("synchronous OS APIs warn by their public API name while sharing Native calls", () => {
  const { niva, calls, warnings } = makeNiva((method) => method.split(".")[1]);
  const os = runtime.createOsModule(niva);

  for (const name of ["cpus", "freemem", "networkInterfaces", "uptime"]) {
    assert.equal(os[name](), name);
  }

  assert.deepEqual(warnings, ["os.cpus", "os.freemem", "os.networkInterfaces", "os.uptime"]);
  assert.deepEqual(calls, [
    ["os.cpus", []],
    ["os.freemem", []],
    ["os.networkInterfaces", []],
    ["os.uptime", []],
  ]);
});

test("dns.getServers and Resolver.getServers have distinct warning labels", () => {
  const { niva, calls, warnings } = makeNiva(["1.1.1.1", "2001:4860:4860::8888"]);
  const dns = runtime.createDnsModule(niva);

  assert.deepEqual(dns.getServers(), ["1.1.1.1", "2001:4860:4860::8888"]);
  assert.deepEqual(new dns.Resolver().getServers(), ["1.1.1.1", "2001:4860:4860::8888"]);
  assert.deepEqual(warnings, ["dns.getServers", "dns.Resolver.getServers"]);
  assert.deepEqual(calls, [
    ["os.dnsServers", []],
    ["os.dnsServers", []],
  ]);
});

test("async DNS resolution obtains system servers over async IPC and keeps CNAME recursion async", async () => {
  const asyncCalls = [];
  const syncCalls = [];
  const streamCalls = [];
  const bridge = {
    call(method, args) {
      asyncCalls.push([method, args]);
      if (method === "os.dnsServers") return Promise.resolve(["1.1.1.1"]);
      return Promise.reject(new Error("Unexpected async Native method: " + method));
    },
    callSync(method, args) {
      syncCalls.push([method, args]);
      throw new Error("Async DNS resolution must not use synchronous XHR");
    },
    stream(method, args, handlers) {
      const id = streamCalls.length + 1;
      const entry = { id, method, args, handlers: handlers || {} };
      streamCalls.push(entry);
      if (method === "socket.udpBind") {
        queueMicrotask(() => entry.handlers.onEvent("listening", { socketId: 41, address: "0.0.0.0", port: 53000 }));
        return { id, cancel() {}, promise: new Promise(() => {}) };
      }
      if (method === "socket.udpSend") return { id, cancel() {}, promise: Promise.resolve({ sent: 1 }) };
      if (method === "socket.control") {
        const action = args[0].action;
        if (action === "close") queueMicrotask(() => bindHandlers?.onEvent("close", {}));
        return { id, cancel() {}, promise: Promise.resolve(true) };
      }
      throw new Error("Unexpected DNS stream method: " + method);
    },
    streamSend(id, data) {
      const packet = runtime.vendor.dnsPacket;
      const query = packet.decode(Buffer.from(data));
      const question = query.questions[0];
      const answer = question.name === "example.test"
        ? { name: question.name, type: "CNAME", ttl: 60, data: "alias.test" }
        : { name: question.name, type: "A", ttl: 60, data: "192.0.2.42" };
      const response = packet.encode({
        type: "response",
        id: query.id,
        flags: packet.RECURSION_DESIRED,
        questions: query.questions,
        answers: [answer],
      });
      const bindEntry = streamCalls.filter((entry) => entry.method === "socket.udpBind").at(-1);
      bindEntry.handlers.onEvent("datagram", { address: "1.1.1.1", port: 53, size: response.length });
      bindEntry.handlers.onBlob({ arrayBuffer: () => Promise.resolve(response.buffer.slice(response.byteOffset, response.byteOffset + response.byteLength)) });
      return true;
    },
  };
  let bindHandlers;
  const originalStream = bridge.stream;
  bridge.stream = function (method, args, handlers) {
    const call = originalStream.call(bridge, method, args, handlers);
    if (method === "socket.udpBind") bindHandlers = handlers;
    return call;
  };
  bridge[Symbol.for("niva.internal.bridge.streamRelated")] = function (_owner, method, args, handlers) {
    return bridge.stream(method, args, handlers);
  };

  const dns = runtime.createDnsModule({ bridge });
  const records = await new Promise((resolve, reject) => {
    dns.resolve4("example.test", (error, value) => error ? reject(error) : resolve(value));
  });

  assert.deepEqual(records, ["192.0.2.42"]);
  assert.deepEqual(asyncCalls, [["os.dnsServers", []]]);
  assert.deepEqual(syncCalls, []);
  assert.equal(streamCalls.filter((entry) => entry.method === "socket.udpSend").length, 2);
});

test("path labels only methods whose current implementation reads Native cwd", () => {
  const previousNiva = globalThis.Niva;
  const { niva, calls, warnings } = makeNiva("/workspace/project");
  globalThis.Niva = niva;

  try {
    const path = runtime.createPathModule();
    assert.equal(path.posix.resolve("/absolute/file"), "/absolute/file");
    assert.equal(path.posix.normalize("/a/../b"), "/b");
    assert.equal(path.posix.join("a", "b"), "a/b");
    assert.equal(path.posix.toNamespacedPath("/absolute/file"), "/absolute/file");
    assert.deepEqual(calls, [["process.currentDir", []]]);
    assert.deepEqual(warnings, ["path.resolve"]);

    assert.equal(path.posix.relative("/same", "/same"), "");
    assert.equal(calls.length, 1, "equal relative paths return before reading cwd");
    assert.equal(path.posix.relative("/from", "/to"), "../to");
    assert.equal(calls.length, 3, "unequal relative paths resolve both sides against cwd");
    assert.deepEqual(warnings, ["path.resolve", "path.relative"]);

    assert.equal(path.win32.toNamespacedPath("C:\\absolute\\file"), "\\\\?\\C:\\absolute\\file");
    assert.equal(path.win32._makeLong("C:\\absolute\\file"), "\\\\?\\C:\\absolute\\file");
    assert.equal(calls.length, 5, "Windows namespacing resolves against cwd");
    assert.deepEqual(warnings, ["path.resolve", "path.relative", "path.toNamespacedPath", "path._makeLong"]);
  } finally {
    if (previousNiva === undefined) delete globalThis.Niva;
    else globalThis.Niva = previousNiva;
  }
});
