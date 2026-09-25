"""Deterministic regression tests for the stdio runner's frame backlog."""

import importlib.util
import io
import json
import queue
import subprocess
import unittest
import urllib.request
from contextlib import redirect_stdout
from pathlib import Path
from types import SimpleNamespace
from unittest.mock import patch


RUNNER_PATH = Path(__file__).with_name("run.py")
SPEC = importlib.util.spec_from_file_location("niva_macos_api_smoke_runner", RUNNER_PATH)
assert SPEC is not None and SPEC.loader is not None
RUNNER = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(RUNNER)
RUN_ALL_SPEC = importlib.util.spec_from_file_location(
    "niva_macos_api_smoke_run_all", Path(__file__).with_name("run_all.py")
)
assert RUN_ALL_SPEC is not None and RUN_ALL_SPEC.loader is not None
RUN_ALL = importlib.util.module_from_spec(RUN_ALL_SPEC)
RUN_ALL_SPEC.loader.exec_module(RUN_ALL)


class FakeHarness(RUNNER.Harness):
    def __init__(self, frames):
        self.process = SimpleNamespace(poll=lambda: None)
        self.frames = queue.Queue()
        self.backlog = []
        self.next_frame_calls = 0
        for frame in frames:
            self.frames.put(frame)

    def next_frame(self, timeout):
        self.next_frame_calls += 1
        if self.next_frame_calls > 1:
            raise AssertionError("wait loop cycled an unrelated backlog frame")
        return super().next_frame(timeout)


class HarnessBacklogTests(unittest.TestCase):
    def test_wait_message_reads_a_fresh_frame_after_an_unrelated_message(self):
        unrelated = {"protocol": "niva-fixture", "version": 1, "event": "message", "name": "unrelated"}
        target = {"protocol": "niva-fixture", "version": 1, "event": "message", "name": "wanted"}
        harness = FakeHarness([unrelated, target])

        self.assertEqual(harness.wait_message("wanted", timeout=0.2), target)
        self.assertEqual(harness.backlog, [unrelated])
        self.assertEqual(harness.next_frame_calls, 0)

    def test_progress_sink_records_only_method_and_phase(self):
        sink = RUNNER.ProgressSink()
        try:
            with urllib.request.urlopen(f"{sink.url}?method=Niva.resource.read&phase=start", timeout=1) as response:
                self.assertEqual(response.status, 204)
            self.assertEqual(sink.last_progress, {"method": "Niva.resource.read", "phase": "start"})
        finally:
            sink.close()


