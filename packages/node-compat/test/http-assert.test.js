import test from "node:test";
import nodeAssert from "node:assert/strict";
import { AssertionError, deepStrictEqual, rejects, strictEqual, throws } from "../src/assert.js";
import looseAssert from "../src/assert.js";
import strictAssert from "../src/assert-strict.js";
import { pipeline } from "../src/stream.js";
import http from "../src/http.js";
import https from "../src/https.js";
import "../src/index.js";

const runtime = globalThis[Symbol.for("niva.node-compat.runtime")];

function makeHttpNiva(responses) {
  const calls = [];
  let nextId = 0;
  return {
    calls,
    niva: {
      stream(method, args, handlers) {
        calls.push([method, args]);
        const response = responses.shift() || { status: 200, headers: {}, body: "" };
        return {
          id: ++nextId,
          cancel() { response.cancelled = true; },
          promise: Promise.resolve().then(() => {
            if (response.error) throw response.error;
            handlers.onEvent("head", { status: response.status, headers: response.headers || {} });
            if (response.body) handlers.onBlob(new Blob([response.body]));
            return { status: response.status };
          }),
        };
      },
    },
  };
}

test("assert provides useful strict, deep, sync-error, and async-error checks", async () => {
  looseAssert.equal(1, "1");
  looseAssert.deepEqual({ value: 1 }, { value: "1" });
  nodeAssert.throws(() => strictAssert.equal(1, "1"), AssertionError);
  nodeAssert.throws(() => strictAssert.deepEqual({ value: 1 }, { value: "1" }), AssertionError);
  strictEqual(NaN, NaN);
  deepStrictEqual({ a: [1, 2] }, { a: [1, 2] });
  throws(() => { throw new TypeError("bad input"); }, TypeError);
  throws(() => { throw new Error("bad input"); }, /bad input/);
  nodeAssert.throws(() => strictEqual(-0, 0), AssertionError);
  nodeAssert.throws(() => deepStrictEqual({ a: 1 }, { a: 2 }), AssertionError);
  await rejects(Promise.reject(new Error("async failure")), /async failure/);
  await nodeAssert.rejects(rejects(Promise.resolve(1)), AssertionError);
});

test("ESM subpaths expose HTTP, assert, and stream adapters", async () => {
  const pkg = await import("@niva/node-compat");
  const httpPath = await import("@niva/node-compat/http");
  const httpsPath = await import("@niva/node-compat/https");
  const assertPath = await import("@niva/node-compat/assert/strict");
  const streamPath = await import("@niva/node-compat/stream/promises");
  nodeAssert.equal(typeof pkg.http.get, "function");
  nodeAssert.equal(typeof pkg.https.request, "function");
  nodeAssert.equal(typeof httpPath.createServer, "function");
  nodeAssert.equal(typeof httpsPath.get, "function");
  nodeAssert.equal(typeof assertPath.deepStrictEqual, "function");
  nodeAssert.equal(typeof streamPath.pipeline, "function");
  nodeAssert.equal(typeof pipeline, "function");
  nodeAssert.equal(typeof http.get, "function");
  nodeAssert.equal(typeof https.get, "function");
});
