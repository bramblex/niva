import "./setup-runtime.js";
import test from "node:test";
import assert from "node:assert/strict";

const runtime = globalThis[Symbol.for("niva.node-compat.runtime")];

function pendingBridge() {
  const calls = [];
  const callWaiters = [];
  let nextId = 0;
  return {
    calls,
    waitForCall(method, timeoutMs = 1000) {
      const existing = calls.find((call) => call.method === method);
      if (existing) return Promise.resolve(existing);
      return new Promise((resolve, reject) => {
        const waiter = { method, resolve, timer: setTimeout(() => {
          const index = callWaiters.indexOf(waiter);
          if (index >= 0) callWaiters.splice(index, 1);
          reject(new Error(`Timed out waiting for ${method}`));
        }, timeoutMs) };
        callWaiters.push(waiter);
      });
    },
    bridge: {
      stream(method, args, handlers) {
        let reject;
        let settled = false;
        let cancelCalls = 0;
        const promise = new Promise((_resolve, rejectPromise) => { reject = rejectPromise; });
        const call = {
          id: ++nextId,
          method,
          args,
          handlers,
          promise,
          cancel() {
            cancelCalls += 1;
            if (settled) return false;
            settled = true;
            reject(Object.assign(new Error("cancelled"), { code: "ABORT_ERR" }));
            return true;
          },
          get cancelCalls() { return cancelCalls; },
        };
        calls.push(call);
        for (let index = callWaiters.length - 1; index >= 0; index -= 1) {
          const waiter = callWaiters[index];
          if (waiter.method !== method) continue;
          clearTimeout(waiter.timer);
          callWaiters.splice(index, 1);
          waiter.resolve(call);
        }
        return call;
      },
      call() { return Promise.resolve(undefined); },
      callSync() { return "/"; },
    },
    bootstrap: {
      os: { platform: "linux" },
      process: { version: "v0.9.9", versions: { niva: "0.9.9" }, argv: [], env: {} },
    },
  };
}

test("session cleanup cancels each live child once and fails its completion", async () => {
  const niva = pendingBridge();
  const child = runtime.createChildProcessModule(niva).spawn("helper", []);
  const errors = [];
  const closes = [];
  child.stdin.on("error", () => {});
  child.on("error", (error) => errors.push(error));
  child.on("close", (...args) => closes.push(args));

  const expired = Object.assign(new Error("session expired"), { code: "ERR_NIVA_SESSION_EXPIRED" });
  runtime.invalidateResources(expired);
  child.__nivaInvalidate(expired);
  await assert.rejects(child.completion, (error) => error === expired);

  assert.equal(niva.calls.length, 1);
  assert.equal(child.cancel(), false);
  assert.equal(niva.calls[0].cancelCalls, 1);
  assert.deepEqual(errors, [expired]);
  assert.deepEqual(closes, [[null, null]]);
});

test("process.stdin is a tracked stream and is invalidated on session cleanup", async () => {
  const niva = pendingBridge();
  const processModule = runtime.createProcessModule(niva);
  const errors = [];
  processModule.stdin.on("error", (error) => errors.push(error));
  const stdinCall = niva.waitForCall("process.stdin");
  processModule.stdin.resume();
  assert.equal(await stdinCall, niva.calls[0]);
  assert.equal(niva.calls.length, 1);

  const expired = Object.assign(new Error("session expired"), { code: "ERR_NIVA_SESSION_EXPIRED" });
  const streamError = new Promise((resolve) => processModule.stdin.once("error", resolve));
  runtime.invalidateResources(expired);
  assert.equal(await streamError, expired);

  assert.equal(processModule.stdin.destroyed, true);
  assert.equal(niva.calls.length, 1);
  assert.equal(niva.calls[0].cancelCalls, 1);
  assert.deepEqual(errors, [expired]);
});
