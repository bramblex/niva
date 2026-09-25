import { build } from "esbuild";
import { readFile, readdir, mkdir, writeFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const packageRoot = path.resolve(scriptDir, "..");
const entryPoint = "src/vendor/index.ts";
const outputFile = "src/runtime/vendor.js";
const noticesFile = "src/vendor/THIRD_PARTY_NOTICES.txt";

async function readJson(filePath) {
  return JSON.parse(await readFile(filePath, "utf8"));
}

async function packageForInput(input) {
  // esbuild browser:false virtual modules contain no source/package files.
  if (input.startsWith("(disabled):")) return null;
  const absolute = path.resolve(packageRoot, input);
  const marker = `${path.sep}node_modules${path.sep}`;
  const markerIndex = absolute.lastIndexOf(marker);
  if (markerIndex < 0) return null;
  const packageParts = absolute.slice(markerIndex + marker.length).split(path.sep);
  const packageLength = packageParts[0].startsWith("@") ? 2 : 1;
  const dir = path.join(absolute.slice(0, markerIndex + marker.length), ...packageParts.slice(0, packageLength));
  return { dir, data: await readJson(path.join(dir, "package.json")) };
}

async function licenseText(dir) {
  const files = await readdir(dir);
  const candidate = files
    .filter((name) => /^(licen[cs]e|copying)(?:[-_.].*)?$/i.test(name))
    .sort((a, b) => {
      const rank = (name) => (name.toUpperCase() === "LICENSE" ? 0 : 1);
      return rank(a) - rank(b) || a.localeCompare(b);
    })[0];
  return candidate ? { file: candidate, text: (await readFile(path.join(dir, candidate), "utf8")).trim() } : null;
}

const result = await build({
  absWorkingDir: packageRoot,
  entryPoints: [entryPoint],
  outfile: outputFile,
  bundle: true,
  format: "iife",
  platform: "browser",
  target: ["es2022"],
  minify: true,
  treeShaking: true,
  legalComments: "none",
  charset: "utf8",
  sourcemap: false,
  metafile: true,
  write: false,
});

const externalImports = [];
for (const [input, info] of Object.entries(result.metafile.inputs)) {
  for (const imported of info.imports || []) {
    if (imported.external) externalImports.push({ input, path: imported.path });
  }
}
if (externalImports.length) {
  throw new Error(`Vendor bundle has external imports: ${JSON.stringify(externalImports)}`);
}

const packageMap = new Map();
for (const input of Object.keys(result.metafile.inputs)) {
  const found = await packageForInput(input);
  if (found) packageMap.set(found.data.name, found);
}
const packageRecords = [];
for (const [name, found] of [...packageMap.entries()].sort(([a], [b]) => a.localeCompare(b))) {
  const license = await licenseText(found.dir);
  const licenseId = typeof found.data.license === "string"
    ? found.data.license
    : found.data.license?.type || "UNSPECIFIED";
  if (!license && licenseId === "UNSPECIFIED") {
    throw new Error(`No license metadata or license file found for ${name}@${found.data.version}`);
  }
  packageRecords.push({
    name,
    version: found.data.version,
    license: licenseId,
    licenseFile: license?.file || null,
    text: license?.text || `SPDX license identifier: ${licenseId}`,
  });
}

const noticeText = packageRecords.map((record) => [
  `================================================================================`,
  `${record.name}@${record.version} — ${record.license}`,
  record.licenseFile ? `License text from ${record.licenseFile}:` : "",
  record.text,
].filter(Boolean).join("\n")).join("\n\n").split(/\r?\n/).map(line => line.trimEnd()).join("\n");
const noticeComment = [
  "",
  "// Third-party license notices for this generated vendor bundle.",
  ...noticeText.split(/\r?\n/).map((line) => `// ${line}`.trimEnd()),
  "",
].join("\n");
const bundleText = `if (!globalThis[Symbol.for("niva.node-compat.runtime")]?.vendor) {\n${result.outputFiles[0].text.trimEnd()}\n}\n${noticeComment}`;

await mkdir(path.dirname(path.join(packageRoot, outputFile)), { recursive: true });
await mkdir(path.dirname(path.join(packageRoot, noticesFile)), { recursive: true });
await writeFile(path.join(packageRoot, outputFile), bundleText, "utf8");
await writeFile(path.join(packageRoot, noticesFile), `${noticeText}\n`, "utf8");

console.log(JSON.stringify({
  outputFile,
  bytes: Buffer.byteLength(bundleText),
  externalImports: externalImports.length,
  packages: packageRecords.map(({ name, version, license }) => ({ name, version, license })),
}, null, 2));
