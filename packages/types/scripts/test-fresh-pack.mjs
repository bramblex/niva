#!/usr/bin/env node
import assert from "node:assert/strict";
import { createRequire } from "node:module";
import { execFileSync, spawnSync } from "node:child_process";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";

const require = createRequire(import.meta.url);
const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const packageRoot = path.resolve(scriptDir, "..");
const workspaceRoot = path.resolve(packageRoot, "../..");
const fixtureRoot = path.join(packageRoot, "test/fresh-pack");
const requestedDist = process.argv[2];
if (process.argv.length > 3) throw new Error("Usage: test-fresh-pack.mjs [dist-directory]");
const distRoot = requestedDist ? path.resolve(process.cwd(), requestedDist) : path.join(packageRoot, "dist");
const tsc = require.resolve("typescript/bin/tsc", { paths: [packageRoot, workspaceRoot] });
const ts = require(require.resolve("typescript", { paths: [packageRoot, workspaceRoot] }));
const nodeTypesPackage = require.resolve("@types/node/package.json", { paths: [packageRoot, workspaceRoot] });
const nodeTypesRoot = path.dirname(nodeTypesPackage);
const packageManifest = JSON.parse(fs.readFileSync(path.join(packageRoot, "package.json"), "utf8"));

assert.equal(packageManifest.peerDependencies?.["@types/node"], "22.14.0", "Node typings peer must stay version-pinned");
assert.equal(packageManifest.peerDependenciesMeta?.["@types/node"]?.optional, true, "Node typings must remain optional for browser consumers");
assert.equal(packageManifest.dependencies?.["@types/node"], undefined, "Node typings must not be auto-installed into browser projects");
const erasedImport = ts.transpileModule('import type {} from "@niva/types"; console.log(typeof Niva);', {
  compilerOptions: { module: ts.ModuleKind.ESNext, verbatimModuleSyntax: true },
}).outputText;
assert.equal(erasedImport.includes("@niva/types"), false, "type-only package imports must not create runtime imports");

for (const required of [
  "dist/Niva_zh.d.ts",
  "dist/Niva_commonjs.d.ts",
  "dist/Niva_node.d.ts",
  "dist/node-types/LICENSE",
  "dist/node-types/SNAPSHOT.json",
  "dist/node-types/node_modules/niva-internal-undici-types/LICENSE",
  "LICENSE",
]) {
  const candidate = required === "LICENSE" ? path.join(packageRoot, required) : path.join(distRoot, path.relative("dist", required));
  assert.ok(fs.existsSync(candidate), `Missing generated package artifact: ${candidate}`);
}

const tempRoot = fs.mkdtempSync(path.join(os.tmpdir(), "niva-types-packed-consumers-"));
let succeeded = false;

function run(command, args, cwd) {
  const result = spawnSync(command, args, { cwd, encoding: "utf8" });
  if (result.status !== 0) {
    throw new Error(`${command} ${args.join(" ")} failed in ${cwd}:\n${result.stdout || ""}${result.stderr || ""}`);
  }
  return result.stdout || "";
}

function installPackedTypes(consumerRoot, tarball, includeNodeTypes) {
  fs.writeFileSync(path.join(consumerRoot, "package.json"), '{"private":true}\n');
  run("npm", ["install", "--offline", "--ignore-scripts", "--no-audit", "--no-fund", "--no-save", "--package-lock=false", tarball], consumerRoot);
  const typesDirectory = path.join(consumerRoot, "node_modules/@niva/types");
  assert.ok(fs.existsSync(typesDirectory), "npm did not install the freshly packed @niva/types archive");
  const installedManifest = JSON.parse(fs.readFileSync(path.join(typesDirectory, "package.json"), "utf8"));
  assert.equal(installedManifest.peerDependenciesMeta?.["@types/node"]?.optional, true, "packed Node typings peer must be optional");
  assert.equal(installedManifest.dependencies?.["@types/node"], undefined, "packed package must not install Node typings as a dependency");
  if (includeNodeTypes) {
    const ambientDirectory = path.join(consumerRoot, "node_modules/@types");
    fs.mkdirSync(ambientDirectory, { recursive: true });
    fs.cpSync(nodeTypesRoot, path.join(ambientDirectory, "node"), { recursive: true });
    const undiciPackage = createRequire(nodeTypesPackage).resolve("undici-types/package.json");
    fs.cpSync(path.dirname(undiciPackage), path.join(consumerRoot, "node_modules/undici-types"), { recursive: true });
  } else {
    assert.equal(fs.existsSync(path.join(consumerRoot, "node_modules/@types/node")), false, "browser fixture unexpectedly has @types/node installed");
  }
}

