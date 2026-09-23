#!/usr/bin/env python3
"""Run the stdio example app and exchange one NDJSON message with it."""

import json
import subprocess
import sys
from pathlib import Path


def read_frame(process):
    line = process.stdout.readline()
    if not line:
        raise RuntimeError(f"Niva exited before sending a frame (status {process.poll()})")
    return json.loads(line.decode("utf-8"))


def main():
    if len(sys.argv) != 2:
        raise SystemExit("usage: python examples/stdio_host.py /path/to/niva")

    binary = Path(sys.argv[1]).resolve()
    resource_dir = Path(__file__).resolve().parent / "stdio-host"
    process = subprocess.Popen(
        [str(binary), "--stdio", f"--debug-resource={resource_dir}"],
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
    )

    try:
        ready = read_frame(process)
        if ready != {"t": "ready", "v": 1}:
            raise RuntimeError(f"unexpected first frame: {ready!r}")

        # Invalid input is reported on stderr and must not terminate the bridge.
        process.stdin.write(b"not-json\n")
        process.stdin.flush()

        while True:
            frame = read_frame(process)
            if frame.get("t") == "msg" and frame.get("name") == "page:ready":
                break

        request = {"t": "msg", "name": "sayHello", "data": {"name": "parent"}}
        process.stdin.write((json.dumps(request) + "\n").encode("utf-8"))
        process.stdin.flush()

        while True:
            frame = read_frame(process)
            if frame.get("t") == "msg" and frame.get("name") == "sayHelloResult":
                print(frame["data"]["text"])
                break

        process.stdin.close()
        status = process.wait(timeout=15)
        if status != 0:
            raise RuntimeError(f"Niva exited with status {status}")
    finally:
        if process.poll() is None:
            if process.stdin and not process.stdin.closed:
                process.stdin.close()
            process.terminate()
            process.wait(timeout=5)


if __name__ == "__main__":
    main()
