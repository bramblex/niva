import { createHash } from "node:crypto";
import { mkdir, readFile, readdir, rm, writeFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { build, transform } from "esbuild";

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const packageRoot = path.resolve(scriptDir, "..");
const workspaceRoot = path.resolve(packageRoot, "../..");
const typesRoot = path.join(workspaceRoot, "packages/types");
const dist = path.join(packageRoot, "dist");
const distTypes = path.join(dist, "types");
const skipTypes = process.argv.includes("--skip-types");
const sha256 = (value) => createHash("sha256").update(value).digest("hex");

async function walk(dir) {
  const output = [];
  for (const entry of await readdir(dir, { withFileTypes: true })) {
    const absolute = path.join(dir, entry.name);
    if (entry.isDirectory()) output.push(...await walk(absolute));
    else if (entry.isFile()) output.push(absolute);
  }
  return output.sort();
}

async function generatedVendor() {
  const { spawnSync } = await import("node:child_process");
  const result = spawnSync(process.execPath, [path.join(scriptDir, "build-vendor.mjs")], { cwd: packageRoot, encoding: "utf8" });
  if (result.status !== 0) throw new Error(`Vendor build failed:\n${result.stdout || ""}${result.stderr || ""}`);
}

await generatedVendor();
const runtimeFiles = JSON.parse(await readFile(path.join(packageRoot, "runtime-files.json"), "utf8"));
const entries = Object.entries(runtimeFiles.entries || {});
if (!runtimeFiles.bootstrap || entries.length === 0) throw new Error("runtime-files.json must declare bootstrap and ESM entries");
for (const [specifier, source] of [["<bootstrap>", runtimeFiles.bootstrap], ["<packageRoot>", runtimeFiles.packageRoot], ...entries]) {
  if (!source.startsWith("src/") || source.includes("..")) throw new Error(`Invalid runtime source entry ${specifier}: ${source}`);
}

await rm(dist, { recursive: true, force: true });
await mkdir(dist, { recursive: true });
await mkdir(distTypes, { recursive: true });

const bootstrapResult = await build({
  absWorkingDir: packageRoot,
  entryPoints: [runtimeFiles.bootstrap],
  outfile: path.join(dist, "bootstrap.js"),
  bundle: true,
  format: "iife",
  platform: "browser",
  target: ["es2022"],
  minify: true,
  treeShaking: true,
  legalComments: "none",
  charset: "utf8",
  sourcemap: false,
  write: false,
});
await writeFile(path.join(dist, "bootstrap.js"), bootstrapResult.outputFiles[0].contents);

// Emit a Node-loadable, type-stripped mirror for the upstream harness and unit
// tests. Production embeds only bootstrap.js and the stable ESM facades.
const sourceFiles = await walk(path.join(packageRoot, "src"));
for (const absolute of sourceFiles.filter((file) => file.endsWith(".ts"))) {
  const relative = path.relative(path.join(packageRoot, "src"), absolute);
  const input = await readFile(absolute, "utf8");
  const transformed = await transform(input, { loader: "ts", format: "esm", target: "es2022", sourcefile: relative });
  const output = transformed.code.replace(/((?:from\s*|import\s*)["'])(\.{1,2}\/[^"']+)(["'])/g, (match, before, specifier, after) => {
    if (specifier.endsWith(".ts")) return `${before}${specifier.slice(0, -3)}.js${after}`;
    if (path.extname(specifier)) return match;
    const sourceCandidate = path.resolve(path.dirname(absolute), specifier + ".ts");
    return sourceFiles.includes(sourceCandidate) ? `${before}${specifier}.js${after}` : match;
  });
  const destination = path.join(dist, "source", relative.replace(/\.ts$/, ".js"));
  await mkdir(path.dirname(destination), { recursive: true });
  await writeFile(destination, output);
}
await mkdir(path.join(dist, "source/runtime"), { recursive: true });
await writeFile(path.join(dist, "source/runtime/vendor.js"), await readFile(path.join(packageRoot, "src/runtime/vendor.js")));

const facadeEntries = [{ in: runtimeFiles.packageRoot, out: "esm/index" }, ...entries.map(([specifier, source]) => ({
  in: source,
  out: `esm/${specifier}`,
}))];
const esmResult = await build({
  absWorkingDir: packageRoot,
  entryPoints: facadeEntries,
  outdir: dist,
  bundle: true,
  splitting: true,
  format: "esm",
  platform: "browser",
  target: ["es2022"],
  entryNames: "[dir]/[name]",
  chunkNames: "esm/chunks/[name]-[hash]",
  outExtension: { ".js": ".mjs" },
  treeShaking: true,
  legalComments: "none",
  charset: "utf8",
  sourcemap: false,
  metafile: true,
  write: false,
});
for (const file of esmResult.outputFiles) {
  await mkdir(path.dirname(file.path), { recursive: true });
  await writeFile(file.path, file.contents);
}

const modules = Object.create(null);
const shared = [];
for (const [outputPath, metadata] of Object.entries(esmResult.metafile.outputs)) {
  const route = path.relative(dist, path.resolve(packageRoot, outputPath)).split(path.sep).join("/");
  if (metadata.entryPoint) {
    if (path.resolve(packageRoot, runtimeFiles.packageRoot) === path.resolve(packageRoot, metadata.entryPoint)) continue;
    const specifier = entries.find(([, source]) => path.resolve(packageRoot, source) === path.resolve(packageRoot, metadata.entryPoint))?.[0];
    if (!specifier) throw new Error(`ESM output has an unknown entry point: ${metadata.entryPoint}`);
    modules[specifier] = [route];
  } else if (route.startsWith("esm/chunks/")) {
    shared.push(route);
  }
}
for (const [specifier] of entries) {
  if (!modules[specifier]) throw new Error(`No ESM facade was emitted for ${specifier}`);
}
const assetsManifest = { bootstrap: "bootstrap.js", shared: shared.sort(), modules };
const assetsManifestPath = path.join(dist, "runtime-assets.json");
await writeFile(assetsManifestPath, `${JSON.stringify(assetsManifest, null, 2)}\n`);

if (!skipTypes) {
  const { spawnSync } = await import("node:child_process");
  const result = spawnSync(path.join(workspaceRoot, "node_modules/.bin/tsc"), ["-p", path.join(packageRoot, "tsconfig.types.json")], { cwd: workspaceRoot, encoding: "utf8" });
  if (result.status !== 0) throw new Error(`Declaration build failed:\n${result.stdout || ""}${result.stderr || ""}`);

  const declarationRoot = path.join(distTypes, "source");
  await mkdir(path.join(distTypes, "modules"), { recursive: true });
  const facadeSources = [[".", runtimeFiles.packageRoot], ...entries];
  for (const [specifier, source] of facadeSources) {
    const relativeSource = path.relative(path.join(packageRoot, "src"), path.resolve(packageRoot, source)).replace(/\.ts$/, ".d.ts");
    const from = path.join(declarationRoot, relativeSource);
    const route = specifier === "." ? "index.d.ts" : `${specifier}.d.ts`;
    const to = path.join(distTypes, "modules", route);
    await mkdir(path.dirname(to), { recursive: true });
    const declaration = await readFile(from, "utf8");
    const linked = declaration.replaceAll('import("./contracts")', 'import("@niva/types/contracts")');
    await writeFile(to, linked);
  }

  // Generate all public/browser and opt-in Node type graphs from the exact
  // runtime declaration source. The runtime facades above point back to the
  // canonical @niva/types/contracts subpath without embedding another copy.
  const generator = path.join(typesRoot, "scripts/build-private-types.mjs");
  const privateTypes = spawnSync(process.execPath, [
    generator,
    "--source-dir", declarationRoot,
    "--runtime-files", path.join(packageRoot, "runtime-files.json"),
    "--out-dir", path.join(typesRoot, "dist"),
  ], { cwd: workspaceRoot, encoding: "utf8" });
  if (privateTypes.status !== 0) throw new Error(`Private type graph build failed:\n${privateTypes.stdout || ""}${privateTypes.stderr || ""}`);
}

const inputFiles = [...new Set([
  ...(await walk(path.join(packageRoot, "src"))),
  path.join(packageRoot, "runtime-files.json"),
  path.join(packageRoot, "package.json"),
  path.join(packageRoot, "tsconfig.json"),
  path.join(packageRoot, "scripts/build-runtime.mjs"),
  path.join(packageRoot, "scripts/build-vendor.mjs"),
  ...(await walk(typesRoot)).filter((absolute) => !absolute.startsWith(path.join(typesRoot, "dist") + path.sep)),
  path.join(typesRoot, "package.json"),
  path.join(typesRoot, "tsconfig.json"),
  path.join(workspaceRoot, "package.json"),
  path.join(workspaceRoot, "package-lock.json"),
])].sort();
const inputRecords = [];
for (const absolute of inputFiles) {
  const relative = path.relative(workspaceRoot, absolute).split(path.sep).join("/");
  inputRecords.push({ path: relative, sha256: sha256(await readFile(absolute)) });
}
const outputs = [path.join(dist, "bootstrap.js"), assetsManifestPath, ...await walk(path.join(dist, "esm")), ...await walk(path.join(dist, "source")), ...await walk(distTypes)];
const outputRecords = [];
for (const absolute of outputs) {
  const relative = path.relative(dist, absolute).split(path.sep).join("/");
  outputRecords.push({ path: relative, sha256: sha256(await readFile(absolute)) });
}
const fingerprint = sha256(inputRecords.map((entry) => `${entry.path}:${entry.sha256}`).join("\n"));
const buildManifest = {
  format: 1,
  inputFingerprint: fingerprint,
  inputs: inputRecords,
  outputs: outputRecords,
};
await writeFile(path.join(dist, "build-manifest.json"), `${JSON.stringify(buildManifest, null, 2)}\n`);

console.log(JSON.stringify({
  bootstrapBytes: (await readFile(path.join(dist, "bootstrap.js"))).byteLength,
  esmFacades: Object.keys(modules).length,
  sharedChunks: shared.length,
  inputs: inputRecords.length,
  outputArtifacts: outputRecords.length,
  typesChecked: !skipTypes,
  inputFingerprint: fingerprint,
}, null, 2));
