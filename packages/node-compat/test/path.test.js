import test from "node:test";
import assert from "node:assert/strict";
import nodePath from "node:path";
import path, { posix, win32, setCwd, getCwd } from "../src/path.js";

test("default path object has Node's path surface and host separator", () => {
  assert.equal(path.sep, nodePath.sep);
  assert.equal(path.delimiter, nodePath.delimiter);
  assert.equal(path.posix.sep, "/");
  assert.equal(path.win32.sep, "\\");
  assert.equal(typeof path.matchesGlob, "function");
});

test("POSIX operations match Node path semantics for common roots and segments", () => {
  const cases = [
    "", "/", "/foo", "/foo/bar", "//a", "///a", "/a//b/../c/", "foo", "foo/", "foo/bar",
    "./foo", "./foo/", "../foo", "../", ".", "..", ".hidden", "archive.tar.gz",
  ];
  for (const value of cases) {
    for (const method of ["normalize", "isAbsolute", "dirname", "basename", "extname", "parse"]) {
      assert.deepEqual(posix[method](value), nodePath.posix[method](value), `${method}(${JSON.stringify(value)})`);
    }
  }
  for (const [from, to] of [["/a/b", "/a/c"], ["/a", "/a/b"], ["a/b", "a/c"], ["foo", "bar"]]) {
    assert.equal(posix.relative(from, to), nodePath.posix.relative(from, to));
  }
  assert.equal(posix.format(posix.parse("./dir/name.ext")), nodePath.posix.format(nodePath.posix.parse("./dir/name.ext")));
  assert.equal(posix.join("/a", "b", "..", "c"), "/a/c");
  assert.equal(posix.normalize("a/../../b"), "../b");
});

test("win32 operations match Node path semantics for drives and UNC paths", () => {
  const cases = [
    "", "C:", "C:\\", "C:\\foo", "C:\\foo\\bar", "C:foo", "C:foo\\bar",
    "\\foo", "/foo/bar", "\\\\server\\share", "\\\\server\\share\\",
    "\\\\server\\share\\dir\\file.txt", "\\\\?\\UNC\\server\\share\\foo", "a\\..\\b", ".\\file", "..\\file",
  ];
  for (const value of cases) {
    for (const method of ["normalize", "isAbsolute", "dirname", "basename", "extname", "parse"]) {
      assert.deepEqual(win32[method](value), nodePath.win32[method](value), `${method}(${JSON.stringify(value)})`);
    }
  }
  for (const [from, to] of [
    ["C:\\a\\b", "C:\\a\\c"],
    ["C:\\a", "C:\\a\\b"],
    ["C:\\a", "D:\\b"],
    ["\\\\server\\share\\a", "\\\\server\\share\\b"],
  ]) {
    assert.equal(win32.relative(from, to), nodePath.win32.relative(from, to));
  }
  assert.equal(win32.join("C:\\a", "b", "..", "c"), "C:\\a\\c");
});

test("resolve uses the configured working directory and validates inputs", () => {
  const original = getCwd();
  try {
    setCwd("/workspace/project");
    assert.equal(posix.resolve("src", "index.js"), "/workspace/project/src/index.js");
    setCwd("D:\\workspace\\project");
    assert.equal(win32.resolve("src", "index.js"), "D:\\workspace\\project\\src\\index.js");
    assert.equal(win32.resolve("\\rooted"), "D:\\rooted");
  } finally {
    setCwd(original);
  }
  assert.throws(() => posix.join("ok", 1), TypeError);
});

test("matchesGlob handles common path wildcards", () => {
  assert.equal(posix.matchesGlob("src/app/index.js", "src/**/*.js"), true);
  assert.equal(posix.matchesGlob("src/app/index.ts", "src/**/*.js"), false);
  assert.equal(posix.matchesGlob("file7.txt", "file?.txt"), true);
  assert.equal(win32.matchesGlob("SRC\\APP\\INDEX.JS", "src/**/*.js"), true);
});
