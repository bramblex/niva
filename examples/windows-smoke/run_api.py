"""Exercise reversible Windows native APIs in a disposable packaged Niva app."""

import ctypes
import json
import queue
import subprocess
import sys
import threading
import time
from pathlib import Path


def clipboard_digest():
    user32 = ctypes.WinDLL("user32", use_last_error=True)
    kernel32 = ctypes.WinDLL("kernel32", use_last_error=True)
    user32.GetClipboardData.argtypes = [ctypes.c_uint]
    user32.GetClipboardData.restype = ctypes.c_void_p
    kernel32.GlobalLock.argtypes = [ctypes.c_void_p]
    kernel32.GlobalLock.restype = ctypes.c_void_p
    kernel32.GlobalUnlock.argtypes = [ctypes.c_void_p]
    for _ in range(10):
        if user32.OpenClipboard(None):
            break
        time.sleep(0.05)
    else:
        raise RuntimeError("clipboard is busy; no clipboard content was changed")
    try:
        handle = user32.GetClipboardData(13)  # CF_UNICODETEXT
        if not handle:
            return None
        pointer = kernel32.GlobalLock(handle)
        if not pointer:
            raise RuntimeError("unable to read clipboard text")
        try:
            text = ctypes.wstring_at(pointer)
        finally:
            kernel32.GlobalUnlock(handle)
    finally:
        user32.CloseClipboard()
    value = 0x811C9DC5
    for byte in text.encode("utf-8"):
        value = ((value ^ byte) * 0x01000193) & 0xFFFFFFFF
    return f"{value:08x}"


def read_frames(process, frames):
    for line in process.stdout:
        try:
            frames.put(json.loads(line))
        except json.JSONDecodeError:
            frames.put({"t": "invalid-json"})
    frames.put({"t": "eof"})


def main():
    binary = Path(sys.argv[1]).resolve()
    expected_clipboard = clipboard_digest()
    frames = queue.Queue()
    with open(Path.cwd() / "dist/windows-api-stderr.log", "wb") as errors:
        process = subprocess.Popen([str(binary), "--stdio"], stdin=subprocess.PIPE,
                                   stdout=subprocess.PIPE, stderr=errors)
        threading.Thread(target=read_frames, args=(process, frames), daemon=True).start()
        try:
            assert frames.get(timeout=20) == {"t": "ready", "v": 1}
            result = frames.get(timeout=90)
            if result.get("name") == "api-fatal":
                raise RuntimeError(result.get("data", {}).get("message", "API fixture failed"))
            assert result.get("name") == "api-results", result.get("name")
            data = result["data"]
            checks = data["checks"]
            failed = {name: outcome for name, outcome in checks.items() if outcome != "pass"}
            print(f"API: {len(checks)} behavioral checks, {len(failed)} failures", flush=True)
            assert not failed, failed
            assert len(checks) >= 50, f"expected at least 50 behavioral checks, got {len(checks)}"
            assert data["clipboardDigest"] == expected_clipboard, "clipboard.read differs from native CF_UNICODETEXT"
            process.stdin.close()
            assert process.wait(timeout=10) == 0
            print(f"API: {len(checks)} behavioral checks passed; clipboard digest matched native Windows clipboard")
            print("Methods: " + ", ".join(sorted(checks)))
        finally:
            if process.poll() is None:
                process.terminate()
                process.wait(timeout=5)


if __name__ == "__main__":
    main()
