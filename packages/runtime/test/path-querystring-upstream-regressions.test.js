import "./setup-runtime.js";
import test from "node:test";
import assert from "node:assert/strict";
import path from "../dist/source/path.js";
import querystring from "../dist/source/querystring.js";

test("path.resolve uses the Niva main process cwd and falls back to the bridge in child windows", () => {
  const processDescriptor = Object.getOwnPropertyDescriptor(globalThis, "process");
  const nivaDescriptor = Object.getOwnPropertyDescriptor(globalThis, "Niva");
  const runtime = globalThis[Symbol.for("niva.node-compat.runtime")];
  const dynamicPath = runtime.createPathModule();
  const previousRuntimeProcess = runtime.process;
  let currentDir = "/main/first";
  let nativeDir = "/native/first";
  let processReads = 0;
  let nativeReads = 0;
  let throwFromProcess = false;
  const processError = new Error("main process cwd failed");
  const nivaProcess = {
    cwd() {
      processReads += 1;
      if (throwFromProcess) throw processError;
      return currentDir;
    },
  };
  const niva = {
    bridge: { callSync(method, args) {
      nativeReads += 1;
      assert.equal(method, "process.currentDir");
      assert.deepEqual(args, []);
      return nativeDir;
    } },
  };

  Object.defineProperty(globalThis, "process", { configurable: true, enumerable: false, writable: true, value: nivaProcess });
  runtime.process = nivaProcess;
  globalThis.Niva = niva;
  try {
    assert.equal(dynamicPath.posix.resolve("src", "index.js"), "/main/first/src/index.js");
    currentDir = "/main/second";
    assert.equal(dynamicPath.posix.resolve("src", "index.js"), "/main/second/src/index.js");
    assert.equal(processReads, 2);
    assert.equal(nativeReads, 0);

    throwFromProcess = true;
    assert.throws(() => dynamicPath.posix.resolve(), (error) => error === processError);
    assert.equal(nativeReads, 0);

    Object.defineProperty(globalThis, "process", { configurable: true, enumerable: false, writable: true, value: undefined });
    runtime.process = undefined;
    throwFromProcess = false;
    assert.equal(dynamicPath.posix.resolve("src", "index.js"), "/native/first/src/index.js");
    nativeDir = "/native/second";
    assert.equal(dynamicPath.posix.resolve("src", "index.js"), "/native/second/src/index.js");
    nativeDir = "";
    assert.equal(dynamicPath.posix.resolve(), ".");
    assert.equal(nativeReads, 3);
  } finally {
    if (processDescriptor) Object.defineProperty(globalThis, "process", processDescriptor);
    else delete globalThis.process;
    if (nivaDescriptor) Object.defineProperty(globalThis, "Niva", nivaDescriptor);
    else delete globalThis.Niva;
    runtime.process = previousRuntimeProcess;
  }
});

test("win32 normalization guards drive-like segments from changing the root", () => {
  assert.equal(path.win32.normalize("test/../C:/Windows"), ".\\C:\\Windows");
  assert.equal(path.win32.relative("\\\\foo\\baz-quux", "\\\\foo\\baz"), "..\\baz");
});

test("querystring preserves legacy escape errors and unescapeBuffer bytes", () => {
  assert.throws(() => querystring.escape("\uD801"), { code: "ERR_INVALID_URI", name: "URIError" });
  const bytes = querystring.unescapeBuffer("a+b%FF", true);
  assert.deepEqual([...bytes], [0x61, 0x20, 0x62, 0xff]);
  assert.equal(bytes.toString(), "a b\uFFFD");
});
