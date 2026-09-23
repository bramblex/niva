"""Exercise reversible Windows native APIs in a disposable packaged Niva app."""

import ctypes
import json
import queue
import subprocess
import sys
import threading
import time
from pathlib import Path

EXPECTED_METHODS = set("""
clipboard.read
monitor.current monitor.fromPoint monitor.list monitor.primary
shortcut.list shortcut.register shortcut.unregister shortcut.unregisterAll
tray.create tray.destroy tray.destroyAll tray.list
webview.baseFileSystemUrl webview.baseUrl webview.canGoBack webview.canGoForward
webview.cookies webview.cookiesForUrl webview.deleteCookie webview.evaluateScript
webview.goBack webview.goForward webview.loadHtml webview.loadUrl webview.reload
webview.setCookie webview.url
window.close window.current window.hideMenu window.innerPosition window.innerSize
window.isClosable window.isDecorated window.isMaximizable window.isMaximized
window.isMenuVisible window.isMinimizable window.isMinimized window.isResizable
window.isVisible window.list window.open window.outerPosition window.outerSize
window.scaleFactor window.sendMessage window.setAlwaysOnTop window.setClosable
window.setContentProtection window.setDecorated window.setInnerSize
window.setMaximizable window.setMaximized window.setMenu window.setMinimizable
window.setMinimized window.setOuterPosition window.setResizable window.setTheme
window.setTitle window.setVisible window.setWindowIcon window.showMenu
window.theme window.title
windowExtra.hasUndecoratedShadow windowExtra.setEnable windowExtra.setRtl
windowExtra.setTaskbarIcon windowExtra.setUndecoratedShadow windowExtra.theme
""".split())


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


def native_child_state(pid, name):
    user32 = ctypes.WinDLL("user32", use_last_error=True)
    user32.GetWindowThreadProcessId.argtypes = [ctypes.c_void_p, ctypes.POINTER(ctypes.c_ulong)]
    user32.GetWindowTextLengthW.argtypes = [ctypes.c_void_p]
    user32.GetWindowTextLengthW.restype = ctypes.c_int
    user32.GetWindowTextW.argtypes = [ctypes.c_void_p, ctypes.c_wchar_p, ctypes.c_int]
    callbacks = ctypes.WINFUNCTYPE(ctypes.c_bool, ctypes.c_void_p, ctypes.c_ssize_t)
    found = []

    @callbacks
    def visit(hwnd, _):
        window_pid = ctypes.c_ulong()
        user32.GetWindowThreadProcessId(hwnd, ctypes.byref(window_pid))
        if window_pid.value == pid:
            length = user32.GetWindowTextLengthW(hwnd)
            title = ctypes.create_unicode_buffer(length + 1)
            user32.GetWindowTextW(hwnd, title, length + 1)
            if title.value == "Niva API child" or "api-child.html" in title.value:
                found.append(hwnd)
        return True

    user32.EnumWindows(callbacks(visit), 0)
    if len(found) != 1:
        raise RuntimeError(f"expected one native API child, found {len(found)}")
    hwnd = found[0]
    user32.IsWindowEnabled.argtypes = [ctypes.c_void_p]
    user32.GetWindowLongPtrW.argtypes = [ctypes.c_void_p, ctypes.c_int]
    user32.GetWindowLongPtrW.restype = ctypes.c_ssize_t
    user32.SendMessageW.argtypes = [ctypes.c_void_p, ctypes.c_uint, ctypes.c_size_t, ctypes.c_ssize_t]
    user32.SendMessageW.restype = ctypes.c_ssize_t
    if name == "enabled":
        return bool(user32.IsWindowEnabled(hwnd))
    if name in ("rtl", "topmost"):
        style = user32.GetWindowLongPtrW(hwnd, -20)  # GWL_EXSTYLE
        return bool(style & (0x00400000 if name == "rtl" else 0x00000008))
    if name == "displayAffinity":
        affinity = ctypes.c_uint()
        user32.GetWindowDisplayAffinity.argtypes = [ctypes.c_void_p, ctypes.POINTER(ctypes.c_uint)]
        if not user32.GetWindowDisplayAffinity(hwnd, ctypes.byref(affinity)):
            raise RuntimeError("GetWindowDisplayAffinity failed")
        return affinity.value
    if name in ("iconSmall", "iconBig"):
        return bool(user32.SendMessageW(hwnd, 0x007F, 0 if name == "iconSmall" else 1, 0))
    raise RuntimeError(f"unknown native probe {name}")


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
            while True:
                result = frames.get(timeout=90)
                if result.get("name") != "native-probe":
                    break
                probe = result["data"]
                actual = None
                error = None
                for _ in range(20):
                    try:
                        actual = native_child_state(process.pid, probe["name"])
                        error = None
                    except RuntimeError as caught:
                        error = str(caught)
                    if error is None and actual == probe["expected"]:
                        break
                    time.sleep(0.05)
                answer = {"t": "msg", "name": "native-result", "data": {
                    "name": probe["name"], "value": actual, "error": error,
                }}
                process.stdin.write((json.dumps(answer) + "\n").encode("utf-8"))
                process.stdin.flush()
            if result.get("name") == "api-fatal":
                raise RuntimeError(result.get("data", {}).get("message", "API fixture failed"))
            assert result.get("name") == "api-results", result.get("name")
            data = result["data"]
            checks = data["checks"]
            failed = {name: outcome for name, outcome in checks.items() if outcome != "pass"}
            print(f"API: {len(checks)} behavioral checks, {len(failed)} failures", flush=True)
            assert not failed, failed
            assert set(checks) == EXPECTED_METHODS, {
                "missing": sorted(EXPECTED_METHODS - set(checks)),
                "unexpected": sorted(set(checks) - EXPECTED_METHODS),
            }
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
