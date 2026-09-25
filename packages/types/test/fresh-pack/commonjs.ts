import type {} from "@niva/types/commonjs";

const nivaFs = Niva.fs;
const workingDirectory: string = process.cwd();
const filename: string = __filename;
const directory: string = __dirname;
const loadedFs = require("node:fs");
const value: Buffer = Buffer.from("Niva CommonJS");
const encoding: BufferEncoding = "utf8";
const callback = setImmediate(() => undefined);
const exportShape: unknown = module.exports;
exports.workingDirectory = workingDirectory;
global.setTimeout(() => undefined, 0);

void filename;
void directory;
void nivaFs;
void loadedFs;
void value;
void encoding;
void callback;
void exportShape;
