import json
import queue
import subprocess
import sys
import threading
from pathlib import Path


def read_frames(process, frames):
    for line in process.stdout:
        frames.put(json.loads(line))
    frames.put({"t": "eof"})


def main():
    binary = Path(sys.argv[1]).resolve()
    frames = queue.Queue()
    with open(Path.cwd() / "dist/windows-csp-stderr.log", "wb") as errors:
        process = subprocess.Popen([str(binary), "--stdio"], stdin=subprocess.PIPE,
                                   stdout=subprocess.PIPE, stderr=errors)
        threading.Thread(target=read_frames, args=(process, frames), daemon=True).start()
        try:
            assert frames.get(timeout=20) == {"t": "ready", "v": 1}
            result = frames.get(timeout=20)
            if result.get("name") == "csp-error":
                raise RuntimeError(result["data"]["message"])
            assert result.get("name") == "csp-pass", result.get("name")
            data = result["data"]
            assert data == {
                "origin": "http://niva.app",
                "windowId": 0,
                "pathResult": "a\\b",
                "fileStatus": 200,
                "fileText": "resource-ok",
                "nativeText": "resource-ok",
                "authorInlineBlocked": True,
            }, data
            process.stdin.close()
            assert process.wait(timeout=10) == 0
            print("CSP: native bridge, NodeCompat and file fetch passed; author inline script blocked", flush=True)
        finally:
            if process.poll() is None:
                process.terminate()
                process.wait(timeout=5)


if __name__ == "__main__":
    main()
