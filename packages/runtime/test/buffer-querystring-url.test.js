import "./setup-runtime.js";
import test from "node:test";
import assert from "node:assert/strict";
import nodeBuffer from "node:buffer";
import nodeQuerystring from "node:querystring";
import nodeUrl from "node:url";
import bufferModule from "../dist/source/buffer.js";
import querystring from "../dist/source/querystring.js";
import urlModule from "../dist/source/url.js";

const { Buffer } = bufferModule;

test("Buffer subset agrees with Node for encodings and common byte operations", () => {
  for (const [value, encoding] of [
    ["hello 🌍", "utf8"],
    ["00ff10", "hex"],
    ["AQIDBA==", "base64"],
    ["café", "latin1"],
    ["snow", "utf16le"],
  ]) {
    const actual = Buffer.from(value, encoding);
    const expected = nodeBuffer.Buffer.from(value, encoding);
    assert.deepEqual([...actual], [...expected]);
    assert.equal(actual.toString(encoding), expected.toString(encoding));
  }
  const bytes = Buffer.alloc(4, "ab");
  assert.equal(bytes.toString(), "abab");
  assert.equal(Buffer.concat([Buffer.from("ab"), Buffer.from("cd")]).toString(), "abcd");
  const view = Buffer.from(Uint8Array.from([0, 1, 2, 3]).buffer, 1, 2);
  assert.deepEqual([...view], [1, 2]);
  view.writeUInt16BE(0x1234, 0);
  assert.equal(view.readUInt16BE(0), 0x1234);
  assert.equal(Buffer.from("abcd").indexOf("bc"), 1);
  assert.equal(Buffer.from("abcd").indexOf(""), 0);
  assert.equal(Buffer.from("abcd").lastIndexOf(""), 4);
  assert.throws(() => Buffer.alloc(1).writeUInt8(256, 0), RangeError);
  assert.deepEqual([...Buffer.alloc(4, Buffer.from([1, 2]))], [1, 2, 1, 2]);
  assert.equal(Buffer.isBuffer(bytes), true);
  assert.deepEqual(bytes.toJSON(), { type: "Buffer", data: [97, 98, 97, 98] });
  assert.throws(() => Buffer.alloc(-1), RangeError);
});

test("querystring parse and stringify match the legacy Node subset", () => {
  for (const input of [
    "a=one+two&x=1&x=2&empty&__proto__=safe",
    "a=%E0%A4%A&b=%26%3D",
    "a:1|b:2|b:3",
    "",
  ]) {
    const custom = input.includes(":") ? querystring.parse(input, "|", ":") : querystring.parse(input);
    const expected = input.includes(":") ? nodeQuerystring.parse(input, "|", ":") : nodeQuerystring.parse(input);
    assert.deepEqual(Object.fromEntries(Object.entries(custom)), Object.fromEntries(Object.entries(expected)));
  }
  const value = { a: ["one two", "x"], empty: null, missing: undefined, quote: "!'()*" };
  assert.equal(querystring.stringify(value), nodeQuerystring.stringify(value));
  assert.equal(querystring.stringify({ a: [1, 2] }, ";", ":"), "a:1;a:2");
  assert.equal(Object.getPrototypeOf(querystring.parse("__proto__=safe")), null);
  assert.deepEqual(querystring.parse("a=1&b=2&c=3", "&", "=", { maxKeys: 2 }), Object.assign(Object.create(null), { a: "1", b: "2" }));
});

test("URL helpers preserve URLSearchParams and local file path semantics", () => {
  const params = new urlModule.URLSearchParams("a=1&a=2&space=hello+world");
  assert.deepEqual(params.getAll("a"), ["1", "2"]);
  params.append("new", "a b");
  assert.equal(params.get("space"), "hello world");
  assert.equal(params.toString(), new URLSearchParams("a=1&a=2&space=hello+world&new=a+b").toString());

  for (const file of ["/tmp/a b.txt", "/tmp/100% done.txt", "/tmp/#hash?.txt", "/tmp/中文.txt"]) {
    const ours = urlModule.pathToFileURL(file);
    const expected = nodeUrl.pathToFileURL(file);
    assert.equal(ours.href, expected.href);
    assert.equal(urlModule.fileURLToPath(ours), nodeUrl.fileURLToPath(expected));
  }
  assert.throws(() => urlModule.fileURLToPath("https://example.test/file"), { code: "ERR_INVALID_URL_SCHEME" });
  assert.throws(() => urlModule.fileURLToPath("file:///tmp/a%2Fb"), { code: "ERR_INVALID_FILE_URL_PATH" });
  if (process.platform === "win32") {
    assert.equal(urlModule.fileURLToPath("file://server/share/file"), nodeUrl.fileURLToPath("file://server/share/file"));
  } else {
    assert.throws(() => urlModule.fileURLToPath("file://server/share/file"), { code: "ERR_INVALID_FILE_URL_HOST" });
  }
});
