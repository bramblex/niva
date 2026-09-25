import type {} from "@niva/types";
import type { NivaFs, NivaModule, NivaStream } from "@niva/types/contracts";

const files: NivaFs = Niva.fs;
void files;
const streamConstructor: NivaStream = Niva.stream;
const moduleConstructor: NivaModule = Niva.module;
void streamConstructor;
void moduleConstructor;

// CommonJS globals are injected only when NivaOptions.injectCommonJs is true.
// @ts-expect-error host Node require is not part of the base Niva page globals
require("node:fs");
// @ts-expect-error host Node process is not part of the base Niva page globals
process.cwd();
// @ts-expect-error Buffer is available as Niva.buffer.Buffer or injected by the flag
Buffer.from("text");

// @ts-expect-error Node module names are private inside the browser declaration entry
type HiddenStreamModule = typeof import("node:stream");
