#!/usr/bin/env python3
"""Strict two-lane WebView smoke for the stable IPC and optional WS stream routes."""

from __future__ import annotations

import argparse
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
import hashlib
import json
import os
from pathlib import Path
import shutil
import signal
import subprocess
import sys
import tempfile
import threading
import time
import uuid

HERE = Path(__file__).resolve().parent
EXPECTED_HEX = "0001027f80feff4e697661"
EXPECTED_FILEHANDLE_CHECKSUM = "aaf01dc5"


class SmokeError(RuntimeError):
    pass


def message(event: dict, field: str = "message") -> dict:
    value = event.get(field)
    return value if isinstance(value, dict) else {}


def validate_report(report: dict, mode: str) -> list[str]:
    errors: list[str] = []
    events = report.get("events")
    if not isinstance(events, list):
        return ["fixture report has no event list"]
    if report.get("ok") is not True:
        errors.append(f"fixture reported failure: {report.get('error', 'unknown error')}")
    if report.get("inputHex") != EXPECTED_HEX or report.get("outputHex") != EXPECTED_HEX:
        errors.append("binary child_process stdin/stdout roundtrip did not preserve the expected bytes")
    if report.get("nivaApiHex") != EXPECTED_HEX:
        errors.append("Niva.child_process binary roundtrip did not preserve the expected bytes")
    if report.get("fileHex") != EXPECTED_HEX:
        errors.append("Niva.fs binary write/read roundtrip did not preserve the expected bytes")
    if report.get("fileHandleBytes") != 1024 * 1024 or report.get("fileHandleChecksum") != EXPECTED_FILEHANDLE_CHECKSUM:
        errors.append("FileHandle 1 MiB write/read/stat/close roundtrip failed or returned unexpected bytes")
    if mode == "ws" and (report.get("cwdObserved") is not True or report.get("syncXhr") is not True):
        errors.append("Node process.cwd() did not observe the retained synchronous-XHR compatibility route")
    runtime_events = [event for event in events if event.get("kind") == "runtime-surfaces"]
    if not runtime_events:
        errors.append("fixture did not record runtime installation diagnostics")
    else:
        surfaces = runtime_events[0]
        if surfaces.get("runtimeConfig", {}).get("injectCommonJs") is not True:
            errors.append("Niva.runtimeConfig.injectCommonJs was not true")
        if surfaces.get("runtimeConfig", {}).get("trustedLocal") is not True or surfaces.get("bridgeTrustedLocal") is not True:
            errors.append("trusted local runtime marker was not active")
        if surfaces.get("globalRequire") != "function":
            errors.append("global CommonJS require was not installed")

    ipc_events = [event for event in events if event.get("kind") in ("ipc-send", "wire-serialized")]
    ipc_messages = [message(event) for event in ipc_events]
    method_messages = [item for item in ipc_messages if "method" in item or "args" in item]
    if any(item.get("t") != "api_call" for item in method_messages):
        errors.append("an IPC method/args request did not use t=api_call")
    for event, item in zip(ipc_events, ipc_messages):
        if item.get("t") == "api_call" and str(item.get("method", "")).startswith("fs."):
            if event.get("largeDataFields", 0):
                errors.append(f"{item.get('method')} put a large data body in IPC API args")
            if event.get("serializedBytes", 0) > 8192:
                errors.append(f"{item.get('method')} control message was unexpectedly large ({event.get('serializedBytes')} bytes)")
    if not any(item.get("t") == "api_call" and item.get("method") == "window.title" for item in ipc_messages):
        errors.append("window.title unary request was not observed as IPC t=api_call")
    if sum(item.get("t") == "api_call" and item.get("method") == "window.title" for item in ipc_messages) < 2:
        errors.append("top-level and same-origin iframe unary requests were not both observed over IPC")
    if not any(item.get("t") == "api_call" and item.get("method") == "process.execStream" for item in ipc_messages):
        errors.append("process.execStream owner request was not observed as IPC t=api_call")
    if any(item.get("t") in ("call", "stream", "open") for item in ipc_messages):
        errors.append("legacy IPC call/stream dispatch message was observed")

    replies = [event for event in events if event.get("kind") == "ipc-reply"]
    opened_orders = [event.get("order", 0) for event in replies if event.get("responseType") == "channelOpened"]
    if not opened_orders:
        errors.append("Native eval reply channelOpened (including capability) was not observed")
    elif not any(event.get("hasCapability") for event in replies if event.get("responseType") == "channelOpened"):
        errors.append("channelOpened reply did not expose the channel capability envelope")
    if any(event.get("kind") == "instrumentation-error" for event in events):
        errors.append("fixture could not wrap one or more standard transport entry points")
    hook_kinds = {event.get("kind") for event in events}
    if not hook_kinds.intersection(("ipc-hook-installed", "wire-serialization-hook-installed")):
        errors.append("fixture did not install Wry handler or wire-serialization observation")
    if "wire-serialization-hook-installed" in hook_kinds:
        for event, item in zip(ipc_events, ipc_messages):
            if event.get("kind") != "wire-serialized":
                continue
            if item.get("t") in ("api_call", "channelAttach", "channelSend", "channelAck", "channelCancel"):
                if not any(reply.get("kind") == "ipc-reply" and reply.get("rid") == item.get("rid")
                           and reply.get("order", 0) > event.get("order", 0) for reply in events):
                    errors.append(f"wire-serialized {item.get('t')} rid={item.get('rid')} had no matching later Native eval reply")
    iframe_result = report.get("iframe")
    if not isinstance(iframe_result, dict) or iframe_result.get("sameOrigin") is not True or iframe_result.get("independentSession") is not True:
        errors.append("same-origin iframe unary call did not complete with an independent IPC session")

    ws_text = [message(event) for event in events if event.get("kind") == "ws-send-text"]
    ws_binary = [event for event in events if event.get("kind") == "ws-send-binary"]
    for item in ws_text:
        if "method" in item or "args" in item or item.get("t") in ("call", "api_call"):
            errors.append("WebSocket carried an API method/args request")
            break
    if mode == "ws":
        attaches = [event for event in events
                    if event.get("kind") == "ws-send-text" and message(event).get("t") == "attach"]
        if not attaches:
            errors.append("WS mode never sent an attach message")
        for attach in attaches:
            wire = message(attach)
            matching_open = [event for event in replies
                             if event.get("responseType") == "channelOpened"
                             and event.get("id") == wire.get("id") and event.get("hasCapability")]
            if not matching_open or max(event.get("order", 0) for event in matching_open) >= attach.get("order", 0):
                errors.append(f"WS attach id={wire.get('id')} did not follow its eval channelOpened capability reply")
            if attach.get("readyState") != 1:
                errors.append(f"WS attach id={wire.get('id')} was not sent on a ready WebSocket")
            if not wire.get("hasSessionId") or not wire.get("hasCapability"):
                errors.append(f"WS attach id={wire.get('id')} omitted owner session/capability")
        attach_ids = {message(event).get("id") for event in attaches}
        attached = [event for event in events
                    if event.get("kind") == "ws-recv-text" and message(event).get("t") == "attached"]
        attached_ids = {message(event).get("id") for event in attached if message(event).get("hasSessionId")}
        if not attach_ids.intersection(attached_ids):
            errors.append("Native did not confirm a WS attach for an IPC-owned ticket")
        for confirmation in attached:
            wire = message(confirmation)
            matching_attach = [event for event in attaches if message(event).get("id") == wire.get("id")]
            if not wire.get("hasSessionId") or not matching_attach or max(event.get("order", 0) for event in matching_attach) >= confirmation.get("order", 0):
                errors.append(f"Native WS attached confirmation id={wire.get('id')} did not follow its session-owned attach")
        if not any(event.get("kind") == "ws-incoming-observer-installed" for event in events):
            errors.append("fixture did not observe incoming frames on the attached WebSocket")
        payload_frames = [event for event in ws_binary if event.get("byteLength", 0) > 18]
        if not payload_frames:
            errors.append("WS mode did not send a binary data frame with payload")
        elif attaches and min(event.get("order", 0) for event in payload_frames) <= min(event.get("order", 0) for event in attaches):
            errors.append("WS binary data was sent before the ticket attach")
        if any(event.get("protocolVersion") != 2 for event in ws_binary if event.get("byteLength", 0) >= 18):
            errors.append("WS binary data frame did not use wire version 2")
        if any(event.get("protocolVersion") != 2 for event in events
               if event.get("kind") == "ws-recv-binary" and event.get("byteLength", 0) >= 18):
            errors.append("Native WS binary data frame did not use wire version 2")
        received_frames = []
        for event in events:
            if event.get("kind") == "ws-recv-binary" and event.get("id") is not None:
                received_frames.append((event.get("id"), event.get("seq"), event.get("order", 0), event.get("payloadLength", 0)))
            elif event.get("kind") == "ws-recv-text":
                wire = message(event)
                if wire.get("t") == "channelData":
                    received_frames.append((wire.get("id"), wire.get("seq"), event.get("order", 0), 1))
        if not any(frame[3] > 0 for frame in received_frames):
            errors.append("WS mode did not receive a downlink data frame")
        ack_events = [event for event in events if event.get("kind") == "ws-send-text" and message(event).get("t") == "ack"]
        if not ack_events:
            errors.append("WS mode did not send a data acknowledgement")
        for ack_event in ack_events:
            ack = message(ack_event)
            matching = [frame for frame in received_frames if frame[0] == ack.get("id") and frame[1] == ack.get("seq")]
            if not ack.get("hasSessionId") or not matching or max(frame[2] for frame in matching) >= ack_event.get("order", 0):
                errors.append(f"WS ACK id={ack.get('id')} seq={ack.get('seq')} did not match a prior Native downlink frame")
        if not any(frame[3] > 0 and frame[0] in attach_ids and frame[2] > min(
            event.get("order", 0) for event in attaches if message(event).get("id") == frame[0]
        ) for frame in received_frames):
            errors.append("WS downlink data did not follow the corresponding ticket attach")
        if any(item.get("t") in ("channelAttach", "channelSend", "channelAck") for item in ipc_messages):
            errors.append("WS mode unexpectedly used IPC for channel data transport")
    else:
        controls = {item.get("t") for item in ipc_messages}
        for required in ("channelAttach", "channelSend", "channelAck"):
            if required not in controls:
                errors.append(f"CSP-blocked WS mode did not exercise IPC {required}")
        if not opened_orders:
            errors.append("IPC mode cannot establish channel transport without channelOpened")
        channel_sends = [(event, item) for event, item in zip(ipc_events, ipc_messages) if item.get("t") == "channelSend"]
        if not any(item.get("frameEncodedLength", 0) > 24 for _, item in channel_sends):
            errors.append("IPC mode did not send a binary channel frame containing payload")
        attached_orders = [event.get("order", 0) for event in replies if event.get("responseType") == "channelAttached"]
        if not attached_orders:
            errors.append("IPC mode did not receive channelAttached after channelAttach")
        elif channel_sends and min(event.get("order", 0) for event, _ in channel_sends) <= min(attached_orders):
            errors.append("IPC binary data was sent before channelAttached")
        ipc_attach_ids = {item.get("id") for item in ipc_messages if item.get("t") == "channelAttach"}
        if not any(item.get("id") in ipc_attach_ids and item.get("hasSessionId")
                   and isinstance(item.get("seq"), int) and item.get("seq", 0) > 0
                   for item in ipc_messages if item.get("t") == "channelAck"):
            errors.append("IPC mode did not ACK a downlink frame on the attached owner session")
        ack_orders = [event.get("order", 0) for event, item in zip(ipc_events, ipc_messages)
                      if item.get("t") == "channelAck"]
        if attached_orders and ack_orders and min(ack_orders) <= min(attached_orders):
            errors.append("IPC channel ACK was sent before the channel was attached")
        for item in ipc_messages:
            if item.get("t") == "channelAttach":
                if not item.get("hasSessionId") or not item.get("hasCapability"):
                    errors.append(f"IPC channelAttach id={item.get('id')} omitted owner session/capability")
                matching_open = [event for event in replies
                                 if event.get("responseType") == "channelOpened"
                                 and event.get("id") == item.get("id") and event.get("hasCapability")]
                if not matching_open or max(event.get("order", 0) for event in matching_open) >= next(
                    (event.get("order", 0) for event in ipc_events if message(event) is item), 0
                ):
                    errors.append(f"IPC channelAttach id={item.get('id')} did not follow its eval channelOpened capability reply")
        if any(event.get("kind") == "ws-send-text" and message(event).get("t") == "attach" for event in events):
            errors.append("CSP-blocked mode unexpectedly sent a WS attach")
    return errors


