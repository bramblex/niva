import "./setup-runtime.js";
import test from "node:test";
import assert from "node:assert/strict";
import { getEventListeners } from "node:events";
import "../dist/source/child_process.js";

function mockNiva(platform = process.platform) {
  const calls = [];
  const writes = [];
  let nextId = 1;
  const niva = {
    bootstrap: { os: { platform } },
    calls,
    writes,
    bridge: {
    stream(method, args, handlers) {
      let resolve;
      let reject;
      const promise = new Promise((res, rej) => { resolve = res; reject = rej; });
      const call = { id: `call-${nextId++}`, method, args, handlers, promise, resolve, reject, cancel() {} };
      calls.push(call);
      return call;
    },
    streamSend(id, data, end) {
      writes.push({ id, bytes: data.byteLength, end });
      return true;
    },
    },
  };
  return niva;
}

function apiFor(niva) {
  return globalThis[Symbol.for("niva.node-compat.runtime")].createChildProcessModule(niva);
}

test("execFile completes on a negative close event with a general system-error name", async () => {
  const niva = mockNiva("darwin");
  const api = apiFor(niva);
  let child;
  const callbackResult = new Promise((resolve) => {
    child = api.execFile("/node", (error, stdout, stderr) => resolve({ error, stdout, stderr }));
  });
  const execCall = niva.calls.find((call) => call.method === "process.execStream");
  assert.ok(execCall, "execFile must use the Niva process stream");
  assert.equal(niva.writes.at(-1).end, true, "execFile closes child stdin");

  child.kill();
  child.emit("close", -1, null);
  execCall.resolve({ status: null, signal: 15 });
  const { error, stdout, stderr } = await callbackResult;
  assert.equal(error.name, "Error");
  assert.equal(error.code, "EPERM");
  assert.equal(error.message, "Command failed: /node");
  assert.equal(error.killed, true);
  assert.equal(error.signal, null);
  assert.equal(error.cmd, "/node");
  assert.equal(stdout, "");
  assert.equal(stderr, "");
});

test("negative close errors use Node 22's Windows libuv names and unknown fallback", async () => {
  async function closeWithStatus(status) {
    const niva = mockNiva("win32");
    const api = apiFor(niva);
    let child;
    const result = new Promise((resolve) => { child = api.execFile("/node", (error) => resolve(error)); });
    const execCall = niva.calls.find((call) => call.method === "process.execStream");
    child.kill();
    child.emit("close", status, null);
    execCall.resolve({ status: null, signal: 15 });
    return result;
  }
  assert.equal((await closeWithStatus(-4048)).code, "EPERM");
  assert.equal((await closeWithStatus(-1)).code, "Unknown system error -1");
});

test("execFile does not spawn for a pre-aborted signal and returns AbortError", async () => {
  const niva = mockNiva();
  const api = apiFor(niva);
  const controller = new AbortController();
  controller.abort();
  let synchronous = true;
  const callbackResult = new Promise((resolve) => {
    api.execFile("/node", [], { signal: controller.signal }, (error, stdout, stderr) => {
      resolve({ error, stdout, stderr, synchronous });
    });
  });
  synchronous = false;
  assert.equal(niva.calls.length, 0, "an already-aborted signal must not issue process.execStream");
  assert.equal(niva.writes.length, 0, "pre-aborted stdin finalization must not send an undefined stream id");
  const { error, stdout, stderr, synchronous: callbackWasSynchronous } = await callbackResult;
  assert.equal(callbackWasSynchronous, false);
  assert.equal(error.name, "AbortError");
  assert.equal(error.code, "ABORT_ERR");
  assert.equal(error.signal, undefined);
  assert.equal(stdout, "");
  assert.equal(stderr, "");
});

test("execFile aborts a running Native process and removes the AbortSignal listener", async () => {
  const niva = mockNiva();
  const api = apiFor(niva);
  const controller = new AbortController();
  let child;
  const callbackResult = new Promise((resolve) => {
    child = api.execFile("/node", [], { signal: controller.signal }, (error, stdout, stderr) => resolve({ error, stdout, stderr }));
  });
  const execCall = niva.calls.find((call) => call.method === "process.execStream");
  execCall.handlers.onEvent("spawn", { pid: 12345 });
  assert.equal(getEventListeners(controller.signal, "abort").length, 1);
  controller.abort();
  assert.ok(niva.calls.some((call) => call.method === "process.signal"));
  execCall.resolve({ status: null, signal: 15 });
  const { error, stdout, stderr } = await callbackResult;
  assert.equal(error.name, "AbortError");
  assert.equal(error.code, "ABORT_ERR");
  assert.equal(error.signal, undefined);
  assert.equal(stdout, "");
  assert.equal(stderr, "");
  assert.equal(getEventListeners(controller.signal, "abort").length, 0);
});
