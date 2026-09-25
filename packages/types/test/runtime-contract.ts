const trustedBridge: NivaBridge = Niva.bridge;
const routeHint: boolean = trustedBridge.isIpcOnly();
const osSnapshot: NivaOsInfo = Niva.os.info;
const dirs: Promise<NivaAppDirs> = Niva.os.dirs();
const fileText: Promise<string> = Niva.fs.promises.readFile("notes.txt", "utf8");
const fileMetadata: Promise<NivaFileStats> = Niva.fs.promises.stat("notes.txt");
const childText: Promise<NivaExecTextResult> = Niva.child_process.execFileText("node", ["-v"]);
const httpText: Promise<NivaHttpTextResult> = Niva.https.requestText({ url: "https://example.com" });
const compatVersion: "22.14.0" = Niva.process.versions.nodeCompat;
const streamConstructor: NivaPageStream = Niva.stream;
const moduleConstructor: NivaModule = Niva.module;

void routeHint;
void osSnapshot;
void dirs;
void fileText;
void fileMetadata;
void childText;
void httpText;
void compatVersion;
void streamConstructor;
void moduleConstructor;

// Removed aliases must remain absent from the published page contract.
// @ts-expect-error use Niva.bridge for native transport calls
Niva.call("window.close", []);
// @ts-expect-error namespace routing is direct on Niva
Niva.api.window.close();
import type {
  NivaAppDirs,
  NivaBridge,
  NivaExecTextResult,
  NivaFileStats,
  NivaFs,
  NivaHttpTextResult,
  NivaModule,
  NivaOsInfo,
} from "../../runtime/src/contracts";
import type { NivaStream as NivaPageStream } from "@niva/types/contracts";
