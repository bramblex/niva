#!/usr/bin/env python3
"""Run an isolated macOS Niva app and assert selected native API behavior."""

from __future__ import annotations

import argparse
import json
import os
import queue
import shutil
import struct
import subprocess
import sys
import tempfile
import threading
import time
import uuid
import zlib
from pathlib import Path


HERE = Path(__file__).resolve().parent
REPO = HERE.parents[1]
AUTOMATIC_METHODS = {
    "monitor.list",
    "monitor.current",
    "monitor.primary",
    "monitor.fromPoint",
    "window.current",
    "window.title",
    "window.list",
    "window.innerSize",
    "window.outerSize",
    "window.innerPosition",
    "window.outerPosition",
    "window.scaleFactor",
    "window.isResizable",
    "window.setResizable",
    "window.isVisible",
    "window.setVisible",
    "window.setFocus",
    "window.isFocused",
    "window.setTitle",
    "window.setInnerSize",
    "window.open",
    "window.close",
    "webview.baseUrl",
    "webview.baseFileSystemUrl",
    "webview.url",
    "webview.canGoBack",
    "webview.canGoForward",
    "webview.evaluateScript",
}
TRAY_METHODS = {"tray.create", "tray.list", "tray.update", "tray.destroy", "tray.destroyAll"}
SHORTCUT_METHODS = {
    "shortcut.register",
    "shortcut.list",
    "shortcut.unregister",
    "shortcut.unregisterAll",
}
WEBVIEW_HISTORY_METHODS = {
    "webview.loadUrl",
    "webview.goBack",
    "webview.goForward",
    "webview.reload",
}


class SmokeError(RuntimeError):
    pass


class Harness:
    def __init__(self, process: subprocess.Popen[str]):
        self.process = process
        self.frames: queue.Queue[dict] = queue.Queue()
        self.backlog: list[dict] = []
        self.reader = threading.Thread(target=self._read_stdout, daemon=True)
        self.reader.start()

    def _read_stdout(self) -> None:
        assert self.process.stdout is not None
        for line in self.process.stdout:
            try:
                self.frames.put(json.loads(line))
            except json.JSONDecodeError as error:
                self.frames.put({"t": "bad-json", "error": str(error), "line": line.rstrip()})
        self.frames.put({"t": "eof"})

    def _check_error(self, frame: dict) -> None:
        if frame.get("t") == "bad-json":
            raise SmokeError(f"Niva stdout was not NDJSON: {frame}")
        name = frame.get("name")
        if name in {"smoke-error", "command-error", "dialog-error", "main-reloaded-error", "main-restored-error", "secondary-error"}:
            raise SmokeError(f"page reported {name}: {frame.get('data')}")

    def next_frame(self, timeout: float) -> dict:
        if self.backlog:
            frame = self.backlog.pop(0)
            self._check_error(frame)
            return frame
        try:
            frame = self.frames.get(timeout=timeout)
        except queue.Empty as error:
            raise SmokeError(f"timed out after {timeout:.1f}s waiting for Niva output") from error
        self._check_error(frame)
        if frame.get("t") == "eof":
            raise SmokeError(f"Niva stdout closed (status {self.process.poll()})")
        return frame

    def wait_message(self, name: str, timeout: float = 20) -> dict:
        for index, frame in enumerate(self.backlog):
            if frame.get("t") == "msg" and frame.get("name") == name:
                self.backlog.pop(index)
                return frame
        deadline = time.monotonic() + timeout
        while True:
            remaining = deadline - time.monotonic()
            if remaining <= 0:
                raise SmokeError(f"timed out after {timeout:.1f}s waiting for message {name!r}")
            frame = self.next_frame(remaining)
            if frame.get("t") == "msg" and frame.get("name") == name:
                return frame
            self.backlog.append(frame)

    def wait_one_of(self, names: set[str], timeout: float = 20) -> dict:
        for index, frame in enumerate(self.backlog):
            if frame.get("t") == "msg" and frame.get("name") in names:
                self.backlog.pop(index)
                return frame
        deadline = time.monotonic() + timeout
        while True:
            remaining = deadline - time.monotonic()
            if remaining <= 0:
                raise SmokeError(f"timed out after {timeout:.1f}s waiting for one of {sorted(names)}")
            frame = self.next_frame(remaining)
            if frame.get("t") == "msg" and frame.get("name") in names:
                return frame
            self.backlog.append(frame)

    def send(self, name: str, data: dict | None = None) -> None:
        if self.process.stdin is None or self.process.stdin.closed:
            raise SmokeError("Niva stdin is closed")
        frame: dict = {"t": "msg", "name": name}
        if data is not None:
            frame["data"] = data
        self.process.stdin.write(json.dumps(frame, separators=(",", ":")) + "\n")
        self.process.stdin.flush()


