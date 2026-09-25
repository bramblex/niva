#!/usr/bin/env python3
"""Run observable macOS window API cases in a disposable Niva process.

This runner is intentionally independent from run.py so the supervised window
cases can be run in a fresh app/profile. It does not automate mouse clicks,
Space switching, menus, Dock interactions, or other UI-only observations.
"""

from __future__ import annotations

import argparse
import json
import os
import queue
import re
import shutil
import subprocess
import sys
import tempfile
import threading
import time
import uuid
from pathlib import Path


HERE = Path(__file__).resolve().parent
REPO = HERE.parents[1]
WINDOW_CASES = HERE / "window-cases.js"
RUNNABLE_METHODS = {
    "window.sendMessage",
    "window.setAlwaysOnTop",
    "window.setAlwaysOnBottom",
    "window.setContentProtection",
    "window.setBackgroundColor",
    "window.setFocusable",
    "window.cursorPosition",
    "window.setCursorPosition",
    "window.setCursorIcon",
    "window.setCursorVisible",
    "window.blockCloseRequested",
    "windowExtra.setTrafficLightInset",
}
CONDITIONALLY_OBSERVED = {
    "window.setBackgroundColor",
    "window.setCursorIcon",
    "window.setCursorVisible",
    "windowExtra.setTrafficLightInset",
    "window.blockCloseRequested",
}
COVERED_BY_INVOCATION = {
    "window.requestRedraw",
    "window.requestUserAttention",
    "window.setVisibleOnAllWorkspaces",
    "window.setCursorGrab",
    "window.setIgnoreCursorEvents",
    "window.setImePosition",
}
COVERED_BY_SYSTEM_SUITE = {
    "windowExtra.setActivationPolicyAtRuntime",
    "windowExtra.setDockVisibility",
    "windowExtra.setBadgeLabel",
    "window.setProgressBar",
}

SWIFT_OBSERVER = r'''import AppKit
import CoreGraphics
import Foundation
import ScreenCaptureKit

@main
struct WindowObserver {
  static func main() async throws {
guard (CommandLine.arguments.count == 2 || CommandLine.arguments.count == 3 || CommandLine.arguments.count == 4),
          let pid = Int32(CommandLine.arguments[1]) else {
      fputs("usage: observer <pid> [window-id]\\n", stderr)
      exit(2)
    }
    let all = CGWindowListCopyWindowInfo(.optionAll, kCGNullWindowID) as? [[String: Any]] ?? []
    let windows: [[String: Any]] = all.compactMap { info in
      guard let owner = (info[kCGWindowOwnerPID as String] as? NSNumber)?.int32Value,
            owner == pid else { return nil }
      var row: [String: Any] = [
        "title": info[kCGWindowName as String] as? String ?? "",
        "owner": info[kCGWindowOwnerName as String] as? String ?? "",
        "windowId": (info[kCGWindowNumber as String] as? NSNumber)?.intValue ?? -1,
        "layer": (info[kCGWindowLayer as String] as? NSNumber)?.intValue ?? 0,
      ]
      if let state = info[kCGWindowSharingState as String] as? NSNumber {
        row["sharingState"] = state.intValue
      }
      return row
    }
    var output: [String: Any] = ["windows": windows]
    if CommandLine.arguments.count >= 3, let rawID = UInt32(CommandLine.arguments[2]) {
      do {
        let content = try await SCShareableContent.excludingDesktopWindows(false, onScreenWindowsOnly: false)
        guard let window = content.windows.first(where: { $0.windowID == rawID }) else {
          output["capture"] = ["available": false, "reason": "ScreenCaptureKit did not expose the requested window"]
          try write(output)
          return
        }
        let filter = SCContentFilter(desktopIndependentWindow: window)
        let configuration = SCStreamConfiguration()
        configuration.width = max(1, Int(window.frame.width * 2))
        configuration.height = max(1, Int(window.frame.height * 2))
        let showsCursor = CommandLine.arguments.count == 4 && CommandLine.arguments[3] == "1"
        configuration.showsCursor = showsCursor
        if showsCursor, let pointer = CGEvent(source: nil)?.location {
          CGEvent(mouseEventSource: nil, mouseType: .mouseMoved,
                  mouseCursorPosition: pointer, mouseButton: .left)?.post(tap: .cghidEventTap)
          usleep(120_000)
        }
        let image = try await SCScreenshotManager.captureImage(contentFilter: filter, configuration: configuration)
        let bitmap = NSBitmapImageRep(cgImage: image)
        let rawPixels = image.dataProvider?.data as Data? ?? Data()
        var imageHash: UInt64 = 1469598103934665603
        for byte in rawPixels {
          imageHash = (imageHash ^ UInt64(byte)) &* 1099511628211
        }
        func rgba(_ x: Int, _ y: Int) -> [Int]? {
          guard let color = bitmap.colorAt(x: x, y: y)?.usingColorSpace(.deviceRGB) else { return nil }
          return [
            Int((color.redComponent * 255).rounded()),
            Int((color.greenComponent * 255).rounded()),
            Int((color.blueComponent * 255).rounded()),
            Int((color.alphaComponent * 255).rounded()),
          ]
        }
        if let topLeft = rgba(8, max(0, bitmap.pixelsHigh - 8)),
           let topRight = rgba(max(0, bitmap.pixelsWide - 8), max(0, bitmap.pixelsHigh - 8)),
           let bottomLeft = rgba(8, 8),
           let bottomRight = rgba(max(0, bitmap.pixelsWide - 8), 8) {
          output["capture"] = [
            "available": true,
            "width": bitmap.pixelsWide,
            "height": bitmap.pixelsHigh,
            "digest": String(format: "%016llx", imageHash),
            "corners": [
              "topLeft": topLeft,
              "topRight": topRight,
              "bottomLeft": bottomLeft,
              "bottomRight": bottomRight,
            ],
          ]
        } else {
          output["capture"] = ["available": false, "reason": "captured image has no convertible device-RGB pixel"]
        }
      } catch {
        output["capture"] = ["available": false, "reason": String(describing: error)]
      }
    }
    try write(output)
  }

  private static func write(_ output: [String: Any]) throws {
    let data = try JSONSerialization.data(withJSONObject: output, options: [.sortedKeys])
    print(String(data: data, encoding: .utf8)!)
  }
}
'''


