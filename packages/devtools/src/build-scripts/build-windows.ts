import { pathJoin, tempDirWith } from "../common/utils";
import { versionInfoTemplate } from "../templates/windows-version-info-template";
import {
  resolveNodeCompatAssets,
  stageProjectResourcesWithNodeCompat,
} from "./node-compat";
import type { BuildParams } from './base';

export async function buildWindowsApp(params: BuildParams): Promise<string> {
  const { project, file, progress } = params;
  const { process, fs, resource } = Niva.api;
  const { locale } = project.app.state;

  const currentExe = await process.currentExe();
  if (!file) {
    throw new Error(locale.t("UNSELECTED_EXE_FILE"));
  }

  const targetExe = file.endsWith(".exe") ? file : file + ".exe";
  const projectResourcePath = pathJoin(
    project.state.path,
    project.state.config.build?.resource
  );
  const nodeCompatOption = project.state.config.nodeCompat;
  const nodeCompatSelection = resolveNodeCompatAssets(nodeCompatOption);
  const buildPath = tempDirWith(
    `${project.state.name}_${project.state.uuid.slice(0, 8)}`
  );
  const stagedResourcePath = pathJoin(buildPath, "node-compat-resources");
  const resourcePathForPackager = nodeCompatSelection.enabled
    ? stagedResourcePath
    : projectResourcePath;
  const packagerPath = pathJoin(buildPath, "win_packager.exe");
  const versionInfoPath = pathJoin(buildPath, "VERSION_INFO");

  progress.addTask(locale.t("PREPARE_BUILD_ENVIRONMENT"), async () => {
    await fs.createDirAll(buildPath);
    if (nodeCompatSelection.enabled) {
      await stageProjectResourcesWithNodeCompat(
        projectResourcePath,
        stagedResourcePath,
        nodeCompatOption,
      );
    }
  });

  progress.addTask(locale.t("BUILD_EXECUTABLE_FILE"), async () => {
    await resource.extract("windows/win_packager.exe", packagerPath);
    await fs.write(
      versionInfoPath,
      versionInfoTemplate(project.state.config)
    );

    const args = [
      "--exe",
      currentExe,
      "--save-as",
      targetExe,
      "--resource-dir",
      resourcePathForPackager,
      "--config",
      project.state.configPath,
      "--version-info",
      versionInfoPath,
      "--lang",
      "1033",
    ];
    if (project.state.config.icon) {
      args.push(
        "--icon-png",
        pathJoin(projectResourcePath, project.state.config.icon),
        "--delete-icon-ids",
        "1,2,3,4,5,6,7"
      );
    }

    try {
      await progress.runCommand(locale.t("BUILD_EXECUTABLE_FILE"), packagerPath, args);
    } catch (err) {
      await process.open(buildPath);
      throw err;
    }
  });

  progress.addTask(locale.t("CLEAN_BUILD_ENVIRONMENT"), async () => {
    await fs.remove(buildPath);
  });

  return targetExe;
}
