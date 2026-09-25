(async () => {
  try {
    await NivaFixture.ready("windows-stream");
    const fs = require("node:fs/promises");
    const path = require("node:path");
    const process = require("node:process");
    const childProcess = require("node:child_process");
    const output = path.join(process.cwd(), "dist", "windows-stream-stage", "output.txt");
    await fs.mkdir(path.dirname(output), { recursive: true });
    const payload = "A".repeat(150000);
    await fs.writeFile(output, payload, "utf8");
    await fs.appendFile(output, "tail", "utf8");
    const fileText = await fs.readFile(output, "utf8");
    const resourceText = await Niva.resource.read("large.txt", "utf8");
    const child = await new Promise((resolve, reject) => childProcess.execFile(
      "cmd.exe", ["/d", "/c", "echo STDOUT & echo STDERR 1>&2"],
      (error, stdout, stderr) => error ? reject(error) : resolve({ status: 0, stdout, stderr }),
    ));
    await NivaFixture.send("stream-ok", {
      fileLength: fileText.length,
      fileBoundary: fileText.slice(0, 1) + fileText.slice(-4),
      resourceLength: resourceText.length,
      resourceBoundary: resourceText.slice(0, 1) + resourceText.slice(-1),
      child,
    });
  } catch (error) {
    await NivaFixture.send("stream-error", { message: String(error) });
  }
})();