class SmokeError(RuntimeError):
    pass


class Harness:
    def __init__(self, process: subprocess.Popen[str]):
        self.process = process
        self.frames: queue.Queue[dict] = queue.Queue()
        self.reader = threading.Thread(target=self._read_stdout, daemon=True)
        self.reader.start()

    def _read_stdout(self) -> None:
        assert self.process.stdout is not None
        for line in self.process.stdout:
            try:
                frame = json.loads(line)
                if not isinstance(frame, dict) or frame.get("protocol") != "niva-fixture" or frame.get("version") != 1:
                    self.frames.put({"event": "bad-json", "frame": frame, "line": line.rstrip()})
                else:
                    self.frames.put(frame)
            except json.JSONDecodeError as error:
                self.frames.put({"event": "bad-json", "error": str(error), "line": line.rstrip()})
        self.frames.put({"protocol": "niva-fixture", "version": 1, "event": "eof"})

    def next_frame(self, timeout: float) -> dict:
        try:
            frame = self.frames.get(timeout=timeout)
        except queue.Empty as error:
            raise SmokeError(f"timed out after {timeout:.1f}s waiting for Niva output") from error
        if frame.get("event") == "bad-json":
            raise SmokeError(f"fixture stdout was not NDJSON: {frame}")
        if frame.get("event") == "protocol-error":
            raise SmokeError(f"fixture stdin protocol failed: {frame}")
        if frame.get("event") == "eof":
            raise SmokeError(f"fixture stdout closed (status {self.process.poll()})")
        return frame

    def send(self, name: str, data: dict | None = None) -> None:
        if self.process.stdin is None or self.process.stdin.closed:
            raise SmokeError("Niva stdin is closed")
        frame: dict = {"protocol": "niva-fixture", "version": 1, "event": "command", "name": name, "data": data}
        self.process.stdin.write(json.dumps(frame, separators=(",", ":")) + "\n")
        self.process.stdin.flush()