def _png_chunk(kind: bytes, payload: bytes) -> bytes:
    return struct.pack(">I", len(payload)) + kind + payload + struct.pack(">I", zlib.crc32(kind + payload) & 0xFFFFFFFF)


def make_tray_png(path: Path) -> None:
    """Create a small opaque icon using only Python's standard library."""
    width = height = 16
    scanlines = []
    for y in range(height):
        row = bytearray([0])
        for x in range(width):
            inside = 2 <= x <= 13 and 2 <= y <= 13
            dot = 5 <= x <= 10 and 5 <= y <= 10
            row.extend((42, 93, 150, 255) if inside and not dot else (230, 239, 248, 255) if dot else (0, 0, 0, 0))
        scanlines.append(bytes(row))
    image = b"\x89PNG\r\n\x1a\n"
    image += _png_chunk(b"IHDR", struct.pack(">2I5B", width, height, 8, 6, 0, 0, 0))
    image += _png_chunk(b"IDAT", zlib.compress(b"".join(scanlines), level=9))
    image += _png_chunk(b"IEND", b"")
    path.write_bytes(image)


def read_snapshot(path: Path) -> dict:
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        raise SmokeError(f"cannot read clipboard snapshot {path.name}: {error}") from error


class ClipboardGuard:
    """Preserve all NSPasteboard item representations when clipboard tests are opted in."""

    def __init__(self, root: Path):
        self.helper = HERE / "clipboard_state.swift"
        self.before_path = root / "clipboard-before.json"
        self.after_path = root / "clipboard-after.json"
        self.current_path = root / "clipboard-current.json"
        self.before: dict | None = None
        self.after: dict | None = None
        self.sentinel: str | None = None

    def _run(self, operation: str, path: Path) -> dict:
        result = subprocess.run(
            ["swift", str(self.helper), operation, str(path)],
            check=False,
            capture_output=True,
            text=True,
            timeout=30,
        )
        if result.returncode != 0:
            detail = result.stderr.strip() or result.stdout.strip() or f"swift exited {result.returncode}"
            raise SmokeError(f"clipboard {operation} failed before mutation: {detail}")
        return read_snapshot(path)

    def capture_before(self) -> dict:
        self.before = self._run("snapshot", self.before_path)
        return self.before

    def capture_after_write(self, sentinel: str) -> dict | None:
        snapshot = self._run("snapshot", self.after_path)
        if snapshot.get("plainText") != sentinel:
            return None
        checked = subprocess.run(
            ["swift", str(self.helper), "matches-sentinel", sentinel, str(snapshot["changeCount"])],
            check=False,
            capture_output=True,
            text=True,
            timeout=30,
        )
        if checked.returncode == 4:
            return None
        if checked.returncode != 0:
            detail = checked.stderr.strip() or checked.stdout.strip() or f"swift exited {checked.returncode}"
            raise SmokeError(f"clipboard sentinel check failed: {detail}")
        self.after = snapshot
        return snapshot

    def _restore(self, *arguments: str) -> tuple[bool, str]:
        result = subprocess.run(
            ["swift", str(self.helper), *arguments],
            check=False,
            capture_output=True,
            text=True,
            timeout=30,
        )
        if result.returncode == 4:
            return False, "clipboard changed after the smoke write; preserving the newer contents"
        if result.returncode != 0:
            detail = result.stderr.strip() or result.stdout.strip() or f"swift exited {result.returncode}"
            return False, f"restore failed: {detail}"
        return True, ""

    @staticmethod
    def _same_contents(first: dict, second: dict) -> bool:
        return first.get("items") == second.get("items") and first.get("plainText") == second.get("plainText")

    def restore_if_unchanged(self) -> tuple[bool, str]:
        if self.before is None:
            return True, "clipboard was not touched"
        with tempfile.TemporaryDirectory(prefix="niva-clipboard-restore-") as temporary:
            root = Path(temporary)
            before_path = root / "before.json"
            current_path = root / "current.json"
            expected_path = root / "expected.json"
            before_path.write_text(json.dumps(self.before, separators=(",", ":")), encoding="utf-8")
            if self.after is None:
                current = self._run("snapshot", current_path)
                if current == self.before:
                    return True, "clipboard still matches its original contents"
                if self._same_contents(current, self.before):
                    return True, "clipboard contents already match the original snapshot"
                if self.sentinel is None:
                    return False, "no post-write snapshot or sentinel was available; preserving current clipboard contents"
                restored, detail = self._restore(
                    "restore-if-sentinel",
                    str(before_path),
                    self.sentinel,
                    str(current["changeCount"]),
                )
            else:
                expected_path.write_text(json.dumps(self.after, separators=(",", ":")), encoding="utf-8")
                restored, detail = self._restore(
                    "restore-if-snapshot",
                    str(before_path),
                    str(expected_path),
                )
            if not restored:
                return False, detail
            current_after_restore = self._run("snapshot", current_path)
            if not self._same_contents(current_after_restore, self.before):
                return False, "restore completed but the pasteboard contents differ from the saved snapshot"
            return True, "all pasteboard items and representations match the saved snapshot"


