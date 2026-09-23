# Windows smoke test report

Tested on Windows, branch `codex/windows-api-test-smoke`, against the current
`codex/api-test-coverage` work plus the Windows separator assertion fix.

## Passed

- Rust format, workspace check, Clippy, workspace tests, and Windows target check.
- Node compatibility tests (47/47), Devtools script tests (19/19), Devtools build,
  and `build_Windows.cmd` packaging.
- `build.ps1`: CSP, origin and iframe permissions, file-token HTTP behavior,
  NodeCompat, 150 KB streams, process output, reversible clipboard and profile
  tests, and 73/73 native API behavior checks. This includes child WebView
  navigation, reload, `loadHtml`, and clearing a cookie in a disposable profile.
- The clipboard write case ran only after all six existing formats could be
  snapshotted; it restored all six byte-for-byte. On a machine with an
  unsupported clipboard format, the runner skips that case before writing.

## Pending manual UI testing

- All dialog visible and selected-path behavior: `showMessage`, `pickFile`,
  `pickFiles`, `pickDir`, `pickDirs`, and `saveFile`. The temporary automated
  dialog driver passed cancellation and two Open-path trials but failed to
  select the Save As filename. None of these preliminary trials is counted as
  complete dialog coverage.
- Print preview appearance and cancellation (`webview.print`). A disposable
  `build_print.ps1` harness is available; the final UI assessment is left to
  the user. It never requests a print job.
- DevTools window visibility, window focus/fullscreen/cursor/drag/resize and
  taskbar appearance, tray update appearance, overlay icon/taskbar tab, IME,
  redraw, and dead-key behavior. See [INTERACTIVE.md](INTERACTIVE.md) for the
  individual actions and restoration steps.
- Native menu click and global shortcut key events need `build.ps1 -Interactive`
  and actual input. The default `build.ps1` checks registration and related
  state, not these physical interactions.

The Windows backend of Wry 0.57 reports `false` for `isDevtoolsOpen()` and
implements `closeDevtools()` as a no-op; those methods cannot establish actual
window visibility on this backend.