def app_dirs(app_name: str, app_uuid: str) -> list[Path]:
    id_name = str(uuid.UUID(app_uuid))
    return [
        Path.home() / "Library" / "Application Support" / id_name,
        Path.home() / "Library" / "Caches" / id_name,
    ]


def manual_cases() -> list[dict[str, str]]:
    """Read the UI-only action, expected observation, and cleanup from the shared case catalog."""
    import re

    pattern = re.compile(
        r'^\s*ui\("([^"]+)", "([^"]+)", "([^"]+)", "([^"]+)"\);$',
        re.MULTILINE,
    )
    text = WINDOW_CASES.read_text(encoding="utf-8")
    return [
        {"method": method, "action": action, "expected": expected, "restore": restore}
        for method, action, expected, restore in pattern.findall(text)
    ]


def validate_catalog() -> list[dict[str, str]]:
    cases = manual_cases()
    methods = {item["method"] for item in cases}
    expected = {
        "window.sendMessage", "window.setAlwaysOnTop", "window.setAlwaysOnBottom",
        "window.setBackgroundColor", "window.setContentProtection", "window.setFocusable",
        "window.setImePosition", "window.setProgressBar", "window.requestRedraw",
        "window.requestUserAttention", "window.setVisibleOnAllWorkspaces", "window.setCursorIcon",
        "window.cursorPosition", "window.setCursorPosition", "window.setCursorGrab",
        "window.setCursorVisible", "window.dragWindow", "window.setIgnoreCursorEvents",
        "window.blockCloseRequested", "windowExtra.setTrafficLightInset",
        "windowExtra.setActivationPolicyAtRuntime", "windowExtra.setDockVisibility",
        "windowExtra.setBadgeLabel",
    }
    if len(methods) != len(cases):
        raise SmokeError("window-cases.js contains duplicate supervised method descriptors")
    if methods != expected:
        raise SmokeError(
            "supervised window catalog mismatch; "
            f"missing={sorted(expected - methods)}, unexpected={sorted(methods - expected)}"
        )
    covered = RUNNABLE_METHODS | COVERED_BY_INVOCATION | COVERED_BY_SYSTEM_SUITE
    ui_only = [item for item in cases if item["method"] not in covered]
    if len(ui_only) != len(expected - covered):
        raise SmokeError(
            f"UI-only manual case count does not match the {len(expected - covered)} remaining methods"
        )
    return ui_only


def compile_observer(root: Path) -> Path:
    source = root / "window_observer.swift"
    binary = root / "window_observer"
    source.write_text(SWIFT_OBSERVER, encoding="utf-8")
    result = subprocess.run(
        ["swiftc", "-parse-as-library", "-O", str(source), "-o", str(binary)],
        check=False,
        capture_output=True,
        text=True,
        timeout=120,
    )
    if result.returncode != 0:
        detail = result.stderr.strip() or result.stdout.strip() or f"swiftc exited {result.returncode}"
        raise SmokeError(f"cannot compile CoreGraphics observer: {detail}")
    return binary


def compile_image_digest(root: Path) -> Path:
    binary = root / "image_digest"
    result = subprocess.run(
        ["swiftc", "-O", str(HERE / "image_digest.swift"), "-o", str(binary)],
        capture_output=True, text=True, timeout=120,
    )
    if result.returncode:
        raise SmokeError(f"cannot compile cursor image probe: {result.stderr.strip()}")
    return binary


