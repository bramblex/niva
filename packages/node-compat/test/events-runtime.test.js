import test from "node:test";
import assert from "node:assert/strict";
import events, { EventEmitter, once, on } from "../src/events.js";

test("events default export is the EventEmitter constructor with Node-style statics", () => {
  assert.equal(events, EventEmitter);
  assert.equal(events.EventEmitter, EventEmitter);
  assert.equal(events.once, once);
  assert.equal(events.on, on);
  const emitter = new events();
  assert.equal(emitter instanceof EventEmitter, true);
});

test("events.once resolves arguments and removes temporary listeners", async () => {
  const emitter = new EventEmitter();
  const result = once(emitter, "value");
  assert.equal(emitter.listenerCount("value"), 1);
  assert.equal(emitter.listenerCount("error"), 1);
  emitter.emit("value", 42, "extra");
  assert.deepEqual(await result, [42, "extra"]);
  assert.equal(emitter.listenerCount("value"), 0);
  assert.equal(emitter.listenerCount("error"), 0);

  const expected = new Error("once error");
  const rejected = once(emitter, "never");
  emitter.emit("error", expected);
  await assert.rejects(rejected, error => error === expected);
  assert.equal(emitter.listenerCount("never"), 0);
  assert.equal(emitter.listenerCount("error"), 0);
});

test("events.once validates options and supports EventTarget and AbortSignal", async () => {
  const emitter = new EventEmitter();
  for (const options of [1, "options", null, false, () => {}, Symbol(), 1n]) {
    await assert.rejects(once(emitter, "value", options), { code: "ERR_INVALID_ARG_TYPE" });
  }
  for (const signal of [1, {}, "signal", null, false]) {
    await assert.rejects(once(emitter, "value", { signal }), { code: "ERR_INVALID_ARG_TYPE" });
  }

  const target = new EventTarget();
  const event = new Event("ready");
  const targetResult = once(target, "ready");
  target.dispatchEvent(event);
  assert.deepEqual(await targetResult, [event]);

  const controller = new AbortController();
  controller.signal.addEventListener("abort", value => value.stopImmediatePropagation(), { once: true });
  const aborted = once(new EventEmitter(), "never", { signal: controller.signal });
  controller.abort();
  await assert.rejects(aborted, { name: "AbortError" });
});

test("events.on drains queued values before an error and removes listeners", async () => {
  const emitter = new EventEmitter();
  const iterator = on(emitter, "value");
  emitter.emit("value", 1);
  emitter.emit("value", 2, 3);
  assert.deepEqual(await iterator.next(), { value: [1], done: false });
  assert.deepEqual(await iterator.next(), { value: [2, 3], done: false });
  const expected = new Error("iterator error");
  emitter.emit("error", expected);
  await assert.rejects(iterator.next(), error => error === expected);
  assert.equal(emitter.listenerCount("value"), 0);
  assert.equal(emitter.listenerCount("error"), 0);
});
