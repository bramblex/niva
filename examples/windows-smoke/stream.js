(async () => {
  try {
    const cwd = await Niva.api.process.currentDir();
    const output = cwd + "\\dist\\windows-stream-stage\\output.txt";
    const payload = "A".repeat(150000);
    await Niva.api.fs.write(output, payload);
    await Niva.api.fs.append(output, "tail");
    const fileText = await Niva.api.fs.read(output);
    const resourceText = await Niva.api.resource.read("large.txt");
    const child = await Niva.api.process.exec(
      "cmd.exe", ["/d", "/c", "echo STDOUT & echo STDERR 1>&2"]
    );
    await Niva.api.host.send("stream-ok", {
      fileLength: fileText.length,
      fileBoundary: fileText.slice(0, 1) + fileText.slice(-4),
      resourceLength: resourceText.length,
      resourceBoundary: resourceText.slice(0, 1) + resourceText.slice(-1),
      child,
    });
  } catch (error) {
    await Niva.api.host.send("stream-error", { message: String(error) });
  }
})();