def safe_app_dirs(app_name: str, app_uuid: str) -> list[Path]:
    short_id = app_uuid[:8]
    id_name = f"{app_name.lower()}_{short_id}"
    return [
        Path.home() / "Library" / "Application Support" / id_name,
        Path.home() / "Library" / "Caches" / id_name,
    ]


def validate_method_coverage(data: dict) -> dict[str, str]:
    coverage = data.get("coverage")
    if not isinstance(coverage, list):
        raise SmokeError("smoke result did not include the exact-method coverage list")
    result: dict[str, str] = {}
    for item in coverage:
        if not isinstance(item, dict) or not isinstance(item.get("method"), str) or not isinstance(item.get("assertion"), str):
            raise SmokeError(f"malformed method coverage record: {item!r}")
        method = item["method"]
        if method in result:
            raise SmokeError(f"duplicate method coverage record: {method}")
        result[method] = item["assertion"]
    missing = AUTOMATIC_METHODS - result.keys()
    unexpected = result.keys() - AUTOMATIC_METHODS
    if missing or unexpected:
        raise SmokeError(f"automatic coverage mismatch; missing={sorted(missing)}, unexpected={sorted(unexpected)}")
    return result


def register_case(cases: dict[str, str], method: str, assertion: str) -> None:
    if method in cases:
        raise SmokeError(f"method {method} was recorded twice")
    cases[method] = assertion


def expect_message_data(harness: Harness, name: str, timeout: float = 20) -> dict:
    frame = harness.wait_message(name, timeout)
    data = frame.get("data")
    if not isinstance(data, dict):
        raise SmokeError(f"message {name!r} did not contain an object: {frame!r}")
    return data


def run_clipboard(harness: Harness, guard: ClipboardGuard, cases: dict[str, str]) -> str:
    before = guard.capture_before()
    text_before = before.get("plainText")
    sentinel = f"niva-macos-api-smoke-{uuid.uuid4()}"
    guard.sentinel = sentinel
    harness.send("smoke-command", {"command": "clipboard", "sentinel": sentinel})
    observed_before = expect_message_data(harness, "clipboard-before")
    if observed_before.get("value") != text_before:
        raise SmokeError("clipboard.read did not match the saved plain-text representation")
    register_case(cases, "clipboard.read", "matches the original NSPasteboard plain-text representation (or null)")
    written = expect_message_data(harness, "clipboard-written")
    if written.get("sentinel") != sentinel:
        raise SmokeError("clipboard.write completion acknowledgement did not match the run sentinel")
    written_snapshot = guard.capture_after_write(sentinel)
    if written_snapshot is None:
        raise SmokeError("clipboard.write acknowledgement did not leave the unique sentinel on NSPasteboard")
    observed_after = expect_message_data(harness, "clipboard-after")
    if observed_after.get("value") != sentinel or observed_after.get("sentinel") != sentinel:
        raise SmokeError("clipboard.write/read did not return the unique run sentinel")
    register_case(cases, "clipboard.write", "writes and reads back a unique per-run sentinel")
    return sentinel


