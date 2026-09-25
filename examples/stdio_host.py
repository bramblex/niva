#!/usr/bin/env python3
"""Exchange newline-delimited text with a Niva app's ordinary process streams."""

import subprocess
import sys
from pathlib import Path


def read_line(process):
    line = process.stdout.readline()
    if not line:
        raise RuntimeError(f"Niva exited before writing a line (status {process.poll()})")
    return line.decode("utf-8").rstrip("\r\n")


def main():
    if len(sys.argv) != 2:
        raise SystemExit("usage: python examples/stdio_host.py /path/to/niva")

    binary = Path(sys.argv[1]).resolve()
    resource_dir = Path(__file__).resolve().parent / "stdio-host"
    process = subprocess.Popen(
        [str(binary), f"--resource={resource_dir}"],
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
    )

    try:
        ready = read_line(process)
        if ready != "ready":
            raise RuntimeError(f"unexpected first line: {ready!r}")

        process.stdin.write(b"parent\n")
        process.stdin.flush()
        reply = read_line(process)
        print(reply)
        if reply != "Hello, parent!":
            raise RuntimeError(f"unexpected reply: {reply!r}")

        # EOF finishes only process.stdin. The Niva window remains open until
        # the user closes it; this blocks here intentionally for the demo.
        process.stdin.close()
        eof = read_line(process)
        if eof != "stdin-eof":
            raise RuntimeError(f"unexpected EOF notification: {eof!r}")
        print("Input stream ended; close the Niva window to finish.", flush=True)
        status = process.wait()
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
