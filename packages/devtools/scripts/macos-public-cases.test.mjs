import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import test from "node:test";
import vm from "node:vm";
import ts from "typescript";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../../..");
const read = (relative) => readFileSync(path.join(root, relative), "utf8");

test("every documented public Niva API has exactly one macOS case definition", () => {
  const source = ts.createSourceFile("Niva_zh.d.ts", read("packages/types/Niva_zh.d.ts"), ts.ScriptTarget.Latest, true);
  const interfaces = new Map(source.statements.filter(ts.isInterfaceDeclaration).map((item) => [item.name.text, item]));
  const api = interfaces.get("NivaObj").members.find((item) => item.name.getText(source) === "api");
  assert.ok(api && ts.isTypeLiteralNode(api.type));
  const expected = api.type.members.flatMap((namespace) => {
    const name = namespace.name.getText(source);
    return interfaces.get(namespace.type.typeName.getText(source)).members
      .filter(ts.isMethodSignature)
      .map((method) => `${name}.${method.name.getText(source)}`);
  });

  const existingRunner = read("examples/macos-api-smoke/run.py").split("EXTENDED_WINDOW_METHODS =")[0];
  const existingPages = ["index.html", "headless.html", "secondary.html"]
    .map((name) => read(`examples/macos-api-smoke/${name}`));
  const dynamicExisting = new Set([
    "clipboard.read", "clipboard.write", "dialog.showMessage", "dialog.pickFile", "dialog.saveFile",
  ]);
  const old = expected.filter((method) =>
    dynamicExisting.has(method) || existingRunner.includes(`"${method}"`)
    || existingPages.some((page) => page.includes(`record("${method}"`)));
  const sandbox = { window: {} };
  for (const name of ["window-cases.js", "system-cases.js"]) {
    vm.runInNewContext(read(`examples/macos-api-smoke/${name}`), sandbox);
  }
  const extra = ["NivaMacWindowCases", "NivaMacSystemCases"].flatMap((name) => {
    const suite = sandbox.window[name];
    assert.ok(suite);
    return [...suite.automatic, ...suite.supervised].map((item) => {
      assert.ok(item.assertion || (item.action && item.expected && item.restore), `${item.method} has no assertion`);
      return item.method;
    });
  });
  assert.equal(extra.length, new Set(extra).size, "duplicate new API case definition");
  const cases = [...new Set(old), ...extra];
  assert.equal(cases.length, new Set(cases).size, "new API cases overlap existing cases");
  assert.deepEqual(cases.sort(), expected.sort());
  assert.equal(expected.length, 167);
});