function writeConfig(consumerRoot, fileName, sourceName, mode) {
  const compilerOptions = {
    target: "ES2022",
    module: "ESNext",
    moduleResolution: "Bundler",
    lib: ["ESNext", "DOM", "DOM.Iterable"],
    strict: true,
    skipLibCheck: false,
    noEmit: true,
    forceConsistentCasingInFileNames: true,
  };
  if (mode === "node") compilerOptions.types = ["node"];
  fs.writeFileSync(path.join(consumerRoot, fileName), `${JSON.stringify({ compilerOptions, include: [sourceName] }, null, 2)}\n`);
}

function typecheck(consumerRoot, config) {
  run(process.execPath, [tsc, "-p", config], consumerRoot);
}

try {
  const packDirectory = path.join(tempRoot, "pack");
  const packageStaging = path.join(tempRoot, "package");
  fs.mkdirSync(packDirectory, { recursive: true });
  fs.mkdirSync(packageStaging, { recursive: true });
  fs.cpSync(distRoot, path.join(packageStaging, "dist"), { recursive: true });
  for (const fileName of ["package.json", "README.md", "LICENSE"]) {
    fs.copyFileSync(path.join(packageRoot, fileName), path.join(packageStaging, fileName));
  }
  const packedJson = execFileSync("npm", ["pack", "--ignore-scripts", "--json", "--pack-destination", packDirectory], {
    cwd: packageStaging,
    encoding: "utf8",
  });
  const packed = JSON.parse(packedJson);
  assert.equal(packed.length, 1, "npm pack should create exactly one package archive");
  const tarball = path.join(packDirectory, packed[0].filename);
  assert.ok(fs.existsSync(tarball), "npm pack did not produce its reported archive");
  const packedFiles = new Set(packed[0].files.map((file) => file.path));
  for (const required of [
    "LICENSE",
    "README.md",
    "dist/node-types/LICENSE",
    "dist/node-types/SNAPSHOT.json",
    "dist/node-types/node_modules/niva-internal-undici-types/LICENSE",
  ]) {
    assert.ok(packedFiles.has(required), `Packed archive omits required license or snapshot: ${required}`);
  }

  for (const mode of ["browser-default", "browser-with-node-types", "commonjs", "node"]) {
    const consumerRoot = path.join(tempRoot, mode);
    const usesNodeTypes = mode === "browser-with-node-types" || mode === "node";
    fs.mkdirSync(consumerRoot, { recursive: true });
    const sourceName = "consumer.ts";
    fs.copyFileSync(path.join(fixtureRoot, `${mode}.ts`), path.join(consumerRoot, sourceName));
    installPackedTypes(consumerRoot, tarball, usesNodeTypes);
    const config = `tsconfig.${mode}.json`;
    writeConfig(consumerRoot, config, sourceName, usesNodeTypes ? "node" : "browser");
    typecheck(consumerRoot, config);
    console.log(`PASS fresh packed ${mode} consumer (strict, skipLibCheck=false)`);
  }

  succeeded = true;
  console.log(`Packed consumer checks passed; @types/node@${JSON.parse(fs.readFileSync(nodeTypesPackage, "utf8")).version} is external to browser fixtures.`);
} finally {
  if (succeeded) fs.rmSync(tempRoot, { recursive: true, force: true });
  else console.error(`Fresh consumer worktree retained for diagnosis: ${tempRoot}`);
}
