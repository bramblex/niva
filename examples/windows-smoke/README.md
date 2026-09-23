# Windows real-machine smoke test

From the repository root, run `npm ci` once, then:

```powershell
& .\examples\windows-smoke\build.ps1
```

The script builds packaged Windows apps and checks the WebView2 page origin,
native calls from the main page and a same-origin iframe, packaged resources,
NodeCompat (`path`, `url`, `fs/promises`, `assert/strict`), file-token HTTP behavior,
cross-origin iframe denial, a strict CSP page, 150 KB fs/resource streams,
`process.exec` stdout/stderr, and 73 behavior checks for reversible
clipboard, monitor, window, webview, tray, shortcut, and Windows windowExtra
methods. Windows-only enablement, RTL, taskbar/window icons, topmost, and
content protection are independently inspected through Win32. Disposable
child-page navigation covers `loadUrl`, back/forward history, reload, and
`loadHtml`; a random one-use profile tests `clearAllBrowsingData`. Clipboard
read compares a digest without printing content. Clipboard write runs only
when every existing format can be snapshotted, then restores all formats
byte-for-byte; unsupported clipboard data causes a safe skip. The app
cleans up its cookie, tray icons, shortcuts, child window, and generated file.
The original smoke also checks menu show/hide and the Win32 owner of a child
window. For the native menu click and global shortcut check,
run `& .\examples\windows-smoke\build.ps1 -Interactive`; when it prints
`WAITING_FOR_UI`, click **Smoke → Ping** in the test window and press
**Ctrl+Alt+Shift+F12**. The process reports both event IDs and exits.

To verify `webview.print`, run
`& .\examples\windows-smoke\build_print.ps1` after the default build. Check
that the print preview contains **Niva print preview marker**, click
**Cancel** (never Print), then press Enter in the terminal. The runner checks
the page bridge and print call after cancellation and removes its random
one-use profile.

The current results and cases reserved for manual UI testing are recorded in
[TEST_REPORT.md](TEST_REPORT.md). The generated executables, staging files, and stderr logs stay under ignored
`dist/`. The test is a focused smoke check; it does not cover the full
permission or CSP matrix, signing, or every native API. Methods requiring a
dialog, printer, or system UI change are listed separately in
[INTERACTIVE.md](INTERACTIVE.md) with restoration steps.
