Niva.addEventListener("host:message", async (_event, message) => {
  if (message.name !== "start-clipboard") return;
  try {
    const marker = message.data.marker;
    await Niva.api.clipboard.write(marker);
    const match = (await Niva.api.clipboard.read()) === marker;
    await Niva.api.host.send("clipboard-result", { match });
  } catch (error) {
    await Niva.api.host.send("clipboard-error", { message: String(error) });
  }
});
Niva.api.host.send("page-ready");
