import type {} from "@niva/types";
import type { NivaOsInfo } from "@niva/types/contracts";

// Standard Node globals here are supplied by the consumer's explicit
// `types: ["node"]`, not by the browser declaration entry.
const workingDirectory: string = process.cwd();
const nodeReadFile: typeof import("node:fs").readFile = Niva.fs.readFile;
const nivaInfo: NivaOsInfo = Niva.os.info;

void workingDirectory;
void nodeReadFile;
void nivaInfo;