def run_shortcut(harness: Harness, cases: dict[str, str]) -> None:
    harness.send(
        "smoke-command",
        {
            "command": "shortcut",
            "accelerator": "Command+Option+Control+Shift+F19",
            "key": "F19",
            "secondAccelerator": "Command+Option+Control+Shift+F18",
            "thirdAccelerator": "Command+Option+Control+Shift+F17",
        },
    )
    expect_message_data(harness, "shortcut-result")
    for method in SHORTCUT_METHODS:
        register_case(cases, method, {
            "shortcut.register": "registers a rare four-modifier F-key chord and gets its action id",
            "shortcut.list": "reads registered id/key, then confirms removals through list readback",
            "shortcut.unregister": "unregisters one chord and confirms its id disappears",
            "shortcut.unregisterAll": "removes two remaining smoke-owned chords and confirms an empty list",
        }[method])


def run_tray(harness: Harness, cases: dict[str, str]) -> None:
    harness.send("smoke-command", {"command": "tray"})
    expect_message_data(harness, "tray-result")
    for method in TRAY_METHODS:
        register_case(cases, method, {
            "tray.create": "creates a short-lived status item and receives its id in tray.list",
            "tray.list": "observes the created id and its removal",
            "tray.update": "updates the item and confirms it remains registered",
            "tray.destroy": "destroys the test item and observes it disappear from tray.list",
            "tray.destroyAll": "removes a second test item and confirms tray.list is empty",
        }[method])


def run_webview_history(harness: Harness, cases: dict[str, str]) -> None:
    harness.send("smoke-command", {"command": "webview-navigate"})
    first_secondary = expect_message_data(harness, "secondary-ready")
    if not first_secondary.get("url", "").endswith("/secondary.html") or first_secondary.get("canGoBack") is not True:
        raise SmokeError(f"webview.loadUrl did not create a back-history entry: {first_secondary!r}")
    register_case(cases, "webview.loadUrl", "loads the isolated secondary page and creates back history")

    harness.send("smoke-command", {"command": "webview-back"})
    main_frame = harness.wait_one_of({"main-restored", "main-reloaded"}, timeout=15)
    main_state = main_frame.get("data", {})
    if not main_state.get("url", "").endswith("/index.html") or main_state.get("canGoForward") is not True:
        raise SmokeError(f"back navigation has the wrong main-page state: {main_state!r}")
    register_case(cases, "webview.goBack", "returns to the main page and exposes forward history")
    cases["webview.canGoBack"] += "; reports back history on the secondary document"
    cases["webview.canGoForward"] += "; reports forward history after navigating back"

    harness.send("smoke-command", {"command": "webview-reload"})
    reloaded = expect_message_data(harness, "main-reloaded", timeout=15)
    if not reloaded.get("url", "").endswith("/index.html") or reloaded.get("canGoForward") is not True:
        raise SmokeError(f"webview.reload did not retain the current history state: {reloaded!r}")
    register_case(cases, "webview.reload", "reloads the same local document while retaining forward history")

    harness.send("smoke-command", {"command": "webview-forward"})
    second_secondary = expect_message_data(harness, "secondary-ready", timeout=15)
    if not second_secondary.get("url", "").endswith("/secondary.html") or second_secondary.get("canGoBack") is not True:
        raise SmokeError(f"webview.goForward did not restore the secondary page: {second_secondary!r}")
    register_case(cases, "webview.goForward", "returns to the secondary page and restores back history")
    cases["webview.url"] += "; reads both fixture URLs and follows navigation"


def run_dialog(harness: Harness, cases: dict[str, str], method: str, start_dir: Path) -> None:
    harness.send("smoke-command", {"command": "dialog", "method": method, "startDir": str(start_dir)})
    started = expect_message_data(harness, "dialog-started", timeout=15)
    if started.get("method") != method:
        raise SmokeError(f"expected {method} to start, got {started!r}")
    heartbeat = expect_message_data(harness, "dialog-heartbeat", timeout=8)
    if heartbeat.get("method") != method:
        raise SmokeError(f"dialog heartbeat came from an unexpected call: {heartbeat!r}")
    print(f"DIALOG OPEN: {method}; close the native dialog to continue (picker start directory is isolated).", flush=True)
    result = expect_message_data(harness, "dialog-result", timeout=180)
    if result.get("method") != method:
        raise SmokeError(f"dialog result does not match the active call: {result!r}")
    value = result.get("value")
    if method == "showMessage":
        if value is not None:
            raise SmokeError(f"dialog.showMessage should resolve void, got {value!r}")
        assertion = "shows an explicit smoke-only message and resolves after dismissal while the page heartbeat continues"
    else:
        if value is not None:
            path = Path(value).resolve()
            try:
                path.relative_to(start_dir.resolve())
            except ValueError as error:
                raise SmokeError(f"{method} returned a path outside the isolated picker directory: {path}") from error
            if method == "pickFile" and not path.is_file():
                raise SmokeError(f"dialog.pickFile returned a missing fixture file: {path}")
            if method == "saveFile" and path.exists():
                raise SmokeError(f"dialog.saveFile created or selected an existing fixture path: {path}")
        assertion = f"opens in the disposable picker directory, keeps JS heartbeat live, and resolves to null or a path within that directory"
    register_case(cases, f"dialog.{method}", assertion)


