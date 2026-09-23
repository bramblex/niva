(async () => {
  const api = Niva.api;
  const checks = {};
  const expect = (value, message) => { if (!value) throw new Error(message); };
  const sleep = ms => new Promise(resolve => setTimeout(resolve, ms));
  const check = async (name, action) => {
    try { await action(); checks[name] = "pass"; }
    catch (error) { checks[name] = String(error); }
  };
  const sameMonitor = (a, b) => a && b && a.name === b.name &&
    a.physicalPosition.x === b.physicalPosition.x &&
    a.physicalPosition.y === b.physicalPosition.y &&
    a.physicalSize.width === b.physicalSize.width &&
    a.physicalSize.height === b.physicalSize.height;
  const fnv = value => {
    if (value === null) return null;
    let hash = 0x811c9dc5;
    for (const byte of new TextEncoder().encode(value)) {
      hash = Math.imul(hash ^ byte, 0x01000193) >>> 0;
    }
    return hash.toString(16).padStart(8, "0");
  };
  const nativeWaiters = new Map();
  Niva.addEventListener("host:message", (_event, message) => {
    if (message.name === "native-result") {
      nativeWaiters.get(message.data.name)?.(message.data);
      nativeWaiters.delete(message.data.name);
    }
  });
  const nativeState = async (name, expected) => {
    const response = new Promise((resolve, reject) => {
      nativeWaiters.set(name, resolve);
      setTimeout(() => {
        if (nativeWaiters.delete(name)) reject(new Error(`native ${name} probe timed out`));
      }, 3000);
    });
    await api.host.send("native-probe", { name, expected });
    const result = await response;
    expect(!result.error && result.value === expected, `native ${name}: ${JSON.stringify(result)}`);
  };

  let clipboardDigest = null;
  await check("clipboard.read", async () => {
    const value = await api.clipboard.read();
    expect(value === null || typeof value === "string", "unexpected clipboard type");
    clipboardDigest = fnv(value);
  });

  let monitors, primary;
  await check("monitor.list", async () => {
    monitors = await api.monitor.list();
    expect(Array.isArray(monitors) && monitors.length > 0, "no monitors");
    expect(monitors.every(m => m.physicalSize.width > 0 && m.physicalSize.height > 0 && m.scaleFactor > 0), "invalid monitor geometry");
  });
  await check("monitor.primary", async () => {
    primary = await api.monitor.primary();
    expect(monitors.some(m => sameMonitor(m, primary)), "primary not in list");
  });
  await check("monitor.current", async () => {
    const current = await api.monitor.current();
    expect(monitors.some(m => sameMonitor(m, current)), "current not in list");
  });
  await check("monitor.fromPoint", async () => {
    const x = primary.physicalPosition.x + Math.floor(primary.physicalSize.width / 2);
    const y = primary.physicalPosition.y + Math.floor(primary.physicalSize.height / 2);
    expect(sameMonitor(await api.monitor.fromPoint(x, y), primary), "center point did not return primary");
  });

  await check("webview.url", async () => {
    expect((await api.webview.url()).startsWith("http://niva.app/api-safe.html"), "unexpected page URL");
  });
  await check("webview.baseUrl", async () => {
    expect(/^http:\/\/127\.0\.0\.1:\d+\/$/.test(await api.webview.baseUrl()), "unexpected loopback base URL");
  });
  await check("webview.baseFileSystemUrl", async () => {
    const base = await api.webview.baseFileSystemUrl();
    const cwd = (await api.process.currentDir()).replaceAll("\\", "/");
    const response = await fetch(base + encodeURIComponent(cwd + "/examples/windows-smoke/probe.txt"));
    expect(response.status === 200 && (await response.text()).trim() === "resource-ok", "file-token fetch failed");
  });
  await check("webview.evaluateScript", async () => {
    await api.webview.evaluateScript("document.body.dataset.nivaApiProbe = 'evaluated'");
    for (let i = 0; i < 20 && document.body.dataset.nivaApiProbe !== "evaluated"; i++) await sleep(25);
    expect(document.body.dataset.nivaApiProbe === "evaluated", "script did not execute");
  });
  await check("webview.canGoBack", async () => {
    expect((await api.webview.canGoBack()) === false, "fresh page unexpectedly has back history");
  });
  await check("webview.canGoForward", async () => {
    expect((await api.webview.canGoForward()) === false, "fresh page unexpectedly has forward history");
  });
  const cookieName = "niva_api_smoke_" + Date.now();
  const cookie = `${cookieName}=verified; Domain=niva.app; Path=/; Max-Age=60`;
  try {
    await check("webview.setCookie", async () => {
      await api.webview.setCookie(cookie);
      expect((await api.webview.cookiesForUrl("http://niva.app/")).some(c => c.includes(cookieName + "=verified")), "cookie not stored");
    });
    await check("webview.cookiesForUrl", async () => {
      expect((await api.webview.cookiesForUrl("http://niva.app/")).some(c => c.includes(cookieName + "=verified")), "URL cookie missing");
    });
    await check("webview.cookies", async () => {
      expect((await api.webview.cookies()).some(c => c.includes(cookieName + "=verified")), "shared cookie missing");
    });
  } finally {
    await check("webview.deleteCookie", async () => {
      await api.webview.deleteCookie(`${cookieName}=; Domain=niva.app; Path=/`);
      expect(!(await api.webview.cookiesForUrl("http://niva.app/")).some(c => c.includes(cookieName + "=")), "cookie remained after delete");
    });
  }

  await check("window.current", async () => expect((await api.window.current()) === 0, "main window ID is not zero"));
  let childId = null;
  let childReady;
  let childReply;
  const ready = new Promise(resolve => { childReady = resolve; });
  const reply = new Promise(resolve => { childReply = resolve; });
  Niva.addEventListener("window.message", (_event, payload) => {
    if (payload.message === "child-ready") childReady(payload.from);
    if (payload.message === "pong") childReply(payload.from);
  });
  await check("window.open", async () => {
    childId = await api.window.open({ entry: "api-child.html", title: "Niva API child", visible: false, size: { width: 300, height: 220 }, ownerWindow: 0 });
    expect(Number.isInteger(childId) && childId > 0, "invalid child ID");
    expect((await Promise.race([ready, sleep(5000).then(() => -1)])) === childId, "child did not signal ready");
  });
  await check("window.list", async () => {
    expect((await api.window.list()).some(w => w.id === childId && w.visible === false), "hidden child not listed");
  });
  await check("window.sendMessage", async () => {
    await api.window.sendMessage("ping", childId);
    expect((await Promise.race([reply, sleep(5000).then(() => -1)])) === childId, "child did not reply");
  });
  await check("window.scaleFactor", async () => expect((await api.window.scaleFactor(childId)) > 0, "invalid scale factor"));
  await check("window.innerPosition", async () => {
    const p = await api.window.innerPosition(childId);
    expect(Number.isFinite(p.x) && Number.isFinite(p.y), "invalid inner position");
  });
  await check("window.outerPosition", async () => {
    const p = await api.window.outerPosition(childId);
    expect(Number.isFinite(p.x) && Number.isFinite(p.y), "invalid outer position");
  });
  await check("window.innerSize", async () => {
    const s = await api.window.innerSize(childId);
    expect(s.width > 0 && s.height > 0, "invalid inner size");
  });
  await check("window.outerSize", async () => {
    const s = await api.window.outerSize(childId);
    expect(s.width > 0 && s.height > 0, "invalid outer size");
  });
  await check("window.title", async () => expect((await api.window.title(childId)).length > 0, "empty child title"));
  await check("window.setTitle", async () => {
    const before = await api.window.title(childId);
    try {
      await api.window.setTitle("Niva API title probe", childId);
      expect((await api.window.title(childId)) === "Niva API title probe", "title did not change");
    } finally { await api.window.setTitle(before, childId); }
  });
  await check("window.setInnerSize", async () => {
    const before = await api.window.innerSize(childId);
    try {
      await api.window.setInnerSize({ width: before.width + 20, height: before.height + 10 }, childId);
      const after = await api.window.innerSize(childId);
      expect(Math.abs(after.width - before.width - 20) <= 3 && Math.abs(after.height - before.height - 10) <= 3, "inner size did not change");
    } finally { await api.window.setInnerSize(before, childId); }
  });
  await check("window.setOuterPosition", async () => {
    const before = await api.window.outerPosition(childId);
    try {
      await api.window.setOuterPosition({ x: before.x + 12, y: before.y + 12 }, childId);
      const after = await api.window.outerPosition(childId);
      expect(Math.abs(after.x - before.x - 12) <= 3 && Math.abs(after.y - before.y - 12) <= 3, "outer position did not change");
    } finally { await api.window.setOuterPosition(before, childId); }
  });
  for (const [setter, getter] of [
    ["setResizable", "isResizable"], ["setMinimizable", "isMinimizable"],
    ["setMaximizable", "isMaximizable"], ["setClosable", "isClosable"],
    ["setDecorated", "isDecorated"],
  ]) {
    await check("window." + setter, async () => {
      const before = await api.window[getter](childId);
      try {
        await api.window[setter](!before, childId);
        expect((await api.window[getter](childId)) === !before, `${getter} did not reflect toggle`);
      } finally { await api.window[setter](before, childId); }
    });
    await check("window." + getter, async () => expect(typeof await api.window[getter](childId) === "boolean", "invalid boolean response"));
  }
  await check("window.isVisible", async () => expect((await api.window.isVisible(childId)) === false, "child unexpectedly visible"));
  await check("window.setVisible", async () => {
    try {
      await api.window.setVisible(true, childId);
      expect((await api.window.isVisible(childId)) === true, "child not shown");
    } finally { await api.window.setVisible(false, childId); }
    expect((await api.window.isVisible(childId)) === false, "child not hidden again");
  });
  await check("window.isMinimized", async () => expect((await api.window.isMinimized(childId)) === false, "child unexpectedly minimized"));
  await check("window.setMinimized", async () => {
    try {
      await api.window.setMinimized(true, childId);
      expect((await api.window.isMinimized(childId)) === true, "child did not minimize");
    } finally { await api.window.setMinimized(false, childId); }
    expect((await api.window.isMinimized(childId)) === false, "child did not restore from minimized");
  });
  await check("window.isMaximized", async () => expect((await api.window.isMaximized(childId)) === false, "child unexpectedly maximized"));
  await check("window.setMaximized", async () => {
    try {
      await api.window.setMaximized(true, childId);
      expect((await api.window.isMaximized(childId)) === true, "child did not maximize");
    } finally { await api.window.setMaximized(false, childId); }
    expect((await api.window.isMaximized(childId)) === false, "child did not restore from maximized");
  });
  await check("window.theme", async () => expect(["light", "dark", "system"].includes(await api.window.theme(childId)), "invalid theme"));
  await check("window.setTheme", async () => {
    const before = await api.window.theme(childId);
    try {
      await api.window.setTheme("dark", childId);
      expect((await api.window.theme(childId)) === "dark", "child theme did not change");
    } finally { await api.window.setTheme(before, childId); }
  });
  await check("windowExtra.theme", async () => {
    expect((await api.windowExtra.theme(childId)) === (await api.window.theme(childId)), "windowExtra theme differs");
  });
  await check("windowExtra.hasUndecoratedShadow", async () => {
    expect(typeof await api.windowExtra.hasUndecoratedShadow(childId) === "boolean", "invalid shadow state");
  });
  await check("windowExtra.setUndecoratedShadow", async () => {
    const before = await api.windowExtra.hasUndecoratedShadow(childId);
    try {
      await api.windowExtra.setUndecoratedShadow(!before, childId);
      expect((await api.windowExtra.hasUndecoratedShadow(childId)) === !before, "shadow did not toggle");
    } finally { await api.windowExtra.setUndecoratedShadow(before, childId); }
  });
  await check("windowExtra.setEnable", async () => {
    try {
      await api.windowExtra.setEnable(false, childId);
      await nativeState("enabled", false);
    } finally { await api.windowExtra.setEnable(true, childId); }
    await nativeState("enabled", true);
  });
  await check("windowExtra.setRtl", async () => {
    try {
      await api.windowExtra.setRtl(true, childId);
      await nativeState("rtl", true);
    } finally { await api.windowExtra.setRtl(false, childId); }
    await nativeState("rtl", false);
  });
  await check("window.setAlwaysOnTop", async () => {
    try {
      await api.window.setAlwaysOnTop(true, childId);
      await nativeState("topmost", true);
    } finally { await api.window.setAlwaysOnTop(false, childId); }
    await nativeState("topmost", false);
  });
  await check("window.setContentProtection", async () => {
    try {
      await api.window.setContentProtection(true, childId);
      await nativeState("displayAffinity", 0x11);
    } finally { await api.window.setContentProtection(false, childId); }
    await nativeState("displayAffinity", 0);
  });
  await check("window.setWindowIcon", async () => {
    try {
      await api.window.setWindowIcon("icon.png", childId);
      await nativeState("iconSmall", true);
    } finally { await api.window.setWindowIcon(null, childId); }
    await nativeState("iconSmall", false);
  });
  await check("windowExtra.setTaskbarIcon", async () => {
    await api.windowExtra.setTaskbarIcon("icon.png", childId);
    await nativeState("iconBig", true);
  });
  await check("window.setMenu", async () => {
    await api.window.setMenu([{ label: "API", children: [{ type: "item", id: 8, label: "Probe" }] }], childId);
    expect((await api.window.isMenuVisible(childId)) === true, "menu not attached");
  });
  await check("window.hideMenu", async () => {
    await api.window.hideMenu(childId);
    expect((await api.window.isMenuVisible(childId)) === false, "menu remained visible");
  });
  await check("window.showMenu", async () => {
    await api.window.showMenu(childId);
    expect((await api.window.isMenuVisible(childId)) === true, "menu not restored");
  });
  await check("window.isMenuVisible", async () => expect((await api.window.isMenuVisible(childId)) === true, "menu visibility mismatch"));
  await api.window.setMenu(null, childId).catch(() => {});

  let trayId = null;
  try {
    await check("tray.create", async () => {
      trayId = await api.tray.create({ icon: "icon.png", tooltip: "Niva API smoke" });
      expect((await api.tray.list()).includes(trayId), "created tray icon missing");
    });
    await check("tray.list", async () => expect((await api.tray.list()).includes(trayId), "tray list missing ID"));
    await check("tray.destroy", async () => {
      await api.tray.destroy(trayId);
      expect(!(await api.tray.list()).includes(trayId), "tray icon remained after destroy");
      trayId = null;
    });
    await check("tray.destroyAll", async () => {
      const a = await api.tray.create({ icon: "icon.png", tooltip: "Niva API A" });
      const b = await api.tray.create({ icon: "icon.png", tooltip: "Niva API B" });
      expect((await api.tray.list()).includes(a) && (await api.tray.list()).includes(b), "tray icons not created");
      await api.tray.destroyAll();
      expect((await api.tray.list()).length === 0, "tray icons remained after destroyAll");
    });
  } finally { await api.tray.destroyAll().catch(() => {}); }

  let firstShortcut = null;
  try {
    await check("shortcut.register", async () => {
      firstShortcut = await api.shortcut.register("Ctrl+Alt+Shift+F10");
      const listed = await api.shortcut.list();
      expect(listed.some(s => s.id === firstShortcut), "registered shortcut missing: " + JSON.stringify(listed));
    });
    await check("shortcut.list", async () => {
      const listed = await api.shortcut.list();
      expect(listed.some(s => s.id === firstShortcut), "shortcut list missing ID: " + JSON.stringify(listed));
    });
    await check("shortcut.unregister", async () => {
      await api.shortcut.unregister(firstShortcut);
      expect(!(await api.shortcut.list()).some(s => s.id === firstShortcut), "shortcut remained after unregister");
      firstShortcut = null;
    });
    await check("shortcut.unregisterAll", async () => {
      const second = await api.shortcut.register("Ctrl+Alt+Shift+F11");
      const listed = await api.shortcut.list();
      expect(listed.some(s => s.id === second), "second shortcut missing: " + JSON.stringify(listed));
      await api.shortcut.unregisterAll();
      expect((await api.shortcut.list()).length === 0, "shortcuts remained after unregisterAll");
    });
  } finally { await api.shortcut.unregisterAll().catch(() => {}); }

  await check("window.close", async () => {
    await api.window.close(childId);
    for (let i = 0; i < 20 && (await api.window.list()).some(w => w.id === childId); i++) await sleep(25);
    expect(!(await api.window.list()).some(w => w.id === childId), "child remained after close");
    childId = null;
  });
  if (childId !== null) await api.window.close(childId).catch(() => {});

  await api.host.send("api-results", { checks, clipboardDigest });
})().catch(error => Niva.api.host.send("api-fatal", { message: String(error) }));