def cursor_image_digest(root: Path, digester: Path, stage: str) -> str:
    activated = subprocess.run(
        ["osascript", "-e", 'tell application "System Events" to set frontmost of process "niva" to true'],
        capture_output=True, text=True, timeout=15,
    )
    if activated.returncode:
        raise SmokeError(f"cannot activate disposable Niva window for cursor capture: {activated.stderr.strip()}")
    time.sleep(0.2)
    script = '''import CoreGraphics
let p = CGEvent(source: nil)!.location
let shifted = CGPoint(x: p.x + 3, y: p.y + 3)
CGEvent(mouseEventSource: nil, mouseType: .mouseMoved,
        mouseCursorPosition: shifted, mouseButton: .left)?.post(tap: .cghidEventTap)
CGEvent(mouseEventSource: nil, mouseType: .mouseMoved,
        mouseCursorPosition: p, mouseButton: .left)?.post(tap: .cghidEventTap)
print("\\(p.x),\\(p.y)")
'''
    location = subprocess.run(["swift", "-e", script], capture_output=True, text=True, timeout=30)
    if location.returncode:
        raise SmokeError(f"cannot read pointer location: {location.stderr.strip()}")
    x_text, y_text = location.stdout.strip().split(",")
    x, y = round(float(x_text)), round(float(y_text))
    path = root / f"cursor-{stage.replace(':', '-')}.png"
    shot = subprocess.run(
        ["screencapture", "-x", "-C", "-R", f"{x - 32},{y - 32},64,64", str(path)],
        capture_output=True, text=True, timeout=15,
    )
    if shot.returncode or not path.is_file():
        raise SmokeError(f"could not capture pointer: {shot.stderr.strip()}")
    digest = subprocess.run([str(digester), str(path)], capture_output=True, text=True, timeout=15)
    if digest.returncode:
        raise SmokeError(f"could not hash pointer capture: {digest.stderr.strip()}")
    return digest.stdout.strip()


def observe_windows(observer: Path, pid: int, window_id: int | None = None, shows_cursor: bool = False) -> dict:
    command = [str(observer), str(pid)]
    if window_id is not None:
        command.append(str(window_id))
        command.append("1" if shows_cursor else "0")
    result = subprocess.run(
        command,
        check=False,
        capture_output=True,
        text=True,
        timeout=15,
    )
    if result.returncode != 0:
        detail = result.stderr.strip() or result.stdout.strip() or f"observer exited {result.returncode}"
        raise SmokeError(f"CoreGraphics window query failed: {detail}")
    try:
        snapshot = json.loads(result.stdout)
    except json.JSONDecodeError as error:
        raise SmokeError(f"CoreGraphics observer returned invalid JSON: {error}") from error
    if not isinstance(snapshot.get("windows"), list):
        raise SmokeError("CoreGraphics observer omitted its windows array")
    return snapshot


def traffic_light_position_script(pid: int, title: str) -> str:
    apple_title = json.dumps(title)
    return f'''tell application "System Events"
  set targetProcess to first application process whose unix id is {pid}
  tell targetProcess
    set targetWindow to first window whose name is {apple_title}
    set closePosition to position of (first button of targetWindow whose subrole is "AXCloseButton")
    return closePosition
  end tell
end tell'''


def traffic_light_button_positions(pid: int, title: str) -> list[list[float]]:
    script = traffic_light_position_script(pid, title)
    result = subprocess.run(
        ["osascript", "-e", script],
        check=False,
        capture_output=True,
        text=True,
        timeout=15,
    )
    if result.returncode != 0:
        detail = result.stderr.strip() or result.stdout.strip() or f"osascript exited {result.returncode}"
        raise SmokeError(f"System Events could not read the disposable titlebar controls: {detail}")
    values = re.findall(r"-?\d+(?:\.\d+)?", result.stdout)
    if len(values) != 2:
        raise SmokeError(f"System Events returned malformed titlebar button positions: {result.stdout!r}")
    coordinates = [float(value) for value in values]
    return [coordinates]


def native_close_press_script(pid: int, title: str) -> str:
    apple_title = json.dumps(title)
    return f'''tell application "System Events"
  set targetProcess to first application process whose unix id is {pid}
  tell targetProcess
    set targetWindow to first window whose name is {apple_title}
    set closeButton to (first button of targetWindow whose subrole is "AXCloseButton")
    perform action "AXPress" of closeButton
  end tell
end tell'''


def press_native_close_button(pid: int, title: str) -> None:
    script = native_close_press_script(pid, title)
    result = subprocess.run(
        ["osascript", "-e", script],
        check=False,
        capture_output=True,
        text=True,
        timeout=15,
    )
    if result.returncode != 0:
        detail = result.stderr.strip() or result.stdout.strip() or f"osascript exited {result.returncode}"
        raise SmokeError(f"System Events could not press the disposable close button: {detail}")


def tail(path: Path) -> str:
    try:
        return "\n".join(path.read_text(encoding="utf-8", errors="replace").splitlines()[-80:])
    except OSError:
        return ""


