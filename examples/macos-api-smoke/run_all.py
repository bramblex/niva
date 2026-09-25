#!/usr/bin/env python3
"""Run the disposable macOS smoke suites and reconcile their preserved cases."""

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
    "Niva.addEventListener",
    "Niva.removeEventListener", "Niva.removeAllEventListeners", "Niva.bridge.call",
    "Niva.bridge.callSync", "Niva.bridge.stream", "Niva.bridge.streamSend",
}

# COVERAGE.md retains the original 167-row case denominator. The fixtures
# report current Node and direct-Niva routes; normalize only the one-to-one
# names changed by this migration when comparing with that frozen catalog.
CASE_ID_CROSSWALK = {
    "process.cwd": "process.currentDir",
    "process.execPath": "process.currentExe",
    "process.argv": "process.args",
    "Niva.os.info": "os.info",
    "Niva.os.dirs": "os.dirs",
    "path.sep": "os.sep",
    "os.EOL": "os.eol",
    "Niva.bridge.callOsLocale": "os.locale",
    "Niva.resource.exists": "resource.exists",
    "Niva.resource.read": "resource.read",
    "fs.access": "fs.exists",
    "fs.mkdir": "fs.createDir",
    "fs.mkdirRecursive": "fs.createDirAll",
    "fs.writeFile": "fs.write",
    "fs.readFile": "fs.read",
    "fs.appendFile": "fs.append",
    "fs.copyFile": "fs.copy",
    "fs.rename": "fs.move",
    "fs.readdir": "fs.readDir",
    "fs.readdirRecursive": "fs.readDirAll",
    "fs.rm": "fs.remove",
    "process.chdir": "process.setCurrentDir",
    "child_process.execFile": "process.exec",
    "Niva.resource.extract": "resource.extract",
    "Niva.bridge.callProcessOpen": "process.open",
    "fixture.processStream": "host.send",
}


def case_id(method: str) -> str:
    return CASE_ID_CROSSWALK.get(method, method)


def expected_methods() -> set[str]:
    rows = re.findall(r"^\| `([^`]+)` \|", (HERE / "COVERAGE.md").read_text(encoding="utf-8"), re.M)
    if len(rows) != 164 or len(rows) != len(set(rows)):
        raise RuntimeError("COVERAGE.md must list exactly the 164 distinct Native case IDs in its current table")
    return set(rows)


def read_method_evidence(output: str, kind: str) -> tuple[set[str], set[str]]:
    behavior = set()
    if kind != "window-supervised":
        behavior = {
            method for method, detail in re.findall(r"^  ([A-Za-z][A-Za-z0-9]*(?:\.[A-Za-z][A-Za-z0-9]*)+): ([^\n]*)", output, re.M)
            if not detail.startswith("NOT RUN")
        }
    behavior.update(re.findall(r"^PASS ([A-Za-z][A-Za-z0-9]*(?:\.[A-Za-z][A-Za-z0-9]*)+): ", output, re.M))
    invocation = set(re.findall(r"^INVOKED ([A-Za-z][A-Za-z0-9]*(?:\.[A-Za-z][A-Za-z0-9]*)+): ", output, re.M))
    if kind == "drag":
        behavior.update(re.findall(r"^macOS window drag smoke: PASS — ([A-Za-z][A-Za-z0-9]*(?:\.[A-Za-z][A-Za-z0-9]*)+):", output, re.M))
    return {case_id(method) for method in behavior}, {case_id(method) for method in invocation}


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
            methods = set(re.findall(r"^  (Niva(?:\.[A-Za-z]+){1,2}): ", output, re.M))
            if methods != BRIDGE_METHODS:
                print(f"top-level bridge mismatch: missing={sorted(BRIDGE_METHODS - methods)}, extra={sorted(methods - BRIDGE_METHODS)}", file=sys.stderr)
                return 1
            print(f"PASS bridge-top-level: {len(methods)} exact methods", flush=True)
            continue
        passed, invoked = read_method_evidence(output, name)
        behavior.update(passed)
        invocation.update(invoked)
        if name == "node-compat":
            match = re.search(r"^PASS: (\d+) documented runtime method and alias checks in (\d+) cases", output, re.M)
            if not match or match.groups() != ("185", "16"):
                print("Node runtime result count did not match its exact catalog", file=sys.stderr)
                return 1
            print(f"PASS {name}: {match.group(1)} checks in {match.group(2)} cases", flush=True)
        else:
            print(f"PASS {name}: {len(passed)} behavior methods, {len(invoked)} invocation-only methods", flush=True)
    expected = expected_methods()
    unmapped = set(CASE_ID_CROSSWALK.values()) - expected
    if unmapped:
        print(f"fixture case crosswalk points to absent legacy IDs: {sorted(unmapped)}", file=sys.stderr)
        return 1
    exercised = behavior | invocation
    missing = expected - exercised
    unexpected = exercised - expected
    if missing or unexpected:
        print(f"macOS public API mismatch: missing={sorted(missing)}, unexpected={sorted(unexpected)}", file=sys.stderr)
        return 1
    invocation_only = invocation - behavior
    print(f"macOS migrated Native fixtures: {len(exercised)}/{len(expected)} tracked Native case IDs exercised; {len(behavior & expected)} behavior-verified; {len(invocation_only)} invocation-only")
    print(f"Niva bridge macOS: {len(BRIDGE_METHODS)}/{len(BRIDGE_METHODS)} methods behavior-verified")
    for method in sorted(invocation_only):
        print(f"  INVOCATION ONLY {method}")
    print(f"Suite logs: {report_dir}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
