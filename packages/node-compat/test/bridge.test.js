import test from "node:test";
import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { fileURLToPath } from "node:url";
import path from "node:path";
import vm from "node:vm";
import {
  buffer as esmBuffer,
  child_process as esmChildProcess,
  crypto as esmCrypto,
  events as esmEvents,
  fs as esmFs,
  os as esmOs,
  path as esmPath,
  querystring as esmQuerystring,
  registerNodeCompat,
  url as esmUrl,
  util as esmUtil,
  zlib as esmZlib,
} from "@niva/node-compat";
import classicPath from "@niva/node-compat/path";
import { Buffer as esmBufferClass } from "@niva/node-compat/buffer";
import { spawn as esmSpawn } from "@niva/node-compat/child_process";
import fsPromisesDefault from "@niva/node-compat/fs/promises";
import { randomBytes } from "@niva/node-compat/crypto";
import { EventEmitter } from "@niva/node-compat/events";
import { readFile as esmReadFile } from "@niva/node-compat/fs";
import { platform as esmPlatform } from "@niva/node-compat/os";
import { parse as parseQuery } from "@niva/node-compat/querystring";
import { URL as esmURL } from "@niva/node-compat/url";
import { format as esmFormat } from "@niva/node-compat/util";
import { gzip as esmGzip } from "@niva/node-compat/zlib";
import strictAssertDefault from "@niva/node-compat/assert/strict";
import "../src/index.js";

const runtime = globalThis[Symbol.for("niva.node-compat.runtime")];

test("package ESM exports resolve and registration enforces the bridge version", () => {
  assert.equal(classicPath, esmPath);
  assert.equal(typeof esmFs.readFile, "function");
  assert.equal(typeof esmOs.platform, "function");
  assert.equal(typeof esmChildProcess.spawn, "function");
  assert.equal(typeof esmEvents.EventEmitter, "function");
  assert.equal(typeof esmUtil.format, "function");
  assert.equal(typeof esmQuerystring.parse, "function");
  assert.equal(typeof esmBuffer.Buffer, "function");
  assert.equal(typeof esmUrl.URL, "function");
  assert.equal(typeof esmCrypto.randomUUID, "function");
  assert.equal(typeof esmZlib.gzip, "function");
  assert.equal(esmBufferClass, esmBuffer.Buffer);
  assert.equal(EventEmitter, esmEvents.EventEmitter);
  assert.equal(typeof parseQuery, "function");
  assert.equal(typeof randomBytes, "function");
  assert.equal(typeof esmSpawn, "function");
  assert.equal(typeof esmReadFile, "function");
  assert.equal(typeof esmPlatform, "function");
  assert.equal(typeof esmURL, "function");
  assert.equal(typeof esmFormat, "function");
  assert.equal(typeof esmGzip, "function");
  assert.equal(fsPromisesDefault, esmFs.promises);
  assert.equal(strictAssertDefault, globalThis[Symbol.for("niva.node-compat.runtime")].strictAssert);
  assert.equal(typeof registerNodeCompat, "function");
  assert.throws(() => runtime.registerNodeCompat({ bridgeVersion: 0, registerModule() {} }), /version 1 or newer/);
});

