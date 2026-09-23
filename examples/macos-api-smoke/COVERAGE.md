# macOS public API coverage matrix

Source: `packages/types/Niva_zh.d.ts`; exactly 167 public `Niva.api` methods are listed. The fixture defines a case for every method. On 2026-09-24 the sequential real-macOS run exercised **167/167** methods: **161** have a behavior assertion or external native observation, and **6** have a successful native invocation with their effect explicitly unobserved. Exact per-suite logs are saved by `run_all.py`; case definitions alone never count as a pass. Picker cancellation paths passed; selecting files/folders remains a separate branch.

The 10 Windows-only handlers are registered under `cfg(target_os = "windows")`; their macOS cases assert bridge error code `-1` and `api not found`. The supervised suites use disposable windows, Finder, Dock and Accessibility/CoreGraphics observations, with cleanup after each run.

| Public method | macOS scenario and expected result | Fixture / case | Actual execution status |
|---|---|---|---|
| `clipboard.read` | Snapshot NSPasteboard; require text equals saved plainText or null; restore snapshot if unchanged. | optional run.py --clipboard | PASS real macOS: clipboard-shortcut |
| `clipboard.write` | Write unique sentinel and require native pasteboard plus API readback; restore original items if unchanged. | optional run.py --clipboard | PASS real macOS: clipboard-shortcut |
| `dialog.showMessage` | Show smoke-only info message; keep heartbeat active while open; dismiss and require void resolution. | optional run.py --dialogs; operator dismisses UI | PASS real macOS: dialogs |
| `dialog.pickFile` | Open in disposable folder containing probe.txt; select or cancel; require returned path stays inside folder or null. | optional run.py --dialogs; operator dismisses UI | PASS real macOS: dialogs |
| `dialog.pickFiles` | Select two temporary text files in the multi-file picker, then cancel a second picker; expect both paths are within the disposable directory; cancelled call returns null; cleanup: close panel and remove only fixture files | system-cases.js / supervised | PASS real macOS: dialogs |
| `dialog.pickDir` | Select one temporary child directory, then cancel a second picker; expect returned path is that child; cancelled call returns null; cleanup: close panel and remove fixture tree | system-cases.js / supervised | PASS real macOS: dialogs |
| `dialog.pickDirs` | Select two temporary child directories, then cancel a second picker; expect both returned paths are within fixture root; cancelled call returns null; cleanup: close panel and remove fixture tree | system-cases.js / supervised | PASS real macOS: dialogs |
| `dialog.saveFile` | Open save panel in disposable folder; choose new path or cancel; require path stays inside; remove created file. | optional run.py --dialogs; operator dismisses UI | PASS real macOS: dialogs |
| `extra.hideApplication` | Hide only the disposable Niva app; expect its window disappears while process remains active; cleanup: call showApplication and verify window returns | system-cases.js / supervised | PASS real macOS: system-supervised |
| `extra.showApplication` | Show the disposable app after hideApplication; expect its window reappears; cleanup: restore prior activation state and exit app | system-cases.js / supervised | PASS real macOS: system-supervised |
| `extra.hideOtherApplications` | Snapshot visible apps, then hide them from the disposable Niva app; expect only other app windows hide while Niva remains visible; cleanup: restore all previously visible apps | system-cases.js / supervised | PASS real macOS: system-supervised |
| `extra.setActivationPolicy` | Switch the disposable app to accessory then regular; expect Dock/activation state follows both policies; cleanup: restore regular before exiting | system-cases.js / supervised | PASS real macOS: system-supervised |
| `extra.getActiveWindowId` | returns a macOS process/window identity for the smoke app | system-cases.js / automatic | PASS real macOS: extended |
| `extra.focusByWindowId` | focuses this app's own native window ID | system-cases.js / automatic | PASS real macOS: extended |
| `fs.stat` | Require file flag and exact byte size; require fixture paths report directory flags. | headless-safe run.py / headless.html | PASS real macOS: headless |
| `fs.exists` | Check absent before creation, present after creation/move, then absent after removal. | headless-safe run.py / headless.html | PASS real macOS: headless |
| `fs.read` | Require exact UTF-8 bytes after write and append. | headless-safe run.py / headless.html | PASS real macOS: headless |
| `fs.write` | Write unique text under run-scoped temporary root and verify by readback. | headless-safe run.py / headless.html | PASS real macOS: headless |
| `fs.append` | Append a second line and require both original and appended content. | headless-safe run.py / headless.html | PASS real macOS: headless |
| `fs.move` | Move copied file into nested directory; require source absent and destination bytes preserved. | headless-safe run.py / headless.html | PASS real macOS: headless |
| `fs.copy` | Copy fixture and require destination bytes equal source. | headless-safe run.py / headless.html | PASS real macOS: headless |
| `fs.remove` | Remove fixture files and require exists false. | headless-safe run.py / headless.html | PASS real macOS: headless |
| `fs.createDir` | Create direct child directories and verify via exists. | headless-safe run.py / headless.html | PASS real macOS: headless |
| `fs.createDirAll` | Create nested path and require leaf exists. | headless-safe run.py / headless.html | PASS real macOS: headless |
| `fs.readDir` | Require direct entries include sample.txt and exclude nested moved.txt. | headless-safe run.py / headless.html | PASS real macOS: headless |
| `fs.readDirAll` | Require recursive paths simple/sample.txt and nested/a/b/moved.txt. | headless-safe run.py / headless.html | PASS real macOS: headless |
| `host.send` | Send unique nonce through stdio and require exact nonce echoed by harness. | default or headless-safe run.py stdio echo | PASS real macOS: default |
| `http.request` | sends an explicit GET and preserves status/body | system-cases.js / automatic | PASS real macOS: extended |
| `http.get` | GETs a local fixture and returns its exact text | system-cases.js / automatic | PASS real macOS: extended |
| `http.post` | returns the local server's explicit non-GET status | system-cases.js / automatic | PASS real macOS: extended |
| `monitor.list` | Require a nonempty display list; each display has name, positive logical and physical dimensions, and positive scale factor. | default run.py / index.html | PASS real macOS: default |
| `monitor.current` | Require valid display geometry for the visible temporary smoke window. | default run.py / index.html | PASS real macOS: default |
| `monitor.primary` | Require valid geometry for the primary display. | default run.py / index.html | PASS real macOS: default |
| `monitor.fromPoint` | Query the primary display center and require the same display name. | default run.py / index.html | PASS real macOS: default |
| `os.info` | Require macOS OS name and nonempty architecture/version strings. | headless-safe run.py / headless.html | PASS real macOS: headless |
| `os.dirs` | Require temp path under current run and nonempty app data path. | headless-safe run.py / headless.html | PASS real macOS: headless |
| `os.sep` | Require slash path separator. | headless-safe run.py / headless.html | PASS real macOS: headless |
| `os.eol` | Require newline line ending. | headless-safe run.py / headless.html | PASS real macOS: headless |
| `os.locale` | Require nonempty system locale. | headless-safe run.py / headless.html | PASS real macOS: headless |
| `process.pid` | Require positive integer equal to spawned child PID. | headless-safe run.py / headless.html | PASS real macOS: headless |
| `process.currentDir` | Require runner-selected repository working directory. | headless-safe run.py / headless.html | PASS real macOS: headless |
| `process.currentExe` | Require resolved spawned Niva binary path. | headless-safe run.py / headless.html | PASS real macOS: headless |
| `process.env` | returns a string-keyed environment without printing its values | system-cases.js / automatic | PASS real macOS: extended |
| `process.args` | Require --stdio and this run’s isolated config/resource paths. | headless-safe run.py / headless.html | PASS real macOS: headless |
| `process.setCurrentDir` | changes only the disposable child cwd then restores it | system-cases.js / automatic | PASS real macOS: extended |
| `process.exit` | After temp file cleanup, request exit and require child status 0. | headless-safe run.py / headless.html | PASS real macOS: default |
| `process.exec` | runs a harmless fixed command and checks its stdout/status | system-cases.js / automatic | PASS real macOS: extended |
| `process.open` | Open a disposable text file through the system opener; expect default editor opens that exact temporary path; cleanup: close spawned editor window and delete only test file | system-cases.js / supervised | PASS real macOS: system-supervised |
| `process.version` | Require package version from crates/niva/Cargo.toml. | headless-safe run.py / headless.html | PASS real macOS: headless |
| `resource.exists` | Require true for headless-resource.txt in isolated debug resources. | headless-safe run.py / headless.html | PASS real macOS: headless |
| `resource.read` | Require exact UTF-8 text headless-resource-ok plus newline. | headless-safe run.py / headless.html | PASS real macOS: headless |
| `resource.extract` | extracts an isolated resource to a temp file and removes it | system-cases.js / automatic | PASS real macOS: extended |
| `shortcut.register` | Register rare four-modifier F19 chord; require returned id in list and unregister in cleanup. | optional run.py --shortcut | PASS real macOS: clipboard-shortcut |
| `shortcut.unregister` | Unregister one owned chord and require id disappears from list. | optional run.py --shortcut | PASS real macOS: clipboard-shortcut |
| `shortcut.unregisterAll` | Register two owned chords, unregisterAll, require list empty. | optional run.py --shortcut | PASS real macOS: clipboard-shortcut |
| `shortcut.list` | Read id/key then confirm entries disappear after unregister. | optional run.py --shortcut | PASS real macOS: clipboard-shortcut |
| `tray.create` | Create bundled tray.png item with unique title; require id in list; destroy in finally. | default run.py / tray command | PASS real macOS: default |
| `tray.destroy` | Destroy created item and require id absent from list. | default run.py / tray command | PASS real macOS: default |
| `tray.destroyAll` | Create two test items; call destroyAll and require empty list. | default run.py / tray command | PASS real macOS: default |
| `tray.list` | Require temporary item id present after create and absent after destroy. | default run.py / tray command | PASS real macOS: default |
| `tray.update` | Change title/tooltip of temporary item; require resolve and same id remains listed. | default run.py / tray command | PASS real macOS: default |
| `webview.evaluateScript` | Evaluate script reading fixture h1; require result and host nonce acknowledgement. | default run.py / index.html and secondary.html | PASS real macOS: default |
| `webview.loadUrl` | Load fixture secondary.html; require its URL and back-history entry. | default run.py / index.html and secondary.html | PASS real macOS: default |
| `webview.loadHtml` | Navigate a disposable child WebView to HTML with a unique marker; expect child DOM and URL reflect the requested document; cleanup: close the child | system-cases.js / supervised | PASS real macOS: system-supervised |
| `webview.reload` | Reload index.html and require same URL and retained forward history. | default run.py / index.html and secondary.html | PASS real macOS: default |
| `webview.url` | Require index.html initially and secondary.html after navigation. | default run.py / index.html and secondary.html | PASS real macOS: default |
| `webview.print` | Open print preview for a disposable marked page; expect preview contains the marker and no job is submitted; cleanup: cancel the panel | system-cases.js / supervised | PASS real macOS: system-supervised |
| `webview.goBack` | Return to index.html and require forward history. | default run.py / index.html and secondary.html | PASS real macOS: default |
| `webview.goForward` | Return to secondary.html and require back history. | default run.py / index.html and secondary.html | PASS real macOS: default |
| `webview.canGoBack` | Require back-history state on secondary page. | default run.py / index.html and secondary.html | PASS real macOS: default |
| `webview.canGoForward` | Require forward history after back navigation. | default run.py / index.html and secondary.html | PASS real macOS: default |
| `webview.cookies` | reads the smoke cookie from the shared disposable WebContext | system-cases.js / automatic | PASS real macOS: extended |
| `webview.cookiesForUrl` | queries the smoke cookie for the fixture URL | system-cases.js / automatic | PASS real macOS: extended |
| `webview.setCookie` | stores a uniquely named cookie in the disposable profile | system-cases.js / automatic | PASS real macOS: extended |
| `webview.deleteCookie` | deletes only the uniquely named smoke cookie | system-cases.js / automatic | PASS real macOS: extended |
| `webview.clearAllBrowsingData` | clears a unique cookie in the throwaway app profile | system-cases.js / automatic | PASS real macOS: extended |
| `webview.isDevtoolsOpen` | reads the Devtools state before and after open/close | system-cases.js / automatic | PASS real macOS: extended |
| `webview.openDevtools` | opens Devtools in the disposable app | system-cases.js / automatic | PASS real macOS: extended |
| `webview.closeDevtools` | closes Devtools opened by the smoke app | system-cases.js / automatic | PASS real macOS: extended |
| `webview.baseUrl` | Require isolated loopback server root URL. | default run.py / index.html and secondary.html | PASS real macOS: default |
| `webview.baseFileSystemUrl` | Require loopback __niva_fs URL containing per-window token segment. | default run.py / index.html and secondary.html | PASS real macOS: default |
| `window.current` | Require main window id 0. | default run.py / index.html | PASS real macOS: default |
| `window.open` | Open bundled child.html as a temporary window; require new id and visible list row. | default run.py / index.html | PASS real macOS: default |
| `window.close` | Close the temporary child window and require its id absent from list. | default run.py / index.html | PASS real macOS: default |
| `window.list` | Require visible main-window row with id 0 and configured title. | default run.py / index.html | PASS real macOS: default |
| `window.sendMessage` | Send a unique nonce from the main window to a disposable child; expect child echoes nonce and source window ID; cleanup: close the child | window-cases.js / supervised | PASS real macOS: window-supervised |
| `window.setMenu` | attaches a disposable menu to the child window | window-cases.js / automatic | PASS real macOS: extended |
| `window.hideMenu` | hides the disposable menu and reads it back | window-cases.js / automatic | PASS real macOS: extended |
| `window.showMenu` | restores the disposable menu and reads it back | window-cases.js / automatic | PASS real macOS: extended |
| `window.isMenuVisible` | reads the menu state after attach/hide/show | window-cases.js / automatic | PASS real macOS: extended |
| `window.scaleFactor` | Require finite positive scale factor. | default run.py / index.html | PASS real macOS: default |
| `window.innerPosition` | Require finite screen x/y coordinates. | default run.py / index.html | PASS real macOS: default |
| `window.outerPosition` | Require finite screen x/y coordinates. | default run.py / index.html | PASS real macOS: default |
| `window.setOuterPosition` | moves a disposable child and restores its original position | window-cases.js / automatic | PASS real macOS: extended |
| `window.innerSize` | Require positive content width and height. | default run.py / index.html | PASS real macOS: default |
| `window.setInnerSize` | Set distinct valid size, verify with innerSize(), restore original size. | default run.py / index.html | PASS real macOS: default |
| `window.outerSize` | Require outer dimensions at least as large as inner dimensions. | default run.py / index.html | PASS real macOS: default |
| `window.setMinInnerSize` | enforces then clears a minimum child size | window-cases.js / automatic | PASS real macOS: extended |
| `window.setMaxInnerSize` | enforces then clears a maximum child size | window-cases.js / automatic | PASS real macOS: extended |
| `window.setWindowIcon` | rejects the unsupported macOS window-icon operation | window-cases.js / automatic | PASS real macOS: extended |
| `window.setTheme` | changes the disposable app theme and restores system choice | window-cases.js / automatic | PASS real macOS: extended |
| `window.dragResizeWindow` | rejects edge resize drag when macOS Tao does not support it | window-cases.js / automatic | PASS real macOS: extended |
| `window.setProgressBar` | Set a temporary Dock progress value; expect Dock progress reflects value; cleanup: clear progress and close child | window-cases.js / supervised | PASS real macOS: system-supervised |
| `window.requestRedraw` | Request redraw after changing child content marker; expect native redraw event or changed captured pixels; cleanup: close child | window-cases.js / supervised | INVOKED real macOS: window-supervised; effect not independently observed |
| `window.setImePosition` | Move IME candidate origin on a disposable text field; expect candidate location follows requested point; cleanup: restore prior input method/close child | window-cases.js / supervised | INVOKED real macOS: window-supervised; effect not independently observed |
| `window.setBackgroundColor` | Set a unique RGBA child background; expect captured child pixels show the color; cleanup: restore previous color or close child | window-cases.js / supervised | PASS real macOS: window-supervised |
| `window.setFocusable` | Make the child non-focusable and request focus; expect isFocused remains false; cleanup: restore focusable and close child | window-cases.js / supervised | PASS real macOS: window-supervised |
| `window.setTitle` | Set unique title, verify with title(), restore configured title. | default run.py / index.html | PASS real macOS: default |
| `window.title` | Require configured temporary title to be nonempty. | default run.py / index.html | PASS real macOS: default |
| `window.isVisible` | Read visible, hidden, and restored visibility states. | default run.py / index.html | PASS real macOS: default |
| `window.setVisible` | Hide and reveal temporary main window; read back false then true. | default run.py / index.html | PASS real macOS: default |
| `window.isFocused` | Require true after setFocus on the temporary main window. | default run.py / index.html | PASS real macOS: default |
| `window.setFocus` | Focus visible temporary main window and require isFocused true. | default run.py / index.html | PASS real macOS: default |
| `window.isResizable` | Read original flag, toggle it, read changed value, restore and verify. | default run.py / index.html | PASS real macOS: default |
| `window.setResizable` | Toggle resizable, verify with isResizable, restore original state. | default run.py / index.html | PASS real macOS: default |
| `window.isMinimizable` | reads the disposable child's isMinimizable state | window-cases.js / automatic | PASS real macOS: extended |
| `window.setMinimizable` | toggles isMinimizable, verifies readback, and restores it | window-cases.js / automatic | PASS real macOS: extended |
| `window.isMaximizable` | reads the disposable child's isMaximizable state | window-cases.js / automatic | PASS real macOS: extended |
| `window.setMaximizable` | toggles isMaximizable, verifies readback, and restores it | window-cases.js / automatic | PASS real macOS: extended |
| `window.isClosable` | reads the disposable child's isClosable state | window-cases.js / automatic | PASS real macOS: extended |
| `window.isMinimized` | reads the disposable child's isMinimized state | window-cases.js / automatic | PASS real macOS: extended |
| `window.setMinimized` | toggles isMinimized, verifies readback, and restores it | window-cases.js / automatic | PASS real macOS: extended |
| `window.isMaximized` | reads the disposable child's isMaximized state | window-cases.js / automatic | PASS real macOS: extended |
| `window.setMaximized` | toggles isMaximized, verifies readback, and restores it | window-cases.js / automatic | PASS real macOS: extended |
| `window.setClosable` | toggles isClosable, verifies readback, and restores it | window-cases.js / automatic | PASS real macOS: extended |
| `window.isDecorated` | reads the disposable child's isDecorated state | window-cases.js / automatic | PASS real macOS: extended |
| `window.setDecorated` | toggles isDecorated, verifies readback, and restores it | window-cases.js / automatic | PASS real macOS: extended |
| `window.fullscreen` | reads the disposable child's fullscreen state | window-cases.js / automatic | PASS real macOS: extended |
| `window.setFullscreen` | toggles disposable-child fullscreen and restores it | window-cases.js / automatic | PASS real macOS: extended |
| `window.setAlwaysOnTop` | Raise the child over a second smoke window; expect CGWindow layer/order changes as requested; cleanup: set false and close both children | window-cases.js / supervised | PASS real macOS: window-supervised |
| `window.setAlwaysOnBottom` | Place the child below a second smoke window; expect CGWindow layer/order changes as requested; cleanup: set false and close both children | window-cases.js / supervised | PASS real macOS: window-supervised |
| `window.requestUserAttention` | Request attention for the disposable child; expect Dock/notification attention cue appears; cleanup: focus child and close it | window-cases.js / supervised | INVOKED real macOS: window-supervised; effect not independently observed |
| `window.setContentProtection` | Enable content protection on a disposable child; expect screen capture excludes child content; cleanup: disable protection and close child | window-cases.js / supervised | PASS real macOS: window-supervised |
| `window.setVisibleOnAllWorkspaces` | Show child across a temporary second Space; expect child remains visible in both Spaces; cleanup: restore false and close child | window-cases.js / supervised | INVOKED real macOS: window-supervised; effect not independently observed |
| `window.setCursorIcon` | Set a distinct cursor above the child; expect captured cursor icon changes; cleanup: restore default icon and close child | window-cases.js / supervised | PASS real macOS: window-supervised |
| `window.cursorPosition` | Move pointer within the disposable child; expect API reports the observed pointer coordinate; cleanup: restore original pointer location | window-cases.js / supervised | PASS real macOS: window-supervised |
| `window.setCursorPosition` | Set pointer to a child-local point; expect cursorPosition reports that point within tolerance; cleanup: restore original pointer location | window-cases.js / supervised | PASS real macOS: window-supervised |
| `window.setCursorGrab` | Capture pointer in the child; expect pointer cannot leave as configured; cleanup: release grab and close child | window-cases.js / supervised | INVOKED real macOS: window-supervised; effect not independently observed |
| `window.setCursorVisible` | Hide pointer above the child; expect pointer is absent from visual capture; cleanup: restore visible and close child | window-cases.js / supervised | PASS real macOS: window-supervised |
| `window.dragWindow` | Press on a drag handle then invoke dragWindow; expect child outerPosition changes with pointer movement; cleanup: release pointer and restore position | window-cases.js / supervised | PASS real macOS: drag |
| `window.setIgnoreCursorEvents` | Ignore pointer events on child; expect click passes through to test window behind; cleanup: restore false and close child | window-cases.js / supervised | INVOKED real macOS: window-supervised; effect not independently observed |
| `window.theme` | returns a recognized native theme | window-cases.js / automatic | PASS real macOS: extended |
| `window.blockCloseRequested` | Block close on a disposable child and request close; expect window.closeRequested event fires while child remains; cleanup: set false and close child | window-cases.js / supervised | PASS real macOS: window-supervised |
| `windowExtra.setEnable` | On macOS call `Niva.api.windowExtra.setEnable(false, 0)`; require bridge rejection code -1 and message `api not found` because this handler is registered only for Windows. | window-cases.js / expected macOS error | PASS real macOS: extended |
| `windowExtra.setTaskbarIcon` | On macOS call `Niva.api.windowExtra.setTaskbarIcon("niva-smoke-unique", 0)`; require bridge rejection code -1 and message `api not found` because this handler is registered only for Windows. | window-cases.js / expected macOS error | PASS real macOS: extended |
| `windowExtra.theme` | On macOS call `Niva.api.windowExtra.theme(0)`; require bridge rejection code -1 and message `api not found` because this handler is registered only for Windows. | window-cases.js / expected macOS error | PASS real macOS: extended |
| `windowExtra.resetDeadKeys` | On macOS call `Niva.api.windowExtra.resetDeadKeys(0)`; require bridge rejection code -1 and message `api not found` because this handler is registered only for Windows. | window-cases.js / expected macOS error | PASS real macOS: extended |
| `windowExtra.beginResizeDrag` | On macOS call `Niva.api.windowExtra.beginResizeDrag(1, 1, 1, 1, 0)`; require bridge rejection code -1 and message `api not found` because this handler is registered only for Windows. | window-cases.js / expected macOS error | PASS real macOS: extended |
| `windowExtra.setSkipTaskbar` | On macOS call `Niva.api.windowExtra.setSkipTaskbar(false, 0)`; require bridge rejection code -1 and message `api not found` because this handler is registered only for Windows. | window-cases.js / expected macOS error | PASS real macOS: extended |
| `windowExtra.setUndecoratedShadow` | On macOS call `Niva.api.windowExtra.setUndecoratedShadow(false, 0)`; require bridge rejection code -1 and message `api not found` because this handler is registered only for Windows. | window-cases.js / expected macOS error | PASS real macOS: extended |
| `windowExtra.setOverlayIcon` | On macOS call `Niva.api.windowExtra.setOverlayIcon("niva-smoke-unique", 0)`; require bridge rejection code -1 and message `api not found` because this handler is registered only for Windows. | window-cases.js / expected macOS error | PASS real macOS: extended |
| `windowExtra.setRtl` | On macOS call `Niva.api.windowExtra.setRtl(false, 0)`; require bridge rejection code -1 and message `api not found` because this handler is registered only for Windows. | window-cases.js / expected macOS error | PASS real macOS: extended |
| `windowExtra.hasUndecoratedShadow` | On macOS call `Niva.api.windowExtra.hasUndecoratedShadow(0)`; require bridge rejection code -1 and message `api not found` because this handler is registered only for Windows. | window-cases.js / expected macOS error | PASS real macOS: extended |
| `windowExtra.simpleFullscreen` | reads simple fullscreen state | window-cases.js / automatic | PASS real macOS: extended |
| `windowExtra.setSimpleFullscreen` | toggles simple fullscreen and restores it | window-cases.js / automatic | PASS real macOS: extended |
| `windowExtra.hasShadow` | reads the child window's shadow state | window-cases.js / automatic | PASS real macOS: extended |
| `windowExtra.setHasShadow` | toggles native shadow and restores it | window-cases.js / automatic | PASS real macOS: extended |
| `windowExtra.setIsDocumentEdited` | toggles document-edited flag and restores it | window-cases.js / automatic | PASS real macOS: extended |
| `windowExtra.isDocumentEdited` | reads the disposable child document-edited flag | window-cases.js / automatic | PASS real macOS: extended |
| `windowExtra.setAllowsAutomaticWindowTabbing` | toggles automatic tabbing and restores it | window-cases.js / automatic | PASS real macOS: extended |
| `windowExtra.allowsAutomaticWindowTabbing` | reads automatic tabbing state | window-cases.js / automatic | PASS real macOS: extended |
| `windowExtra.setTabbingIdentifier` | sets a unique tab identifier and restores it | window-cases.js / automatic | PASS real macOS: extended |
| `windowExtra.tabbingIdentifier` | reads a string tabbing identifier | window-cases.js / automatic | PASS real macOS: extended |
| `windowExtra.setTrafficLightInset` | Offset child traffic light controls; expect button positions shift by the requested inset; cleanup: restore original inset or close child | window-cases.js / supervised | PASS real macOS: window-supervised |
| `windowExtra.setActivationPolicyAtRuntime` | Switch disposable app to accessory then regular; expect Dock/activation state follows each policy; cleanup: restore regular and exit app | window-cases.js / supervised | PASS real macOS: system-supervised |
| `windowExtra.setDockVisibility` | Hide then show disposable app Dock icon; expect Dock visibility changes; cleanup: restore visible and exit app | window-cases.js / supervised | PASS real macOS: system-supervised |
| `windowExtra.setBadgeLabel` | Set a unique disposable Dock badge; expect badge text appears; cleanup: clear badge and exit app | window-cases.js / supervised | PASS real macOS: system-supervised |

## Fixture and cleanup notes

- Default suite: `run.py` automatic registry, `run_webview_history`, `run_tray` and `index.html`.
- Headless-safe suite: `run.py --headless-safe`; exact methods are registered in `HEADLESS_PAGE_METHODS` and invoked in `headless.html`.
- Optional clipboard suite snapshots all NSPasteboard items and conditionally restores only if unchanged.
- Optional shortcut suite registers and removes rare F-key chords; it does not synthesize keypresses.
- `--dialogs-auto` opens and cancels all native panels; `--dialogs` also allows selecting fixtures from the disposable directory.
- `process.open` opens a disposable directory in Finder; the runner checks the exact target and closes that Finder window.
- Window/app mutations must run in this temporary app instance. Capture original state and restore it in `finally`; close child windows, remove temporary files, destroy tray items, and unregister shortcuts before exit.
