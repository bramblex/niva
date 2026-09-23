try {
  const path = await import("path");
  const fs = await Niva.import("fs/promises");
  const cwd = await Niva.api.process.currentDir();
  const filePath = cwd.replaceAll("\\", "/") + "/examples/windows-smoke/probe.txt";
  const base = await Niva.api.webview.baseFileSystemUrl();
  const response = await fetch(base + encodeURIComponent(filePath));
  const nativeText = (await fs.readFile(filePath, "utf8")).trim();
  await Niva.api.host.send("csp-pass", {
    origin: location.origin,
    windowId: await Niva.api.window.current(),
    pathResult: path.default.join("a", "b"),
    fileStatus: response.status,
    fileText: (await response.text()).trim(),
    nativeText,
    authorInlineBlocked: document.documentElement.dataset.authorInline !== "ran",
  });
} catch (error) {
  await Niva.api.host.send("csp-error", { message: String(error) });
}
