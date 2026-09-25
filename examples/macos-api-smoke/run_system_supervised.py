#!/usr/bin/env python3
"""Exercise macOS system APIs with disposable UI and external state checks."""

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

from run import Harness, SmokeError, REPO, safe_app_dirs, tail


HERE = Path(__file__).resolve().parent


def apple(script: str, timeout: int = 10) -> str:
    result = subprocess.run(["osascript", "-e", script], capture_output=True, text=True, timeout=timeout)
    if result.returncode:
        raise SmokeError(f"macOS UI observation failed: {result.stderr.strip() or result.stdout.strip()}")
    return result.stdout.strip()


def app_visible() -> bool:
    return apple('tell application "System Events" to tell process "niva" to get visible') == "true"


def wait_for(predicate, label: str, seconds: float = 8) -> None:
    until = time.monotonic() + seconds
    while time.monotonic() < until:
        if predicate():
            return
        time.sleep(0.2)
    raise SmokeError(f"timed out observing {label}")


def next_message(harness: Harness, name: str, timeout: float = 30) -> dict:
    deadline = time.monotonic() + timeout
    while True:
        frame = harness.next_frame(max(0.1, deadline - time.monotonic()))
        if frame.get("event") == "message" and frame.get("name") == "system-error":
            raise SmokeError(f"page reported {frame.get('data')}")
        if frame.get("event") == "message" and frame.get("name") == name:
            data = frame.get("data")
            if not isinstance(data, dict):
                raise SmokeError(f"{name} had no object payload")
            return data
        if time.monotonic() >= deadline:
            raise SmokeError(f"timed out waiting for {name}")


def command(harness: Harness, kind: str, **data: str) -> None:
    harness.send("system-command", {"kind": kind, **data})


def phase(harness: Harness, expected: str, timeout: float = 30) -> dict:
    data = next_message(harness, "system-phase", timeout)
    if data.get("phase") != expected:
        raise SmokeError(f"expected system phase {expected}, got {data!r}")
    return data


def ack(harness: Harness, name: str) -> None:
    command(harness, "ack", phase=name)


def finder_front_target() -> str:
    return apple('tell application "Finder" to get POSIX path of (target of front window as alias)')


def print_panel_open() -> bool:
    return apple('tell application "System Events" to tell process "niva" to get count of sheets of window 1') != "0"


def dock_names() -> str:
    return apple('tell application "System Events" to tell process "Dock" to get name of every UI element of list 1')


def dock_descriptions() -> str:
    return apple('tell application "System Events" to tell process "Dock" to get description of every UI element of list 1')


def dock_values() -> str:
    return apple('tell application "System Events" to tell process "Dock" to get value of every UI element of list 1')


def dock_icon_bounds() -> tuple[int, int, int, int]:
    raw = apple('tell application "System Events" to tell process "Dock" to get {position, size} of (first UI element of list 1 whose name is "niva")')
    values = [int(part.strip()) for part in raw.split(",")]
    if len(values) != 4 or min(values[2:]) <= 0:
        raise SmokeError(f"invalid Niva Dock icon bounds: {raw}")
    return values[0], values[1], values[2], values[3]


def move_pointer_for_dock(x: int) -> None:
    script = f'''import CoreGraphics
let p = CGPoint(x: {x}, y: CGDisplayBounds(CGMainDisplayID()).maxY - 1)
CGWarpMouseCursorPosition(p)
CGEvent(mouseEventSource: nil, mouseType: .mouseMoved,
        mouseCursorPosition: p, mouseButton: .left)?.post(tap: .cghidEventTap)
'''
    result = subprocess.run(["swift", "-e", script], capture_output=True, text=True, timeout=30)
    if result.returncode:
        raise SmokeError(f"cannot reveal Dock: {result.stderr.strip()}")
    time.sleep(0.6)


def pointer_position() -> tuple[float, float]:
    result = subprocess.run(["swift", "-e", 'import CoreGraphics; let p = CGEvent(source: nil)!.location; print("\\(p.x),\\(p.y)")'],
                            capture_output=True, text=True, timeout=30)
    if result.returncode:
        raise SmokeError(f"cannot snapshot pointer location: {result.stderr.strip()}")
    x, y = result.stdout.strip().split(",")
    return float(x), float(y)


