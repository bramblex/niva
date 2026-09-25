#!/usr/bin/env python3
"""Verify window.dragWindow with a real synthetic macOS mouse drag."""

from __future__ import annotations

import argparse
import json
import os
import shutil
import subprocess
import sys
import tempfile
import uuid
from pathlib import Path

from run import Harness, REPO, SmokeError, safe_app_dirs, tail


HERE = Path(__file__).resolve().parent


def apple(script: str) -> str:
    result = subprocess.run(["osascript", "-e", script], capture_output=True, text=True, timeout=15)
    if result.returncode:
        raise SmokeError(result.stderr.strip() or "AppleScript failed")
    return result.stdout.strip()


def wait_message(harness: Harness, name: str, timeout: float = 20) -> dict:
    import time

    deadline = time.monotonic() + timeout
    while True:
        frame = harness.next_frame(max(0.1, deadline - time.monotonic()))
        if frame.get("event") == "message" and frame.get("name") == "drag-error":
            raise SmokeError(f"native drag API reported {frame.get('data')}")
        if frame.get("event") == "message" and frame.get("name") == name:
            data = frame.get("data")
            if not isinstance(data, dict):
                raise SmokeError(f"{name} returned no object")
            return data


def run(binary: Path, native_titlebar_control: bool = False) -> int:
    app_name = "NivaDragWindowSmoke"
    app_uuid = str(uuid.uuid4())
    dirs = safe_app_dirs(app_name, app_uuid)
    process: subprocess.Popen[str] | None = None
    stderr_path: Path | None = None
    previous_frontmost = apple('tell application "System Events" to get name of first process whose frontmost is true')
    original_pointer: tuple[float, float] | None = None
    failure: str | None = None
    evidence: str | None = None
    stderr_tail = ""
    try:
        with tempfile.TemporaryDirectory(prefix="niva-drag-window-") as temporary:
            root = Path(temporary)
            resources = root / "resources"
            resources.mkdir()
            shutil.copy2(HERE / "drag.html", resources / "drag.html")
            shutil.copy2(HERE / "fixture-protocol.js", resources / "fixture-protocol.js")
            config = root / "niva.json"
            config.write_text(json.dumps({
                "name": app_name, "uuid": app_uuid,
                "injectCommonJs": True, "injectEsm": True,
                "window": {"entry": "drag.html", "title": "Niva window drag smoke",
                           "size": {"width": 360, "height": 250},
                           "position": {"x": 300, "y": 200}, "visible": True,
                           "resizable": True, "decorations": True},
            }) + "\n", encoding="utf-8")
            dragger = root / "drag_pointer"
            built = subprocess.run(["swiftc", "-O", str(HERE / "drag_pointer.swift"), "-o", str(dragger)],
                                   capture_output=True, text=True, timeout=120)
            if built.returncode:
                raise SmokeError(f"cannot compile drag event helper: {built.stderr.strip()}")
            log_fd, log_path = tempfile.mkstemp(prefix="niva-drag-window-", suffix=".log")
            os.close(log_fd)
            stderr_path = Path(log_path)
            environment = os.environ.copy()
            environment["TMPDIR"] = str(root)
            with stderr_path.open("wb") as stderr_file:
                process = subprocess.Popen(
                    [str(binary), f"--config={config}", f"--resource={resources}"],
                    cwd=REPO, env=environment, stdin=subprocess.PIPE, stdout=subprocess.PIPE,
                    stderr=stderr_file, text=True, bufsize=1,
                )
            harness = Harness(process)
            ready_event = harness.next_frame(30)
            if ready_event.get("event") != "ready" or ready_event.get("name") != "drag-window":
                raise SmokeError(f"unexpected fixture ready event: {ready_event!r}")
            ready = wait_message(harness, "drag-ready", 30)
            before = ready.get("before")
            if not isinstance(before, dict) or not all(isinstance(before.get(axis), (int, float)) for axis in ("x", "y")):
                raise SmokeError(f"invalid initial native window position: {ready!r}")
            point = apple('tell application "System Events" to set frontmost of process "niva" to true')
            _ = point
            location = subprocess.run(["swift", "-e", 'import CoreGraphics; let p = CGEvent(source: nil)!.location; print("\\(p.x),\\(p.y)")'],
                                      capture_output=True, text=True, timeout=30)
            if location.returncode:
                raise SmokeError(f"cannot snapshot pointer: {location.stderr.strip()}")
            original_pointer = tuple(float(value) for value in location.stdout.strip().split(","))

            start_x = before["x"] + 130
            start_y = before["y"] + (12 if native_titlebar_control else 115)
            drag = subprocess.run([str(dragger), str(start_x), str(start_y), str(start_x + 105), str(start_y + 65)],
                                  capture_output=True, text=True, timeout=25)
            if drag.returncode:
                raise SmokeError(f"native mouse drag helper failed: {drag.stderr.strip()}")
            if native_titlebar_control:
                raw_position = apple('tell application "System Events" to tell process "niva" to get position of window 1')
                x_text, y_text = raw_position.split(",")
                after = {"x": float(x_text), "y": float(y_text)}
            else:
                result = wait_message(harness, "drag-result", 20)
                after = result.get("after")
            if not isinstance(after, dict) or not all(isinstance(after.get(axis), (int, float)) for axis in ("x", "y")):
                raise SmokeError(f"invalid final native window position: {after!r}")
            delta_x, delta_y = after["x"] - before["x"], after["y"] - before["y"]
            if abs(delta_x) < 15 and abs(delta_y) < 15:
                raise SmokeError(f"window.dragWindow did not move the native window: before={before}, after={after}")
            evidence = f"mouseDown/drag/up moved window from {before} to {after} (delta {delta_x},{delta_y})"
            if process.stdin and not process.stdin.closed:
                harness.send("fixture-exit", 0)
                process.stdin.close()
            if process.wait(timeout=15) != 0:
                raise SmokeError("Niva did not exit cleanly after fixture exit")
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
        if original_pointer is not None:
            x, y = original_pointer
            subprocess.run(["swift", "-e", f'import CoreGraphics; CGWarpMouseCursorPosition(CGPoint(x:{x},y:{y}))'],
                           capture_output=True, text=True, timeout=30)
        if previous_frontmost and previous_frontmost != "niva":
            escaped = previous_frontmost.replace('"', '\\"')
            subprocess.run(["osascript", "-e", f'tell application "System Events" to set frontmost of process "{escaped}" to true'],
                           capture_output=True, text=True, timeout=15)
        for item in dirs:
            if item.is_dir() and not item.is_symlink():
                shutil.rmtree(item)
        if stderr_path is not None:
            stderr_tail = tail(stderr_path)
            stderr_path.unlink(missing_ok=True)
    if failure:
        print(f"macOS window drag smoke: FAIL — {failure}", file=sys.stderr)
        if stderr_tail:
            print("Niva stderr tail:\n" + stderr_tail, file=sys.stderr)
        return 1
    print(f"macOS window drag smoke: PASS — window.dragWindow: {evidence}")
    return 0


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("binary", help="path to a locally built macOS Niva binary")
    parser.add_argument("--native-titlebar-control", action="store_true", help="diagnose whether synthetic mouse input moves the native titlebar")
    args = parser.parse_args()
    if sys.platform != "darwin":
        parser.error("this fixture requires macOS")
    binary = Path(args.binary).expanduser().resolve()
    if not binary.is_file():
        parser.error(f"Niva binary does not exist: {binary}")
    return run(binary, args.native_titlebar_control)


if __name__ == "__main__":
    raise SystemExit(main())
