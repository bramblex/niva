Niva.addEventListener("host:message", async (_event, message) => {
  if (message.name === "post-cancel-ping") {
    await Niva.api.host.send("post-cancel-pong");
    return;
  }
  if (message.name !== "start-print") return;
  Niva.api.webview.print().then(
    () => Niva.api.host.send("print-call-returned"),
    error => Niva.api.host.send("print-error", { message: String(error) }),
  );
  await Niva.api.host.send("print-dispatched");
});
Niva.api.host.send("page-ready");
