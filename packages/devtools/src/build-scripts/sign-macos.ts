import { pathJoin } from "../common/utils";
import type { ProjectModel } from "../models/project.model";
import type { ProgressModel } from "../models/modal.model";

const { process, fs } = Niva.api;

/**
 * One-stop macOS signing for a built .app (user brings their own identity).
 *
 * - `identity`: codesign identity, e.g. "Developer ID Application: Foo (TEAMID)".
 *   Ad-hoc (`-`) works for local testing only.
 * - `entitlements`: project-relative entitlements plist (optional).
 * - `notarize.profile`: notarytool keychain profile (recommended, no secrets
 *   in config); or `appleId` + `teamId` with the password from env
 *   NIVA_APPLE_ID_PASSWORD (devtools inherits its own env).
 */
export async function signMacOsApp(params: {
  project: ProjectModel;
  progress: ProgressModel;
  appPath: string;
}): Promise<string> {
  const { project, progress, appPath } = params;
  const { locale } = project.app.state;
  const sign = project.state.config.sign?.macos;

  if (!sign?.identity) {
    throw new Error(locale.t("SIGN_NO_IDENTITY"));
  }

  const entitlements = sign.entitlements
    ? [pathJoin(project.state.path, sign.entitlements)]
    : [];
  const entitlementsFlag = sign.entitlements ? ["--entitlements"] : [];

  progress.addTask("codesign", async () => {
    await progress.runCommand("codesign", "codesign", [
      "--deep",
      "--force",
      "--options",
      "runtime",
      "--sign",
      sign.identity,
      ...entitlementsFlag,
      ...entitlements,
      appPath,
    ]);
  });

  progress.addTask("codesign --verify", async () => {
    await progress.runCommand("codesign --verify", "codesign", [
      "--verify",
      "--deep",
      "--strict",
      "--verbose=2",
      appPath,
    ]);
  });

  if (sign.notarize) {
    const zipPath = `${appPath}.zip`;
    progress.addTask("ditto (notarization zip)", async () => {
      await progress.runCommand("ditto (notarization zip)", "ditto", [
        "-c",
        "-k",
        "--sequesterRsrc",
        "--keepParent",
        appPath,
        zipPath,
      ]);
    });

    progress.addTask("notarytool submit", async () => {
      const creds = sign.notarize!.profile
        ? ["--keychain-profile", sign.notarize!.profile]
        : [
            "--apple-id",
            sign.notarize!.appleId!,
            "--team-id",
            sign.notarize!.teamId!,
            "--password",
            await applePassword(),
          ];
      await progress.runCommand("notarytool submit", "xcrun", [
        "notarytool",
        "submit",
        zipPath,
        ...creds,
        "--wait",
      ]);
    });

    progress.addTask("stapler staple", async () => {
      await progress.runCommand("stapler staple", "xcrun", ["stapler", "staple", appPath]);
      await fs.remove(zipPath);
    });
  }

  return appPath;
}

async function applePassword(): Promise<string> {
  const env = await process.env();
  const password = env["NIVA_APPLE_ID_PASSWORD"];
  if (!password) {
    throw new Error("NIVA_APPLE_ID_PASSWORD is not set");
  }
  return password;
}
