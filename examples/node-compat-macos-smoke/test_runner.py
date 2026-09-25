"""Static runner and catalog tests; these do not launch a native app."""

from __future__ import annotations

import json
import subprocess
import sys
import unittest
from pathlib import Path

import run


HERE = Path(__file__).resolve().parent


def load_catalog() -> dict:
    source = HERE / "cases.js"
    script = r"""
      const fs = require('node:fs');
      const vm = require('node:vm');
      const sandbox = { console, URL, URLSearchParams, TextEncoder, TextDecoder };
      sandbox.window = sandbox;
      sandbox.globalThis = sandbox;
      sandbox.addEventListener = () => {};
      vm.createContext(sandbox);
      vm.runInContext(fs.readFileSync(process.argv[1], 'utf8'), sandbox, { filename: process.argv[1] });
      process.stdout.write(JSON.stringify({
        modules: sandbox.NivaNodeCompatMacCases.modules,
        cases: sandbox.NivaNodeCompatMacCases.catalog,
      }));
    """
    result = subprocess.run(
        ["node", "-e", script, str(source)],
        cwd=run.REPO,
        text=True,
        capture_output=True,
        check=False,
        timeout=15,
    )
    if result.returncode != 0:
        raise AssertionError(result.stderr or f"catalog extraction exited {result.returncode}")
    return json.loads(result.stdout)


class NodeCompatMacSmokeStaticTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.catalog = load_catalog()

    def test_catalog_covers_exact_documented_module_set(self):
        modules = self.catalog["modules"]
        self.assertEqual(set(modules), run.EXPECTED_MODULES)
        self.assertEqual(len(modules), len(run.EXPECTED_MODULES))

    def test_case_ids_and_api_entries_are_unique_and_nonempty(self):
        cases = self.catalog["cases"]
        ids = [case["id"] for case in cases]
        methods = [method for case in cases for method in case["methods"]]
        self.assertEqual(len(ids), len(set(ids)))
        self.assertTrue(all(methods))
        self.assertEqual(len(methods), len(set(methods)))
        self.assertGreaterEqual(len(methods), 130)

    def test_documented_subpaths_and_native_bridge_cases_are_present(self):
        methods = {method for case in self.catalog["cases"] for method in case["methods"]}
        for required in {
            "CommonJS.require", "browser.import", "fs/promises", "alias:assert/strict", "stream/promises.pipeline",
            "fs.writeFile", "child_process.spawn", "http.request", "https.get",
            "stream.pipeline", "crypto.createHash", "zlib.gunzip", "importmap:fs",
        }:
            with self.subTest(method=required):
                self.assertIn(required, methods)

    def test_report_rejects_an_incomplete_passed_list(self):
        catalog = self.catalog["cases"]
        expected = [method for case in catalog for method in case["methods"]]
        report = {
            "modules": self.catalog["modules"],
            "cases": catalog,
            "passedMethods": expected[:-1],
        }
        with self.assertRaisesRegex(run.SmokeError, "do not exactly match"):
            run.validate_report(report)

    def test_report_rejects_duplicate_methods(self):
        catalog = [dict(case) for case in self.catalog["cases"]]
        catalog[1] = {"id": catalog[1]["id"], "methods": catalog[1]["methods"] + [catalog[1]["methods"][0]]}
        report = {
            "modules": self.catalog["modules"],
            "cases": catalog,
            "passedMethods": [method for case in catalog for method in case["methods"]],
        }
        with self.assertRaisesRegex(run.SmokeError, "duplicate NodeCompat API"):
            run.validate_report(report)

    def test_frame_reader_consumes_fixture_process_stream_ndjson(self):
        child = subprocess.Popen(
            [sys.executable, "-c", "import json; print(json.dumps({'protocol':'niva-fixture','version':1,'event':'ready','name':'test'}))"],
            stdout=subprocess.PIPE,
        )
        reader = run.FrameReader(child)
        try:
            self.assertEqual(reader.next(2), {"protocol": "niva-fixture", "version": 1, "event": "ready", "name": "test"})
            self.assertEqual(child.wait(timeout=2), 0)
        finally:
            reader.close()
            if child.stdout is not None:
                child.stdout.close()
            if child.poll() is None:
                child.kill()
                child.wait(timeout=2)


if __name__ == "__main__":
    unittest.main()
