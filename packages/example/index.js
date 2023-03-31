
(function () {
  Niva.api.window.current().then(id => console.log(`Current window id: ${id}`));


  const { window, monitor, webview, extra } = Niva.api;

  webview.openDevtools();

  async function setWindowToCurrentMonitorCenter() {
    const cursorPosition = await window.cursorPosition();
    const currentMonitor = await monitor.fromPoint(cursorPosition.x, cursorPosition.y)
      || await monitor.current();
    const { position, size, } = currentMonitor;

    const outerSize = await window.outerSize();
    await window.setOuterPosition({
      x: position.x + (size.width / 2) - (outerSize.width / 2),
      y: position.y + (size.height / 2) - (outerSize.height / 2),
    });
  }

  let lastActiveProcessId = null;

  async function recordActiveProcess() {
    lastActiveProcessId = await extra.getActiveWindowId();
  }

  function backToLastActiveProcess() {
    if (lastActiveProcessId) {
      extra.focusByWindowId(lastActiveProcessId);
    }
  }

  function showWindow() {
    window.setFocus();
    window.setVisible(true);
    setTimeout(() => {
      document.getElementById('testinput').focus()
    }, 300)
  }

  function hideWindow() {
    window.setVisible(false);
  }

  Niva.addEventListener('shortcut.emit', (_, id) => {
    if (id === 1) {
      (async () => {

        // const currentActiveWindow = await Niva.api.extra.getActiveWindow();
        // console.log("currentActiveWindow:", currentActiveWindow);
        const [isVisible, isFocused] = await Promise.all([window.isVisible(), window.isFocused()]);

        if (isVisible && isFocused) {
          hideWindow();
          backToLastActiveProcess();
        } else if (isVisible && !isFocused) {
          await recordActiveProcess();
          await setWindowToCurrentMonitorCenter();
          showWindow();
        } else {
          await recordActiveProcess();
          await setWindowToCurrentMonitorCenter();
          showWindow();
        }
      })();
    }
  });

})();
