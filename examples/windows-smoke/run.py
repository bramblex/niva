import json
import ctypes
import queue
import re
import subprocess
import sys
import threading
import urllib.error
import urllib.parse
import urllib.request
from pathlib import Path


def check_native_owner(pid, expected_host):
    user32 = ctypes.WinDLL("user32", use_last_error=True)
    user32.EnumWindows.argtypes = [ctypes.WINFUNCTYPE(ctypes.c_bool, ctypes.c_void_p, ctypes.c_ssize_t), ctypes.c_ssize_t]
    user32.GetWindowTextLengthW.argtypes = [ctypes.c_void_p]
    user32.GetWindowTextLengthW.restype = ctypes.c_int
    user32.GetWindowTextW.argtypes = [ctypes.c_void_p, ctypes.c_wchar_p, ctypes.c_int]
    user32.GetWindowThreadProcessId.argtypes = [ctypes.c_void_p, ctypes.POINTER(ctypes.c_ulong)]
    user32.GetWindow.argtypes = [ctypes.c_void_p, ctypes.c_uint]
    user32.GetWindow.restype = ctypes.c_void_p
    found = {}
    candidates = []

    def visit(hwnd, _):
        window_pid = ctypes.c_ulong()
        user32.GetWindowThreadProcessId(hwnd, ctypes.byref(window_pid))
        length = user32.GetWindowTextLengthW(hwnd)
        title = ctypes.create_unicode_buffer(length + 1)
        user32.GetWindowTextW(hwnd, title, length + 1)
        if window_pid.value == pid:
            candidates.append((title.value, window_pid.value, hwnd))
        if window_pid.value == pid and title.value in (f"{expected_host}/index.html", f"{expected_host}/child.html"):
            found[title.value] = hwnd
        return True

    enum_result = user32.EnumWindows(ctypes.WINFUNCTYPE(ctypes.c_bool, ctypes.c_void_p, ctypes.c_ssize_t)(visit), 0)
    main_title, child_title = f"{expected_host}/index.html", f"{expected_host}/child.html"
    assert set(found) == {main_title, child_title}, (pid, found, candidates, enum_result, ctypes.get_last_error())
    owner = user32.GetWindow(found[child_title], 4)
    assert owner == found[main_title], (found, owner)
    return True


def read_frame(process, frames):
    for line in process.stdout:
        try:
            frame = json.loads(line)
            if not isinstance(frame, dict) or frame.get("protocol") != "niva-fixture" or frame.get("version") != 1:
                frames.put({"event": "bad-json", "frame": frame, "line": repr(line)})
            else:
                frames.put(frame)
        except json.JSONDecodeError as error:
            frames.put({"event": "bad-json", "error": str(error), "line": repr(line)})
    frames.put({"event": "eof"})


def request(url, origin=None):
    headers = {"Origin": origin} if origin else {}
    req = urllib.request.Request(url, headers=headers)
    try:
        with urllib.request.urlopen(req, timeout=5) as response:
            return response.status, response.headers.get("Access-Control-Allow-Origin"), response.read()
    except urllib.error.HTTPError as error:
        return error.code, error.headers.get("Access-Control-Allow-Origin"), error.read()


