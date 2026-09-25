#!/usr/bin/env python3
"""Run NodeCompat cases in a disposable, real macOS Niva WebView."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import selectors
import shutil
import subprocess
import sys
import tempfile
import time
import uuid
import threading
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
from typing import Any


HERE = Path(__file__).resolve().parent
REPO = HERE.parents[1]
EXPECTED_MODULES = {
    "path", "os", "fs", "child_process", "events", "util", "querystring",
    "buffer", "url", "crypto", "zlib", "http", "https", "assert", "stream", "tty",
}
RESULT_NAMES = {"nodecompat-macos-result", "nodecompat-macos-failure"}


class SmokeError(RuntimeError):
    pass


class FrameReader:
    """Read only the disposable fixture-owned process stream protocol."""

    def __init__(self, process: subprocess.Popen[bytes]):
        if process.stdout is None:
            raise SmokeError("Niva stdout pipe was not created")
        self.process = process
        self.selector = selectors.DefaultSelector()
        self.selector.register(process.stdout, selectors.EVENT_READ)
        self.buffer = bytearray()

    def close(self) -> None:
        self.selector.close()

    def next(self, timeout: float) -> dict[str, Any]:
        deadline = time.monotonic() + timeout
        while True:
            newline = self.buffer.find(b"\n")
            if newline >= 0:
                line = bytes(self.buffer[:newline])
                del self.buffer[: newline + 1]
                try:
                    frame = json.loads(line)
                except json.JSONDecodeError as error:
                    raise SmokeError(f"invalid fixture JSON frame: {error}: {line[:200]!r}") from error
                if not isinstance(frame, dict):
                    raise SmokeError(f"unexpected non-object fixture frame: {frame!r}")
                if frame.get("protocol") != "niva-fixture" or frame.get("version") != 1:
                    raise SmokeError(f"unexpected fixture stdout frame: {frame!r}")
                if frame.get("event") == "protocol-error":
                    raise SmokeError(f"fixture stdin protocol failed: {frame.get('message')}")
                return frame

            remaining = deadline - time.monotonic()
            if remaining <= 0:
                raise TimeoutError("timed out waiting for fixture process-stream frame")
            events = self.selector.select(remaining)
            if not events:
                raise TimeoutError("timed out waiting for fixture process-stream frame")
            assert self.process.stdout is not None
            chunk = os.read(self.process.stdout.fileno(), 65536)
            if not chunk:
                raise SmokeError(f"Niva stdout closed (status {self.process.poll()})")
            self.buffer.extend(chunk)


def _run(command: list[str], *, cwd: Path, timeout: float = 120) -> None:
    result = subprocess.run(command, cwd=cwd, text=True, capture_output=True, timeout=timeout, check=False)
    if result.returncode != 0:
        detail = result.stderr.strip() or result.stdout.strip() or f"exit status {result.returncode}"
        raise SmokeError(f"command failed: {' '.join(command)}\n{detail}")
    if result.stdout.strip():
        print(result.stdout.strip())


def prepare_resources(resources: Path) -> None:
    resources.mkdir(parents=True)
    shutil.copy2(HERE / "index.html", resources / "index.html")
    shutil.copy2(HERE / "cases.js", resources / "cases.js")
    shutil.copy2(HERE / "fixture-protocol.js", resources / "fixture-protocol.js")


def create_config(config: Path, run_id: str) -> None:
    config.write_text(
        json.dumps(
            {
                "name": "NivaNodeCompatSmoke",
                "uuid": run_id,
                "injectCommonJs": True,
                "injectEsm": True,
                "window": {
                    "entry": "index.html",
                    "title": "Niva NodeCompat macOS smoke",
                    "size": {"width": 760, "height": 520},
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


def read_log(path: Path) -> str:
    try:
        lines = path.read_text(encoding="utf-8", errors="replace").splitlines()
    except OSError:
        return ""
    return "\n".join(lines[-80:])


def validate_report(report: dict[str, Any]) -> tuple[int, list[str]]:
    modules = report.get("modules")
    if not isinstance(modules, list) or set(modules) != EXPECTED_MODULES or len(modules) != len(EXPECTED_MODULES):
        raise SmokeError(f"NodeCompat module registration mismatch: {modules!r}")
    cases = report.get("cases")
    if not isinstance(cases, list):
        raise SmokeError("NodeCompat report did not include its case catalog")
    expected_methods: list[str] = []
    case_ids: set[str] = set()
    for case in cases:
        if not isinstance(case, dict) or not isinstance(case.get("id"), str) or not isinstance(case.get("methods"), list):
            raise SmokeError(f"malformed NodeCompat case catalog row: {case!r}")
        if case["id"] in case_ids:
            raise SmokeError(f"duplicate NodeCompat case id: {case['id']}")
        case_ids.add(case["id"])
        expected_methods.extend(case["methods"])
    if len(expected_methods) != len(set(expected_methods)):
        duplicate_methods = sorted(method for method in set(expected_methods) if expected_methods.count(method) > 1)
        raise SmokeError(f"duplicate NodeCompat API coverage entries: {duplicate_methods}")
    passed = report.get("passedMethods")
    if not isinstance(passed, list) or passed != expected_methods:
        raise SmokeError("reported passing methods do not exactly match the executed case catalog")
    return len(passed), expected_methods


def run(binary: Path, timeout: float, keep_workdir: bool) -> int:
    if sys.platform != "darwin":
        raise SmokeError("this integration suite must run on macOS")
    binary = binary.expanduser().resolve()
    if not binary.is_file():
        raise SmokeError(f"Niva binary not found: {binary}; build it first with cargo build --bin niva")

    root = Path(tempfile.mkdtemp(prefix="niva-node-compat-macos-"))
    log_path = root / "niva-stderr.log"
    process: subprocess.Popen[bytes] | None = None
    frames: FrameReader | None = None
    result_report: dict[str, Any] | None = None
    class HttpFixture(BaseHTTPRequestHandler):
        def log_message(self, *args): pass
        def do_GET(self):
            body = b'NodeCompat macOS integration smoke'
            self.send_response(200)
            self.send_header('Content-Length', str(len(body)))
            self.end_headers()
            self.wfile.write(body)
        def do_POST(self):
            body = self.rfile.read(int(self.headers.get('Content-Length', '0')))
            if body != b'buffered':
                self.send_response(400)
                self.end_headers()
                return
            body = b'POST accepted'
            self.send_response(201)
            self.send_header('Content-Length', str(len(body)))
            self.end_headers()
            self.wfile.write(body)
    http_fixture = ThreadingHTTPServer(('127.0.0.1', 0), HttpFixture)
    threading.Thread(target=http_fixture.serve_forever, daemon=True).start()

    try:
        home = root / "home"
        tmp = root / "tmp"
        home.mkdir()
        tmp.mkdir()
        resources = root / "resources"
        config = root / "niva.json"
        run_id = str(uuid.uuid4())
        prepare_resources(resources)
        create_config(config, run_id)
        child_env = os.environ.copy()
        child_env["HOME"] = str(home)
        child_env["TMPDIR"] = str(tmp)
        child_env["NIVA_SMOKE_HTTP_URL"] = f'http://127.0.0.1:{http_fixture.server_port}/'
        with log_path.open("wb") as log_file:
            process = subprocess.Popen(
                [
                    str(binary), f"--config={config}", f"--resource={resources}",
                ],
                cwd=REPO,
                env=child_env,
                stdin=subprocess.PIPE,
                stdout=subprocess.PIPE,
                stderr=log_file,
            )

        frames = FrameReader(process)
        deadline = time.monotonic() + timeout
        ready = frames.next(min(45, max(0.1, deadline - time.monotonic())))
        if ready.get("event") != "ready" or ready.get("name") != "node-compat-macos":
            raise SmokeError(f"unexpected fixture ready event: {ready!r}")
        print(f"Niva WebView launched: pid={process.pid}; disposable profile and resources={root}", flush=True)

        while time.monotonic() < deadline:
            frame = frames.next(max(0.1, deadline - time.monotonic()))
            if frame.get("event") != "message" or frame.get("name") not in RESULT_NAMES:
                continue
            report = frame.get("data")
            if not isinstance(report, dict):
                raise SmokeError(f"malformed NodeCompat result frame: {frame!r}")
            if frame.get("name") == "nodecompat-macos-failure":
                case = report.get("case")
                detail = report.get("message")
                passed = report.get("passedMethods", [])
                raise SmokeError(f"NodeCompat case failed{f' at {case}' if case else ''}: {detail}; passed before failure: {passed}")
            result_report = report
            break

        if result_report is None:
            raise TimeoutError(f"NodeCompat case page did not report within {timeout:.1f}s")
        method_count, methods = validate_report(result_report)
        (root / "result.json").write_text(json.dumps({
            "ok": True, "engine": "niva-webview", "methodCount": method_count,
            "nativeBinarySha256": hashlib.sha256(binary.read_bytes()).hexdigest(),
            **result_report,
        }, indent=2) + "\n")

        try:
            status = process.wait(timeout=8)
        except subprocess.TimeoutExpired:
            process.terminate()
            status = process.wait(timeout=5)
        if status != 0:
            raise SmokeError(f"Niva exited with status {status} after the page reported success")

        print(f"PASS: {method_count} documented runtime method and alias checks in {len(result_report['cases'])} cases", flush=True)
        print("Modules: " + ", ".join(sorted(result_report["modules"])), flush=True)
        for case in result_report["cases"]:
            print(f"  {case['id']}: {len(case['methods'])} methods", flush=True)
        print("HTTPS limitation: request/get wrappers are checked for protocol rejection; outbound TLS is covered separately by ipc-fallback-smoke.", flush=True)
        return 0
    except BaseException as error:
        (root / "result.json").write_text(json.dumps({
            "ok": False, "engine": "niva-webview", "error": str(error),
            "nativeBinarySha256": hashlib.sha256(binary.read_bytes()).hexdigest(),
        }, indent=2) + "\n")
        if process is not None and process.poll() is None:
            process.terminate()
            try:
                process.wait(timeout=5)
            except subprocess.TimeoutExpired:
                process.kill()
                process.wait(timeout=5)
        details = read_log(log_path)
        if details:
            print("Niva stderr tail:\n" + details, file=sys.stderr)
        raise SmokeError(str(error)) from error
    finally:
        http_fixture.shutdown()
        http_fixture.server_close()
        if frames is not None:
            frames.close()
        if process is not None and process.stdout is not None:
            process.stdout.close()
        if process is not None and process.poll() is None:
            process.kill()
            process.wait(timeout=5)
        if keep_workdir:
            print(f"Disposable smoke files retained at {root}", flush=True)
        else:
            shutil.rmtree(root, ignore_errors=True)


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--binary",
        type=Path,
        default=REPO / "target" / "debug" / "niva",
        help="path to the built Niva executable (default: target/debug/niva)",
    )
    parser.add_argument("--timeout", type=float, default=240, help="overall page test timeout in seconds")
    parser.add_argument("--keep-workdir", action="store_true", help="retain isolated profile, resources and stderr log")
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    try:
        return run(args.binary, args.timeout, args.keep_workdir)
    except (SmokeError, TimeoutError, subprocess.TimeoutExpired) as error:
        print(f"FAIL: {error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
