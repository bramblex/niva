# Windows UI methods reserved for manual testing

Use a disposable project profile and temporary files under `dist/`. Keep the
API smoke window in view, perform one operation at a time, and check both the
visible result and any returned error. These cases are intentionally outside
`build.ps1` because they need a human-visible OS dialog, affect shared state,
or have no reliable automatic getter.

These checks are pending user testing. The automatic results are in
[TEST_REPORT.md](TEST_REPORT.md). A trial of the dialog file-selection driver
did not complete the Save As path case, so no dialog path result is counted as
verified coverage.

| Methods | Action and observable result | Restore |
| --- | --- | --- |
| `clipboard.write` when the current clipboard contains non-HGLOBAL formats | The default script safely skips this case. Use a disposable clipboard or a manager that can preserve every present format, then write a marker and read it back. | Restore every original format and verify it. On this machine, the automatic conditional test snapshotted and restored all six available formats byte-for-byte. |
| `webview.print` | Run `build_print.ps1`, inspect the actual `edge://print/` preview and its page marker. Do not send a print job. | Click **Cancel**, then press Enter in the terminal; the runner verifies the bridge and print call returned and removes its one-use profile. |
| `webview.openDevtools`, `closeDevtools`, `isDevtoolsOpen` | Open DevTools and inspect the actual window. Wry 0.57's Windows backend currently returns `false` from `isDevtoolsOpen()` and implements `closeDevtools()` as a no-op, so those calls cannot verify visibility. Record this platform limitation rather than reporting a pass. | Close the DevTools window manually before ending the disposable app. |
| `window.setFullscreen`, `setFocus`, `isFocused`, `setAlwaysOnBottom` | Apply to a disposable child window and visually confirm each state or foreground order. | Restore the original state, then close the child. |
| `window.setMinInnerSize`, `setMaxInnerSize` | Drag a disposable child smaller/larger than each limit and compare the resulting `innerSize()` to the bound. | Clear each limit with `null`, restore the original size, then close the child. |
| `window.dragWindow`, `dragResizeWindow`, `cursorPosition`, `setCursorGrab`, `setCursorPosition`, `setCursorVisible`, `setIgnoreCursorEvents`, `setCursorIcon` | With a disposable child window, perform the mouse gesture and observe actual movement, resize, cursor capture or cursor image. | Release cursor grab and ignored events, restore cursor position/visibility, then close the child. |
| `window.setBackgroundColor`, `setProgressBar`, `requestUserAttention` | Inspect the taskbar or window appearance on a disposable child after each call. | Clear progress, restore the original color, then close the child. |
| `window.requestRedraw`, `setImePosition`, `setFocusable`, `blockCloseRequested` | Observe a repaint, IME candidate location, focus refusal, or intercepted close request on a disposable child. | Restore focusability and close handling, then close the child through the main window. |
| `tray.update` | Create a temporary tray icon, update its tooltip/menu, and inspect the changed tooltip/menu in the system tray. | Destroy the icon and verify `tray.list()` is empty. |
| `windowExtra.setOverlayIcon`, `setSkipTaskbar` | Apply one change to a disposable child and inspect the taskbar overlay or taskbar tab. | Clear overlay, restore taskbar visibility, then close the child. |
| `windowExtra.beginResizeDrag`, `resetDeadKeys` | Use a disposable child, a real pointer gesture, and a keyboard layout with dead keys to verify visible resize/typing behavior. | End the gesture and restore the prior keyboard layout. |
| `dialog.showMessage`, `pickFile`, `pickFiles`, `pickDir`, `pickDirs`, `saveFile` | Use only temporary files and folders under `dist/`; check cancel and selected-path behavior separately. Never confirm a save to an existing user file. | Close every dialog and remove only the temporary files created for the test. |

The automatic suite already verifies `webview.loadUrl/loadHtml/reload/goBack/goForward`
on a disposable child, and clears a cookie in a randomly generated profile
before deleting that profile. It also asserts native enabled/RTL/topmost/display-affinity
and window icon states through Win32, then restores the disposable child.
It also toggles and restores minimization, maximization, theme, visibility,
window decoration, menu, and global shortcut registration. This manual list
is for the remaining UI-specific outcomes.

Other native APIs outside these categories remain unverified by this Windows
suite. A successful method call alone is not counted as behavioral evidence.
