# Windows UI methods left for supervised testing

Use a disposable project profile and temporary files under `dist/`. Keep the
API smoke window in view, perform one operation at a time, and check both the
visible result and any returned error. These cases are intentionally outside
`build.ps1` because they need a human-visible OS dialog, affect shared state,
or have no reliable automatic getter.

| Methods | Action and observable result | Restore |
| --- | --- | --- |
| `clipboard.write` | First save all existing clipboard formats with a clipboard manager. Write a unique marker, then `clipboard.read` must return that marker. | Restore the saved clipboard contents, including non-text formats. The automatic test only reads and hashes clipboard text. |
| `webview.print` | Open the print UI on the disposable page and confirm its preview targets the page. Do not send a print job. | Cancel the print dialog. |
| `webview.openDevtools`, `closeDevtools` | Open DevTools and confirm `isDevtoolsOpen()` becomes true; close it and confirm false. | Close any remaining DevTools window. |
| `webview.loadUrl`, `loadHtml`, `reload`, `goBack`, `goForward`, `canGoBack`, `canGoForward` | Use a disposable child WebView with two local pages; verify URL, body marker, and history state after each navigation. | Close the child. Do not navigate the main stdio page, which owns the test harness. |
| `webview.clearAllBrowsingData` | In a throwaway Niva profile only, create a unique cookie, clear browsing data, then verify the cookie is absent. | Delete the throwaway profile after the test. Never run this against a normal Devtools profile. |
| `window.setFullscreen`, `setMaximized`, `setMinimized`, `setFocus`, `setVisible`, `setAlwaysOnTop`, `setAlwaysOnBottom` | Apply to a disposable child window and visually confirm each state or foreground order. | Restore the original state, then close the child. |
| `window.dragWindow`, `dragResizeWindow`, `setCursorGrab`, `setCursorPosition`, `setCursorVisible`, `setIgnoreCursorEvents`, `setCursorIcon` | With a disposable child window, perform the mouse gesture and observe actual movement, resize, cursor capture or cursor image. | Release cursor grab and ignored events, restore cursor position/visibility, then close the child. |
| `window.setWindowIcon`, `setBackgroundColor`, `setTheme`, `setProgressBar`, `setContentProtection`, `requestUserAttention` | Inspect the taskbar or window appearance on a disposable child after each call. For content protection, compare a screenshot with protection on and off. | Clear progress and content protection, restore original theme/icon/color, then close the child. |
| `tray.update` | Create a temporary tray icon, update its tooltip/menu, and inspect the changed tooltip/menu in the system tray. | Destroy the icon and verify `tray.list()` is empty. |
| `windowExtra.setTaskbarIcon`, `setOverlayIcon`, `setSkipTaskbar`, `setRtl`, `setEnable` | Apply one change to a disposable child and inspect the taskbar, RTL layout, or native enabled state. | Clear overlay, restore taskbar visibility/RTL/enabled state, then close the child. |
| `windowExtra.beginResizeDrag`, `resetDeadKeys` | Use a disposable child, a real pointer gesture, and a keyboard layout with dead keys to verify visible resize/typing behavior. | End the gesture and restore the prior keyboard layout. |
| Dialog and file-picker methods | Use only temporary files under `dist/`; check cancel and selected-path behavior separately. | Close every dialog and remove only the temporary files created for the test. |

Other native APIs outside these categories remain unverified by this Windows
suite. A successful method call alone is not counted as behavioral evidence.
