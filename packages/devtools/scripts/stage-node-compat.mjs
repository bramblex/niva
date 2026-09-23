import { cp, copyFile, mkdir, readdir, rm } from "node:fs/promises";
import { spawnSync } from "node:child_process";
import path from "node:path";
import { fileURLToPath } from "node:url";

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const devtoolsRoot = path.resolve(scriptDir, "..");
const repositoryRoot = path.resolve(devtoolsRoot, "../..");
const nodeCompatRoot = path.join(repositoryRoot, "packages/node-compat");
const publicCompatRoot = path.join(devtoolsRoot, "public/__niva_compat");

await mkdir(path.join(nodeCompatRoot, "dist"), { recursive: true });
const build = spawnSync(
  process.execPath,
  [path.join(nodeCompatRoot, "scripts/build-classic.mjs")],
  { cwd: nodeCompatRoot, stdio: "inherit" },
);
if (build.error) throw build.error;
if (build.status !== 0) {
  throw new Error(`NodeCompat classic build failed with status ${build.status}`);
}

await rm(publicCompatRoot, { recursive: true, force: true });
await mkdir(publicCompatRoot, { recursive: true });
await copyFile(
  path.join(nodeCompatRoot, "dist/niva-node-compat.js"),
  path.join(publicCompatRoot, "node-compat.js"),
);
await cp(path.join(nodeCompatRoot, "src"), path.join(publicCompatRoot, "src"), {
  recursive: true,
});

const stagedFiles = [];
async function collectFiles(directory, prefix = "") {
  const entries = await readdir(directory, { withFileTypes: true });
  for (const entry of entries.sort((left, right) => left.name.localeCompare(right.name))) {
    const relativePath = prefix ? `${prefix}/${entry.name}` : entry.name;
    if (entry.isDirectory()) {
      await collectFiles(path.join(directory, entry.name), relativePath);
    } else if (entry.isFile()) {
      stagedFiles.push(relativePath);
    }
  }
}
await collectFiles(publicCompatRoot);
process.stdout.write(`Staged ${stagedFiles.length} NodeCompat files in packages/devtools/public/__niva_compat\n`);