function makeFsNiva() {
  const calls = [];
  const stats = new Map([
    ["/file.bin", { isFile: true, isDir: false, isSymlink: false, size: 3, modified: 20, accessed: 10, created: 1 }],
    ["/folder", { isFile: false, isDir: true, isSymlink: false, size: 0, modified: 20, accessed: 10, created: 1 }],
    ["/folder/a.txt", { isFile: true, isDir: false, isSymlink: false, size: 1, modified: 20, accessed: 10, created: 1 }],
  ]);
  const fs = {
    read(file, encoding) {
      calls.push(["read", file, encoding]);
      const bytes = Uint8Array.from([0, 255, 65]);
      return Promise.resolve(encoding === "base64" ? Buffer.from(bytes).toString("base64") : "read text");
    },
    write(file, content, encoding) { calls.push(["write", file, content, encoding]); return Promise.resolve(); },
    append(file, content, encoding) { calls.push(["append", file, content, encoding]); return Promise.resolve(); },
    stat(file) { calls.push(["stat", file]); return stats.has(file) ? Promise.resolve(stats.get(file)) : Promise.reject({ code: -1, message: "No such file or directory" }); },
    exists(file) { calls.push(["exists", file]); return Promise.resolve(stats.has(file)); },
    createDir(file) { calls.push(["createDir", file]); return Promise.resolve(); },
    createDirAll(file) { calls.push(["createDirAll", file]); return Promise.resolve(); },
    readDir(file) { calls.push(["readDir", file]); return Promise.resolve(file === "/folder" ? ["a.txt"] : []); },
    move(from, to, options) { calls.push(["move", from, to, options]); return Promise.resolve(); },
    remove(file) { calls.push(["remove", file]); return Promise.resolve(); },
    copy(from, to, options) { calls.push(["copy", from, to, options]); return Promise.resolve(); },
  };
  return { niva: { api: { fs } }, calls };
}

test("fs promise adapters bridge text, bytes, writes, stats, and directory entries", async () => {
  const { niva, calls } = makeFsNiva();
  const fs = runtime.createFsModule(niva);

  assert.equal(await fs.readFile("/note.txt", "utf8"), "read text");
  const binary = await fs.readFile("/file.bin", { encoding: null });
  assert.equal(runtime.buffer.Buffer.isBuffer(binary), true);
  assert.deepEqual([...binary], [0, 255, 65]);
  await fs.writeFile("/note.txt", "hello");
  await fs.writeFile("/file.bin", Uint8Array.from([0, 255, 65]));
  await fs.appendFile("/note.txt", "!");
  await assert.rejects(fs.writeFile("/note.txt", "hello", { mode: 0o600 }), { code: "ENOTSUP" });
  assert.deepEqual(calls.filter((call) => ["read", "write", "append"].includes(call[0])), [
    ["read", "/note.txt", "utf8"],
    ["read", "/file.bin", "base64"],
    ["write", "/note.txt", "hello", "utf8"],
    ["write", "/file.bin", "AP9B", "base64"],
    ["append", "/note.txt", "!", "utf8"],
  ]);

  await fs.mkdir("/new");
  await fs.mkdir("/new/deep", { recursive: true });
  await assert.rejects(fs.mkdir("/private", { mode: 0o700 }), { code: "ENOTSUP" });
  assert.deepEqual(calls.slice(-2), [["createDir", "/new"], ["createDirAll", "/new/deep"]]);
  const stats = await fs.stat("/file.bin");
  assert.equal(stats.size, 3);
  assert.equal(stats.isFile(), true);
  assert.equal(stats.isDirectory(), false);
  assert.equal(stats.mtimeMs, 20);
  assert.equal((await fs.readdir("/folder"))[0], "a.txt");
  const [entry] = await fs.readdir("/folder", { withFileTypes: true });
  assert.equal(entry.name, "a.txt");
  assert.equal(entry.parentPath, "/folder");
  assert.equal(entry.isFile(), true);
  await fs.access("/file.bin");
  await assert.rejects(fs.access("/file.bin", fs.constants.R_OK), { code: "ENOTSUP" });
  await assert.rejects(fs.stat("/missing"));
});

