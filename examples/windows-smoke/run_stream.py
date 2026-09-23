"""Verify large Windows bridge streams through a packaged WebView2 app."""

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
    output = Path.cwd() / "dist/windows-stream-stage/output.txt"
    frames = queue.Queue()
    with open(Path.cwd() / "dist/windows-stream-stderr.log", "wb") as errors:
        process = subprocess.Popen([str(binary), "--stdio"], stdin=subprocess.PIPE,
                                   stdout=subprocess.PIPE, stderr=errors)
        threading.Thread(target=read_frames, args=(process, frames), daemon=True).start()
        try:
            assert frames.get(timeout=20) == {"t": "ready", "v": 1}
            result = frames.get(timeout=30)
            if result.get("name") == "stream-error":
                raise RuntimeError(result["data"]["message"])
            assert result.get("name") == "stream-ok", result.get("name")
            data = result["data"]
            assert data["fileLength"] == 150004 and data["fileBoundary"] == "Atail", data
            assert data["resourceLength"] == 150000 and data["resourceBoundary"] == "RR", data
            child = data["child"]
            assert child["status"] == 0 and "STDOUT" in child["stdout"] and "STDERR" in child["stderr"], data
            assert output.read_bytes() == b"A" * 150000 + b"tail", "native file content differs from streamed read"
            process.stdin.close()
            assert process.wait(timeout=10) == 0
            print("Streams: 150 KB fs.write/append/read, 150 KB resource.read and process.exec stdout/stderr passed")
        finally:
            if process.poll() is None:
                process.terminate()
                process.wait(timeout=5)
            output.unlink(missing_ok=True)


if __name__ == "__main__":
    main()
