import assert from "node:assert/strict";
import { existsSync, readFileSync, readdirSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import test from "node:test";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../../..");
const read = (name) => readFileSync(path.join(root, name), "utf8");

function sourceFiles(directory) {
  return readdirSync(path.join(root, directory), { withFileTypes: true }).flatMap((entry) => {
    const name = path.join(directory, entry.name);
    if (entry.isDirectory()) return sourceFiles(name);
    return /\.(?:ts|tsx)$/.test(entry.name) ? [name] : [];
  });
}

test("Devtools source uses direct Niva namespaces and Node-style imports", () => {
  const source = sourceFiles("packages/devtools/src")
    .map((name) => read(name))
    .join("\n");
  assert.doesNotMatch(source, /Niva\.api\b/);
  assert.doesNotMatch(source, /Niva\.stream(?:Send)?\b/);
  assert.doesNotMatch(source, /process\.execStream/);
  for (const specifier of [
    "node:fs/promises", "node:path", "node:os", "node:process",
    "node:child_process", "node:https",
  ]) {
    assert.ok(source.includes(specifier), `Devtools must use ${specifier}`);
  }
  for (const namespace of ["window", "dialog", "clipboard", "webview", "os"]) {
    assert.ok(source.includes(`niva.${namespace}`), `Devtools must use Niva.${namespace} directly`);
  }
});

test("Vite maps Node-style imports to the live Niva module objects", () => {
  const vite = read("packages/devtools/vite.config.ts");
  const config = JSON.parse(read("packages/devtools/niva.json"));
  assert.match(vite, /name: "niva-node-builtins"/);
  assert.match(vite, /globalThis\.Niva\.fs\.promises/);
  for (const namespace of ["path", "os", "process", "child_process", "https"]) {
    assert.match(vite, new RegExp(`globalThis\\.Niva\\.${namespace}`));
  }
  assert.equal(config.injectCommonJs, false);
  assert.equal(config.injectEsm, false);
});

test("GUI and CLI builds share the unified packager and report CLI failures on stderr", () => {
  const model = read("packages/devtools/src/models/project.model.ts");
  const panel = read("packages/devtools/src/pages/project/multi-target-build.tsx");
  const helper = read("packages/devtools/src/build-scripts/packager.ts");
  const app = read("packages/devtools/src/app.tsx");
  assert.match(model, /runPackager\(this,/);
  assert.match(model, /`--config=\$\{configPath\}`/);
  assert.match(model, /`--resource=\$\{resource\}`/);
  assert.match(panel, /runPackager\(project,/);
  assert.match(panel, /setResourceLayout/);
  assert.match(helper, /execFile\(/);
  for (const option of ["--manifest", "--config", "--resource-dir", "--output-dir", "--target"]) {
    assert.ok(helper.includes(option), `packager call is missing ${option}`);
  }
  assert.match(app, /writeCliResult\("stderr"/);
  assert.match(app, /process\.exit\(1\)/);
  assert.doesNotMatch(app.slice(app.indexOf("async function runBuildCli"), app.indexOf("export function App")), /modal\.alert/);
  for (const obsolete of ["base.ts", "build-macos.ts", "build-windows.ts", "sign-macos.ts", "sign-windows.ts"]) {
    assert.equal(existsSync(path.join(root, "packages/devtools/src/build-scripts", obsolete)), false);
  }
});

test("closing a project after save reloads persisted config before recording history", () => {
  const source = read("packages/devtools/src/models/project.model.ts");
  const dispose = source.slice(source.indexOf("async dispose()"), source.indexOf("async refresh()"));
  assert.match(dispose, /await this\.loadConfig\(\)/);
  assert.match(dispose, /await this\.app\.state\.history\.record\(this\)/);
});