def print_manifest(cases: list[dict[str, str]]) -> None:
    print(f"Executable native observation cases ({len(RUNNABLE_METHODS)}):")
    explanations = {
        "window.sendMessage": "child receives a unique nonce and returns an exact IPC echo",
        "window.setAlwaysOnTop": "CoreGraphics kCGWindowLayer rises, then restores",
        "window.setAlwaysOnBottom": "CoreGraphics kCGWindowLayer falls, then restores",
        "window.setContentProtection": "CoreGraphics sharing state changes to None, then restores",
        "window.setBackgroundColor": "samples a transparent child window's native color; requires Screen Recording access",
        "window.setFocusable": "a non-focusable child rejects a native focus request",
        "window.cursorPosition": "native pointer coordinates read back at the requested location",
        "window.setCursorPosition": "moves the native pointer to a child-local logical coordinate",
        "window.setCursorIcon": "ScreenCaptureKit image digest changes for the requested cursor shape; requires Screen Recording access",
        "window.setCursorVisible": "ScreenCaptureKit image digest changes when the cursor is hidden and returns when shown; requires Screen Recording access",
        "window.blockCloseRequested": "AXPress requests native close; closeRequested is echoed while the window stays visible",
        "windowExtra.setTrafficLightInset": "System Events reports the close/minimize/zoom controls at the requested inset",
    }
    for method in sorted(RUNNABLE_METHODS):
        condition = " (conditional: Accessibility access)" if method == "windowExtra.setTrafficLightInset" else (
            " (conditional: Screen Recording access)" if method in CONDITIONALLY_OBSERVED else ""
        )
        print(f"  {method}{condition}: {explanations[method]}")
    print(f"Additional native cases covered by run_system_supervised.py ({len(COVERED_BY_SYSTEM_SUITE)}):")
    for method in sorted(COVERED_BY_SYSTEM_SUITE):
        print(f"  {method}")
    invocation_descriptions = {
        "window.requestRedraw": "API call resolves; redraw side effect is not independently observed",
        "window.requestUserAttention": "informational attention request resolves, then the test app is focused",
        "window.setVisibleOnAllWorkspaces": "true and restore-false calls resolve; Space visibility is not independently observed",
        "window.setCursorGrab": "grab and release calls resolve; physical pointer capture is not independently observed",
        "window.setIgnoreCursorEvents": "ignore and restore calls resolve; click-through is not independently observed",
        "window.setImePosition": "position call resolves while the disposable child input is focused; IME candidate location is not independently observed",
    }
    print(f"API invocation-only checks ({len(COVERED_BY_INVOCATION)}): effect not asserted")
    for method in sorted(COVERED_BY_INVOCATION):
        print(f"  {method}: {invocation_descriptions[method]}")
    print(f"UI-only native observation cases ({len(cases)}): NOT RUN by this automation")
    for item in cases:
        print(f"  {item['method']}")
        print(f"    action: {item['action']}")
        print(f"    observe: {item['expected']}")
        print(f"    restore: {item['restore']}")