test("fs mutation adapters preserve bridge semantics and reject unsupported rm/cp cases", async () => {
  const { niva, calls } = makeFsNiva();
  const fs = runtime.createFsModule(niva);

  await fs.rename("/file.bin", "/renamed.bin");
  assert.deepEqual(calls.at(-1), ["move", "/file.bin", "/renamed.bin", { contentOnly: true, overwrite: true }]);
  await assert.rejects(fs.rm("/folder"), { code: "ERR_FS_EISDIR" });
  await fs.rm("/folder", { recursive: true });
  assert.deepEqual(calls.at(-1), ["remove", "/folder"]);
  const beforeForce = calls.length;
  await fs.rm("/missing", { force: true });
  assert.equal(calls.length, beforeForce + 1);
  await assert.rejects(fs.cp("/folder", "/copy"), { code: "ERR_FS_CP_DIR_TO_NON_DIR" });
  await fs.cp("/folder", "/copy", { recursive: true });
  assert.deepEqual(calls.at(-1), ["copy", "/folder", "/copy", { contentOnly: true, overwrite: true, skipExist: false }]);
  await fs.copyFile("/file.bin", "/copy.bin");
  assert.deepEqual(calls.at(-1), ["copy", "/file.bin", "/copy.bin", { overwrite: true, contentOnly: true }]);
  await fs.copyFile("/file.bin", "/exclusive.bin", fs.constants.COPYFILE_EXCL);
  assert.deepEqual(calls.at(-1), ["copy", "/file.bin", "/exclusive.bin", { overwrite: false, contentOnly: true }]);
  assert.notEqual(fs.promises, fs);
  assert.equal(fs.promises.readFile, fs.readFile);
  await assert.rejects(fs.stat("/file.bin", { bigint: true }), { code: "ENOTSUP" });
});

test("os maps Niva native values to Node platform and architecture names", async () => {
  const os = runtime.createOsModule({
    api: {
      os: {
        info: () => Promise.resolve({ os: "Mac OS X", arch: "aarch64", version: "15" }),
        dirs: () => Promise.resolve({ temp: "/tmp", home: "/Users/test" }),
      },
    },
  });
  assert.equal(await os.platform(), "darwin");
  assert.equal(await os.arch(), "arm64");
  assert.equal(await os.homedir(), "/Users/test");
  assert.equal(await os.tmpdir(), "/tmp");
  assert.equal((await os.info()).version, "15");
  assert.equal((await os.dirs()).temp, "/tmp");
});

function makeProcessNiva(status = 0) {
  const calls = [];
  const sent = [];
  const niva = {
    stream(method, args, handlers) {
      calls.push([method, args]);
      return {
        id: 7,
        cancel() { calls.push(["cancel"]); },
        promise: Promise.resolve().then(async () => {
          handlers.onBlob(new Blob(["hello "]), false);
          handlers.onBlob(new Blob(["world"]), false);
          handlers.onBlob(new Blob(["warn"]), true);
          await Promise.resolve();
          return { status };
        }),
      };
    },
    streamSend(id, data, end) { sent.push([id, [...data], end]); return true; },
  };
  return { niva, calls, sent };
}

test("spawn exposes event streams, stdin, status, and bridge limitations", async () => {
  const { niva, calls, sent } = makeProcessNiva();
  const childProcess = runtime.createChildProcessModule(niva);
  const child = childProcess.spawn("cat", [], { cwd: "/work", env: { MODE: "test" } });
  const stdout = [];
  const stderr = [];
  const events = [];
  child.stdout.on("data", (chunk) => stdout.push(chunk));
  child.stderr.on("data", (chunk) => stderr.push(chunk));
  child.on("spawn", () => events.push("spawn"));
  child.on("close", (code) => events.push(["close", code]));
  assert.equal(child.stdin.write("input"), true);
  child.stdin.end();
  assert.deepEqual(await child.completion, { status: 0, detached: false });
  assert.equal(runtime.buffer.Buffer.isBuffer(stdout[0]), true);
  assert.deepEqual(stdout.map((chunk) => [...chunk]), [[104, 101, 108, 108, 111, 32], [119, 111, 114, 108, 100]]);
  assert.deepEqual(stderr.map((chunk) => [...chunk]), [[119, 97, 114, 110]]);
  assert.deepEqual(sent, [[7, [105, 110, 112, 117, 116], false], [7, [], true]]);
  assert.deepEqual(events, ["spawn", ["close", 0]]);
  assert.deepEqual(calls[0], ["process.execStream", ["cat", [], { currentDir: "/work", env: { MODE: "test" } }]]);
  assert.equal(child.kill(), false);
  assert.throws(() => childProcess.spawn("cat", [], { timeout: 20 }), { code: "ENOTSUP" });
});

