import assert from "node:assert/strict";
import { EventEmitter } from "node:events";
import { readFile } from "node:fs/promises";
import { test } from "node:test";
import vm from "node:vm";

const fixturePaths = [
  new URL("./fixture-protocol.js", import.meta.url),
  new URL("../node-compat-macos-smoke/fixture-protocol.js", import.meta.url),
  new URL("../node-compat-integration/fixture-protocol.js", import.meta.url),
  new URL("../windows-smoke/fixture-protocol.js", import.meta.url),
];

for (const fixturePath of fixturePaths) {
  test(`${fixturePath.pathname} speaks the versioned process-stream protocol`, async () => {
    const output = [];
    const exits = [];
    const nivaListeners = new Map();
    const childInput = new EventEmitter();
    const process = {
      stdin: childInput,
      stdout: {
        write(text, callback) {
          output.push(JSON.parse(text));
          callback?.(null);
          return true;
        },
      },
      exit(code) { exits.push(code); },
    };
    const Niva = {
      addEventListener(name, listener) { nivaListeners.set(name, listener); },
    };
    const source = await readFile(fixturePath, "utf8");
    const context = { process, Niva, TextDecoder };
    vm.runInNewContext(source, context);
    const fixture = context.NivaFixture;

    let received;
    fixture.onCommand("probe", (data) => { received = data; });
    await fixture.ready("probe-page", { path: "/index.html" });
    await fixture.send("page-result", { ok: true });
    childInput.emit("data", Buffer.from(JSON.stringify({
      protocol: "niva-fixture", version: 1, event: "command", name: "probe", data: { nonce: "same" },
    }) + "\n"));
    await new Promise((resolve) => setImmediate(resolve));
    assert.deepEqual(JSON.parse(JSON.stringify(received)), { nonce: "same" });

    nivaListeners.get("window.message")?.("window.message", {
      from: 1,
      message: JSON.stringify({
        protocol: "niva-fixture", version: 1, event: "message", name: "child-result", data: { child: true },
      }),
    });
    await new Promise((resolve) => setImmediate(resolve));
    fixture.exit(7);

    assert.deepEqual(output.map(({ event, name }) => [event, name]), [
      ["ready", "probe-page"],
      ["message", "page-result"],
      ["message", "child-result"],
    ]);
    assert.deepEqual(exits, [7]);
  });
}
