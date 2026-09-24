import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import test from "node:test";
import ts from "typescript";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../../..");

test("macOS real-WebView fixture defines all top-level Niva bridge methods", () => {
  const typePath = path.join(root, "packages/types/Niva_zh.d.ts");
  const source = ts.createSourceFile(typePath, readFileSync(typePath, "utf8"), ts.ScriptTarget.Latest, true);
  const niva = source.statements.find((item) => ts.isInterfaceDeclaration(item) && item.name.text === "NivaObj");
  assert.ok(niva);
  const expected = niva.members.filter(ts.isMethodSignature).map((item) => `Niva.${item.name.getText(source)}`);
  const page = readFileSync(path.join(root, "examples/macos-api-smoke/bridge-top-level.html"), "utf8");
  const actual = [...page.matchAll(/record\("(Niva\.[A-Za-z]+)"/g)].map((match) => match[1]);
  assert.equal(actual.length, new Set(actual).size, "duplicate bridge method case");
  assert.deepEqual(actual.sort(), expected.sort());
  assert.equal(actual.length, 11);
});