def tail(path: Path, limit: int = 80) -> str:
    try:
        return "\n".join(path.read_text(encoding="utf-8", errors="replace").splitlines()[-limit:])
    except OSError:
        return ""


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("binary", help="path to a locally built macOS niva binary")
    parser.add_argument("--clipboard", action="store_true", help="exercise clipboard.read/write with complete NSPasteboard snapshot/restore")
    parser.add_argument("--shortcut", action="store_true", help="briefly register and remove uncommon global F-key chords")
    parser.add_argument("--dialogs", action="store_true", help="open controlled native dialogs and wait for the operator to dismiss each one")
    args = parser.parse_args()

    if sys.platform != "darwin":
        raise SystemExit("This fixture requires macOS; it does not emulate native APIs on another OS.")
    binary = Path(args.binary).expanduser().resolve()
    if not binary.is_file():
        raise SystemExit(f"Niva binary does not exist: {binary}")

    app_name = "NivaMacApiSmoke"
    app_uuid = str(uuid.uuid4())
    app_dirs = safe_app_dirs(app_name, app_uuid)
    if any(path.exists() for path in app_dirs):
        raise SystemExit("generated app identity unexpectedly already exists; retry the run")

    cases: dict[str, str] = {}
    clipboard_guard: ClipboardGuard | None = None
    process: subprocess.Popen[str] | None = None
    stderr_path: Path | None = None
    clipboard_restoration = "not requested"
    success = False
    error_message: str | None = None
    stderr_tail = ""

    log_fd, log_name = tempfile.mkstemp(prefix="niva-macos-api-smoke-", suffix=".log")
    os.close(log_fd)
    stderr_path = Path(log_name)

    try:
        with tempfile.TemporaryDirectory(prefix="niva-macos-api-smoke-") as temporary:
            root = Path(temporary)
            resources = root / "resources"
            resources.mkdir()
            for filename in ("index.html", "secondary.html"):
                shutil.copy2(HERE / filename, resources / filename)
            (resources / "child.html").write_text(
                "<!doctype html><html><body><h1>Isolated child</h1></body></html>\n",
                encoding="utf-8",
            )
            (resources / "probe.txt").write_text("niva-smoke-only\n", encoding="utf-8")
            make_tray_png(resources / "tray.png")
            dialog_root = root / "dialog-root"
            dialog_root.mkdir()
            (dialog_root / "probe.txt").write_text("niva-dialog-fixture\n", encoding="utf-8")

            config = root / "niva.json"
            config.write_text(
                json.dumps(
                    {
                        "name": app_name,
                        "uuid": app_uuid,
                        "window": {
                            "entry": "index.html",
                            "title": "Niva macOS API smoke",
                            "size": {"width": 640, "height": 420},
                            "visible": True,
                            "resizable": True,
                            "decorations": True,
                        },
                    },
                    indent=2,
                )
                + "\n",
                encoding="utf-8",
            )
            stderr_file = stderr_path.open("wb")
            child_env = os.environ.copy()
            child_env["TMPDIR"] = str(root)
            try:
                process = subprocess.Popen(
                    [
                        str(binary),
                        "--stdio",
                        f"--debug-config={config}",
                        f"--debug-resource={resources}",
                    ],
                    cwd=REPO,
                    env=child_env,
                    stdin=subprocess.PIPE,
                    stdout=subprocess.PIPE,
                    stderr=stderr_file,
                    text=True,
                    bufsize=1,
                )
            finally:
                stderr_file.close()
            harness = Harness(process)

            ready = harness.next_frame(timeout=30)
            if ready != {"t": "ready", "v": 1}:
                raise SmokeError(f"unexpected first stdio frame: {ready!r}")
            smoke = expect_message_data(harness, "smoke", timeout=60)
            cases.update(validate_method_coverage(smoke))
            initial_url = smoke.get("initialUrl", "")
            if not initial_url.startswith("http://127.0.0.1:"):
                raise SmokeError(f"smoke page did not use an isolated loopback origin: {initial_url!r}")
            print(f"APP READY: pid={process.pid} origin={initial_url.split('/')[0]}")

            nonce = f"echo-{uuid.uuid4()}"
            harness.send("smoke-command", {"command": "echo", "nonce": nonce})
            echo = expect_message_data(harness, "host-echo-result")
            if echo.get("nonce") != nonce:
                raise SmokeError(f"host.send echo changed its nonce: {echo!r}")
            register_case(cases, "host.send", "sends a unique payload through stdio and receives the exact echo")

            run_webview_history(harness, cases)
            if args.clipboard:
                clipboard_guard = ClipboardGuard(root)
                run_clipboard(harness, clipboard_guard, cases)
            if args.shortcut:
                run_shortcut(harness, cases)
            run_tray(harness, cases)

            if args.dialogs:
                for method in ("showMessage", "pickFile", "saveFile"):
                    run_dialog(harness, cases, method, dialog_root)

            harness.send("smoke-command", {"command": "exit"})
            exit_started = expect_message_data(harness, "exit-started", timeout=15)
            if "process.exit" in exit_started.get("coverage", []):
                raise SmokeError("process.exit was reported before it was called")
            status = process.wait(timeout=15)
            if status != 0:
                raise SmokeError(f"process.exit child returned status {status}")
            register_case(cases, "process.exit", "terminates the isolated Niva child with status 0 after all temporary API state is cleaned up")

            expected = set(AUTOMATIC_METHODS) | WEBVIEW_HISTORY_METHODS | {"host.send", "process.exit"} | TRAY_METHODS
            if args.clipboard:
                expected |= {"clipboard.read", "clipboard.write"}
            if args.shortcut:
                expected |= SHORTCUT_METHODS
            if args.dialogs:
                expected |= {"dialog.showMessage", "dialog.pickFile", "dialog.saveFile"}
            missing = expected - cases.keys()
            unexpected = cases.keys() - expected
            if missing or unexpected:
                raise SmokeError(f"final coverage mismatch; missing={sorted(missing)}, unexpected={sorted(unexpected)}")
            success = True
    except Exception as error:
        error_message = f"{type(error).__name__}: {error}"
    finally:
        if clipboard_guard is not None:
            try:
                restored, clipboard_restoration = clipboard_guard.restore_if_unchanged()
                if not restored:
                    error_message = (error_message + "; " if error_message else "") + f"clipboard restoration: {clipboard_restoration}"
                    success = False
            except Exception as error:
                clipboard_restoration = f"restoration check failed: {error}"
                error_message = (error_message + "; " if error_message else "") + clipboard_restoration
                success = False

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
        for path in app_dirs:
            if path.is_dir() and not path.is_symlink():
                shutil.rmtree(path)
        if stderr_path is not None:
            stderr_tail = tail(stderr_path)
            stderr_path.unlink(missing_ok=True)

    print(f"CLIPBOARD RESTORE: {clipboard_restoration}")
    if not success:
        print(f"macOS real-app smoke: FAIL — {error_message}", file=sys.stderr)
        if stderr_tail:
            print("Niva stderr tail:\n" + stderr_tail, file=sys.stderr)
        return 1

    print(f"macOS real-app smoke: PASS ({len(cases)} method cases)")
    for method in sorted(cases):
        print(f"  {method}: {cases[method]}")
    if not args.dialogs:
        print("  dialog.showMessage/dialog.pickFile/dialog.saveFile: NOT RUN (pass --dialogs for controlled dismissal)")
    if not args.clipboard:
        print("  clipboard.read/clipboard.write: NOT RUN (pass --clipboard to snapshot and restore NSPasteboard)")
    if not args.shortcut:
        print("  shortcut.*: NOT RUN (pass --shortcut to briefly register and remove uncommon global chords)")
    print("  process.open: NOT RUN (opens an external application; excluded from unattended smoke)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