test("exec runs through the shell and simulates callback errors with captured output", async () => {
  const { niva, calls, sent } = makeProcessNiva(7);
  const childProcess = runtime.createChildProcessModule(niva);
  const callbackResult = new Promise((resolve) => {
    const child = childProcess.exec("echo hello", { cwd: "/work" }, (error, stdout, stderr) => resolve({ error, stdout, stderr }));
    child.result.catch(() => {});
  });
  const result = await callbackResult;
  assert.equal(calls[0][0], "process.execStream");
  assert.deepEqual(calls[0][1][1], ["-c", "echo hello"]);
  assert.equal(calls[0][1][2].currentDir, "/work");
  assert.equal(result.error.status, 7);
  assert.equal(result.error.code, 7);
  assert.equal(result.stdout, "hello world");
  assert.equal(result.stderr, "warn");
  assert.deepEqual(sent, [[7, [], true]]);
  assert.throws(() => childProcess.exec("echo", { timeout: 1 }), { code: "ENOTSUP" });
});

test("classic entry registers the four modules and async cwd bootstrap", async () => {
  const packageRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
  const source = await readFile(path.join(packageRoot, "dist/niva-node-compat.js"), "utf8");
  const modules = new Map();
  const niva = {
    bridgeVersion: 1,
    registerModule(name, value) { modules.set(name, value); },
    require(name) {
      if (!modules.has(name)) throw new Error("unknown module: " + name);
      return modules.get(name);
    },
    api: { process: { currentDir: () => Promise.resolve("/workspace/app") } },
  };
  const context = vm.createContext({ Niva: niva, Symbol, Promise, Map, Set, TextEncoder, TextDecoder, Blob });
  vm.runInContext(source, context, { filename: "niva-node-compat.js" });
  await context.NivaNodeCompatReady;
  for (const name of ["path", "os", "fs", "fs/promises", "child_process", "events", "util", "querystring", "buffer", "url", "crypto", "zlib", "http", "https", "assert", "assert/strict", "stream", "stream/promises", "node:path", "node:os", "node:fs", "node:child_process", "node:events", "node:util", "node:querystring", "node:buffer", "node:url", "node:crypto", "node:zlib", "node:http", "node:https", "node:assert", "node:assert/strict", "node:stream", "node:stream/promises"]) {
    assert.ok(modules.has(name), `registered ${name}`);
  }
  assert.equal(modules.get("path").resolve("notes.txt"), "/workspace/app/notes.txt");
  assert.notEqual(modules.get("fs/promises"), modules.get("fs"));
  assert.equal(modules.get("fs/promises").readFile, modules.get("fs").readFile);
  assert.equal(niva.require("child_process"), modules.get("child_process"));
  assert.equal(context.Buffer, modules.get("buffer").Buffer);
});

test("classic registration honors the selected-module allowlist", async () => {
  const packageRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
  const source = await readFile(path.join(packageRoot, "dist/niva-node-compat.js"), "utf8");
  const modules = new Map();
  const niva = {
    bridgeVersion: 1,
    registerModule(name, value) { modules.set(name, value); },
    require(name) {
      if (!modules.has(name)) throw new Error("niva: unknown module '" + name + "'");
      return modules.get(name);
    },
    api: { process: { currentDir: () => Promise.resolve("/workspace/app") } },
  };
  const context = vm.createContext({ Niva: niva, __niva_node_compat_modules: ["path", "http"], Symbol, Promise, Map, Set, TextEncoder, TextDecoder, Blob });
  vm.runInContext(source, context, { filename: "niva-node-compat.js" });
  await context.NivaNodeCompatReady;
  assert.deepEqual([...modules.keys()], ["path", "node:path", "http", "node:http"]);
  assert.equal(context.Buffer, undefined);
  assert.throws(() => niva.require("fs"), /unknown module/);
  assert.throws(() => niva.require("node:https"), /unknown module/);
});
