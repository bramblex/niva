# Windows real-machine smoke test

From the repository root, run `npm ci` once, then:

```powershell
& .\examples\windows-smoke\build.ps1
```

The script builds packaged Windows apps and checks the WebView2 page origin,
native calls from the main page and a same-origin iframe, packaged resources,
NodeCompat (`path`, `url`, `fs/promises`, `assert/strict`), file-token HTTP behavior,
cross-origin iframe denial, a strict CSP page, 150 KB fs/resource streams,
`process.exec` stdout/stderr, and more than 50 behavior checks for reversible
clipboard, monitor, window, webview, tray, shortcut, and Windows windowExtra
methods. Clipboard content is compared by digest without printing it; the app
cleans up its cookie, tray icons, shortcuts, child window, and generated file.
The original smoke also checks menu show/hide and the Win32 owner of a child
window. For the native menu click and global shortcut check,
run `& .\examples\windows-smoke\build.ps1 -Interactive`; when it prints
`WAITING_FOR_UI`, click **Smoke → Ping** in the test window and press
**Ctrl+Alt+Shift+F12**. The process reports both event IDs and exits.

The generated executables, staging files, and stderr logs stay under ignored
`dist/`. The test is a focused smoke check; it does not cover the full
permission or CSP matrix, signing, or every native API. Methods requiring a
dialog, printer, or system UI change are listed separately in
[INTERACTIVE.md](INTERACTIVE.md) with restoration steps.
