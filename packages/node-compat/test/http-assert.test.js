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

test("http get delivers response events and header/body promises", async () => {
  const { niva, calls } = makeHttpNiva([{ status: 201, headers: { "content-type": "application/json", "x-test": "yes" }, body: '{"ok":true}' }]);
  const httpApi = runtime.createHttpModule("http", niva);
  let callbackResponse;
  let callbackThis;
  const data = [];
  let ended = false;
  const request = httpApi.get("http://example.test/api#client-fragment", function (response) {
    callbackResponse = response;
    callbackThis = this;
    response.on("data", (chunk) => data.push(chunk));
    response.on("end", () => { ended = true; });
  });
  const response = await request.response;
  nodeAssert.equal(callbackResponse, response);
  nodeAssert.equal(callbackThis, request);
  nodeAssert.equal(response.statusCode, 201);
  nodeAssert.equal(response.headers["content-type"], "application/json");
  nodeAssert.deepEqual(await response.json(), { ok: true });
  nodeAssert.equal(data.length, 1);
  nodeAssert.equal(runtime.buffer.Buffer.isBuffer(data[0]), true);
  nodeAssert.equal(ended, true);
  nodeAssert.equal(await request.result, response);
  nodeAssert.equal(calls[0][0], "http.requestStream");
  nodeAssert.deepEqual(calls[0][1], [{ method: "GET", url: "http://example.test/api", headers: null }]);
});

test("request buffers UTF-8 writes and maps Node request options to the Niva stream", async () => {
  const { niva, calls } = makeHttpNiva([{ status: 503, headers: { "retry-after": "3" }, body: "unavailable" }]);
  const httpApi = runtime.createHttpModule("http", niva);
  const request = httpApi.request({ hostname: "api.example.test", port: 8080, path: "/submit?q=1", method: "POST", headers: { "x-first": "one" } });
  request.setHeader("x-second", "two");
  request.write("hello ");
  request.end("world");
  const response = await request.response;
  nodeAssert.equal(response.statusCode, 503);
  nodeAssert.equal(await response.text(), "unavailable");
  const sent = calls[0][1][0];
  nodeAssert.equal(sent.method, "POST");
  nodeAssert.equal(sent.url, "http://api.example.test:8080/submit?q=1");
  nodeAssert.deepEqual(Object.fromEntries(Object.entries(sent.headers)), { "x-first": "one", "x-second": "two" });
  nodeAssert.equal(sent.body, "hello world");
  nodeAssert.throws(() => httpApi.request({ url: "http://api.example.test", proxy: "http://proxy.test:8888" }), { code: "ENOTSUP" });
  nodeAssert.throws(() => httpApi.request("https://api.example.test"), { code: "ERR_INVALID_PROTOCOL" });
  const binaryRequest = httpApi.request("http://api.example.test");
  nodeAssert.throws(() => binaryRequest.write(Uint8Array.of(0xff)), { code: "ERR_HTTP_BODY_ENCODING" });
});

test("https wrappers enforce scheme and post convenience submits the body", async () => {
  const { niva, calls } = makeHttpNiva([{ status: 200, headers: {}, body: "accepted" }]);
  const httpsApi = runtime.createHttpModule("https", niva);
  const request = httpsApi.post("https://secure.example.test/submit", "payload", { headers: { "content-type": "text/plain" } });
  nodeAssert.equal((await request.result).statusCode, 200);
  nodeAssert.equal(calls[0][1][0].method, "POST");
  nodeAssert.equal(calls[0][1][0].body, "payload");
  nodeAssert.throws(() => httpsApi.get("http://plain.example.test"), { code: "ERR_INVALID_PROTOCOL" });
});

test("HTTP bridge failures reject promises and emit a request error", async () => {
  const error = new Error("connection refused");
  const httpApi = runtime.createHttpModule("http", {
    stream() { return { id: 1, cancel() {}, promise: Promise.reject(error) }; },
  });
  const request = httpApi.get("http://offline.example.test");
  let emitted;
  request.on("error", (value) => { emitted = value; });
  await nodeAssert.rejects(request.response, /connection refused/);
  await nodeAssert.rejects(request.result, /connection refused/);
  nodeAssert.equal(emitted, error);
});

test("pipeline connects a buffered HTTP response adapter to an HTTP request adapter", async () => {
  const { niva, calls } = makeHttpNiva([
    { status: 200, headers: { "content-type": "text/plain" }, body: "through pipeline" },
    { status: 201, headers: {}, body: "stored" },
    { status: 200, headers: {}, body: "callback source" },
    { status: 204, headers: {}, body: "" },
  ]);
  const httpApi = runtime.createHttpModule("http", niva);
  const response = await httpApi.get("http://source.test/value").result;
  const destination = httpApi.request("http://sink.test/value", { method: "POST" });
  const pipelineResult = await pipeline(response, destination);
  nodeAssert.equal(pipelineResult, destination);
  nodeAssert.equal(calls[1][1][0].body, "through pipeline");
  nodeAssert.equal((await destination.result).statusCode, 201);

  const callbackSource = await httpApi.get("http://source.test/callback").result;
  const callbackDestination = httpApi.request("http://sink.test/callback", { method: "POST" });
  const callbackResult = new Promise((resolve) => {
    nodeAssert.equal(pipeline(callbackSource, callbackDestination, resolve), callbackDestination);
  });
  nodeAssert.equal(await callbackResult, null);
  nodeAssert.equal(calls[3][1][0].body, "callback source");
  nodeAssert.equal((await callbackDestination.result).statusCode, 204);
});

test("ESM subpaths expose HTTP, assert, and stream adapters", async () => {
  const pkg = await import("@niva/node-compat");
  const httpPath = await import("@niva/node-compat/http");
  const httpsPath = await import("@niva/node-compat/https");
  const assertPath = await import("@niva/node-compat/assert/strict");
  const streamPath = await import("@niva/node-compat/stream/promises");
  nodeAssert.equal(typeof pkg.http.get, "function");
  nodeAssert.equal(typeof pkg.https.request, "function");
  nodeAssert.equal(typeof httpPath.post, "function");
  nodeAssert.equal(typeof httpsPath.get, "function");
  nodeAssert.equal(typeof assertPath.deepStrictEqual, "function");
  nodeAssert.equal(typeof streamPath.pipeline, "function");
  nodeAssert.equal(typeof pipeline, "function");
  nodeAssert.equal(typeof http.get, "function");
  nodeAssert.equal(typeof https.get, "function");
});
