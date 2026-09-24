#!/usr/bin/env python3
"""Exercise macOS window APIs from the asynchronous WebView bridge.

This disposable smoke test checks that AppKit-backed calls resolve without a
main-thread trap. It does not claim to observe visual effects or Space changes.
"""

from __future__ import annotations

import argparse
import json
import os
import selectors
import shutil
import subprocess
import sys
import tempfile
import time
import uuid
from pathlib import Path


PAGE = r'''<!doctype html><html><body><script>
window.addEventListener("load", () => setTimeout(async () => {
  const windowApi = Niva.api.window;
  const methods = [];
  let active = "startup";
  try {
    for (const name of [
      "list", "scaleFactor", "innerPosition", "outerPosition", "outerSize",
      "title", "isVisible", "isFocused", "isResizable", "isMinimizable",
      "isMaximizable", "isClosable", "isMinimized", "isMaximized"
    ]) {
      active = name;
      await windowApi[name]();
      methods.push(name);
    }
    active = "setOuterPosition";
    await windowApi.setOuterPosition(await windowApi.outerPosition());
    methods.push(active);
    for (const [getter, setter] of [
      ["isResizable", "setResizable"],
      ["isMinimizable", "setMinimizable"],
      ["isMaximizable", "setMaximizable"],
      ["isClosable", "setClosable"]
    ]) {
      active = setter;
      await windowApi[setter](await windowApi[getter]());
      methods.push(setter);
    }
    active = "setFullscreen";
    await windowApi.setFullscreen(false);
    methods.push(active);
    active = "requestUserAttention";
    await windowApi.requestUserAttention("informational");
    methods.push(active);
    active = "setVisibleOnAllWorkspaces";
    try {
      await windowApi.setVisibleOnAllWorkspaces(true);
    } finally {
      await windowApi.setVisibleOnAllWorkspaces(false);
    }
    methods.push(active);
    active = "setCursorIcon";
    try {
      await windowApi.setCursorIcon("crosshair");
      const iconCursor = getComputedStyle(document.documentElement).cursor;
      if (iconCursor !== "crosshair") {
        throw new Error(`setCursorIcon computed cursor was ${iconCursor}`);
      }
      methods.push(active);

      active = "setCursorVisible";
      await windowApi.setCursorVisible(false);
      const hiddenCursor = getComputedStyle(document.documentElement).cursor;
      if (hiddenCursor !== "none") {
        throw new Error(`setCursorVisible(false) computed cursor was ${hiddenCursor}`);
      }
      methods.push(active);
    } finally {
      try {
        await windowApi.setCursorVisible(true);
      } finally {
        await windowApi.setCursorIcon("default");
      }
    }
    await Niva.api.host.send("thread-affinity-result", { methods });
  } catch (error) {
    await Niva.api.host.send("thread-affinity-result", {
      methods, failed: active, error: String(error)
    }).catch(() => {});
  }
}, 100));
</script></body></html>'''

EXPECTED = [
    "list", "scaleFactor", "innerPosition", "outerPosition", "outerSize",
    "title", "isVisible", "isFocused", "isResizable", "isMinimizable",
    "isMaximizable", "isClosable", "isMinimized", "isMaximized",
    "setOuterPosition", "setResizable", "setMinimizable", "setMaximizable",
    "setClosable", "setFullscreen", "requestUserAttention",
    "setVisibleOnAllWorkspaces", "setCursorIcon", "setCursorVisible",
]


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("binary", type=Path)
    args = parser.parse_args()
    if sys.platform != "darwin":
        parser.error("macOS is required")
    binary = args.binary.expanduser().resolve()
    if not binary.is_file():
        parser.error(f"binary not found: {binary}")

    with tempfile.TemporaryDirectory(prefix="niva-thread-affinity-") as temporary:
        root = Path(temporary)
        app_uuid = str(uuid.uuid4())
        name = "NivaThreadAffinitySmoke"
        id_name = f"{name.lower()}_{app_uuid[:8]}"
        (root / "niva.json").write_text(json.dumps({
            "name": name,
            "uuid": app_uuid,
            "window": {
                "entry": "index.html",
                "title": "Niva thread affinity smoke",
                "size": {"width": 260, "height": 180},
            },
        }), encoding="utf-8")
        (root / "index.html").write_text(PAGE, encoding="utf-8")
        env = os.environ.copy()
        env["TMPDIR"] = temporary
        process = subprocess.Popen(
            [str(binary), "--stdio", f"--debug-config={root / 'niva.json'}",
             f"--debug-resource={root}"],
            cwd=Path(__file__).resolve().parents[2],
            env=env,
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
            bufsize=1,
        )
        result = None
        ready = False
        try:
            selector = selectors.DefaultSelector()
            assert process.stdout is not None
            selector.register(process.stdout, selectors.EVENT_READ)
            deadline = time.monotonic() + 45
            while time.monotonic() < deadline and process.poll() is None:
                for _key, _mask in selector.select(timeout=0.5):
                    line = process.stdout.readline()
                    if not line:
                        break
                    frame = json.loads(line)
                    if frame == {"t": "ready", "v": 1}:
                        ready = True
                    elif frame.get("t") == "msg" and frame.get("name") == "thread-affinity-result":
                        result = frame.get("data")
                        break
                if result is not None:
                    break
        finally:
            if process.poll() is None:
                process.terminate()
            try:
                process.wait(timeout=5)
            except subprocess.TimeoutExpired:
                process.kill()
                process.wait(timeout=5)
            for folder in [
                Path.home() / "Library/Application Support" / id_name,
                Path.home() / "Library/Caches" / id_name,
            ]:
                if folder.exists():
                    shutil.rmtree(folder)

        if not ready or not isinstance(result, dict) or result.get("methods") != EXPECTED:
            assert process.stderr is not None
            print(f"FAIL ready={ready} result={result} stderr={process.stderr.read()[-700:]}", file=sys.stderr)
            return 1
        print(f"PASS: {len(EXPECTED)} async window calls resolved on macOS without a main-thread trap")
        print("Cursor icon and visibility were checked through WebView computed CSS; screen pixels and Space effects were not asserted")
        return 0


if __name__ == "__main__":
    raise SystemExit(main())
