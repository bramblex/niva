import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { mkdtempSync, mkdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import os from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";
import test from "node:test";

import {
  MAX_RUNTIME_BYTES,
  createRuntimeManifest,
  parseBuildReport,
  repositoryRootForScript,
  selectedDependencyPackageIds,
  validateBuildReport,
  windowsArchiveName,
} from "./build-windows-release.mjs";

test("runtime manifest uses measured SHA and a strict Windows size limit", () => {
  const bytes = Buffer.from("MZ runtime bytes");
  const manifest = createRuntimeManifest("0.10.0-beta.1", bytes);
  assert.equal(manifest.schemaVersion, 1);
  assert.equal(manifest.version, "0.10.0-beta.1");
  assert.deepEqual(manifest.runtimes["windows-x86_64"], {
    path: "runtimes/niva-windows-x86_64.exe",
    version: "0.10.0-beta.1",
    sha256: createHash("sha256").update(bytes).digest("hex"),
  });
  assert.throws(
    () => createRuntimeManifest("0.10.0-beta.1", Buffer.alloc(MAX_RUNTIME_BYTES)),
    /strictly below 3500000/,
  );
  const belowLimit = Buffer.alloc(MAX_RUNTIME_BYTES - 1);
  assert.equal(createRuntimeManifest("0.10.0-beta.1", belowLimit).runtimes["windows-x86_64"].sha256,
    createHash("sha256").update(belowLimit).digest("hex"));
});

test("build report parser reads the packager's final JSON output line", () => {
  const report = { results: [{ target: "windows-x86_64", status: "complete" }] };
  assert.deepEqual(parseBuildReport(`packager output\n${JSON.stringify(report)}\n`), report);
  assert.throws(() => parseBuildReport("not JSON"), /could not parse/);
});

test("dependency closure follows Cargo metadata package ID strings", () => {
  const resolve = {
    nodes: [
      { id: "runtime", dependencies: ["shared"] },
      { id: "packager", dependencies: ["shared", "packager-only"] },
      { id: "shared", dependencies: [] },
      { id: "packager-only", dependencies: [] },
      { id: "unrelated", dependencies: [] },
    ],
  };
  assert.deepEqual(
    selectedDependencyPackageIds(resolve, ["runtime", "packager"]),
    new Set(["runtime", "shared", "packager", "packager-only"]),
  );
});

test("validates the reported executable path, PE architecture, and report hash", () => {
  const root = mkdtempSync(path.join(os.tmpdir(), "niva-windows-report-"));
  try {
    const outputDir = path.join(root, "dist");
    mkdirSync(outputDir);
    const artifactPath = path.join(outputDir, "devtools-windows-x86_64.exe");
    const bytes = Buffer.alloc(0x86);
    bytes.write("MZ", 0, "ascii");
    bytes.writeUInt32LE(0x80, 0x3c);
    bytes.write("PE\0\0", 0x80, "binary");
    bytes.writeUInt16LE(0x8664, 0x84);
    writeFileSync(artifactPath, bytes);
    const hash = createHash("sha256").update(bytes).digest("hex");
    const report = {
      results: [
        {
          target: "windows-x86_64",
          resourceLayout: "embedded",
          status: "complete",
          runtimeVersion: "0.10.0-beta.1",
          path: artifactPath,
          sha256: hash,
        },
      ],
    };

    assert.deepEqual(
      validateBuildReport(report, {
        appName: "devtools",
        version: "0.10.0-beta.1",
        outputDir,
      }),
      { path: artifactPath, sizeBytes: bytes.length, sha256: hash },
    );
    assert.throws(
      () => validateBuildReport({ ...report, results: [{ ...report.results[0], sha256: "0".repeat(64) }] }, {
        appName: "devtools",
        version: "0.10.0-beta.1",
        outputDir,
      }),
      /SHA256 does not match/,
    );
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});

test("derives the repository root from the script path and preserves archive naming", () => {
  const scriptPath = fileURLToPath(new URL("./build-windows-release.mjs", import.meta.url));
  assert.equal(repositoryRootForScript(scriptPath), path.resolve(path.dirname(scriptPath), ".."));
  assert.equal(windowsArchiveName("v0.10.0-beta.1"), "NivaDevtools_v0_10_0-beta_1_Windows.zip");
  assert.throws(() => windowsArchiveName("bad/name"), /cannot be used/);
});

test("Windows batch entry builds runtime and packager before invoking the current helper", () => {
  const batch = readFileSync(new URL("../build_Windows.cmd", import.meta.url), "utf8");
  for (const expected of [
    'pushd "%~dp0"',
    "call npm ci",
    "call npm run build --workspace=packages/runtime",
    "call npm run build --workspace=packages/devtools",
    "cargo build --release -p niva -p niva-packager",
    'node "scripts\\build-windows-release.mjs"',
    "Compress-Archive",
  ]) {
    const source = expected === "Compress-Archive"
      ? readFileSync(new URL("./build-windows-release.mjs", import.meta.url), "utf8")
      : batch;
    assert.ok(source.includes(expected), `Windows build is missing ${expected}`);
  }
  for (const obsolete of ["win_packager.exe", "--debug-config", "--debug-resource", "--project", "--build="]) {
    assert.ok(!batch.includes(obsolete), `Windows build still uses ${obsolete}`);
  }
  const runtimeBuild = batch.indexOf("call npm run build --workspace=packages/runtime");
  const cargoBuild = batch.indexOf("cargo build --release -p niva -p niva-packager");
  const packageBuild = batch.indexOf('node "scripts\\build-windows-release.mjs"');
  assert.ok(runtimeBuild >= 0 && runtimeBuild < cargoBuild && cargoBuild < packageBuild);
});
