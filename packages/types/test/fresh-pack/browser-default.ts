import type {} from "@niva/types";
import type { NivaAppDirs, NivaExecTextResult, NivaFileStats, NivaFs, NivaOsInfo } from "@niva/types/contracts";

const fileSystem: NivaFs = Niva.fs;
const osSnapshot: NivaOsInfo = Niva.os.info;
const options: NivaOptions = { name: "Types fixture", uuid: "types-fixture" };
const appDirs: Promise<NivaAppDirs> = Niva.os.dirs();
const text: Promise<string> = Niva.fs.promises.readFile("data.txt", "utf8");
const metadata: Promise<NivaFileStats> = Niva.fs.promises.stat("data.txt");
const command: Promise<NivaExecTextResult> = Niva.child_process.execFileText("tool", ["--version"]);
const buffer = Niva.buffer.Buffer.from("Niva page types");

void fileSystem;
void osSnapshot;
void options;
void appDirs;
void text;
void metadata;
void command;
void buffer;

// The base browser entry must not inherit host Node globals or ambient modules.
// @ts-expect-error Node globals are opt-in, independent of runtime configuration
process.cwd();
// @ts-expect-error Node globals are opt-in, independent of runtime configuration
require("node:fs");
// @ts-expect-error Node globals are opt-in, independent of runtime configuration
module.exports;
// @ts-expect-error Node globals are opt-in, independent of runtime configuration
exports.value;
// @ts-expect-error Node globals are opt-in, independent of runtime configuration
global.setTimeout(() => undefined, 0);
// @ts-expect-error Node globals are opt-in, independent of runtime configuration
__filename;
// @ts-expect-error Node globals are opt-in, independent of runtime configuration
__dirname;
// @ts-expect-error Buffer's global value is opt-in
Buffer.from("text");
// @ts-expect-error Buffer's global type is opt-in
type HiddenBuffer = Buffer;
// @ts-expect-error BufferEncoding is a Node-only global type
type HiddenEncoding = BufferEncoding;
// @ts-expect-error NodeJS is a Node-only global namespace
type HiddenEnvironment = NodeJS.ProcessEnv;
// @ts-expect-error Node timer globals are opt-in
setImmediate(() => undefined);
// @ts-expect-error node module names are intentionally private to Niva declarations
type HiddenNodeFs = typeof import("node:fs");
// @ts-expect-error node module names are intentionally private to Niva declarations
type HiddenBareFs = typeof import("fs");
// @ts-expect-error Node augments Error only in Node environments
Error.captureStackTrace;
// @ts-expect-error no Node global is installed on the browser global object
globalThis.Buffer;
