#!/usr/bin/env python3
"""Run every disposable macOS Niva API and NodeCompat suite in sequence."""

from __future__ import annotations

import argparse
import re
import subprocess
import sys
from datetime import datetime
from pathlib import Path


HERE = Path(__file__).resolve().parent
REPO = HERE.parents[1]
NODE_COMPAT = REPO / "examples/node-compat-macos-smoke/run.py"
BRIDGE_METHODS = {
    "Niva.registerModule", "Niva.require", "Niva.import", "Niva.addEventListener",
    "Niva.removeEventListener", "Niva.removeAllEventListeners", "Niva.call", "Niva.stream", "Niva.streamSend",
}


def expected_methods() -> set[str]:
    rows = re.findall(r"^\| `([^`]+)` \|", (HERE / "COVERAGE.md").read_text(encoding="utf-8"), re.M)
    if len(rows) != 167 or len(rows) != len(set(rows)):
        raise RuntimeError("COVERAGE.md must list exactly 167 distinct public API methods")
    return set(rows)


def read_method_evidence(output: str, kind: str) -> tuple[set[str], set[str]]:
    behavior = set()
    if kind != "window-supervised":
        behavior = {
            method for method, detail in re.findall(r"^  ([A-Za-z]+\.[A-Za-z0-9]+): ([^\n]*)", output, re.M)
            if not detail.startswith("NOT RUN")
        }
    behavior.update(re.findall(r"^PASS ([A-Za-z]+\.[A-Za-z0-9]+): ", output, re.M))
    invocation = set(re.findall(r"^INVOKED ([A-Za-z]+\.[A-Za-z0-9]+): ", output, re.M))
    if kind == "drag":
        behavior.update(re.findall(r"^macOS window drag smoke: PASS — ([A-Za-z]+\.[A-Za-z0-9]+):", output, re.M))
    return behavior, invocation


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("binary", type=Path, help="path to a locally built macOS Niva binary")
    parser.add_argument("--report-dir", type=Path, help="save exact suite stdout/stderr here")
    args = parser.parse_args()
    if sys.platform != "darwin":
        parser.error("these suites require a real macOS desktop")
    binary = args.binary.expanduser().resolve()
    if not binary.is_file():
        parser.error(f"Niva binary does not exist: {binary}")
    report_dir = args.report_dir or Path("/tmp") / f"niva-macos-all-{datetime.now().strftime('%Y%m%d-%H%M%S')}"
    report_dir.mkdir(parents=True, exist_ok=False)
    python = sys.executable
    suites = [
        ("default", [python, "-B", str(HERE / "run.py"), str(binary)]),
        ("extended", [python, "-B", str(HERE / "run.py"), str(binary), "--extended-automatic"]),
        ("headless", [python, "-B", str(HERE / "run.py"), str(binary), "--headless-safe"]),
        ("clipboard-shortcut", [python, "-B", str(HERE / "run.py"), str(binary), "--clipboard", "--shortcut"]),
        ("dialogs", [python, "-B", str(HERE / "run.py"), str(binary), "--dialogs-auto"]),
        ("window-supervised", [python, "-B", str(HERE / "run_window_supervised.py"), str(binary)]),
        ("system-supervised", [python, "-B", str(HERE / "run_system_supervised.py"), str(binary),
                               "--activation", "--hide-others", "--window-extra", "--dock-badge", "--progress"]),
        ("drag", [python, "-B", str(HERE / "run_drag_window.py"), str(binary)]),
        ("bridge-top-level", [python, "-B", str(HERE / "run_bridge_top_level.py"), str(binary)]),
        ("node-compat", [python, "-B", str(NODE_COMPAT), "--binary", str(binary)]),
    ]
    behavior: set[str] = set()
    invocation: set[str] = set()
    for name, command in suites:
        print(f"RUN {name}", flush=True)
        completed = subprocess.run(command, cwd=REPO, capture_output=True, text=True, timeout=300)
        output = completed.stdout + completed.stderr
        (report_dir / f"{name}.log").write_text(output, encoding="utf-8")
        if completed.returncode:
            print(f"FAIL {name} (exit {completed.returncode}); log: {report_dir / (name + '.log')}", file=sys.stderr)
            print("\n".join(output.splitlines()[-12:]), file=sys.stderr)
            return 1
        if name == "bridge-top-level":
            methods = set(re.findall(r"^  (Niva\.[A-Za-z]+): ", output, re.M))
            if methods != BRIDGE_METHODS:
                print(f"top-level bridge mismatch: missing={sorted(BRIDGE_METHODS - methods)}, extra={sorted(methods - BRIDGE_METHODS)}", file=sys.stderr)
                return 1
            print("PASS bridge-top-level: 9 exact methods", flush=True)
            continue
        passed, invoked = read_method_evidence(output, name)
        behavior.update(passed)
        invocation.update(invoked)
        if name == "node-compat":
            match = re.search(r"^PASS: (\d+) documented NodeCompat method and alias checks in (\d+) cases", output, re.M)
            if not match or match.groups() != ("185", "16"):
                print("NodeCompat result count did not match its exact catalog", file=sys.stderr)
                return 1
            print(f"PASS {name}: {match.group(1)} checks in {match.group(2)} cases", flush=True)
        else:
            print(f"PASS {name}: {len(passed)} behavior methods, {len(invoked)} invocation-only methods", flush=True)
    expected = expected_methods()
    exercised = behavior | invocation
    missing = expected - exercised
    unexpected = exercised - expected
    if missing or unexpected:
        print(f"macOS public API mismatch: missing={sorted(missing)}, unexpected={sorted(unexpected)}", file=sys.stderr)
        return 1
    invocation_only = invocation - behavior
    print(f"Niva.api macOS: {len(exercised)}/167 methods exercised; {len(behavior & expected)} behavior-verified; {len(invocation_only)} invocation-only")
    print("Niva bridge macOS: 9/9 methods behavior-verified")
    for method in sorted(invocation_only):
        print(f"  INVOCATION ONLY {method}")
    print(f"Suite logs: {report_dir}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
