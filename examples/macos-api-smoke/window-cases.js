// Extra macOS window cases for the disposable Niva API smoke app.
// The runner loads this after initialize_script.js and invokes runAutomatic.
(function () {
  "use strict";

  const automatic = [];
  const supervised = [];
  const add = (method, assertion, run) => automatic.push({ method, assertion, run });
  const ui = (method, action, expected, restore) => supervised.push({ method, action, expected, restore });
  const expect = (condition, message) => { if (!condition) throw new Error(message); };
  const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));
  async function until(read, expected, label) {
    for (let i = 0; i < 40; i += 1) {
      if (await read() === expected) return;
      await sleep(50);
    }
    throw new Error(`${label}: state did not become ${expected}`);
  }

  add("window.setOuterPosition", "moves a disposable child and restores its original position", async ({ win, id }) => {
    const before = await win.outerPosition(id);
    const target = { x: before.x + 12, y: before.y + 12 };
    try {
      await win.setOuterPosition(target, id);
      const after = await win.outerPosition(id);
      expect(Math.abs(after.x - target.x) <= 3 && Math.abs(after.y - target.y) <= 3, "outer position did not move");
    } finally { await win.setOuterPosition(before, id); }
  });

  for (const [getter, setter] of [
    ["isMinimizable", "setMinimizable"], ["isMaximizable", "setMaximizable"],
    ["isClosable", "setClosable"], ["isDecorated", "setDecorated"],
    ["isMinimized", "setMinimized"], ["isMaximized", "setMaximized"],
  ]) {
    add(`window.${getter}`, `reads the disposable child's ${getter} state`, async ({ win, id }) => {
      expect(typeof await win[getter](id) === "boolean", `${getter} did not return a boolean`);
    });
    add(`window.${setter}`, `toggles ${getter}, verifies readback, and restores it`, async ({ win, id }) => {
      const before = await win[getter](id);
      try {
        await win[setter](!before, id);
        await until(() => win[getter](id), !before, setter);
      } finally {
        await win[setter](before, id);
        await until(() => win[getter](id), before, `${setter} restore`);
      }
    });
  }

  add("window.theme", "returns a recognized native theme", async ({ win, id }) => {
    expect(["light", "dark", "system"].includes(await win.theme(id)), "unknown window theme");
  });
  add("window.setTheme", "changes the disposable app theme and restores system choice", async ({ win, id }) => {
    const before = await win.theme(id);
    try {
      await win.setTheme("dark", id);
      await until(() => win.theme(id), "dark", "setTheme");
    } finally { await win.setTheme(before === "system" ? null : before, id); }
  });
  add("window.fullscreen", "reads the disposable child's fullscreen state", async ({ win, id }) => {
    expect(typeof await win.fullscreen(id) === "boolean", "fullscreen getter did not return boolean");
  });
  add("window.setFullscreen", "toggles disposable-child fullscreen and restores it", async ({ win, id }) => {
    const before = await win.fullscreen(id);
    try {
      await win.setFullscreen(!before, null, id);
      await until(() => win.fullscreen(id), !before, "setFullscreen");
    } finally { await win.setFullscreen(before, null, id); }
  });

  add("window.setMinInnerSize", "enforces then clears a minimum child size", async ({ win, id }) => {
    const before = await win.innerSize(id);
    const minimum = { width: before.width + 40, height: before.height + 40 };
    try {
      await win.setMinInnerSize(minimum, id);
      await until(async () => {
        const actual = await win.innerSize(id);
        return actual.width >= minimum.width - 3 && actual.height >= minimum.height - 3;
      }, true, "minimum size");
    } finally {
      await win.setMinInnerSize(null, id);
      await win.setInnerSize(before, id);
    }
  });
  add("window.setMaxInnerSize", "enforces then clears a maximum child size", async ({ win, id }) => {
    const before = await win.innerSize(id);
    const maximum = { width: before.width - 40, height: before.height - 40 };
    try {
      await win.setMaxInnerSize(maximum, id);
      await until(async () => {
        const actual = await win.innerSize(id);
        return actual.width <= maximum.width + 3 && actual.height <= maximum.height + 3;
      }, true, "maximum size");
    } finally {
      await win.setMaxInnerSize(null, id);
      await win.setInnerSize(before, id);
    }
  });

  add("window.setWindowIcon", "rejects the unsupported macOS window-icon operation", async ({ win, id }) => {
    let error;
    try { await win.setWindowIcon(null, id); } catch (caught) { error = caught; }
    expect(error && error.code === -1 && /unsupported on this platform/.test(error.message), "setWindowIcon did not reject on macOS");
  });
  add("window.dragResizeWindow", "rejects edge resize drag when macOS Tao does not support it", async ({ win, id }) => {
    let error;
    try { await win.dragResizeWindow("east", id); } catch (caught) { error = caught; }
    expect(error && error.code === -1, "dragResizeWindow unexpectedly succeeded on macOS");
  });

  add("window.setMenu", "attaches a disposable menu to the child window", async ({ win, id }) => {
    await win.setMenu([{ label: "Niva smoke", children: [{ type: "item", id: 17, label: "Probe" }] }], id);
    expect(await win.isMenuVisible(id), "menu was not visible after setMenu");
  });
  add("window.isMenuVisible", "reads the menu state after attach/hide/show", async ({ win, id }) => {
    expect(typeof await win.isMenuVisible(id) === "boolean", "menu visibility was not boolean");
  });
  add("window.hideMenu", "hides the disposable menu and reads it back", async ({ win, id }) => {
    await win.hideMenu(id);
    expect((await win.isMenuVisible(id)) === false, "menu remained visible");
  });
  add("window.showMenu", "restores the disposable menu and reads it back", async ({ win, id }) => {
    await win.showMenu(id);
    expect((await win.isMenuVisible(id)) === true, "menu did not return");
  });

  const extra = (name, assertion, run) => add(`windowExtra.${name}`, assertion, run);
  extra("hasShadow", "reads the child window's shadow state", async ({ ext, id }) => {
    expect(typeof await ext.hasShadow(id) === "boolean", "hasShadow did not return boolean");
  });
  extra("setHasShadow", "toggles native shadow and restores it", async ({ ext, id }) => {
    const before = await ext.hasShadow(id);
    try {
      await ext.setHasShadow(!before, id);
      expect((await ext.hasShadow(id)) === !before, "shadow state did not toggle");
    } finally { await ext.setHasShadow(before, id); }
  });
  extra("isDocumentEdited", "reads the disposable child document-edited flag", async ({ ext, id }) => {
    expect(typeof await ext.isDocumentEdited(id) === "boolean", "edited flag did not return boolean");
  });
  extra("setIsDocumentEdited", "toggles document-edited flag and restores it", async ({ ext, id }) => {
    const before = await ext.isDocumentEdited(id);
    try {
      await ext.setIsDocumentEdited(!before, id);
      expect((await ext.isDocumentEdited(id)) === !before, "edited flag did not toggle");
    } finally { await ext.setIsDocumentEdited(before, id); }
  });
  extra("allowsAutomaticWindowTabbing", "reads automatic tabbing state", async ({ ext, id }) => {
    expect(typeof await ext.allowsAutomaticWindowTabbing(id) === "boolean", "tabbing state did not return boolean");
  });
  extra("setAllowsAutomaticWindowTabbing", "toggles automatic tabbing and restores it", async ({ ext, id }) => {
    const before = await ext.allowsAutomaticWindowTabbing(id);
    try {
      await ext.setAllowsAutomaticWindowTabbing(!before, id);
      expect((await ext.allowsAutomaticWindowTabbing(id)) === !before, "tabbing state did not toggle");
    } finally { await ext.setAllowsAutomaticWindowTabbing(before, id); }
  });
  extra("tabbingIdentifier", "reads a string tabbing identifier", async ({ ext, id }) => {
    expect(typeof await ext.tabbingIdentifier(id) === "string", "tabbing identifier was not string");
  });
  extra("setTabbingIdentifier", "sets a unique tab identifier and restores it", async ({ ext, id }) => {
    const before = await ext.tabbingIdentifier(id);
    try {
      await ext.setTabbingIdentifier("niva-smoke-window", id);
      expect((await ext.tabbingIdentifier(id)) === "niva-smoke-window", "tabbing identifier did not change");
    } finally { await ext.setTabbingIdentifier(before, id); }
  });
  extra("simpleFullscreen", "reads simple fullscreen state", async ({ ext, id }) => {
    expect(typeof await ext.simpleFullscreen(id) === "boolean", "simpleFullscreen was not boolean");
  });
  extra("setSimpleFullscreen", "toggles simple fullscreen and restores it", async ({ ext, id }) => {
    const before = await ext.simpleFullscreen(id);
    try {
      await ext.setSimpleFullscreen(!before, id);
      await until(() => ext.simpleFullscreen(id), !before, "setSimpleFullscreen");
    } finally { await ext.setSimpleFullscreen(before, id); }
  });

  for (const [name, args] of [
    ["setEnable", [true]], ["setTaskbarIcon", ["icon.png"]], ["theme", []],
    ["resetDeadKeys", []], ["beginResizeDrag", [0, 0, 0, 0]], ["setSkipTaskbar", [false]],
    ["setUndecoratedShadow", [false]], ["setOverlayIcon", [null]], ["setRtl", [false]],
    ["hasUndecoratedShadow", []],
  ]) {
    extra(name, "rejects this Windows-only handler on macOS with api not found", async ({ ext, id }) => {
      let error;
      try { await ext[name](...args, id); } catch (caught) { error = caught; }
      expect(error && error.code === -1 && /api not found/.test(error.message), `${name} did not reject as unregistered`);
    });
  }

  ui("window.sendMessage", "Send a unique nonce from the main window to a disposable child", "child echoes nonce and source window ID", "close the child");
  ui("window.setAlwaysOnTop", "Raise the child over a second smoke window", "CGWindow layer/order changes as requested", "set false and close both children");
  ui("window.setAlwaysOnBottom", "Place the child below a second smoke window", "CGWindow layer/order changes as requested", "set false and close both children");
  ui("window.setBackgroundColor", "Set a unique RGBA child background", "captured child pixels show the color", "restore previous color or close child");
  ui("window.setContentProtection", "Enable content protection on a disposable child", "screen capture excludes child content", "disable protection and close child");
  ui("window.setFocusable", "Make the child non-focusable and request focus", "isFocused remains false", "restore focusable and close child");
  ui("window.setImePosition", "Move IME candidate origin on a disposable text field", "candidate location follows requested point", "restore prior input method/close child");
  ui("window.setProgressBar", "Set a temporary Dock progress value", "Dock progress reflects value", "clear progress and close child");
  ui("window.requestRedraw", "Request redraw after changing child content marker", "native redraw event or changed captured pixels", "close child");
  ui("window.requestUserAttention", "Request attention for the disposable child", "Dock/notification attention cue appears", "focus child and close it");
  ui("window.setVisibleOnAllWorkspaces", "Show child across a temporary second Space", "child remains visible in both Spaces", "restore false and close child");
  ui("window.setCursorIcon", "Set a distinct cursor above the child", "captured cursor icon changes", "restore default icon and close child");
  ui("window.cursorPosition", "Move pointer within the disposable child", "API reports the observed pointer coordinate", "restore original pointer location");
  ui("window.setCursorPosition", "Set pointer to a child-local point", "cursorPosition reports that point within tolerance", "restore original pointer location");
  ui("window.setCursorGrab", "Capture pointer in the child", "pointer cannot leave as configured", "release grab and close child");
  ui("window.setCursorVisible", "Hide pointer above the child", "pointer is absent from visual capture", "restore visible and close child");
  ui("window.dragWindow", "Press on a drag handle then invoke dragWindow", "child outerPosition changes with pointer movement", "release pointer and restore position");
  ui("window.setIgnoreCursorEvents", "Ignore pointer events on child", "click passes through to test window behind", "restore false and close child");
  ui("window.blockCloseRequested", "Block close on a disposable child and request close", "window.closeRequested event fires while child remains", "set false and close child");
  ui("windowExtra.setTrafficLightInset", "Offset child traffic light controls", "button positions shift by the requested inset", "restore original inset or close child");
  ui("windowExtra.setActivationPolicyAtRuntime", "Switch disposable app to accessory then regular", "Dock/activation state follows each policy", "restore regular and exit app");
  ui("windowExtra.setDockVisibility", "Hide then show disposable app Dock icon", "Dock visibility changes", "restore visible and exit app");
  ui("windowExtra.setBadgeLabel", "Set a unique disposable Dock badge", "badge text appears", "clear badge and exit app");

  window.NivaMacWindowCases = {
    automatic,
    supervised,
    async runAutomatic(api, record) {
      const win = api.window;
      const ext = api.windowExtra;
      const id = await win.open({ entry: "child.html", title: "Niva extra window cases", size: { width: 360, height: 240 }, visible: true });
      try {
        for (const item of automatic) {
          await api.host.send("extended-progress", { method: item.method }).catch(() => {});
          await item.run({ api, win, ext, id });
          record(item.method, item.assertion);
        }
      } finally {
        await win.setMenu(null, id).catch(() => {});
        await win.close(id).catch(() => {});
      }
    },
  };
})();
