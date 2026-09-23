(async () => {
  try {
    const webview = Niva.api.webview;
    const name = "niva_clear_smoke";
    await webview.setCookie(`${name}=present; Domain=niva.app; Path=/; Max-Age=120`);
    const before = (await webview.cookiesForUrl("http://niva.app/"))
      .some(cookie => cookie.includes(`${name}=present`));
    await webview.clearAllBrowsingData();
    let after = true;
    for (let i = 0; i < 30; i++) {
      after = (await webview.cookiesForUrl("http://niva.app/"))
        .some(cookie => cookie.includes(`${name}=`));
      if (!after) break;
      await new Promise(resolve => setTimeout(resolve, 50));
    }
    await Niva.api.host.send("clear-result", { before, after });
  } catch (error) {
    await Niva.api.host.send("clear-error", { message: String(error) });
  }
})();
