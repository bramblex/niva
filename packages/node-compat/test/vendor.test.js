import test from "node:test";
import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import vm from "node:vm";
import { fileURLToPath } from "node:url";
import path from "node:path";

const packageRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const bundlePath = path.join(packageRoot, "src/runtime/vendor.js");

async function loadVendor() {
  const source = await readFile(bundlePath, "utf8");
  const sandbox = {
    setTimeout,
    clearTimeout,
    queueMicrotask,
    console,
    AbortController,
    AbortSignal,
  };
  const context = vm.createContext(sandbox);
  vm.runInContext("globalThis.self = globalThis; globalThis.window = globalThis;", context);
  vm.runInContext(source, context, { filename: "vendor.js" });
  const runtime = vm.runInContext('globalThis[Symbol.for("niva.node-compat.runtime")]', context);
  assert.ok(runtime);
  return { context, vendor: runtime.vendor };
}

test("browser vendor bundle attaches APIs without Node or Niva globals", async () => {
  const { context, vendor } = await loadVendor();
  assert.deepEqual(Object.keys(vendor).sort(), [
    "Buffer",
    "HTTPParser",
    "StringDecoder",
    "acorn",
    "acornWalk",
    "compression",
    "dnsPacket",
    "hashes",
    "legacyUrl",
    "stream",
  ]);
  assert.equal(typeof vendor.Buffer.from, "function");
  assert.equal(typeof vendor.stream.Readable, "function");
  assert.equal(typeof vendor.stream.PassThrough, "function");
  assert.equal(typeof vendor.stream.promises.pipeline, "function");
  assert.equal(typeof vendor.StringDecoder, "function");
  assert.deepEqual(Object.keys(vendor.hashes).sort(), [
    "hmac", "md5", "pbkdf2", "pbkdf2Async", "scrypt", "scryptAsync",
    "sha1", "sha256", "sha384", "sha512",
  ]);
  assert.deepEqual(Object.keys(vendor.compression).sort(), ["gunzip", "gunzipSync", "gzip", "gzipSync"]);
  assert.equal(typeof vendor.hashes.scryptAsync, "function");
  assert.equal(typeof vendor.compression.gunzipSync, "function");
  assert.equal(typeof vendor.dnsPacket.decode, "function");
  assert.equal(typeof vendor.HTTPParser, "function");
  assert.equal(vm.runInContext('"process" in globalThis', context), false);
  assert.equal(vm.runInContext('"require" in globalThis', context), false);
  assert.equal(vm.runInContext('"Niva" in globalThis', context), false);
  assert.equal(vm.runInContext('"Buffer" in globalThis', context), false);
});

test("vendor Buffer, SHA-256 and fflate round-trip work in a browser-like realm", async () => {
  const { vendor } = await loadVendor();
  const input = vendor.Buffer.from("abc", "utf8");
  assert.equal(input.toString("utf8"), "abc");
  assert.equal(
    vendor.Buffer.from(vendor.hashes.sha256(input)).toString("hex"),
    "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad",
  );
  const zipped = vendor.compression.gzipSync(input);
  assert.equal(vendor.Buffer.from(vendor.compression.gunzipSync(zipped)).toString("utf8"), "abc");
});

test("vendor PassThrough carries Buffer chunks", async () => {
  const { vendor } = await loadVendor();
  const pass = new vendor.stream.PassThrough();
  const chunks = [];
  pass.on("data", (chunk) => chunks.push(chunk));
  const ended = new Promise((resolve, reject) => {
    pass.on("end", resolve);
    pass.on("error", reject);
  });
  pass.end(vendor.Buffer.from("stream"));
  await ended;
  assert.equal(vendor.Buffer.concat(chunks).toString("utf8"), "stream");
});

test("vendor DNS packet and HTTP response parser handle synthetic wire/text fixtures", async () => {
  const { vendor } = await loadVendor();
  const query = vendor.dnsPacket.encode({
    type: "query",
    id: 7,
    flags: vendor.dnsPacket.RECURSION_DESIRED,
    questions: [{ type: "A", name: "example.com" }],
  });
  assert.equal(vendor.dnsPacket.decode(query).questions[0].name, "example.com");

  const parser = new vendor.HTTPParser(vendor.HTTPParser.RESPONSE);
  let statusCode;
  let body = "";
  let complete = false;
  parser.onHeadersComplete = (info) => { statusCode = info.statusCode; };
  parser.onBody = (chunk) => { body += chunk.toString(); };
  parser.onMessageComplete = () => { complete = true; };
  const response = vendor.Buffer.from("HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\nok");
  assert.equal(parser.execute(response), response.length);
  assert.equal(statusCode, 200);
  assert.equal(body, "ok");
  assert.equal(complete, true);
});
