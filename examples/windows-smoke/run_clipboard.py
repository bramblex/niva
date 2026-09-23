"""Conditionally exercise clipboard.write with an in-memory all-format snapshot."""

import ctypes
import json
import queue
import secrets
import subprocess
import sys
import threading
import time
from pathlib import Path


user32 = ctypes.WinDLL("user32", use_last_error=True)
kernel32 = ctypes.WinDLL("kernel32", use_last_error=True)
user32.OpenClipboard.argtypes = [ctypes.c_void_p]
user32.CloseClipboard.argtypes = []
user32.EnumClipboardFormats.argtypes = [ctypes.c_uint]
user32.EnumClipboardFormats.restype = ctypes.c_uint
user32.GetClipboardData.argtypes = [ctypes.c_uint]
user32.GetClipboardData.restype = ctypes.c_void_p
user32.EmptyClipboard.argtypes = []
user32.SetClipboardData.argtypes = [ctypes.c_uint, ctypes.c_void_p]
user32.SetClipboardData.restype = ctypes.c_void_p
kernel32.GlobalSize.argtypes = [ctypes.c_void_p]
kernel32.GlobalSize.restype = ctypes.c_size_t
kernel32.GlobalLock.argtypes = [ctypes.c_void_p]
kernel32.GlobalLock.restype = ctypes.c_void_p
kernel32.GlobalUnlock.argtypes = [ctypes.c_void_p]
kernel32.GlobalAlloc.argtypes = [ctypes.c_uint, ctypes.c_size_t]
kernel32.GlobalAlloc.restype = ctypes.c_void_p
kernel32.GlobalFree.argtypes = [ctypes.c_void_p]


def open_clipboard():
    for _ in range(20):
        if user32.OpenClipboard(None):
            return
        time.sleep(0.05)
    raise RuntimeError("Windows clipboard is busy")


def snapshot():
    """Return all currently available HGLOBAL formats, or unsupported IDs."""
    records = []
    unsupported = []
    open_clipboard()
    try:
        fmt = 0
        while True:
            ctypes.set_last_error(0)
            fmt = user32.EnumClipboardFormats(fmt)
            if not fmt:
                if ctypes.get_last_error():
                    raise RuntimeError("EnumClipboardFormats failed")
                break
            handle = user32.GetClipboardData(fmt)
            if not handle:
                unsupported.append(fmt)
                continue
            size = kernel32.GlobalSize(handle)
            if size == 0 or size > 32 * 1024 * 1024:
                unsupported.append(fmt)
                continue
            pointer = kernel32.GlobalLock(handle)
            if not pointer:
                unsupported.append(fmt)
                continue
            try:
                records.append((fmt, ctypes.string_at(pointer, size)))
            finally:
                kernel32.GlobalUnlock(handle)
    finally:
        user32.CloseClipboard()
    return records, unsupported


def clipboard_text():
    open_clipboard()
    try:
        handle = user32.GetClipboardData(13)  # CF_UNICODETEXT
        if not handle:
            return None
        pointer = kernel32.GlobalLock(handle)
        if not pointer:
            return None
        try:
            return ctypes.wstring_at(pointer)
        finally:
            kernel32.GlobalUnlock(handle)
    finally:
        user32.CloseClipboard()


def restore(records):
    open_clipboard()
    try:
        if not user32.EmptyClipboard():
            raise RuntimeError("EmptyClipboard failed during restoration")
        for fmt, data in records:
            handle = kernel32.GlobalAlloc(0x0002, len(data))  # GMEM_MOVEABLE
            if not handle:
                raise RuntimeError(f"GlobalAlloc failed for format {fmt}")
            pointer = kernel32.GlobalLock(handle)
            if not pointer:
                kernel32.GlobalFree(handle)
                raise RuntimeError(f"GlobalLock failed for format {fmt}")
            ctypes.memmove(pointer, data, len(data))
            kernel32.GlobalUnlock(handle)
            if not user32.SetClipboardData(fmt, handle):
                kernel32.GlobalFree(handle)
                raise RuntimeError(f"SetClipboardData failed for format {fmt}")
    finally:
        user32.CloseClipboard()


def read_frames(process, frames):
    for line in process.stdout:
        frames.put(json.loads(line))
    frames.put({"t": "eof"})


def main():
    if "--inspect" in sys.argv[1:]:
        records, unsupported = snapshot()
        print(f"Clipboard formats: {len(records)} HGLOBAL, {len(unsupported)} unsupported; IDs only: {unsupported}")
        return
    binary = Path(sys.argv[1]).resolve()
    marker = "NivaClipboardSmoke-" + secrets.token_hex(16)
    frames = queue.Queue()
    with open(Path.cwd() / "dist/windows-clipboard-stderr.log", "wb") as errors:
        process = subprocess.Popen([str(binary), "--stdio"], stdin=subprocess.PIPE,
                                   stdout=subprocess.PIPE, stderr=errors)
        threading.Thread(target=read_frames, args=(process, frames), daemon=True).start()
        wrote = False
        records = None
        try:
            assert frames.get(timeout=20) == {"t": "ready", "v": 1}
            assert frames.get(timeout=20).get("name") == "page-ready"
            records, unsupported = snapshot()
            if unsupported:
                print(f"SKIP clipboard.write: {len(unsupported)} formats cannot be safely snapshotted; clipboard unchanged")
                return
            if snapshot() != (records, []):
                print("SKIP clipboard.write: clipboard changed during snapshot; clipboard unchanged")
                return
            command = {"t": "msg", "name": "start-clipboard", "data": {"marker": marker}}
            process.stdin.write((json.dumps(command) + "\n").encode("utf-8"))
            process.stdin.flush()
            result = frames.get(timeout=20)
            wrote = clipboard_text() == marker
            if result.get("name") == "clipboard-error":
                raise RuntimeError(result["data"]["message"])
            assert result.get("name") == "clipboard-result" and result["data"] == {"match": True}
            assert wrote, "native clipboard does not contain the unique test marker"
        finally:
            try:
                if records is not None and clipboard_text() == marker:
                    restore(records)
                    restored, unsupported = snapshot()
                    if unsupported or restored != records:
                        raise RuntimeError("clipboard restoration differed from the all-format snapshot")
                elif wrote:
                    raise RuntimeError("clipboard changed after test marker; refusing to overwrite newer content")
            finally:
                if process.poll() is None:
                    process.stdin.close()
                    try: process.wait(timeout=10)
                    except subprocess.TimeoutExpired: pass
                if process.poll() is None:
                    process.terminate()
                    process.wait(timeout=5)
    print(f"clipboard.write: marker read back and {len(records)} clipboard formats restored byte-for-byte")


if __name__ == "__main__":
    main()