def run(binary: Path, cases: list[dict[str, str]]) -> int:
    app_name = "NivaWindowSupervisedSmoke"
    app_uuid = str(uuid.uuid4())
    dirs = app_dirs(app_name, app_uuid)
    if any(path.exists() for path in dirs):
        raise SmokeError("generated app identity unexpectedly already exists; retry the run")

    process: subprocess.Popen[str] | None = None
    stderr_path: Path | None = None
    failure: str | None = None
    observed: dict[str, str] = {}
    invoked: dict[str, str] = {}
    skipped: dict[str, str] = {}
    expected = set(RUNNABLE_METHODS)
    expected_invocations = set(COVERED_BY_INVOCATION)
    frontmost = subprocess.run(
        ["osascript", "-e", 'tell application "System Events" to get name of first process whose frontmost is true'],
        capture_output=True, text=True, timeout=15,
    )
    previous_frontmost = frontmost.stdout.strip() if frontmost.returncode == 0 else None

    try:
        with tempfile.TemporaryDirectory(prefix="niva-window-supervised-") as temporary:
            root = Path(temporary)
            resources = root / "resources"
            resources.mkdir()
            shutil.copy2(HERE / "window-supervised.html", resources / "window-supervised.html")
            shutil.copy2(HERE / "fixture-protocol.js", resources / "fixture-protocol.js")
            (resources / "window-supervised-child.html").write_text(
                """<!doctype html><html lang=\"en\"><head><meta charset=\"utf-8\"><title>IPC child</title></head>
                <body style=\"margin:0;background:transparent\"><div style=\"position:fixed;left:0;top:0;width:32px;height:32px;background:rgb(241,31,197)\"></div>
                <input id=\"imeProbe\" aria-label=\"IME position probe\" style=\"position:fixed;left:52px;top:62px;width:130px\">
                <script>
                  (() => {
                    Niva.addEventListener(\"window.closeRequested\", async () => {
                      const id = await Niva.window.current();
                      await Niva.window.sendMessage(`close-requested:${id}`, 0).catch(() => {});
                    });
                    Niva.addEventListener(\"window.message\", (_name, payload) => {
                      if (payload && payload.from === 0 && payload.message === \"focus-ime\") {
                        document.querySelector(\"#imeProbe\").focus();
                        setTimeout(async () => {
                          const id = await Niva.window.current();
                          await Niva.window.sendMessage(`ime-focused:${id}`, 0).catch(() => {});
                        }, 120);
                      }
                      if (payload && payload.from === 0 && payload.message.startsWith(\"ping:\")) {
                        Niva.window.sendMessage(payload.message.replace(/^ping:/, \"pong:\"), payload.from).catch(() => {});
                      }
                      if (payload && payload.from === 0 && payload.message === \"cursor-query\") {
                        Niva.window.sendMessage(`cursor-style:${getComputedStyle(document.querySelector(\"div\")).cursor}`, 0).catch(() => {});
                      }
                    });
                    window.addEventListener(\"load\", () => {
                      setTimeout(async () => {
                        const id = await Niva.window.current();
                        await Niva.window.sendMessage(`ready:${id}`, 0);
                      }, 150);
                    }, { once: true });
                  })();
                </script></body></html>\n""",
                encoding="utf-8",
            )
            observer = compile_observer(root)
            image_digester = compile_image_digest(root)
            config = root / "niva.json"
            config.write_text(
                json.dumps(
                    {
                        "name": app_name,
                        "uuid": app_uuid,
                        "injectCommonJs": True,
                        "injectEsm": True,
                        "window": {
                            "entry": "window-supervised.html",
                            "title": "Niva supervised window smoke",
                            "size": {"width": 620, "height": 420},
                            "visible": True,
                            "resizable": True,
                            "decorations": True,
                        },
                    },
                    indent=2,
                ) + "\n",
                encoding="utf-8",
            )
            log_fd, log_name = tempfile.mkstemp(prefix="niva-window-supervised-", suffix=".log")
            os.close(log_fd)
            stderr_path = Path(log_name)
            with stderr_path.open("wb") as stderr_file:
                child_env = os.environ.copy()
                child_env["TMPDIR"] = str(root)
                process = subprocess.Popen(
                    [str(binary), f"--config={config}", f"--resource={resources}"],
                    cwd=REPO,
                    env=child_env,
                    stdin=subprocess.PIPE,
                    stdout=subprocess.PIPE,
                    stderr=stderr_file,
                    text=True,
                    bufsize=1,
                )

            harness = Harness(process)
            ready = harness.next_frame(timeout=30)
            if ready.get("event") != "ready" or ready.get("name") != "window-supervised":
                raise SmokeError(f"unexpected fixture ready event: {ready!r}")

            suite_ready = False
            suite_done: dict | None = None
            deadline = time.monotonic() + 240
            while time.monotonic() < deadline:
                frame = harness.next_frame(timeout=max(0.1, deadline - time.monotonic()))
                if frame.get("event") != "message":
                    continue
                name = frame.get("name")
                data = frame.get("data") or {}
                if name == "window-supervised-ready":
                    suite_ready = True
                    print(f"APP READY: pid={process.pid}; main title={data.get('title')}", flush=True)
                elif name == "window-supervised-observe":
                    request_id = data.get("requestId")
                    title = data.get("title")
                    if not isinstance(request_id, str) or not isinstance(title, str):
                        raise SmokeError(f"malformed native observation request: {data!r}")
                    raw_window_id = data.get("captureWindowId")
                    if raw_window_id is not None and type(raw_window_id) is not int:
                        raise SmokeError(f"native capture request has an invalid window id: {data!r}")
                    raw_shows_cursor = data.get("showsCursor", False)
                    if type(raw_shows_cursor) is not bool:
                        raise SmokeError(f"native capture request has an invalid showsCursor flag: {data!r}")
                    snapshot = observe_windows(observer, process.pid, raw_window_id, raw_shows_cursor)
                    if raw_shows_cursor and raw_window_id is not None:
                        try:
                            snapshot.setdefault("capture", {})["cursorDigest"] = cursor_image_digest(root, image_digester, request_id)
                        except SmokeError as error:
                            snapshot.setdefault("capture", {})["cursorReason"] = str(error)
                    harness.send(
                        "window-supervised-command",
                        {"kind": "observation", "requestId": request_id, "snapshot": snapshot},
                    )
                elif name == "window-supervised-accessibility":
                    request_id = data.get("requestId")
                    title = data.get("title")
                    if not isinstance(request_id, str) or not isinstance(title, str):
                        raise SmokeError(f"malformed Accessibility observation request: {data!r}")
                    try:
                        if data.get("action") == "press-close":
                            press_native_close_button(process.pid, title)
                            response = {"kind": "accessibility", "requestId": request_id, "pressed": True}
                        else:
                            positions = traffic_light_button_positions(process.pid, title)
                            response = {"kind": "accessibility", "requestId": request_id, "positions": positions}
                    except SmokeError as error:
                        response = {"kind": "accessibility", "requestId": request_id, "error": str(error)}
                    harness.send("window-supervised-command", response)
                elif name == "window-supervised-case":
                    method = data.get("method")
                    if method not in expected or method in observed or method in skipped:
                        raise SmokeError(f"unexpected or duplicate case report: {data!r}")
                    if data.get("status") != "passed" or not isinstance(data.get("assertion"), str):
                        raise SmokeError(f"invalid case report: {data!r}")
                    observed[method] = data["assertion"]
                    print(f"PASS {method}: {data['assertion']}", flush=True)
                elif name == "window-supervised-skip":
                    method = data.get("method")
                    reason = data.get("reason")
                    if method not in expected or method in observed or method in skipped or not isinstance(reason, str):
                        raise SmokeError(f"invalid or duplicate skipped-case report: {data!r}")
                    skipped[method] = reason
                    print(f"SKIP {method}: {reason}", flush=True)
                elif name == "window-supervised-invocation":
                    method = data.get("method")
                    evidence = data.get("evidence")
                    if data.get("status") != "invoked-only" or method not in expected_invocations or method in invoked or not isinstance(evidence, str):
                        raise SmokeError(f"invalid or duplicate API invocation report: {data!r}")
                    invoked[method] = evidence
                    print(f"INVOKED {method}: {evidence}", flush=True)
                elif name == "window-supervised-error":
                    failure = f"{data.get('method', 'unknown')}: {data.get('message', data)}"
                    print(f"FAIL {failure}", file=sys.stderr, flush=True)
                elif name == "window-supervised-done":
                    suite_done = data
                    break

            if not suite_ready:
                raise SmokeError("page never reported window-supervised-ready")
            if suite_done is None:
                raise SmokeError("timed out before the supervised page reported completion")
            if suite_done.get("failed") and failure is None:
                failure = str(suite_done["failed"])
            if set(suite_done.get("passed", [])) != observed.keys():
                raise SmokeError("page completion list did not match its exact per-method reports")
            if set(item.get("method") for item in suite_done.get("skipped", [])) != skipped.keys():
                raise SmokeError("page completion skip list did not match its exact per-method reports")
            if set(item.get("method") for item in suite_done.get("invoked", [])) != invoked.keys():
                raise SmokeError("page completion invocation list did not match its exact per-method reports")
            reported = observed.keys() | skipped.keys()
            if reported != expected:
                raise SmokeError(
                    f"native case coverage mismatch; missing={sorted(expected - reported)}, "
                    f"unexpected={sorted(reported - expected)}"
                )
            if invoked.keys() != expected_invocations:
                raise SmokeError(
                    f"API invocation coverage mismatch; missing={sorted(expected_invocations - invoked.keys())}, "
                    f"unexpected={sorted(invoked.keys() - expected_invocations)}"
                )

            harness.send("window-supervised-command", {"kind": "exit-with-child"})
            exit_child = None
            exit_deadline = time.monotonic() + 15
            while time.monotonic() < exit_deadline:
                frame = harness.next_frame(max(0.1, exit_deadline - time.monotonic()))
                if frame.get("event") == "message" and frame.get("name") == "window-exit-child-ready":
                    exit_child = frame.get("data", {}).get("id")
                    break
                if frame.get("event") == "message" and frame.get("name") == "window-supervised-error":
                    raise SmokeError(f"exit-with-child case failed: {frame.get('data')}")
            if not isinstance(exit_child, int) or exit_child <= 0:
                raise SmokeError("process.exit fixture did not confirm a live child window before exit")
            status = process.wait(timeout=20)
            if status != 0:
                raise SmokeError(f"isolated Niva process with child window exited with status {status}")
            print(f"PASS process.exit: main exited cleanly with owned child window {exit_child} still open", flush=True)
    except Exception as error:
        failure = failure or f"{type(error).__name__}: {error}"
    finally:
        if process is not None and process.poll() is None:
            try:
                if process.stdin and not process.stdin.closed:
                    process.stdin.close()
                process.wait(timeout=10)
            except subprocess.TimeoutExpired:
                process.terminate()
                try:
                    process.wait(timeout=5)
                except subprocess.TimeoutExpired:
                    process.kill()
                    process.wait(timeout=5)
        for path in dirs:
            if path.is_dir() and not path.is_symlink():
                shutil.rmtree(path)
        if stderr_path is not None:
            stderr_tail = tail(stderr_path)
            stderr_path.unlink(missing_ok=True)
        else:
            stderr_tail = ""
        if previous_frontmost and previous_frontmost != "niva":
            escaped = previous_frontmost.replace('"', '\\"')
            subprocess.run(
                ["osascript", "-e", f'tell application "System Events" to set frontmost of process "{escaped}" to true'],
                capture_output=True, text=True, timeout=15,
            )

    if failure:
        print(f"macOS supervised window smoke: FAIL — {failure}", file=sys.stderr)
        if stderr_tail:
            print("Niva stderr tail:\n" + stderr_tail, file=sys.stderr)
        print(f"Method cases observed: {len(observed)}/{len(expected)}", file=sys.stderr)
        return 1

    if skipped:
        print(f"macOS supervised window smoke: INCOMPLETE ({len(observed)} behavior passed, {len(skipped)} skipped, {len(invoked)} invocation-only)")
        for method, reason in sorted(skipped.items()):
            print(f"  SKIP {method}: {reason}")
        for method, evidence in sorted(invoked.items()):
            print(f"  INVOKED ONLY {method}: {evidence}")
        print_manifest(cases)
        return 2

    print(f"macOS supervised window smoke: PASS ({len(observed)} behavior cases, {len(invoked)} invocation-only)")
    for method in sorted(observed):
        print(f"  {method}: {observed[method]}")
    for method, evidence in sorted(invoked.items()):
        print(f"  INVOKED ONLY {method}: {evidence}")
    print_manifest(cases)
    return 0


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("binary", nargs="?", help="path to the locally built macOS niva binary")
    parser.add_argument("--list-cases", action="store_true", help="list executable and UI-only cases without launching Niva")
    args = parser.parse_args()
    try:
        cases = validate_catalog()
    except (OSError, SmokeError) as error:
        raise SystemExit(f"case catalog error: {error}") from error
    if args.list_cases:
        print_manifest(cases)
        return 0
    if sys.platform != "darwin":
        raise SystemExit("This fixture requires macOS; it does not emulate native APIs on another OS.")
    if not args.binary:
        parser.error("binary is required unless --list-cases is used")
    binary = Path(args.binary).expanduser().resolve()
    if not binary.is_file():
        raise SystemExit(f"Niva binary does not exist: {binary}")
    try:
        return run(binary, cases)
    except SmokeError as error:
        print(f"macOS supervised window smoke: FAIL — {error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