class Case:
    def __init__(self, binary: Path, output: Path, mode: str, timeout: float, trusted_debug: bool):
        self.binary = binary
        self.output = output
        self.mode = mode
        self.timeout = timeout
        self.trusted_debug = trusted_debug
        self.report: dict | None = None
        self.file_path = str(Path(tempfile.gettempdir()) / f"niva-bridge-route-{uuid.uuid4().hex}.bin")
        self.report_ready = threading.Event()
        self.http: ThreadingHTTPServer | None = None
        self.process: subprocess.Popen | None = None
        self.log_path = output / "niva.log"

    def start_server(self) -> str:
        case = self

        class Handler(BaseHTTPRequestHandler):
            def log_message(self, *_args):
                pass

            def send_body(self, status: int, body: bytes, content_type: str):
                self.send_response(status)
                self.send_header("Content-Type", content_type)
                self.send_header("Content-Length", str(len(body)))
                self.send_header("Cache-Control", "no-store")
                self.send_header("Access-Control-Allow-Origin", "*")
                if case.mode == "ipc":
                    self.send_header("Content-Security-Policy", f"default-src 'self'; script-src 'self'; connect-src 'self' {case.report_origin}")
                self.end_headers()
                self.wfile.write(body)

            def do_OPTIONS(self):
                self.send_response(204)
                self.send_header("Access-Control-Allow-Origin", "*")
                self.send_header("Access-Control-Allow-Headers", "Content-Type")
                self.send_header("Access-Control-Allow-Methods", "POST, OPTIONS")
                self.end_headers()

            def do_GET(self):
                path = self.path.split("?", 1)[0]
                if path == "/":
                    page = (HERE / "index.html").read_text(encoding="utf-8")
                    page = page.replace("__REPORT_ORIGIN__", case.report_origin)
                    self.send_body(200, page.encode(), "text/html; charset=utf-8")
                elif path == "/frame.html":
                    self.send_body(200, (HERE / "frame.html").read_bytes(), "text/html; charset=utf-8")
                elif path in ("/instrument.js", "/smoke.js", "/frame.js"):
                    self.send_body(200, (HERE / path.lstrip("/")).read_bytes(), "text/javascript; charset=utf-8")
                elif path == "/fixture.js":
                    fixture = {"python": sys.executable, "filePath": case.file_path}
                    body = ("window.__bridgeRouteFixture = " + json.dumps(fixture) + ";").encode()
                    self.send_body(200, body, "text/javascript; charset=utf-8")
                else:
                    self.send_body(404, b"not found", "text/plain")

            def do_POST(self):
                if self.path != "/report":
                    self.send_body(404, b"not found", "text/plain")
                    return
                length = int(self.headers.get("Content-Length", "0"))
                if length > 2 * 1024 * 1024:
                    self.send_body(413, b"too large", "text/plain")
                    return
                try:
                    case.report = json.loads(self.rfile.read(length))
                    case.report_ready.set()
                    self.send_body(200, b"ok", "text/plain")
                except (UnicodeDecodeError, json.JSONDecodeError) as error:
                    self.send_body(400, str(error).encode(), "text/plain")

        self.http = ThreadingHTTPServer(("127.0.0.1", 0), Handler)
        port = self.http.server_port
        self.report_origin = f"http://127.0.0.1:{port}"
        threading.Thread(target=self.http.serve_forever, daemon=True).start()
        return self.report_origin

    def run(self) -> dict:
        self.output.mkdir(parents=True, exist_ok=True)
        self.start_server()
        query = f"mode={self.mode}&report={self.report_origin}/report"
        resource = self.output / "resources"
        resource.mkdir(exist_ok=True)
        for name in ("instrument.js", "smoke.js", "frame.html", "frame.js"):
            shutil.copy2(HERE / name, resource / name)
        (resource / "fixture.js").write_text(
            "window.__bridgeRouteFixture = " + json.dumps({"python": sys.executable, "filePath": self.file_path}) + ";\n",
            encoding="utf-8",
        )
        page = (HERE / "index.html").read_text(encoding="utf-8")
        page = page.replace("__REPORT_ORIGIN__", self.report_origin)
        (resource / "index.html").write_text(page, encoding="utf-8")

        config = self.output / "niva.json"
        config.write_text(json.dumps({
            "name": f"Niva bridge route smoke ({self.mode})",
            "uuid": str(uuid.uuid4()),
            "injectCommonJs": True,
            "injectEsm": False,
            "window": {
                "entry": f"index.html?{query}" if self.mode == "ws" else f"/?{query}",
                **({"permissions": {}} if self.mode == "ipc" and self.trusted_debug else {}),
                "visible": True,
                "size": {"width": 760, "height": 520},
            },
        }, indent=2) + "\n", encoding="utf-8")

        command = [str(self.binary), f"--config={config}", f"--resource={resource}"]
        if self.mode == "ipc":
            command.append(f"--debug-entry={self.report_origin}/")
        with self.log_path.open("wb") as log:
            self.process = subprocess.Popen(command, cwd=HERE.parents[1], stdout=log, stderr=subprocess.STDOUT)
            try:
                if not self.report_ready.wait(self.timeout):
                    status = self.process.poll()
                    raise SmokeError(f"{self.mode} mode produced no fixture JSON report within {self.timeout}s (native exit={status}); see {self.log_path}")
                return self.report or {"ok": False, "error": "empty fixture report"}
            finally:
                self.stop()

    def stop(self) -> None:
        if self.process and self.process.poll() is None:
            self.process.terminate()
            try:
                self.process.wait(timeout=5)
            except subprocess.TimeoutExpired:
                self.process.kill()
                self.process.wait(timeout=5)
        if self.http:
            self.http.shutdown()
            self.http.server_close()
        for path in (Path(self.file_path), Path(self.file_path + ".large")):
            try:
                path.unlink(missing_ok=True)
            except OSError:
                pass


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", required=True, help="built Niva native executable (macOS only)")
    parser.add_argument("--output", required=True, help="directory for per-lane logs and result.json")
    parser.add_argument("--mode", choices=("both", "ws", "ipc"), default="both",
                        help="both lanes; WS lane uses the default custom protocol, IPC lane uses explicit debug HTTP + CSP")
    parser.add_argument("--trusted-debug", action="store_true",
                        help="authorize the loopback CSP lane with the explicit debug-origin token; required for IPC stream coverage")
    parser.add_argument("--timeout", type=float, default=45.0)
    args = parser.parse_args()
    output = Path(args.output).expanduser().resolve()
    binary = Path(args.binary).expanduser().resolve()
    result = {
        "ok": False,
        "protocolVersion": 2,
        "binary": str(binary),
        "binarySha256": hashlib.sha256(binary.read_bytes()).hexdigest() if binary.is_file() else None,
        "lanes": {},
    }
    failures: list[str] = []
    try:
        if sys.platform != "darwin":
            raise SmokeError("this smoke requires a real macOS WebView; cross-compilation is not a substitute")
        if not binary.is_file():
            raise SmokeError(f"Niva executable not found: {binary}; build a Native debug binary first")
        output.mkdir(parents=True, exist_ok=True)
        modes = ("ws", "ipc") if args.mode == "both" else (args.mode,)
        if "ipc" in modes and not args.trusted_debug:
            raise SmokeError("CSP-denied IPC lane requires runner option --trusted-debug so it exercises a trusted local stream owner")
        for mode in modes:
            print(f"Running {mode} lane ({'custom protocol' if mode == 'ws' else 'explicit debug HTTP with CSP-blocked WS'})", flush=True)
            try:
                case_report = Case(binary, output / mode, mode, args.timeout, args.trusted_debug).run()
                lane_errors = validate_report(case_report, mode)
                result["lanes"][mode] = {"report": case_report, "errors": lane_errors}
                failures.extend(f"{mode}: {error}" for error in lane_errors)
            except Exception as error:  # preserve per-lane failure diagnostics in JSON
                text = f"{type(error).__name__}: {error}"
                result["lanes"][mode] = {"errors": [text], "log": str(output / mode / "niva.log")}
                failures.append(f"{mode}: {text}")
        result["ok"] = not failures
        result["failures"] = failures
    except Exception as error:
        result["failures"] = [f"{type(error).__name__}: {error}"]
    output.mkdir(parents=True, exist_ok=True)
    report_path = output / "result.json"
    report_path.write_text(json.dumps(result, indent=2) + "\n", encoding="utf-8")
    for mode, lane in result.get("lanes", {}).items():
        lane_errors = lane.get("errors", [])
        print(f"{mode}: {'PASS' if not lane_errors else 'FAIL'}", flush=True)
        for error in lane_errors:
            print(f"  - {error}", flush=True)
    print(f"overall: {'PASS' if result['ok'] else 'FAIL'}", flush=True)
    print(f"JSON report: {report_path}", flush=True)
    return 0 if result["ok"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
