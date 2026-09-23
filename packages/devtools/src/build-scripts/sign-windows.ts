import { pathJoin } from "../common/utils";
import { runCmd } from "../common/utils";
import type { ProjectModel } from "../models/project.model";
import type { ProgressModel } from "../models/modal.model";

const { process, fs } = Niva.api;

const DEFAULT_TIMESTAMP = "http://timestamp.digicert.com";

const SIGNTOOL_CANDIDATES = [
  "signtool.exe",
  "C:\\Program Files (x86)\\Windows Kits\\10\\bin\\10.0.22621.0\\x64\\signtool.exe",
  "C:\\Program Files (x86)\\Windows Kits\\10\\bin\\10.0.22000.0\\x64\\signtool.exe",
  "C:\\Program Files (x86)\\Windows Kits\\10\\bin\\10.0.19041.0\\x64\\signtool.exe",
];

/**
 * One-stop Windows signing for a built .exe (user brings their own cert).
 *
 * - `pfx`: project-relative .pfx path. Password comes from env
 *   NIVA_WIN_CERT_PASSWORD (never stored in niva.json).
 * - `timestamp`: RFC3161 server (defaults to Digicert).
 */
export async function signWindowsApp(params: {
  project: ProjectModel;
  progress: ProgressModel;
  exePath: string;
}): Promise<string> {
  const { project, progress, exePath } = params;
  const { locale } = project.app.state;
  const sign = project.state.config.sign?.windows;

  if (!sign?.pfx) {
    throw new Error(locale.t("SIGN_NO_PFX"));
  }

  const signtool = await findSigntool();
  if (!signtool) {
    throw new Error(locale.t("SIGN_NO_SIGNTOOL"));
  }

  const env = await process.env();
  const password = env["NIVA_WIN_CERT_PASSWORD"];
  if (!password) {
    throw new Error("NIVA_WIN_CERT_PASSWORD is not set");
  }

  const pfxPath = pathJoin(project.state.path, sign.pfx);
  if (!(await fs.exists(pfxPath))) {
    throw new Error(locale.t("SIGN_NO_PFX"));
  }

  progress.addTask("signtool sign", async () => {
    await progress.runCommand("signtool sign", signtool, [
      "sign",
      "/fd",
      "SHA256",
      "/f",
      pfxPath,
      "/p",
      password,
      "/tr",
      sign.timestamp || DEFAULT_TIMESTAMP,
      "/td",
      "SHA256",
      exePath,
    ]);
  });

  return exePath;
}

async function findSigntool(): Promise<string | null> {
  for (const candidate of SIGNTOOL_CANDIDATES) {
    try {
      if (candidate === "signtool.exe") {
        await runCmd("where", ["signtool.exe"]);
        return candidate;
      }
      if (await fs.exists(candidate)) {
        return candidate;
      }
    } catch {
      // try next candidate
    }
  }
  return null;
}
