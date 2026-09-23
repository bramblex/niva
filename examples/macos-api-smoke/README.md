# macOS native API smoke

This fixture launches a temporary Niva app through `--debug-config`,
`--debug-resource`, and `--stdio`. It reports exact public API methods and
checks their return values or native state transitions. It does not alter the
repository's production modules or use a packaged release app.

Run it in an awake, unlocked desktop session. With both displays asleep,
Tao's macOS active-display list can be empty even though `NSScreen` and
`system_profiler` still report connected displays; this fixture keeps that
native monitor assertion strict and fails early in that state.

Build the current checkout's macOS binary and run the automatic cases:

```sh
cargo build -p niva
python3 examples/macos-api-smoke/run.py target/debug/niva
```

The harness creates a per-run app UUID, temporary resource tree, child-window
fixture, tray icon, and dialog directory. It removes the app's unique
Application Support and cache directories after the child exits. The default
run exercises monitor lookup; window geometry/title/visibility/focus and
child-window state; WebView URL/evaluation/history; tray lifecycle; `host.send`;
and `process.exit`. The runner prints one row per exact method it checked.

Optional cases:

```sh
python3 examples/macos-api-smoke/run.py --clipboard --shortcut target/debug/niva
python3 examples/macos-api-smoke/run.py --dialogs target/debug/niva
```

`--clipboard` snapshots every accessible NSPasteboard item/type, writes and
reads a unique sentinel, and restores the full snapshot only if the pasteboard
still matches the state produced by the smoke write. If another app changes
the clipboard during the test, the harness preserves that newer state and
reports that it skipped restoration. It requires the macOS Swift toolchain.

`--shortcut` briefly registers uncommon four-modifier F17/F18/F19 chords, reads
them back, and unregisters them. It does not synthesize a global keypress.
`--dialogs` opens an informational message, a file picker, and a save panel
one at a time. The runner requires a heartbeat while each dialog is open and
waits for the operator to dismiss it. File panels start in a disposable
directory containing one fixture text file; they never use a user directory.

`process.open` is intentionally excluded because it opens an external app.
`tray.update` is asserted to resolve and retain the item id; the public API has
no readback for its title or tooltip. This fixture does not automate native
menu-bar clicks or shortcut key delivery.
