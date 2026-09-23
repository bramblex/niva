"""Supervised print-preview check: verify preview, cancel it, then press Enter."""

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
    with open(Path.cwd() / "dist/windows-print-stderr.log", "wb") as errors:
        process = subprocess.Popen([str(binary), "--stdio"], stdin=subprocess.PIPE,
                                   stdout=subprocess.PIPE, stderr=errors)
        threading.Thread(target=read_frames, args=(process, frames), daemon=True).start()
        try:
            assert frames.get(timeout=20) == {"t": "ready", "v": 1}
            assert frames.get(timeout=20).get("name") == "page-ready"
            command = {"t": "msg", "name": "start-print"}
            process.stdin.write((json.dumps(command) + "\n").encode("utf-8"))
            process.stdin.flush()
            seen = set()
            while True:
                event = frames.get(timeout=10)
                if event.get("name") == "print-error":
                    raise RuntimeError(event["data"]["message"])
                if event.get("name") == "print-dispatched":
                    break
                seen.add(event.get("name"))
            print(f"WAITING_FOR_PRINT_UI PID {process.pid}: verify preview, cancel without printing, then press Enter here", flush=True)
            input()
            ping = {"t": "msg", "name": "post-cancel-ping"}
            process.stdin.write((json.dumps(ping) + "\n").encode("utf-8"))
            process.stdin.flush()
            while seen != {"post-cancel-pong", "print-call-returned"}:
                result = frames.get(timeout=20)
                if result.get("name") == "print-error":
                    raise RuntimeError(result["data"]["message"])
                seen.add(result.get("name"))
            process.stdin.close()
            assert process.wait(timeout=10) == 0
            print("Print preview was supervised and cancelled")
        finally:
            if process.poll() is None:
                process.terminate()
                process.wait(timeout=5)


if __name__ == "__main__":
    main()
