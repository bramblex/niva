import { spawnSync } from "node:child_process";
import { createHash } from "node:crypto";
import { copyFileSync, mkdirSync, mkdtempSync, readFileSync, readdirSync, rmSync, writeFileSync } from "node:fs";
import path from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

export const MAX_RUNTIME_BYTES = 3_500_000;
const RUNTIME_RELATIVE_PATH = "runtimes/niva-windows-x86_64.exe";
const TARGET = "windows-x86_64";
const SCRIPT_PATH = fileURLToPath(import.meta.url);
export function repositoryRootForScript(scriptPath) {
  return path.resolve(path.dirname(scriptPath), "..");
}
const REPO_ROOT = repositoryRootForScript(SCRIPT_PATH);

function sha256(bytes) {
  return createHash("sha256").update(bytes).digest("hex");
}

export function createRuntimeManifest(version, bytes) {
  if (bytes.length >= MAX_RUNTIME_BYTES) {
    throw new Error(
      `Windows Niva runtime is ${bytes.length} bytes; limit is strictly below ${MAX_RUNTIME_BYTES}`,
    );
  }
  if (!version) throw new Error("runtime version is required");
  return {
    schemaVersion: 1,
    version,
    runtimes: {
      [TARGET]: {
        path: RUNTIME_RELATIVE_PATH,
        version,
        sha256: sha256(bytes),
      },
    },
  };
}

export function parseBuildReport(stdout) {
  const lines = stdout.trim().split(/\r?\n/).filter(Boolean);
  if (lines.length === 0) throw new Error("niva-packager returned no build report");
  try {
    return JSON.parse(lines.at(-1));
  } catch (error) {
    throw new Error(`could not parse niva-packager report: ${error.message}`);
  }
}

export function validateBuildReport(report, { appName, version, outputDir }) {
  if (!Array.isArray(report?.results) || report.results.length !== 1) {
    throw new Error("niva-packager report must contain exactly one Windows result");
  }
  const result = report.results[0];
  if (result.target !== TARGET || result.resourceLayout !== "embedded") {
    throw new Error("niva-packager report does not describe an embedded Windows x86_64 build");
  }
  if (result.status !== "complete" || typeof result.path !== "string") {
    throw new Error(`niva-packager did not complete the Windows build: ${result.error ?? result.status}`);
  }
  if (result.runtimeVersion !== version) {
    throw new Error(`packaged runtime version ${result.runtimeVersion} does not match ${version}`);
  }
  if (!/^[a-f0-9]{64}$/i.test(result.sha256 ?? "")) {
    throw new Error("niva-packager report contains an invalid artifact SHA256");
  }

  const artifact = path.resolve(result.path);
  const expected = path.resolve(outputDir, `${appName}-${TARGET}.exe`);
  const normalize = (value) => {
    if (process.platform !== "win32") return value;
    return path.win32
      .resolve(value.replace(/^\\\\\?\\UNC\\/i, "\\\\").replace(/^\\\\\?\\/, ""))
      .toLowerCase();
  };
  if (normalize(artifact) !== normalize(expected)) {
    throw new Error(`niva-packager produced an unexpected path: ${artifact}`);
  }
  // Use our ordinary output-dir spelling after checking the report path. Rust
  // canonicalize may return an extended `\\?\` path that PowerShell's
  // Compress-Archive provider does not accept consistently.
  const bytes = readFileSync(expected);
  if (sha256(bytes).toLowerCase() !== result.sha256.toLowerCase()) {
    throw new Error("packaged executable SHA256 does not match niva-packager report");
  }
  if (bytes.length < 0x86 || bytes.toString("ascii", 0, 2) !== "MZ") {
    throw new Error("packaged output is not a PE executable");
  }
  const peOffset = bytes.readUInt32LE(0x3c);
  if (
    peOffset + 6 > bytes.length ||
    bytes.toString("binary", peOffset, peOffset + 4) !== "PE\0\0" ||
    bytes.readUInt16LE(peOffset + 4) !== 0x8664
  ) {
    throw new Error("packaged output is not a Windows x86_64 executable");
  }
  return { path: expected, sizeBytes: bytes.length, sha256: result.sha256 };
}

