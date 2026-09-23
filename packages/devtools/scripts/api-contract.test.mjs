import assert from "node:assert/strict";
import { readFileSync, readdirSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import test from "node:test";
import ts from "typescript";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../../..");

function publicApiMethods() {
  const typePath = path.join(root, "packages/types/Niva_zh.d.ts");
  const source = ts.createSourceFile(
    typePath,
    readFileSync(typePath, "utf8"),
    ts.ScriptTarget.Latest,
    true,
  );
  const interfaces = new Map(
    source.statements
      .filter(ts.isInterfaceDeclaration)
      .map((declaration) => [declaration.name.text, declaration]),
  );
  const api = interfaces.get("NivaObj")?.members.find(
    (member) => member.name?.getText(source) === "api",
  );
  assert.ok(api && ts.isTypeLiteralNode(api.type), "NivaObj.api must describe the public namespaces");

  const methods = new Set();
  for (const namespace of api.type.members) {
    const name = namespace.name.getText(source);
    const interfaceName = namespace.type.typeName.getText(source);
    const declaration = interfaces.get(interfaceName);
    assert.ok(declaration, `Missing ${interfaceName} for Niva.api.${name}`);
    for (const member of declaration.members) {
      if (ts.isMethodSignature(member)) {
        methods.add(`${name}.${member.name.getText(source)}`);
      }
    }
  }
  return methods;
}

function registeredMethods() {
  const directory = path.join(root, "crates/niva/src/app/api");
  const methods = new Set();
  for (const file of readdirSync(directory).filter((name) => name.endsWith(".rs"))) {
    const source = readFileSync(path.join(directory, file), "utf8");
    for (const match of source.matchAll(/register_(?:blocking_|stream_)?api(?:_with)?\s*\(\s*"([^"]+)"/g)) {
      methods.add(match[1]);
    }
  }
  return methods;
}

function pageOverrides() {
  const source = readFileSync(path.join(root, "crates/niva/assets/initialize_script.js"), "utf8");
  return new Set(
    [...source.matchAll(/overrideApi\(\s*"([^"]+)"\s*,\s*"([^"]+)"/g)]
      .map((match) => `${match[1]}.${match[2]}`),
  );
}

function bridgeMethods() {
  const typePath = path.join(root, "packages/types/Niva_zh.d.ts");
  const source = ts.createSourceFile(typePath, readFileSync(typePath, "utf8"), ts.ScriptTarget.Latest, true);
  const object = source.statements.find((statement) => ts.isInterfaceDeclaration(statement) && statement.name.text === "NivaObj");
  assert.ok(object);
  return new Set(object.members.filter(ts.isMethodSignature).map((method) => `Niva.${method.name.getText(source)}`));
}

test("every typed Niva.api method has a native handler or explicit page override", () => {
  const native = registeredMethods();
  const overrides = pageOverrides();
  const missing = [...publicApiMethods()].filter((method) => !native.has(method) && !overrides.has(method));
  assert.deepEqual(missing.sort(), []);
});

test("native-only API methods are the internal stream handlers", () => {
  const publicMethods = publicApiMethods();
  const nativeOnly = [...registeredMethods()].filter((method) => !publicMethods.has(method));
  assert.deepEqual(nativeOnly.sort(), [
    "fs.readStream",
    "fs.writeStream",
    "http.requestStream",
    "process.execStream",
    "resource.readStream",
  ]);
});

test("the coverage matrix tracks each public API and bridge method exactly once", () => {
  const matrix = readFileSync(path.join(root, "docs/api-test-matrix.md"), "utf8");
  const rows = [...matrix.matchAll(/^\| `(Niva(?:\.api)?\.[^`]+)` \|/gm)].map((match) => match[1]);
  const expected = [
    ...[...publicApiMethods()].map((method) => `Niva.api.${method}`),
    ...bridgeMethods(),
  ];
  assert.equal(rows.length, new Set(rows).size, "coverage matrix contains duplicate methods");
  assert.deepEqual(rows.sort(), expected.sort());
});
