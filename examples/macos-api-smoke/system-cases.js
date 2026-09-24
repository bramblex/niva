// Remaining Niva API cases for the disposable macOS smoke app.
// Automatic cases are runnable with only a temporary app/profile; supervised
// cases describe a concrete native UI observation and its cleanup.
(function () {
  "use strict";

  const automatic = [];
  const supervised = [];
  const add = (method, assertion, run) => automatic.push({ method, assertion, run });
  const ui = (method, action, expected, restore) => supervised.push({ method, action, expected, restore });
  const expect = (condition, message) => { if (!condition) throw new Error(message); };
  async function waitFor(read, expected, label) {
    for (let attempt = 0; attempt < 40; attempt += 1) {
      if (await read() === expected) return;
      await new Promise((resolve) => setTimeout(resolve, 50));
    }
    throw new Error(`${label} did not become ${expected}`);
  }

  add("process.env", "returns a string-keyed environment without printing its values", async ({ api }) => {
    const env = await api.process.env();
    expect(env && typeof env === "object" && !Array.isArray(env), "process.env did not return object");
    expect(Object.entries(env).every(([name, value]) => typeof name === "string" && typeof value === "string"), "process.env contained non-string fields");
  });
  add("process.setCurrentDir", "changes only the disposable child cwd then restores it", async ({ api, tempRoot }) => {
    const before = await api.process.currentDir();
    try {
      await api.process.setCurrentDir(tempRoot);
      expect((await api.process.currentDir()) === tempRoot, "setCurrentDir did not change cwd");
    } finally { await api.process.setCurrentDir(before); }
  });
  add("process.exec", "runs a harmless fixed command and checks its stdout/status", async ({ api }) => {
    const result = await api.process.exec("/usr/bin/printf", ["niva-exec-smoke"]);
    expect(result.status === 0 && result.stdout === "niva-exec-smoke" && result.stderr === "", "process.exec output mismatch");
  });
  add("resource.extract", "extracts an isolated resource to a temp file and removes it", async ({ api, tempRoot }) => {
    const target = `${tempRoot}/extracted-resource.txt`;
    try {
      await api.resource.extract("probe.txt", target);
      expect((await api.fs.read(target)) === "niva-smoke-only\n", "resource.extract bytes differ");
    } finally { await api.fs.remove(target).catch(() => {}); }
  });

  add("extra.getActiveWindowId", "returns a macOS process/window identity for the smoke app", async ({ api }) => {
    const value = await api.extra.getActiveWindowId();
    expect(typeof value === "string" && /^\d+_\d+$/.test(value), "active-window ID did not use process_window format");
  });
  add("extra.focusByWindowId", "focuses this app's own native window ID", async ({ api }) => {
    const id = await api.extra.getActiveWindowId();
    expect(typeof id === "string" && /^\d+_\d+$/.test(id), "no valid owned window ID");
    const result = await api.extra.focusByWindowId(id);
    expect(result === true || result === undefined, "focusByWindowId rejected the owned window");
  });

  add("webview.setCookie", "stores a uniquely named cookie in the disposable profile", async ({ api, cookieName }) => {
    await api.webview.setCookie(`${cookieName}=value; Domain=localhost; Path=/; Max-Age=60`);
    expect((await api.webview.cookies()).some((cookie) => cookie.includes(`${cookieName}=value`)), "cookie not stored");
  });
  add("webview.cookies", "reads the smoke cookie from the shared disposable WebContext", async ({ api, cookieName }) => {
    expect((await api.webview.cookies()).some((cookie) => cookie.includes(`${cookieName}=value`)), "cookies() omitted smoke cookie");
  });
  add("webview.cookiesForUrl", "queries the smoke cookie for the fixture URL", async ({ api, cookieName }) => {
    const url = (await api.webview.baseUrl()).replace("127.0.0.1", "localhost");
    expect((await api.webview.cookiesForUrl(url)).some((cookie) => cookie.includes(`${cookieName}=value`)), "cookiesForUrl omitted smoke cookie");
  });
  add("webview.deleteCookie", "deletes only the uniquely named smoke cookie", async ({ api, cookieName }) => {
    await api.webview.deleteCookie(`${cookieName}=; Domain=localhost; Path=/`);
    expect(!(await api.webview.cookies()).some((cookie) => cookie.includes(`${cookieName}=`)), "smoke cookie remained");
  });
  add("webview.clearAllBrowsingData", "clears a unique cookie in the throwaway app profile", async ({ api, cookieName }) => {
    await api.webview.setCookie(`${cookieName}=clear-me; Domain=localhost; Path=/; Max-Age=60`);
    expect((await api.webview.cookies()).some((cookie) => cookie.includes(`${cookieName}=clear-me`)), "clear precondition missing");
    await api.webview.clearAllBrowsingData();
    expect(!(await api.webview.cookies()).some((cookie) => cookie.includes(`${cookieName}=`)), "clearAllBrowsingData left smoke cookie");
  });
  add("webview.isDevtoolsOpen", "reads the Devtools state before and after open/close", async ({ api }) => {
    expect(typeof await api.webview.isDevtoolsOpen() === "boolean", "Devtools state was not boolean");
  });
  add("webview.openDevtools", "opens Devtools in the disposable app", async ({ api }) => {
    await api.webview.openDevtools();
    await waitFor(() => api.webview.isDevtoolsOpen(), true, "Devtools");
  });
  add("webview.closeDevtools", "closes Devtools opened by the smoke app", async ({ api }) => {
    try {
      await api.webview.closeDevtools();
      await waitFor(() => api.webview.isDevtoolsOpen(), false, "Devtools close");
    } finally { await api.webview.closeDevtools().catch(() => {}); }
  });

  ui("dialog.pickFiles", "Select two temporary text files in the multi-file picker, then cancel a second picker", "both paths are within the disposable directory; cancelled call returns null", "close panel and remove only fixture files");
  ui("dialog.pickDir", "Select one temporary child directory, then cancel a second picker", "returned path is that child; cancelled call returns null", "close panel and remove fixture tree");
  ui("dialog.pickDirs", "Select two temporary child directories, then cancel a second picker", "both returned paths are within fixture root; cancelled call returns null", "close panel and remove fixture tree");
  ui("extra.hideApplication", "Hide only the disposable Niva app", "its window disappears while process remains active", "call showApplication and verify window returns");
  ui("extra.showApplication", "Show the disposable app after hideApplication", "its window reappears", "restore prior activation state and exit app");
  ui("extra.hideOtherApplications", "Snapshot visible apps, then hide them from the disposable Niva app", "only other app windows hide while Niva remains visible", "restore all previously visible apps");
  ui("extra.setActivationPolicy", "Switch the disposable app to accessory then regular", "Dock/activation state follows both policies", "restore regular before exiting");
  ui("process.open", "Open a disposable text file through the system opener", "default editor opens that exact temporary path", "close spawned editor window and delete only test file");
  ui("webview.loadHtml", "Navigate a disposable child WebView to HTML with a unique marker", "child DOM and URL reflect the requested document", "close the child");
  ui("webview.print", "Open print preview for a disposable marked page", "preview contains the marker and no job is submitted", "cancel the panel");

  window.NivaMacSystemCases = {
    automatic,
    supervised,
    async runAutomatic(api, record, tempRoot) {
      const cookieName = `niva_smoke_${Date.now()}`;
      try {
        for (const item of automatic) {
          await api.host.send("extended-progress", { method: item.method }).catch(() => {});
          await item.run({ api, tempRoot, cookieName });
          record(item.method, item.assertion);
        }
      } finally {
        await api.webview.deleteCookie(`${cookieName}=; Domain=localhost; Path=/`).catch(() => {});
        await api.webview.closeDevtools().catch(() => {});
      }
    },
  };
})();
