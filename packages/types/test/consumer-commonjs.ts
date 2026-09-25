import type {} from "@niva/types/commonjs";

const workingDirectory: string = process.cwd();
const filename: string = __filename;
const directory: string = __dirname;
const loadedFs = require("node:fs");
const bytes: Buffer = Buffer.from("niva");
const encoding: BufferEncoding = "utf8";
const immediate = setImmediate(() => undefined);
const exportedDirectory: string = module.exports.cwd ?? "";
exports.workingDirectory = workingDirectory;
global.setTimeout(() => undefined, 0);

void filename;
void directory;
void loadedFs;
void bytes;
void encoding;
void immediate;
void exportedDirectory;
