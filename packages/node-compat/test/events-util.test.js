import test from "node:test";
import assert from "node:assert/strict";
import EventEmitter from "node:events";
import util from "node:util";
import events from "../src/events.js";
import nodeUtil from "../src/util.js";

test("EventEmitter handles order, once, removal, symbols, and unhandled errors", () => {
  const emitter = new events.EventEmitter();
  const symbol = Symbol("private");
  const seen = [];
  function first(value) { seen.push([this, "first", value]); }
  emitter.on("data", first);
  emitter.prependOnceListener("data", (value) => seen.push([emitter, "once", value]));
  emitter.emit("data", 1);
  emitter.emit("data", 2);
  assert.deepEqual(seen.map((entry) => entry.slice(1)), [["once", 1], ["first", 1], ["first", 2]]);
  assert.equal(seen[1][0], emitter);
  assert.equal(emitter.listenerCount("data"), 1);
  assert.deepEqual(emitter.listeners("data"), [first]);
  emitter.on(symbol, first);
  assert.deepEqual(emitter.eventNames(), ["data", symbol]);
  emitter.off("data", first).removeAllListeners(symbol);
  assert.equal(emitter.emit("data"), false);
  assert.throws(() => emitter.emit("error", new Error("unhandled")), /unhandled/);
  const nodeEmitter = new EventEmitter();
  assert.equal(new events.EventEmitter().setMaxListeners(0).getMaxListeners(), 0);
  assert.equal(events.EventEmitter.listenerCount(nodeEmitter, "none"), 0);
  assert.equal(new events.EventEmitter({ captureRejections: true })._captureRejections, true);
});

test("util.format covers common placeholders and extra inspected arguments", () => {
  const cases = [
    ["%s %d %i %f", "word", "2.5", "7px", "1.25"],
    ["%j", { a: 1 }],
    ["%o %O", { a: { b: 1 } }, { a: { b: 1 } }],
    ["%cfoo", "red"],
    ["%cfoo", "red", "extra"],
    ["100%%", "extra"],
    [{ a: 1 }, [1, 2]],
  ];
  for (const args of cases) assert.equal(nodeUtil.format(...args), util.format(...args));
  const circular = {}; circular.self = circular;
  assert.match(nodeUtil.format("%j", circular), /Circular/);
  assert.match(nodeUtil.inspect(circular), /Circular/);
  assert.match(nodeUtil.inspect({ n: 1 }, { colors: true }), /\u001b\[/);
});

test("promisify and callbackify preserve receiver, errors, and async callback timing", async () => {
  const context = {
    offset: 4,
    add(value, callback) { callback(null, value + this.offset); },
    fail(callback) { callback(new Error("failure")); },
  };
  assert.equal(await nodeUtil.promisify(context.add).call(context, 3), 7);
  await assert.rejects(nodeUtil.promisify(context.fail).call(context), /failure/);
  const callbackified = nodeUtil.callbackify(async (value) => value * 2);
  let called = false;
  const result = new Promise((resolve, reject) => callbackified(5, (error, value) => {
    called = true;
    if (error) reject(error); else resolve(value);
  }));
  assert.equal(called, false);
  assert.equal(await result, 10);
  const falsy = nodeUtil.callbackify(() => Promise.reject(null));
  await new Promise((resolve) => falsy((error) => {
    assert.equal(error.code, "ERR_FALSY_VALUE_REJECTION");
    assert.equal(error.reason, null);
    resolve();
  }));
});

test("deep equality checks cycles, prototypes, maps, sets, and typed arrays", () => {
  const a = { values: [1, 2] }; a.self = a;
  const b = { values: [1, 2] }; b.self = b;
  assert.equal(nodeUtil.isDeepStrictEqual(a, b), true);
  assert.equal(nodeUtil.isDeepStrictEqual([1], [1, 2]), false);
  assert.equal(nodeUtil.isDeepStrictEqual(new Set([{ x: 1 }]), new Set([{ x: 1 }])), true);
  assert.equal(nodeUtil.isDeepStrictEqual(new Map([[{ k: 1 }, "v"]]), new Map([[{ k: 1 }, "v"]])), true);
  assert.equal(nodeUtil.isDeepStrictEqual(new Uint8Array([1, 2]), new Uint8Array([1, 2])), true);
  assert.equal(nodeUtil.isDeepStrictEqual(Object.create(null), {}), false);
  assert.equal(nodeUtil.isDeepStrictEqual(new URL("https://a.test"), new URL("https://b.test")), false);
});

test("deprecate emits one process warning while preserving return value and receiver", async () => {
  const warningPromise = new Promise((resolve) => process.once("warning", resolve));
  const wrapped = nodeUtil.deprecate(function (value) { return this.base + value; }, "use replacement", "DEP_TEST");
  const receiver = { base: 2, wrapped };
  assert.equal(receiver.wrapped(3), 5);
  assert.equal(receiver.wrapped(4), 6);
  const warning = await warningPromise;
  assert.equal(warning.name, "DeprecationWarning");
  assert.equal(warning.message, "use replacement");
  assert.equal(warning.code, "DEP_TEST");
});
