#!/usr/bin/env python3
"""Check all nine documented top-level Niva bridge methods in a real WebView."""

from __future__ import annotations

import argparse
import json
import os
import shutil
import subprocess
import sys
import tempfile
import time
import uuid
from pathlib import Path

from run import Harness, REPO, SmokeError, safe_app_dirs, tail


HERE = Path(__file__).resolve().parent
EXPECTED = {
    "Niva.registerModule", "Niva.require", "Niva.import",
    "Niva.addEventListener", "Niva.removeEventListener", "Niva.removeAllEventListeners",
    "Niva.call", "Niva.stream", "Niva.streamSend",
}


def message(harness: Harness, name: str, timeout: float = 30) -> dict:
    deadline = time.monotonic() + timeout
    while True:
        frame = harness.next_frame(max(0.1, deadline - time.monotonic()))
        if frame.get("t") == "msg" and frame.get("name") == "bridge-top-level-error":
            raise SmokeError(f"page failed: {frame.get('data')}")
        if frame.get("t") == "msg" and frame.get("name") == name:
            data = frame.get("data")
            if not isinstance(data, dict):
                raise SmokeError(f"{name} had no object payload")
            return data


def run(binary: Path) -> int:
    app_name = "NivaTopLevelBridgeSmoke"
    app_uuid = str(uuid.uuid4())
    dirs = safe_app_dirs(app_name, app_uuid)
    process: subprocess.Popen[str] | None = None
    stderr_path: Path | None = None
    evidence: dict[str, str] = {}
    failure: str | None = None
    stderr_tail = ""
    try:
        with tempfile.TemporaryDirectory(prefix="niva-top-level-bridge-") as temporary:
            root = Path(temporary).resolve()
            resources = root / "resources"
            resources.mkdir()
            shutil.copy2(HERE / "bridge-top-level.html", resources / "bridge-top-level.html")
            (resources / "probe.txt").write_text("niva-top-level-probe\n", encoding="utf-8")
            config = root / "niva.json"
            config.write_text(json.dumps({
                "name": app_name, "uuid": app_uuid,
                "window": {"entry": "bridge-top-level.html", "title": "Niva top-level bridge smoke",
                           "size": {"width": 500, "height": 300}, "visible": True},
            }) + "\n", encoding="utf-8")
            log_fd, log_path = tempfile.mkstemp(prefix="niva-top-level-bridge-", suffix=".log")
            os.close(log_fd)
            stderr_path = Path(log_path)
            environment = os.environ.copy()
            environment["TMPDIR"] = str(root)
            with stderr_path.open("wb") as stderr_file:
                process = subprocess.Popen(
                    [str(binary), "--stdio", f"--debug-config={config}", f"--debug-resource={resources}"],
                    cwd=REPO, env=environment, stdin=subprocess.PIPE, stdout=subprocess.PIPE,
                    stderr=stderr_file, text=True, bufsize=1,
                )
            harness = Harness(process)
            if harness.next_frame(30) != {"t": "ready", "v": 1}:
                raise SmokeError("Niva stdio bridge did not report ready")
            message(harness, "bridge-top-level-ready")
            harness.send("bridge-top-level-command", {"tempRoot": str(root)})
            result = message(harness, "bridge-top-level-result", 60)
            for item in result.get("coverage", []):
                if not isinstance(item, dict) or not isinstance(item.get("method"), str) or not isinstance(item.get("assertion"), str):
                    raise SmokeError(f"malformed top-level bridge case: {item!r}")
                method = item["method"]
                if method in evidence:
                    raise SmokeError(f"duplicate top-level bridge case: {method}")
                evidence[method] = item["assertion"]
            if evidence.keys() != EXPECTED:
                raise SmokeError(f"top-level bridge coverage mismatch: missing={sorted(EXPECTED - evidence.keys())}, extra={sorted(evidence.keys() - EXPECTED)}")
            if process.stdin and not process.stdin.closed:
                process.stdin.close()
            if process.wait(timeout=15) != 0:
                raise SmokeError("Niva did not exit cleanly after stdio EOF")
    except Exception as error:
        failure = f"{type(error).__name__}: {error}"
    finally:
        if process is not None and process.poll() is None:
            if process.stdin and not process.stdin.closed:
                process.stdin.close()
            try:
                process.wait(timeout=10)
            except subprocess.TimeoutExpired:
                process.terminate()
                process.wait(timeout=5)
        for item in dirs:
            if item.is_dir() and not item.is_symlink():
                shutil.rmtree(item)
        if stderr_path is not None:
            stderr_tail = tail(stderr_path)
            stderr_path.unlink(missing_ok=True)
    if failure:
        print(f"macOS top-level bridge smoke: FAIL — {failure}", file=sys.stderr)
        if stderr_tail:
            print("Niva stderr tail:\n" + stderr_tail, file=sys.stderr)
        return 1
    print("macOS top-level bridge smoke: PASS (9 exact methods)")
    for method, assertion in sorted(evidence.items()):
        print(f"  {method}: {assertion}")
    return 0


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("binary", help="path to a locally built macOS Niva binary")
    args = parser.parse_args()
    if sys.platform != "darwin":
        parser.error("this fixture requires macOS")
    binary = Path(args.binary).expanduser().resolve()
    if not binary.is_file():
        parser.error(f"Niva binary does not exist: {binary}")
    return run(binary)


if __name__ == "__main__":
    raise SystemExit(main())