export function windowsArchiveName(gitVersion) {
  if (!gitVersion || /[<>:"/\\|?*\u0000-\u001f]/.test(gitVersion)) {
    throw new Error("git version cannot be used in a Windows archive filename");
  }
  return `NivaDevtools_${gitVersion.replaceAll(".", "_")}_Windows.zip`;
}

export function selectedDependencyPackageIds(resolve, rootIds) {
  if (!Array.isArray(resolve?.nodes)) throw new Error("cargo metadata has no resolved dependency nodes");
  const nodes = new Map(resolve.nodes.map((node) => [node.id, node]));
  const selected = new Set();
  const visit = (id) => {
    if (selected.has(id)) return;
    const node = nodes.get(id);
    if (!node) throw new Error(`cargo metadata omitted dependency node ${id}`);
    selected.add(id);
    if (Array.isArray(node.dependencies) && node.dependencies.every((dependency) => typeof dependency === "string")) {
      for (const dependency of node.dependencies) visit(dependency);
      return;
    }
    if (Array.isArray(node.deps)) {
      for (const dependency of node.deps) visit(dependency.pkg);
      return;
    }
    throw new Error(`cargo metadata has unsupported dependencies for node ${id}`);
  };
  for (const id of rootIds) visit(id);
  return selected;
}

function runChecked(command, args, options = {}) {
  const result = spawnSync(command, args, {
    cwd: options.cwd ?? REPO_ROOT,
    env: options.env ?? process.env,
    encoding: "utf8",
    maxBuffer: 64 * 1024 * 1024,
    windowsHide: true,
  });
  if (result.stdout && options.printStdout !== false) process.stdout.write(result.stdout);
  if (result.stderr) process.stderr.write(result.stderr);
  if (result.error) throw result.error;
  if (result.status !== 0) {
    throw new Error(`${command} exited with status ${result.status}`);
  }
  return result.stdout;
}

function cargoPackage(metadata, name) {
  const matches = metadata.packages.filter((item) => {
    const relative = path.relative(REPO_ROOT, path.resolve(item.manifest_path));
    const isLocal =
      relative !== "" &&
      relative !== ".." &&
      !relative.startsWith(`..${path.sep}`) &&
      !path.isAbsolute(relative);
    return item.name === name && isLocal;
  });
  if (matches.length !== 1) throw new Error(`cargo metadata must contain one local ${name} package`);
  return matches[0];
}

function writeLicenseMaterials(kitRoot, metadata) {
  const selected = selectedDependencyPackageIds(metadata.resolve, [
    cargoPackage(metadata, "niva").id,
    cargoPackage(metadata, "niva-packager").id,
  ]);

  mkdirSync(path.join(kitRoot, "licenses"), { recursive: true });
  const notices = [];
  for (const item of metadata.packages) {
    if (!selected.has(item.id)) continue;
    const source = item.source
      ? `https://crates.io/api/v1/crates/${item.name}/${item.version}/download`
      : "https://github.com/bramblex/niva";
    notices.push(
      `${item.name} ${item.version}\nLicense: ${item.license}\nCorresponding source: ${source}\n`,
    );
    const packageDir = path.dirname(item.manifest_path);
    const licenseNames = readdirSync(packageDir, { withFileTypes: true })
      .filter((entry) => entry.isFile() && /^(LICENSE|COPYING|NOTICE)/i.test(entry.name))
      .map((entry) => entry.name);
    if (licenseNames.length === 0) continue;
    const destination = path.join(kitRoot, "licenses", `${item.name}-${item.version}`);
    mkdirSync(destination, { recursive: true });
    for (const name of licenseNames) {
      copyFileSync(path.join(packageDir, name), path.join(destination, name));
    }
  }

  const vendorNotices = path.join(
    REPO_ROOT,
    "packages/runtime/src/vendor/THIRD_PARTY_NOTICES.txt",
  );
  const runtimeLicenseDir = path.join(kitRoot, "licenses/runtime");
  mkdirSync(runtimeLicenseDir, { recursive: true });
  copyFileSync(vendorNotices, path.join(runtimeLicenseDir, "THIRD_PARTY_NOTICES.txt"));
  notices.push("Niva runtime JavaScript dependencies\nSee licenses/runtime/THIRD_PARTY_NOTICES.txt\n");
  copyFileSync(path.join(REPO_ROOT, "LICENSE"), path.join(kitRoot, "LICENSE"));
  writeFileSync(path.join(kitRoot, "THIRD_PARTY.txt"), notices.join("\n"), "utf8");
}

function buildWindowsRelease() {
  process.chdir(REPO_ROOT);
  const configPath = path.join(REPO_ROOT, "packages/devtools/niva.json");
  const config = JSON.parse(readFileSync(configPath, "utf8"));
  const runtimePath = path.join(REPO_ROOT, "target/release/niva.exe");
  const packagerPath = path.join(REPO_ROOT, "target/release/niva-packager.exe");
  const outputDir = path.join(REPO_ROOT, "dist");
  const runtimeBytes = readFileSync(runtimePath);
  const metadata = JSON.parse(
    runChecked("cargo", ["metadata", "--locked", "--format-version=1"], { printStdout: false }),
  );
  const runtimePackage = cargoPackage(metadata, "niva");
  const packagerPackage = cargoPackage(metadata, "niva-packager");
  const version = config.version;
  if (runtimePackage.version !== version || packagerPackage.version !== version) {
    throw new Error(
      `devtools, Niva runtime, and niva-packager versions must match (${version}, ${runtimePackage.version}, ${packagerPackage.version})`,
    );
  }
  const toolVersion = runChecked(packagerPath, ["--version"]).trim();
  if (toolVersion !== `niva-packager ${version}`) {
    throw new Error(`built packager version mismatch: ${toolVersion}`);
  }
  const manifest = createRuntimeManifest(version, runtimeBytes);
  const kitRoot = mkdtempSync(path.join(outputDir, ".windows-build-kit-"));
  try {
    const runtimeDestination = path.join(kitRoot, RUNTIME_RELATIVE_PATH);
    mkdirSync(path.dirname(runtimeDestination), { recursive: true });
    copyFileSync(runtimePath, runtimeDestination);
    const stagedRuntimeBytes = readFileSync(runtimeDestination);
    if (sha256(stagedRuntimeBytes) !== manifest.runtimes[TARGET].sha256) {
      throw new Error("staged Windows runtime does not match the built niva.exe SHA256");
    }
    writeLicenseMaterials(kitRoot, metadata);
    const manifestPath = path.join(kitRoot, "manifest.json");
    writeFileSync(manifestPath, `${JSON.stringify(manifest, null, 2)}\n`, "utf8");

    const stdout = runChecked(packagerPath, [
      "build",
      "--manifest",
      manifestPath,
      "--config",
      configPath,
      "--resource-dir",
      path.join(REPO_ROOT, "packages/devtools/build"),
      "--output-dir",
      outputDir,
      "--target",
      TARGET,
      "--resource-layout",
      "embedded",
    ]);
    const artifact = validateBuildReport(parseBuildReport(stdout), {
      appName: config.name,
      version,
      outputDir,
    });
    console.log(`Windows runtime: ${runtimeBytes.length} bytes (limit < ${MAX_RUNTIME_BYTES})`);
    console.log(`Windows packaged executable: ${artifact.sizeBytes} bytes SHA256 ${artifact.sha256}`);

    const gitVersion = runChecked("git", ["describe", "--tags", "--always", "--dirty"]).trim();
    const archivePath = path.join(outputDir, windowsArchiveName(gitVersion));
    runChecked(
      "powershell.exe",
      [
        "-NoLogo",
        "-NoProfile",
        "-NonInteractive",
        "-ExecutionPolicy",
        "Bypass",
        "-Command",
        "try { $ErrorActionPreference = 'Stop'; Compress-Archive -LiteralPath $env:NIVA_ARCHIVE_SOURCE -DestinationPath $env:NIVA_ARCHIVE_DESTINATION -Force } catch { [Console]::Error.WriteLine($_); exit 1 }",
      ],
      {
        env: {
          ...process.env,
          NIVA_ARCHIVE_SOURCE: artifact.path,
          NIVA_ARCHIVE_DESTINATION: archivePath,
        },
      },
    );
    const archiveBytes = readFileSync(archivePath);
    if (archiveBytes.length === 0) throw new Error("Compress-Archive produced an empty ZIP");
    console.log(`Windows archive: ${archivePath} (${archiveBytes.length} bytes)`);
  } finally {
    rmSync(kitRoot, { recursive: true, force: true });
  }
}

if (import.meta.url === pathToFileURL(process.argv[1] ?? "").href) {
  try {
    buildWindowsRelease();
  } catch (error) {
    console.error(error.stack ?? error.message);
    process.exitCode = 1;
  }
}
