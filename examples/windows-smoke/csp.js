import path from "node:path";
import fs from "node:fs/promises";
import process from "node:process";

try {
  await NivaFixture.ready("windows-csp");
  const cwd = process.cwd();
  const filePath = cwd.replaceAll("\\", "/") + "/examples/windows-smoke/probe.txt";
  const base = await Niva.webview.baseFileSystemUrl();
  const response = await fetch(base + encodeURIComponent(filePath));
  const nativeText = (await fs.readFile(filePath, "utf8")).trim();
  await NivaFixture.send("csp-pass", {
    origin: location.origin,
    windowId: await Niva.window.current(),
    pathResult: path.join("a", "b"),
    fileStatus: response.status,
    fileText: (await response.text()).trim(),
    nativeText,
    authorInlineBlocked: document.documentElement.dataset.authorInline !== "ran",
  });
} catch (error) {
  await NivaFixture.send("csp-error", { message: String(error) });
}
