import json
import queue
import subprocess
import sys
import threading
from pathlib import Path


def read_frames(process, frames):
    for line in process.stdout:
        frame = json.loads(line)
        if not isinstance(frame, dict) or frame.get("protocol") != "niva-fixture" or frame.get("version") != 1:
            frames.put({"event": "bad-json", "frame": frame})
        else:
            frames.put(frame)
    frames.put({"event": "eof"})


def main():
    binary = Path(sys.argv[1]).resolve()
    frames = queue.Queue()
    with open(Path.cwd() / "dist/windows-csp-stderr.log", "wb") as errors:
        process = subprocess.Popen([str(binary)], stdin=subprocess.PIPE,
                                   stdout=subprocess.PIPE, stderr=errors)
        threading.Thread(target=read_frames, args=(process, frames), daemon=True).start()
        try:
            ready = frames.get(timeout=20)
            assert ready.get("event") == "ready" and ready.get("name") == "windows-csp", ready
            result = frames.get(timeout=20)
            assert result.get("event") == "message", result
            if result.get("name") == "csp-error":
                raise RuntimeError(result["data"]["message"])
            assert result.get("name") == "csp-pass", result.get("name")
            data = result["data"]
            configured_uuid = json.loads(Path(__file__).with_name("niva-csp.json").read_text())["uuid"]
            expected_origin = "http://niva-" + configured_uuid.replace("-", "").lower() + ".app"
            assert data == {
                "origin": expected_origin,
                "windowId": 0,
                "pathResult": "a\\b",
                "fileStatus": 200,
                "fileText": "resource-ok",
                "nativeText": "resource-ok",
                "authorInlineBlocked": True,
            }, data
            process.stdin.write((json.dumps({"protocol": "niva-fixture", "version": 1, "event": "command", "name": "fixture-exit", "data": 0}) + "\n").encode("utf-8"))
            process.stdin.flush()
            process.stdin.close()
            assert process.wait(timeout=10) == 0
            print("CSP: direct Niva namespaces, Node modules, and file fetch passed; author inline script blocked", flush=True)
        finally:
            if process.poll() is None:
                process.terminate()
                process.wait(timeout=5)


if __name__ == "__main__":
    main()
