const report = async data => Niva.api.window.sendMessage(JSON.stringify(data), 0);

Niva.addEventListener("window.message", async (_event, payload) => {
  if (payload.message === "ping") {
    await Niva.api.window.sendMessage("pong", 0);
    return;
  }
  try {
    const command = JSON.parse(payload.message);
    const webview = Niva.api.webview;
    if (command.action === "loadUrl") await webview.loadUrl(command.url);
    if (command.action === "goBack") await webview.goBack();
    if (command.action === "goForward") await webview.goForward();
    if (command.action === "reload") {
      sessionStorage.setItem("nivaReloadCount", String(Number(sessionStorage.getItem("nivaReloadCount") || 0) + 1));
      await webview.reload();
    }
    if (command.action === "loadHtml") await webview.loadHtml(command.html);
  } catch (error) {
    await report({ kind: "nav-error", message: String(error) });
  }
});

(async () => {
  if (location.pathname.endsWith("api-child.html")) await Niva.api.window.sendMessage("child-ready", 0);
  await report({
    kind: "nav-page",
    page: location.pathname.endsWith("nav-two.html") ? "two" : "one",
    url: await Niva.api.webview.url(),
    canBack: await Niva.api.webview.canGoBack(),
    canForward: await Niva.api.webview.canGoForward(),
    reloadCount: Number(sessionStorage.getItem("nivaReloadCount") || 0),
  });
})();
