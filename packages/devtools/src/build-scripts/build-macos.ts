import pako from "pako";
import { pathJoin } from "../common/utils";
import {
  appendResource,
  arrayBufferToBase64,
  dataKey,
  indexesKey,
  packageResource,
} from "./base";
import { plistTemplate } from "../templates/macos-plist-template";
import type { BuildParams } from './base';

export async function buildMacOsApp(params: BuildParams) {
  const { project, file, progress } = params;
  const { process, fs } = Niva.api;
  const { locale } = project.app.state;

  const currentExe = await process.currentExe();

  if (!file) {
    throw new Error(locale.t("UNSELECTED_APP_FILE"));
  }

  const appPath = file.endsWith(".app") ? file : file + ".app";
  const appContentsPath = pathJoin(appPath, "Contents");
  const appResourcesPath = pathJoin(appContentsPath, "Resources");
  const appMacOSPath = pathJoin(appContentsPath, "MacOS");
  const appExecutablePath = pathJoin(appMacOSPath, project.state.name);
  const appInfoPlistPath = pathJoin(appContentsPath, "Info.plist");
  const appIconPath = pathJoin(appResourcesPath, "icon.icns");
  const appIconsetPath = pathJoin(appResourcesPath, "icon.iconset");

  const projectResourcePath = pathJoin(
    project.state.path,
    project.state.config.build?.resource
  );
  const indexesPath = pathJoin(appResourcesPath, indexesKey);
  const dataPath = pathJoin(appResourcesPath, dataKey);

  progress.addTask(locale.t("CREATING_APP_STRUCTURE"), async () => {
    // make base structure
    await fs.createDir(appPath);
    await fs.createDir(appContentsPath);
    await fs.createDir(appResourcesPath);
    await fs.createDir(appIconsetPath);
    await fs.createDir(appMacOSPath);
  });

  progress.addTask(locale.t("COPYING_EXECUTABLE_FILE"), async () => {
    await fs.copy(currentExe, appExecutablePath);
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
    // create icon
    if (!project.state.config.icon) {
      return;
    }

    const iconPath = pathJoin(projectResourcePath, project.state.config.icon);

    for (let size of [16, 32, 64, 128, 256]) {
      await process.exec("sips", [
        "-z",
        size.toString(),
        size.toString(),
        iconPath,
        "--out",
        pathJoin(appIconsetPath, `icon_${size}x${size}.png`),
      ]);
    }

    await process.exec("iconutil", [
      "-c",
      "icns",
      appIconsetPath,
      "-o",
      appIconPath,
    ]);
    await fs.remove(appIconsetPath);
  });

  progress.addTask(locale.t("GENERATING_INFO_PLIST"), async () => {
    // generate Info.plist
    await fs.write(appInfoPlistPath, plistTemplate(project.state.config));
  });

  return appPath;
}
