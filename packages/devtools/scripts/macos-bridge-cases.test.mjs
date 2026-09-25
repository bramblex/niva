import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import test from "node:test";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../../..");

test("macOS real-WebView fixture covers current bridge transport and event methods", () => {
  const page = readFileSync(path.join(root, "examples/macos-api-smoke/bridge-top-level.html"), "utf8");
  const expected = [
    "Niva.addEventListener", "Niva.removeEventListener", "Niva.removeAllEventListeners",
    "Niva.bridge.call", "Niva.bridge.callSync", "Niva.bridge.stream", "Niva.bridge.streamSend",
  ];
  const actual = [...page.matchAll(/record\("(Niva(?:\.[A-Za-z]+){1,2})"/g)].map((match) => match[1]);
  assert.equal(actual.length, new Set(actual).size, "duplicate bridge method case");
  assert.deepEqual(actual.sort(), expected.sort());
  assert.doesNotMatch(page, /Niva\.(?:call|callSync|stream|streamSend)\b/);
  assert.doesNotMatch(page, /Niva\.(?:registerModule|registerModuleFactory|require|import)\b/);
});
