import pako from "pako";
import { pathJoin, tempDirWith } from "../common/utils";
import { ProjectModel } from "../models/project.model";
import {
  appendResource,
  arrayBufferToBase64,
  dataKey,
  indexesKey,
  packageResource,
} from "./base";
import { versionInfoTemplate } from "../templates/windows-version-info-template";

export async function buildWindowsApp(
  project: ProjectModel,
  _target?: string
): Promise<string> {
  const { process, dialog, fs, resource } = Niva.api;
  const { modal, locale } = project.app.state;

  const currentExe = await process.currentExe();
  const file = _target || (await dialog.saveFile(["exe"]));
  if (!file) {
    throw new Error(locale.t("UNSELECTED_EXE_FILE"));
  }

  const targetExe = file.endsWith(".exe") ? file : file + ".exe";
  const projectResourcePath = pathJoin(
    project.state.path,
    project.state.config.build?.resource
  );
  const buildPath = tempDirWith(
    `${project.state.name}_${project.state.uuid.slice(0, 8)}`
  );
  const indexesPath = pathJoin(buildPath, indexesKey);
  const dataPath = pathJoin(buildPath, dataKey);

  const [progress, close] = modal.progress(locale.t("BUILDING_APP"));

  progress.addTask(locale.t("PREPARE_BUILD_ENVIRONMENT"), async () => {
    await fs.createDirAll(buildPath);
  });

  let fileIndexes: Record<string, [number, number]> = {};
  let buffer = new ArrayBuffer(0);
  progress.addTask(locale.t("PACKAGING_RESOURCES"), async () => {
    const initialResource = await appendResource(
      project.state.configPath,
      "niva.json"
    );
    const [_fileIndex, _buffer] = await packageResource(
      projectResourcePath,
      ...initialResource
    );
    fileIndexes = _fileIndex;
    buffer = _buffer;
  });

  progress.addTask(locale.t("COMPRESSING_RESOURCES"), async () => {
    const compressedBuffer = pako.deflateRaw(buffer).buffer;
    await Promise.all([
      fs.write(indexesPath, JSON.stringify(fileIndexes, null, 2)),
      fs.write(dataPath, arrayBufferToBase64(compressedBuffer), "base64"),
    ]);
  });

  progress.addTask(locale.t("GENERATING_ICON"), async () => {
    if (!project.state.config.icon) {
      return;
    }
    const iconPath = pathJoin(projectResourcePath, project.state.config.icon);
    await resource.extract(
      "windows/icon_creator.exe",
      pathJoin(buildPath, "icon_creator.exe")
    );
    await process.exec(pathJoin(buildPath, "icon_creator.exe"), [
      iconPath,
      pathJoin(buildPath, "icon.ico"),
    ]);
  });

  progress.addTask(locale.t("BUILD_EXECUTABLE_FILE"), async () => {
    await resource.extract(
      "windows/ResourceHacker.exe",
      pathJoin(buildPath, "ResourceHacker.exe")
    );

    const versionInfoPath = pathJoin(buildPath, "VERSION_INFO");
    await fs.write(versionInfoPath, versionInfoTemplate(project.state.config));

    const iconScript = `
-delete ICON,1,0
-delete ICON,2,0
-delete ICON,3,0
-delete ICON,4,0
-delete ICON,5,0
-delete ICON,6,0
-delete ICON,7,0
-addoverwrite ${pathJoin(buildPath, "icon.ico")}, ICONGROUP,1,1033
`;

    const script = `
[FILENAMES]
Exe=    ${currentExe}
SaveAs= ${targetExe}
Log=    ${pathJoin(buildPath, "ResourceHacker.log")}
[COMMANDS]
-addoverwrite ${indexesPath}, RCDATA,${indexesKey},1033
-addoverwrite ${dataPath}, RCDATA,${dataKey},1033
${project.state.config.icon ? iconScript : ""}
`;

    await fs.write(pathJoin(buildPath, "bundle_script.txt"), script);

    await process.exec(pathJoin(buildPath, "ResourceHacker.exe"), [
      "-script",
      pathJoin(buildPath, "bundle_script.txt"),
    ]);
  });

  progress.addTask(locale.t("CLEAN_BUILD_ENVIRONMENT"), async () => {
    await fs.remove(buildPath);
  });

  await progress.run();
  close();

  return targetExe;
}
