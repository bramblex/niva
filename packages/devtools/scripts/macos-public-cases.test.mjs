import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { readFileSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import test from "node:test";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../../..");

test("the migrated macOS runner keeps every tracked case ID and maps superseded host control", () => {
  const script = [
    "import importlib.util, json, sys",
    "from pathlib import Path",
    "root = Path(sys.argv[1])",
    "path = root / 'examples/macos-api-smoke/run_all.py'",
    "spec = importlib.util.spec_from_file_location('niva_macos_run_all_test', path)",
    "module = importlib.util.module_from_spec(spec); spec.loader.exec_module(module)",
    "print(json.dumps({'ids': sorted(module.expected_methods()), 'crosswalk': module.CASE_ID_CROSSWALK}))",
  ].join("\n");
  const result = JSON.parse(execFileSync("python3", ["-c", script, root], { encoding: "utf8" }));
  assert.equal(result.ids.length, 164, "the 164 source-tracked Native case IDs remain the denominator");
  assert.equal(new Set(result.ids).size, 164, "case IDs remain unique");
  assert.ok(result.ids.includes("host.send"), "the old host-send case remains explicitly tracked");
  assert.equal(result.crosswalk["fixture.processStream"], "host.send",
    "the removed Native host route is represented by the fixture process stream case");
  assert.ok(Object.values(result.crosswalk).every((legacyId) => result.ids.includes(legacyId)));

  const sources = [
    "examples/macos-api-smoke/index.html", "examples/macos-api-smoke/headless.html",
    "examples/macos-api-smoke/bridge-top-level.html", "examples/macos-api-smoke/system-supervised.html",
    "examples/macos-api-smoke/window-supervised.html", "examples/node-compat-integration/index.html",
    "examples/windows-smoke/index.html", "examples/windows-smoke/api-safe.js",
  ].map((file) => readFileSync(path.join(root, file), "utf8")).join("\n");
  assert.doesNotMatch(sources, /Niva\.api\b/);
  assert.doesNotMatch(sources, /Niva\.api\.host/);
});
