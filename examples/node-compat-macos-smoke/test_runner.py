"""Static runner and catalog tests; these do not launch a native app."""

from __future__ import annotations

import json
import subprocess
import sys
import threading
import urllib.error
import urllib.request
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
            "fs.writeFile", "child_process.spawn", "http.request", "http.get.concurrent", "https.get",
            "stream.pipeline", "crypto.createHash", "zlib.gunzip", "importmap:fs",
        }:
            with self.subTest(method=required):
                self.assertIn(required, methods)

    def test_http_parallel_fixture_releases_only_after_both_distinct_requests(self):
        server, state = run.start_http_fixture_server(barrier_timeout=1.0)
        thread = threading.Thread(target=server.serve_forever, daemon=True)
        thread.start()
        base = f"http://127.0.0.1:{server.server_port}/parallel?id="

        def read(request_id: str) -> tuple[int, str]:
            with urllib.request.urlopen(base + request_id, timeout=3) as response:
                return response.status, response.read().decode()

        try:
            from concurrent.futures import ThreadPoolExecutor

            with ThreadPoolExecutor(max_workers=2) as pool:
                futures = [pool.submit(read, request_id) for request_id in ("first", "second")]
                results = [future.result(timeout=4) for future in futures]
            self.assertEqual([status for status, _ in results], [200, 200])
            self.assertEqual(
                [body for _, body in results],
                [
                    "parallel response first: both requests arrived",
                    "parallel response second: both requests arrived",
                ],
            )
            self.assertEqual(state["parallel_ids"], {"first", "second"})
            self.assertTrue(state["parallel_released"])
            self.assertEqual(state["parallel_responses"], 2)
        finally:
            server.shutdown()
            server.server_close()
            thread.join(timeout=2)

    def test_http_parallel_fixture_returns_timeout_when_second_request_never_arrives(self):
        server, state = run.start_http_fixture_server(barrier_timeout=0.1)
        thread = threading.Thread(target=server.serve_forever, daemon=True)
        thread.start()
        try:
            with self.assertRaises(urllib.error.HTTPError) as raised:
                urllib.request.urlopen(f"http://127.0.0.1:{server.server_port}/parallel?id=first", timeout=2)
            self.assertEqual(raised.exception.code, 504)
            try:
                self.assertIn(b"barrier timed out", raised.exception.read())
            finally:
                raised.exception.close()
            self.assertTrue(state["parallel_failed"])
            self.assertFalse(state["parallel_released"])
        finally:
            server.shutdown()
            server.server_close()
            thread.join(timeout=2)

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
