import "./setup-runtime.js";
import test from "node:test";
import assert from "node:assert/strict";

const runtime = globalThis[Symbol.for("niva.node-compat.runtime")];
const BufferCompat = globalThis.Niva.buffer.Buffer;

test("sync fd calls preserve typed-array slices and native descriptor lifecycle", () => {
  const calls = [];
  const niva = {
    bridge: {
      isIpcOnly: () => false,
      callSync(method, args) {
        assert.equal(method, "fs.node");
        const [operation, payload] = args;
        calls.push({ operation, payload });
        if (operation === "open") return 41;
        if (operation === "write") return { bytesWritten: BufferCompat.from(payload.data, "base64").length };
        if (operation === "read") return { data: BufferCompat.from("XY").toString("base64"), bytesRead: 2 };
        if (operation === "fstat") return { size: 2, isFile: true, atimeMs: 0, mtimeMs: 0, ctimeMs: 0, birthtimeMs: 0 };
        if (operation === "close") return null;
        throw new Error(`unexpected fs.node operation: ${operation}`);
      },
    },
    bootstrap: {},
  };
  const fs = runtime.createFsModule(niva);
  const fd = fs.openSync("./types.d.ts", "r+");
  assert.equal(fd, 41);
  assert.equal(calls[0].operation, "open");
  assert.deepEqual(calls[0].payload, { path: "./types.d.ts", flag: "r+", mode: 438 });

  assert.equal(fs.writeSync(fd, BufferCompat.from("ABCDE"), 1, 2, 4), 2);
  assert.deepEqual(calls[1].payload, { fd, data: BufferCompat.from("BC").toString("base64"), position: 4 });

  const destination = BufferCompat.alloc(5, 0x2e);
  assert.equal(fs.readSync(fd, destination, 1, 2, 0), 2);
  assert.equal(destination.toString(), ".XY..");
  assert.deepEqual(calls[2].payload, { fd, length: 2, position: 0 });

  assert.equal(fs.fstatSync(fd).isFile(), true);
  assert.equal(calls[3].operation, "fstat");
  assert.equal(fs.closeSync(fd), undefined);
  assert.equal(calls[4].operation, "close");
  assert.throws(() => fs.fstatSync(fd), { code: "EBADF" });
});

test("sync fd calls reject in IPC-only mode before dispatch", () => {
  let dispatched = false;
  const niva = { bridge: { isIpcOnly: () => true, callSync() { dispatched = true; } }, bootstrap: {} };
  const fs = runtime.createFsModule(niva);
  assert.throws(() => fs.openSync("notes.txt"), { code: "ERR_NIVA_IPC_SYNC_UNSUPPORTED" });
  assert.equal(dispatched, false);
});
