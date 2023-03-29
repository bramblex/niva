
(function () {
  console.log('Hello World!')
  Niva.api.window.current().then(id => console.log(`Current window id: ${id}`));
  Niva.addEventListener('*', console.log);

  const { window, monitor, webview } = Niva.api;

  Niva.addEventListener('shortcut.emit', (_, id) => {
    if (id === 1) {
      (async () => {
        const cursorPosition = await window.cursorPosition();
        console.log("cursor position:", cursorPosition);

        const currentMonitor = await monitor.fromPoint(cursorPosition.x, cursorPosition.y)
          || await monitor.current();
        console.log("current monitor:", currentMonitor);

        const {
          position,
          size,
        } = currentMonitor;

        const outerSize = await window.outerSize();

        await window.setOuterPosition({
          x: position.x + (size.width / 2) - (outerSize.width / 2),
          y: position.y + (size.height / 2) - (outerSize.height / 2),
        });


        const [isVisible, isFocused] = await Promise.all([window.isVisible(), window.isFocused()]);
        if (isVisible && isFocused) {
          window.setVisible(false);
          setTimeout(() => {
            document.getElementById('testinput').focus()
          }, 300)
        } else if (isVisible && !isFocused) {
          window.setFocus();
        } else {
          window.setVisible(true);
          window.setFocus();
          setTimeout(() => {
            document.getElementById('testinput').focus()
          }, 300)
        }
      })();
    }
  });

})();
