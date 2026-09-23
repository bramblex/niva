"""Verify clearAllBrowsingData in a freshly generated disposable Niva profile."""

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
    with open(Path.cwd() / "dist/windows-clear-stderr.log", "wb") as errors:
        process = subprocess.Popen([str(binary), "--stdio"], stdin=subprocess.PIPE,
                                   stdout=subprocess.PIPE, stderr=errors)
        threading.Thread(target=read_frames, args=(process, frames), daemon=True).start()
        try:
            assert frames.get(timeout=20) == {"t": "ready", "v": 1}
            result = frames.get(timeout=30)
            if result.get("name") == "clear-error":
                raise RuntimeError(result["data"]["message"])
            assert result.get("name") == "clear-result", result.get("name")
            assert result["data"] == {"before": True, "after": False}, result["data"]
            process.stdin.close()
            assert process.wait(timeout=10) == 0
            print("Browsing data: disposable profile cookie created, cleared, and verified absent")
        finally:
            if process.poll() is None:
                process.terminate()
                process.wait(timeout=5)


if __name__ == "__main__":
    main()