class HeadlessGroupRegistryTests(unittest.TestCase):
    def test_native_case_crosswalk_preserves_only_the_tracked_denominator(self):
        expected = RUN_ALL.expected_methods()
        self.assertEqual(len(expected), 164)
        self.assertTrue(set(RUN_ALL.CASE_ID_CROSSWALK.values()).issubset(expected))
        self.assertEqual(len(RUN_ALL.CASE_ID_CROSSWALK.values()), len(set(RUN_ALL.CASE_ID_CROSSWALK.values())))
        self.assertEqual(RUN_ALL.case_id("fixture.processStream"), "host.send")

    def test_fixture_code_uses_process_streams_and_no_removed_bridge_aliases(self):
        roots = [RUNNER.HERE, RUNNER.HERE.parent / "node-compat-macos-smoke",
                 RUNNER.HERE.parent / "node-compat-integration", RUNNER.HERE.parent / "windows-smoke"]
        sources = "\n".join(
            path.read_text(encoding="utf-8")
            for root in roots
            for pattern in ("*.html", "*.js")
            for path in root.glob(pattern)
        )
        self.assertNotIn("Niva.api", sources)
        self.assertNotIn("Niva.api.host", sources)
        self.assertNotIn("--stdio", sources)
        self.assertIn("process.stdin", sources)
        self.assertIn("process.stdout", sources)

    def test_headless_groups_partition_all_page_methods_and_keep_exact_counts(self):
        groups = RUNNER.HEADLESS_GROUP_METHODS
        self.assertEqual(set(groups), {"process-os", "resource", "fs"})
        self.assertEqual({name: len(methods) + 2 for name, methods in groups.items()}, {
            "process-os": 12,
            "resource": 4,
            "fs": 14,
        })
        sets = [set(methods) for methods in groups.values()]
        self.assertTrue(all(sets[i].isdisjoint(sets[j]) for i in range(len(sets)) for j in range(i + 1, len(sets))))
        self.assertEqual(set.union(*sets), set(RUNNER.HEADLESS_PAGE_METHODS))
        self.assertEqual(len(RUNNER.HEADLESS_METHODS), 26)

    def test_stress_registry_is_exact_union_of_the_three_groups(self):
        self.assertEqual(set(RUNNER.HEADLESS_METHODS), set(RUNNER.HEADLESS_PAGE_METHODS) | {"fixture.processStream", "process.exit"})

    def test_runner_selects_three_independent_groups(self):
        calls = []

        def fake_run(_binary, group):
            calls.append(group)
            return {method: "asserted" for method in RUNNER.HEADLESS_GROUP_EXPECTED[group]}, None, ""

        with patch.object(RUNNER, "run_headless_invocation", side_effect=fake_run):
            with redirect_stdout(io.StringIO()):
                status = RUNNER.run_headless_groups(Path("/fake/niva"))
        self.assertEqual(status, 0)
        self.assertEqual(calls, ["process-os", "resource", "fs"])

    def test_stress_mode_selects_one_all_sequential_group(self):
        calls = []

        def fake_run(_binary, group):
            calls.append(group)
            return {method: "asserted" for method in RUNNER.HEADLESS_METHODS}, None, ""

        with patch.object(RUNNER, "run_headless_invocation", side_effect=fake_run):
            with redirect_stdout(io.StringIO()):
                status = RUNNER.run_headless_groups(Path("/fake/niva"), all_sequential=True)
        self.assertEqual(status, 0)
        self.assertEqual(calls, ["all-sequential"])

    def test_extended_registry_matches_window_and_system_case_sources(self):
        node_script = (
            'const fs=require("fs"),vm=require("vm"),ctx={window:{}};'
            'vm.createContext(ctx);'
            'for(const file of process.argv.slice(1)) vm.runInContext(fs.readFileSync(file,"utf8"),ctx,{filename:file});'
            'process.stdout.write(JSON.stringify({'
            'window:ctx.window.NivaMacWindowCases.automatic.map(x=>x.method),'
            'system:ctx.window.NivaMacSystemCases.automatic.map(x=>x.method),'
            'windowSupervised:ctx.window.NivaMacWindowCases.supervised.map(x=>x.method),'
            'systemSupervised:ctx.window.NivaMacSystemCases.supervised.map(x=>x.method)}));'
        )
        result = subprocess.run(
            [
                "node", "-e", node_script,
                str(RUNNER.HERE / "window-cases.js"),
                str(RUNNER.HERE / "system-cases.js"),
            ],
            check=True,
            capture_output=True,
            text=True,
        )
        methods = json.loads(result.stdout)
        self.assertEqual(set(methods["window"]), RUNNER.EXTENDED_WINDOW_METHODS)
        self.assertEqual(set(methods["system"]), RUNNER.EXTENDED_SYSTEM_METHODS)
        self.assertEqual(set(methods["windowSupervised"]), RUNNER.EXTENDED_WINDOW_SUPERVISED)
        self.assertEqual(set(methods["systemSupervised"]), RUNNER.EXTENDED_SYSTEM_SUPERVISED)
        self.assertEqual(len(methods["window"]), len(set(methods["window"])))
        self.assertEqual(len(methods["system"]), len(set(methods["system"])))
        self.assertEqual(len(RUNNER.EXTENDED_AUTOMATIC_METHODS), 59)
        default_methods = set(RUNNER.AUTOMATIC_METHODS) | RUNNER.WEBVIEW_HISTORY_METHODS | RUNNER.TRAY_METHODS | {"fixture.processStream", "process.exit"}
        self.assertTrue(default_methods.isdisjoint(RUNNER.EXTENDED_AUTOMATIC_METHODS))

    def test_extended_python_registry_matches_case_script_sources(self):
        node_script = (
            'const fs=require("fs"),vm=require("vm"),ctx={window:{}};'
            'vm.createContext(ctx);'
            'for(const file of process.argv.slice(1)) vm.runInContext(fs.readFileSync(file,"utf8"),ctx,{filename:file});'
            'process.stdout.write(JSON.stringify({'
            'window:ctx.window.NivaMacWindowCases.automatic.map(x=>x.method),'
            'system:ctx.window.NivaMacSystemCases.automatic.map(x=>x.method),'
            'windowSupervised:ctx.window.NivaMacWindowCases.supervised.map(x=>x.method),'
            'systemSupervised:ctx.window.NivaMacSystemCases.supervised.map(x=>x.method)}));'
        )
        result = subprocess.run(
            [
                "node", "-e", node_script,
                str(RUNNER.HERE / "window-cases.js"),
                str(RUNNER.HERE / "system-cases.js"),
            ],
            check=True,
            capture_output=True,
            text=True,
        )
        methods = json.loads(result.stdout)
        self.assertEqual(set(methods["window"]), RUNNER.EXTENDED_WINDOW_METHODS)
        self.assertEqual(set(methods["system"]), RUNNER.EXTENDED_SYSTEM_METHODS)
        self.assertEqual(set(methods["windowSupervised"]), RUNNER.EXTENDED_WINDOW_SUPERVISED)
        self.assertEqual(set(methods["systemSupervised"]), RUNNER.EXTENDED_SYSTEM_SUPERVISED)
        self.assertEqual(len(methods["window"]), len(set(methods["window"])))
        self.assertEqual(len(methods["system"]), len(set(methods["system"])))
        self.assertEqual(len(RUNNER.EXTENDED_AUTOMATIC_METHODS), 59)

    def test_wait_one_of_reads_a_fresh_frame_after_an_unrelated_message(self):
        unrelated = {"protocol": "niva-fixture", "version": 1, "event": "message", "name": "unrelated"}
        target = {"protocol": "niva-fixture", "version": 1, "event": "message", "name": "accepted"}
        harness = FakeHarness([unrelated, target])

        self.assertEqual(harness.wait_one_of({"accepted", "also-accepted"}, timeout=0.2), target)
        self.assertEqual(harness.backlog, [unrelated])
        self.assertEqual(harness.next_frame_calls, 0)


if __name__ == "__main__":
    unittest.main()