def restore_pointer(position: tuple[float, float]) -> None:
    x, y = position
    script = f'''import CoreGraphics
let p = CGPoint(x: {x}, y: {y})
CGWarpMouseCursorPosition(p)
CGEvent(mouseEventSource: nil, mouseType: .mouseMoved,
        mouseCursorPosition: p, mouseButton: .left)?.post(tap: .cghidEventTap)
'''
    subprocess.run(["swift", "-e", script], capture_output=True, text=True, timeout=30)


def dock_red_pixels(root: Path, counter: Path, stage: str) -> int:
    x, _, width, _ = dock_icon_bounds()
    move_pointer_for_dock(x + width // 2)
    x, y, width, height = dock_icon_bounds()
    image = root / f"dock-{stage}.png"
    shot = subprocess.run(["screencapture", "-x", "-R", f"{x},{y},{width},{height}", str(image)],
                          capture_output=True, text=True, timeout=15)
    if shot.returncode or not image.is_file():
        raise SmokeError(f"could not capture Niva Dock icon: {shot.stderr.strip()}")
    counted = subprocess.run([str(counter), str(image)], capture_output=True, text=True, timeout=15)
    if counted.returncode:
        raise SmokeError(f"Dock pixel probe failed: {counted.stderr.strip()}")
    return int(counted.stdout.strip())


def visible_apps() -> list[str]:
    names = apple('tell application "System Events" to get name of every application process whose visible is true')
    return [name.strip() for name in names.split(",") if name.strip()]


def run(binary: Path, activation: bool, hide_others: bool, window_extra: bool, dock_badge: bool, progress: bool) -> int:
    app_name = "NivaSystemSupervisedSmoke"
    app_uuid = str(uuid.uuid4())
    dirs = safe_app_dirs(app_name, app_uuid)
    if any(item.exists() for item in dirs):
        raise SmokeError("unique disposable app identity already exists")
    process: subprocess.Popen[str] | None = None
    stderr_path: Path | None = None
    evidence: dict[str, str] = {}
    original_pointer: tuple[float, float] | None = None
    failure: str | None = None
    stderr_tail = ""
    try:
        with tempfile.TemporaryDirectory(prefix="niva-system-supervised-") as temporary:
            root = Path(temporary).resolve()
            resources = root / "resources"
            resources.mkdir()
            shutil.copy2(HERE / "system-supervised.html", resources / "system-supervised.html")
            shutil.copy2(HERE / "fixture-protocol.js", resources / "fixture-protocol.js")
            (root / "finder-target").mkdir()
            config = root / "niva.json"
            config.write_text(json.dumps({
                "name": app_name, "uuid": app_uuid,
                "injectCommonJs": True, "injectEsm": True,
                "window": {"entry": "system-supervised.html", "title": "Niva system supervised smoke",
                           "size": {"width": 520, "height": 360}, "visible": True},
                "api": {"timeoutMs": 180000},
            }) + "\n", encoding="utf-8")
            log_fd, log_path = tempfile.mkstemp(prefix="niva-system-supervised-", suffix=".log")
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
            if ready_event.get("event") != "ready" or ready_event.get("name") != "system-supervised":
                raise SmokeError(f"unexpected fixture ready event: {ready_event!r}")
            next_message(harness, "system-ready")
            print(f"APP READY: pid={process.pid}", flush=True)

            command(harness, "hide-show")
            phase(harness, "hidden")
            wait_for(lambda: not app_visible(), "app hidden")
            evidence["extra.hideApplication"] = "System Events saw the disposable app become hidden"
            ack(harness, "hidden")
            phase(harness, "shown")
            wait_for(app_visible, "app shown")
            evidence["extra.showApplication"] = "System Events saw the same app become visible again"
            ack(harness, "shown")

            if activation:
                baseline = dock_names()
                if "niva" not in baseline.casefold():
                    raise SmokeError(f"Niva Dock icon was absent before activation-policy test: {baseline}")
                command(harness, "activation")
                phase(harness, "accessory")
                wait_for(lambda: "niva" not in dock_names().casefold(), "accessory Dock icon hidden")
                ack(harness, "accessory")
                phase(harness, "regular")
                wait_for(lambda: "niva" in dock_names().casefold(), "regular Dock icon restored")
                evidence["extra.setActivationPolicy"] = "Dock item disappeared in accessory policy and returned in regular policy"
                ack(harness, "regular")

            if window_extra:
                if "niva" not in dock_names().casefold():
                    raise SmokeError("Niva Dock item absent before windowExtra checks")
                command(harness, "activation-extra")
                phase(harness, "extra-accessory")
                wait_for(lambda: "niva" not in dock_names().casefold(), "windowExtra accessory Dock state")
                ack(harness, "extra-accessory")
                phase(harness, "extra-regular")
                wait_for(lambda: "niva" in dock_names().casefold(), "windowExtra regular Dock state")
                evidence["windowExtra.setActivationPolicyAtRuntime"] = "Dock item disappeared in accessory policy and returned in regular policy"
                ack(harness, "extra-regular")

                command(harness, "dock-visibility")
                phase(harness, "dock-hidden")
                wait_for(lambda: "niva" not in dock_names().casefold(), "Dock item hidden")
                ack(harness, "dock-hidden")
                phase(harness, "dock-shown")
                wait_for(lambda: "niva" in dock_names().casefold(), "Dock item shown")
                evidence["windowExtra.setDockVisibility"] = "Dock icon disappeared and returned on the two native calls"
                ack(harness, "dock-shown")

            if dock_badge:
                original_pointer = pointer_position()
                counter = root / "dock-red-pixels"
                build_counter = subprocess.run(["swiftc", "-O", str(HERE / "dock_red_pixels.swift"), "-o", str(counter)],
                                               capture_output=True, text=True, timeout=120)
                if build_counter.returncode:
                    raise SmokeError(f"cannot compile Dock pixel probe: {build_counter.stderr.strip()}")
                before_red = dock_red_pixels(root, counter, "before")
                command(harness, "dock-badge")
                phase(harness, "badge-set")
                badge_red = dock_red_pixels(root, counter, "badge")
                print(f"DOCK RED PIXELS: before={before_red}, badge={badge_red}", flush=True)
                if badge_red <= before_red + 30:
                    raise SmokeError("Dock icon did not gain the expected red badge pixels")
                ack(harness, "badge-set")
                phase(harness, "badge-cleared")
                cleared_red = dock_red_pixels(root, counter, "cleared")
                if cleared_red >= badge_red - 30:
                    raise SmokeError("Dock badge pixels did not disappear after clearing the label")
                evidence["windowExtra.setBadgeLabel"] = f"Dock icon red pixels changed {before_red}->{badge_red}->{cleared_red} across set/clear"
                ack(harness, "badge-cleared")

            if progress:
                if original_pointer is None:
                    original_pointer = pointer_position()
                counter = root / "dock-red-pixels"
                if not counter.is_file():
                    build_counter = subprocess.run(["swiftc", "-O", str(HERE / "dock_red_pixels.swift"), "-o", str(counter)],
                                                   capture_output=True, text=True, timeout=120)
                    if build_counter.returncode:
                        raise SmokeError(f"cannot compile Dock pixel probe: {build_counter.stderr.strip()}")
                before_red = dock_red_pixels(root, counter, "progress-before")
                command(harness, "progress")
                phase(harness, "progress-set")
                progress_red = dock_red_pixels(root, counter, "progress-set")
                print(f"DOCK PROGRESS RED PIXELS: before={before_red}, progress={progress_red}", flush=True)
                if progress_red <= before_red + 20:
                    raise SmokeError("Dock icon did not gain the error-state progress indicator")
                ack(harness, "progress-set")
                phase(harness, "progress-cleared")
                cleared_red = dock_red_pixels(root, counter, "progress-cleared")
                if cleared_red >= progress_red - 20:
                    raise SmokeError("Dock progress indicator did not clear")
                evidence["window.setProgressBar"] = f"Dock icon red pixels changed {before_red}->{progress_red}->{cleared_red} across progress set/clear"
                ack(harness, "progress-cleared")

            if hide_others:
                original_visible = visible_apps()
                if "Google Chrome" not in original_visible:
                    raise SmokeError("Google Chrome was not a visible comparison app")
                command(harness, "hide-others")
                try:
                    phase(harness, "others-hidden")
                    wait_for(lambda: "Google Chrome" not in visible_apps(), "other application hidden")
                    evidence["extra.hideOtherApplications"] = "Chrome was visible beforehand and hidden after the Niva call"
                finally:
                    for name in original_visible:
                        if name == "niva":
                            continue
                        escaped = name.replace('"', '\\"')
                        try:
                            apple(f'tell application "System Events" to set visible of application process "{escaped}" to true')
                        except SmokeError:
                            pass
                    if "Google Chrome" not in visible_apps():
                        raise SmokeError("could not restore Chrome visibility after hideOtherApplications")
                    ack(harness, "others-hidden")

            command(harness, "print")
            phase(harness, "print-opened")
            wait_for(print_panel_open, "native print panel")
            apple('tell application "System Events" to tell process "niva" to click button "Cancel" of splitter group 1 of sheet 1 of window 1')
            wait_for(lambda: not print_panel_open(), "print panel dismissal", 15)
            evidence["webview.print"] = "macOS print sheet appeared and was cancelled without submitting a job"
            ack(harness, "print-opened")

            command(harness, "open", path=str(root / "finder-target"))
            phase(harness, "opened")
            wait_for(lambda: finder_front_target().rstrip("/") == str(root / "finder-target"), "Finder target")
            evidence["Niva.bridge.callProcessOpen"] = "Finder opened the exact disposable directory"
            apple('tell application "Finder" to close front window')
            ack(harness, "opened")

            command(harness, "load-html")
            phase(harness, "load-html-started")
            wait_for(lambda: apple('tell application "System Events" to tell process "niva" to get name of window 1') == "Niva inline HTML verified", "inline HTML title")
            evidence["webview.loadHtml"] = "loaded inline HTML and macOS window title changed to its unique marker"
            harness.send("fixture-exit", 0)
            if process.stdin and not process.stdin.closed:
                process.stdin.close()
            if process.wait(timeout=15) != 0:
                raise SmokeError("Niva did not exit cleanly after fixture exit")
    except Exception as error:
        failure = f"{type(error).__name__}: {error}"
    finally:
        if original_pointer is not None:
            try:
                restore_pointer(original_pointer)
            except (OSError, subprocess.TimeoutExpired):
                pass
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
        print(f"macOS system supervised smoke: FAIL — {failure}", file=sys.stderr)
        if stderr_tail:
            print("Niva stderr tail:\n" + stderr_tail, file=sys.stderr)
        print(f"Method cases observed: {len(evidence)}/{5 + int(activation) + int(hide_others) + 2 * int(window_extra) + int(dock_badge) + int(progress)}", file=sys.stderr)
        return 1
    print(f"macOS system supervised smoke: PASS ({len(evidence)} exact method cases)")
    for method, assertion in sorted(evidence.items()):
        print(f"  {method}: {assertion}")
    if not hide_others:
        print("  extra.hideOtherApplications: NOT RUN (pass --hide-others for a visibility snapshot/restore)")
    if not activation:
        print("  extra.setActivationPolicy: NOT RUN (pass --activation for Dock observation)")
    if not window_extra:
        print("  windowExtra.setActivationPolicyAtRuntime/setDockVisibility: NOT RUN (pass --window-extra)")
    if not dock_badge:
        print("  windowExtra.setBadgeLabel: NOT RUN (pass --dock-badge)")
    if not progress:
        print("  window.setProgressBar: NOT RUN (pass --progress)")
    return 0


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("binary", help="path to a locally built macOS Niva binary")
    parser.add_argument("--activation", action="store_true", help="observe regular/accessory Dock visibility")
    parser.add_argument("--hide-others", action="store_true", help="temporarily hide and restore other visible apps")
    parser.add_argument("--window-extra", action="store_true", help="observe macOS windowExtra Dock operations")
    parser.add_argument("--dock-badge", action="store_true", help="observe macOS Dock badge after setting its label")
    parser.add_argument("--progress", action="store_true", help="observe macOS Dock error progress indicator")
    args = parser.parse_args()
    if sys.platform != "darwin":
        parser.error("this test requires macOS")
    binary = Path(args.binary).expanduser().resolve()
    if not binary.is_file():
        parser.error(f"Niva binary does not exist: {binary}")
    return run(binary, args.activation, args.hide_others, args.window_extra, args.dock_badge, args.progress)


if __name__ == "__main__":
    raise SystemExit(main())