def main():
    binary = Path(sys.argv[1]).resolve()
    frames = queue.Queue()
    with open(Path.cwd() / "dist/windows-smoke-stderr.log", "wb") as errors:
        process = subprocess.Popen([str(binary)], stdin=subprocess.PIPE,
                                   stdout=subprocess.PIPE, stderr=errors)
        threading.Thread(target=read_frame, args=(process, frames), daemon=True).start()
        try:
            messages = []
            while True:
                frame = frames.get(timeout=20)
                messages.append(frame)
                if frame.get("event") == "message" and frame.get("name") in ("smoke", "smoke-error") or frame.get("event") == "eof":
                    break
            assert messages[0].get("event") == "ready" and messages[0].get("name") == "windows-smoke", "missing fixture ready event"
            result = messages[-1]
            assert result.get("event") == "message", result
            if result.get("name") == "smoke-error":
                raise RuntimeError(result.get("data", {}).get("message", "page smoke failed"))
            assert result.get("name") == "smoke", result.get("name")
            data = result["data"]
            public_data = {key: value for key, value in data.items() if key != "fsBase"}
            configured_uuid = json.loads(Path(__file__).with_name("niva.json").read_text())["uuid"]
            expected_host = "niva-" + configured_uuid.replace("-", "").lower() + ".app"
            expected_origin = "http://" + expected_host
            assert data["origin"] == expected_origin, public_data
            assert data["windowId"] == 0, public_data
            assert data["resourceStatus"] == 200 and data["resourceText"] == "resource-ok", public_data
            assert data["fsStatus"] == 200 and data["fsText"] == "resource-ok", public_data
            assert data["iframe"] == {"kind": "iframe", "id": 0, "origin": expected_origin}, public_data
            assert data["crossIframe"]["origin"] == "null" and data["crossIframe"]["outcome"] != "allowed", public_data
            assert data["runtimeModules"] == {
                "path": "a\\b", "fsText": "resource-ok", "assert": "function",
                "fileUrlRoundTrip": True, "namespacedPath": True,
            }, public_data
            assert data["menu"] == {"shown": True, "hidden": True, "restored": True}, public_data
            assert data["shortcutId"] >= 0, public_data
            assert data["childId"] > 0 and len(data["windows"]) == 2, public_data
            print("process", process.pid, "poll", process.poll(), "windows", data["windows"])
            assert check_native_owner(process.pid, expected_host)
            path = str(Path(__file__).with_name("probe.txt")).replace("\\", "/")
            valid_url = data["fsBase"] + urllib.parse.quote(path, safe="")
            status, cors, body = request(valid_url, data["origin"])
            assert (status, cors, body.strip()) == (200, data["origin"], b"resource-ok"), (status, cors, body)
            status, cors, _ = request(valid_url, "https://wrong.example")
            assert status == 200 and cors is None, (status, cors)
            bad_url = re.sub(r"/__niva_fs/[^/]+/", "/__niva_fs/invalid-token/", valid_url)
            status, cors, _ = request(bad_url, data["origin"])
            assert status == 403 and cors is None, (status, cors)
            loopback = urllib.parse.urlsplit(data["fsBase"])
            status, _, _ = request(f"{loopback.scheme}://{loopback.netloc}/probe.txt")
            assert status == 404, status
            print(json.dumps(public_data, ensure_ascii=False))
            print("HTTP: valid 200 + ACAO, wrong origin 200 without ACAO, bad token 403, plain static 404")
            print("Win32: child GW_OWNER points to main window; menu set/hide/show succeeded")
            if "--ui" in sys.argv[2:]:
                print("WAITING_FOR_UI: click Smoke > Ping, then press Ctrl+Alt+Shift+F12", flush=True)
                pending = {"menu-clicked", "shortcut-fired"}
                while pending:
                    event = frames.get(timeout=90)
                    if event.get("event") != "message":
                        if event.get("event") == "eof":
                            raise RuntimeError("Niva exited before UI events")
                        continue
                    if event.get("name") == "menu-clicked":
                        assert event["data"]["id"] == 7, event
                        pending.discard("menu-clicked")
                    elif event.get("name") == "shortcut-fired":
                        assert event["data"]["id"] == data["shortcutId"], event
                        pending.discard("shortcut-fired")
                print("UI: native menu click and global shortcut emitted expected IDs", flush=True)
            process.stdin.write((json.dumps({"protocol": "niva-fixture", "version": 1, "event": "command", "name": "fixture-exit", "data": 0}) + "\n").encode("utf-8"))
            process.stdin.flush()
            process.stdin.close()
            assert process.wait(timeout=10) == 0
        finally:
            if process.poll() is None:
                process.terminate()
                process.wait(timeout=5)


if __name__ == "__main__":
    main()
