import "./setup-runtime.js";
import test from "node:test";
import assert from "node:assert/strict";
import timers from "../dist/source/timers.js";

test("timeout and interval callbacks receive their cancellable handle as this", async () => {
  await new Promise((resolve, reject) => {
    let timeout;
    timeout = timers.setTimeout(function () {
      try {
        assert.equal(this, timeout);
        resolve();
      } catch (error) { reject(error); }
    }, 0);
  });

  let calls = 0;
  await new Promise((resolve, reject) => {
    let interval;
    interval = timers.setInterval(function () {
      try {
        calls++;
        assert.equal(this, interval);
        timers.clearInterval(this);
        resolve();
      } catch (error) { reject(error); }
    }, 1);
  });
  await new Promise((resolve) => setTimeout(resolve, 10));
  assert.equal(calls, 1);
});

test("promise scheduler has Node abort and constructor behavior", async () => {
  const { scheduler } = timers.promises;
  assert.throws(() => new scheduler.constructor(), { code: "ERR_ILLEGAL_CONSTRUCTOR" });
  await assert.rejects(scheduler.wait(10000, { signal: AbortSignal.abort() }), {
    name: "AbortError",
    code: "ABORT_ERR",
    message: "The operation was aborted",
  });
  await scheduler.yield();
});
