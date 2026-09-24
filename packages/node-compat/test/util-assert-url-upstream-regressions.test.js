import test from "node:test";
import assert from "node:assert/strict";
import util from "../src/util.js";
import compatAssert from "../src/assert.js";
import url from "../src/url.js";

test("util format options, promisify callback names, and callbackify context follow Node shapes", async () => {
  assert.equal(util.format(), "");
  assert.equal(util.format("%d %s", -0, 42n), "-0 42n");
  assert.equal(
    util.formatWithOptions({ numericSeparator: true }, "%d %i", 118059162071741130342, 123123123),
    "118_059_162_071_741_140_000 123_123_123",
  );

  function reads(callback) { callback(null, 1, 2); }
  reads[util.customPromisifyArgs] = ["first", "second"];
  assert.deepEqual(await util.promisify(reads)(), { first: 1, second: 2 });

  const holder = {
    task(value) {
      assert.equal(this, holder);
      return Promise.resolve(value);
    },
  };
  const callbackValue = await new Promise((resolve, reject) => {
    util.callbackify(holder.task).call(holder, 42, function (error, value) {
      try {
        assert.ifError(error);
        assert.equal(this, holder);
        resolve(value);
      } catch (caught) { reject(caught); }
    });
  });
  assert.equal(callbackValue, 42);
});

test("assert fail overloads and async assertion return shapes match Node", async () => {
  assert.throws(() => compatAssert.fail(), {
    code: "ERR_ASSERTION",
    name: "AssertionError",
    message: "Failed",
    operator: "fail",
    generatedMessage: true,
  });
  assert.throws(() => compatAssert.fail(new TypeError("given")), { name: "TypeError", message: "given" });
  await compatAssert.rejects(Promise.reject(new TypeError("async")), TypeError);
  await compatAssert.doesNotReject(Promise.resolve());
});

test("deep equality does not mistake Symbol.toStringTag spoofing for CryptoKey branding", () => {
  const makeSpoof = () => {
    const value = {type:"secret", extractable:true, algorithm:{name:"HMAC", hash:{name:"SHA-256"}}, usages:["sign"]};
    Object.defineProperty(value, Symbol.toStringTag, {value:"CryptoKey"});
    return value;
  };
  assert.equal(util.isDeepStrictEqual(makeSpoof(), makeSpoof()), true);
});

test("URL file conversion validates inputs and preserves escaped path characters", () => {
  assert.throws(() => url.fileURLToPath(null), { code: "ERR_INVALID_ARG_TYPE", name: "TypeError" });
  assert.throws(() => url.pathToFileURL({ toString: () => "/tmp/file" }), { code: "ERR_INVALID_ARG_TYPE", name: "TypeError" });
  assert.equal(url.pathToFileURL("/tmp/folder/").href.endsWith("/"), true);
  assert.equal(url.pathToFileURL("/tmp/back\\slash").href.endsWith("/back%5Cslash"), true);
});
