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

To run the complete disposable macOS matrix, including native window,
dialog, Dock, process-open, and NodeCompat WebView suites, use:

```sh
python3 -B examples/macos-api-smoke/run_all.py target/debug/niva
```

For a focused regression check of AppKit thread affinity without monitor or
pixel observations, run:

```sh
python3 -B examples/macos-api-smoke/run_thread_affinity.py target/debug/niva
```

It calls 22 window APIs through the asynchronous WebView bridge, including
`setVisibleOnAllWorkspaces(true)` and its cleanup call. It verifies that all
calls resolve without a main-thread trap; it does not assert their visual
effects or replace the supervised suite.

The aggregate runner saves one log per suite under a reported `/tmp` directory
and checks the exact 167 `Niva.api` methods and nine top-level bridge methods.
Its summary separates
behavior assertions from API calls whose native effect was not independently
observed. The full run temporarily changes the clipboard, pointer, Dock,
application visibility, and foreground app, then restores their prior state.

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
python3 examples/macos-api-smoke/run.py --dialogs-auto target/debug/niva
python3 examples/macos-api-smoke/run.py --extended-automatic target/debug/niva
```

`--extended-automatic` adds 62 automatic window, windowExtra, HTTP, process,
resource, WebView, and extra API cases from `window-cases.js` and
`system-cases.js` to the default suite. The runner verifies the exact
method/assertion records against its expected registry and lists supervised
native UI cases separately without counting them as executed.

Run the separate headless-safe suite when the desktop displays are asleep:

```sh
python3 examples/macos-api-smoke/run.py --headless-safe target/debug/niva
```

This launches three independent Niva processes, each with a fresh app profile
and temporary tree: `process-os` (12 methods), `resource` (4 methods), and `fs`
(14 methods). Each group checks its own exact method registry and runs
`host.send` plus `process.exit`. The mode does not call monitors or claim any
of the default 39-case native suite. Clipboard, shortcut, and dialog options
cannot be combined with it.

For diagnosing sequence or bridge pressure, `--headless-all-sequential` runs
the same three groups in one disposable process. It is a stress diagnostic and
does not replace the three isolated group results.

`--clipboard` snapshots every accessible NSPasteboard item/type, writes and
reads a unique sentinel, and restores the full snapshot only if the pasteboard
still matches the state produced by the smoke write. If another app changes
the clipboard during the test, the harness preserves that newer state and
reports that it skipped restoration. It requires the macOS Swift toolchain.

`--shortcut` briefly registers uncommon four-modifier F17/F18/F19 chords, reads
them back, and unregisters them. It does not synthesize a global keypress.
`--dialogs` opens a message and all five picker types, one at a time. The
runner requires a heartbeat while each native panel is open. With
`--dialogs-auto`, it dismisses the message and cancels each picker, checking
the cancellation result. Picker selection behavior can also be checked by
using `--dialogs` and selecting fixtures from the disposable directory.

`run_system_supervised.py` tests `process.open` by opening only a disposable
directory in Finder and closing that window. It also checks print preview,
inline HTML, app visibility, activation policy, Dock visibility/badge/progress,
and restores other applications after the hide-others case.
`run_window_supervised.py` checks window IPC, native layers, content sharing,
background color, focus, cursor, traffic-light placement, and close-request
blocking; it labels six additional successful native calls as invocation-only
because their visual or input effect has no independent readback in the suite.
`run_drag_window.py` uses a real synthetic mouse gesture and checks the window
position change. `run_bridge_top_level.py` checks all nine `NivaObj` bridge
methods against real native events and streams.
`examples/node-compat-macos-smoke/` covers NodeCompat in a real WebView.

`tray.update` is asserted to resolve and retain the item id; the public API has
no readback for its title or tooltip. This fixture does not automate native
menu-bar clicks or shortcut key delivery.
