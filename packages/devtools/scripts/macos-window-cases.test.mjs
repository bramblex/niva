import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import test from "node:test";
import vm from "node:vm";
import ts from "typescript";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../../..");

test("macOS fixture defines a distinct case for every window and windowExtra method", () => {
  const source = ts.createSourceFile(
    "Niva_zh.d.ts",
    readFileSync(path.join(root, "packages/types/Niva_zh.d.ts"), "utf8"),
    ts.ScriptTarget.Latest,
    true,
  );
  const interfaces = new Map(source.statements.filter(ts.isInterfaceDeclaration).map((item) => [item.name.text, item]));
  const expected = [
    ...interfaces.get("NivaWindow").members.filter(ts.isMethodSignature).map((method) => `window.${method.name.getText(source)}`),
    ...interfaces.get("NivaWindowExtra").members.filter(ts.isMethodSignature).map((method) => `windowExtra.${method.name.getText(source)}`),
  ];

  const fixture = readFileSync(path.join(root, "examples/macos-api-smoke/index.html"), "utf8");
  const existing = [...fixture.matchAll(/record\("(window(?:Extra)?\.[A-Za-z0-9]+)"/g)].map((match) => match[1]);
  const sandbox = { window: {} };
  vm.runInNewContext(readFileSync(path.join(root, "examples/macos-api-smoke/window-cases.js"), "utf8"), sandbox);
  const extra = sandbox.window.NivaMacWindowCases;
  const cases = [...existing, ...extra.automatic.map((item) => item.method), ...extra.supervised.map((item) => item.method)];

  assert.equal(cases.length, new Set(cases).size, "duplicate macOS window test case");
  assert.deepEqual(cases.sort(), expected.sort());
  for (const item of extra.automatic) {
    assert.equal(typeof item.run, "function", `${item.method} has no executable case`);
    assert.ok(item.assertion.length > 12, `${item.method} has no observable assertion description`);
  }
  for (const item of extra.supervised) {
    assert.ok(item.action.trim() && item.expected.trim() && item.restore.trim(),
      `${item.method} needs action, expected result, and cleanup`);
  }
});
