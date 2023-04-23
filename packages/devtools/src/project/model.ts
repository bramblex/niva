import { StateModel } from "@bramblex/state-model";
import { plistTemplate, versionInfoTemplate } from "./template";
import {
  arrayBufferToBase64,
  dirname,
  packageResource,
  pathJoin,
  tempWith,
  tryOrAlertAsync,
  tryOrP,
  uuid,
  withCtx,
  withCtxP,
} from "../utils";
import { modal } from "../modal";
import { OptionsEditor } from "./options-editor";
import pako from "pako";

const { os, fs, process, dialog, resource } = Niva.api;

interface ProjectState {
  name: string;
  uuid: string;
  path: string;
  configPath: string;
  config: any;
}

const indexesKey = "RESOURCE_INDEXES";
const dataKey = "RESOURCE_DATA";

export class ProjectModel extends StateModel<ProjectState | null> {
  constructor() {
    super(null);
  }

  init(path: string) {
    return tryOrAlertAsync(async () => {
      // check path is a directory
      const { isDir } = await tryOrP(fs.stat(path), { isDir: false });

      if (!isDir) {
        return "文件格式错误";
      }

      // check niva.json exists, if not create it
      const configPath = pathJoin(path, "niva.json");
      const config = await this.loadOrCreateConfig(configPath);

      this.setState({
        name: config.name,
        uuid: config.uuid,
        path,
        configPath,
        config
      });
    });
  }

  private async loadOrCreateConfig(configPath: string) {
    if (!(await fs.exists(configPath))) {
      const ok = await modal.confirm(
        "提示",
        `未找到项目配置文件 ”${configPath}", 是否创建?`
      );
      if (ok) {
        const defaultConfig = {
          name: "niva_project",
          uuid: uuid(),
        };
        await withCtxP(
          fs.write(configPath, JSON.stringify(defaultConfig, null, 2)),
          '创建项目配置文件 "niva.json" 失败'
        );
      } else {
        throw new Error("未找到项目配置文件");
      }
    }

    const content = (await withCtxP(
      fs.read(configPath),
      "读取项目配置文件失败"
    )) as string;
    const config = withCtx(
      () => JSON.parse(content),
      "解析项目配置文件失败"
    ) as any;

    if (!config.name && !config.uuid) {
      throw new Error(
        "项目配置文件格式错误, 缺少 name 或 uuid 字段，请检查或删除文件"
      );
    }

    return config;
  }

  // operations
  open() {
    return tryOrAlertAsync(async () => {
      await process.open(this.state!.path);
    });
  }

  close() {
    this.setState(null);
  }

  edit() {
    return tryOrAlertAsync(async () => {
      modal.show(OptionsEditor, { project: this });
    });
  }

  refresh() {
    const projectPath = this.state!.path;
    if (!projectPath) {
      return;
    }
    return this.init(this.state!.path);
  }

  debug() {
    const projectPath = this.state!.path;
    const debugEntry = this.state!.config.debugEntry;

    return tryOrAlertAsync(async () => {
      const exe = await process.currentExe();
      process.exec(
        exe,
        [
          `--debug-resource=${projectPath}`,
          "--debug-devtools=true",
          ...(debugEntry ? [`--debug-entry=${debugEntry}`] : []),
        ],
        { detached: true }
      );
    });
  }

  build(target?: string) {
    return tryOrAlertAsync(async () => {
      const { os: osType } = await os.info();

      let appPath: string;
      if (osType.toLowerCase().replace(/\s/g, "") === "macos") {
        appPath = await this.buildMacOsApp(target);
      } else if (osType.toLowerCase() === "windows") {
        appPath = await this.buildWindowsApp(target);
      } else {
        throw new Error(`不支持当前操作系统 "${osType}"`);
      }

      modal
        .confirm("构建成功", `应用 ${this.state!.name} 已成功构建`)
        .then((ok) => ok && process.open(dirname(appPath)));
    });
  }

  private async buildWindowsApp(_target?: string): Promise<string> {
    const currentExe = await process.currentExe();
    const file = _target || (await dialog.saveFile(["exe"]));
    if (!file) {
      throw new Error("未选择exe文件");
    }
    const targetExe = file.endsWith(".exe") ? file : file + ".exe";
    const projectResourcePath = this.state!.path;
    const buildPath = tempWith(
      `${this.state!.name}_${this.state!.uuid.slice(0, 8)}`
    );
    const indexesPath = pathJoin(buildPath, indexesKey);
    const dataPath = pathJoin(buildPath, dataKey);

    const [progress, close] = modal.progress(
      "正在构建应用",
      "正在构建应用, 请稍候..."
    );

    progress.addTask("正在准备构建环境...", async () => {
      await fs.createDirAll(buildPath);
    });

    let fileIndexes: Record<string, [number, number]> = {};
    let buffer = new ArrayBuffer(0);
    progress.addTask("正在构建资源文件...", async () => {
      const [_fileIndex, _buffer] = await packageResource(projectResourcePath);
      fileIndexes = _fileIndex;
      buffer = _buffer;
    });

    progress.addTask("正在压缩资源文件...", async () => {
      const compressedBuffer = pako.deflateRaw(buffer).buffer;
      await Promise.all([
        fs.write(indexesPath, JSON.stringify(fileIndexes, null, 2)),
        fs.write(dataPath, arrayBufferToBase64(compressedBuffer), "base64"),
      ]);
    });

    progress.addTask("正在生成图标...", async () => {
      if (!this.state!.config.icon) {
        return;
      }
      const iconPath = pathJoin(this.state!.path, this.state!.config.icon);
      await resource.extract(
        "windows/icon_creator.exe",
        pathJoin(buildPath, "icon_creator.exe")
      );
      await process.exec(pathJoin(buildPath, "icon_creator.exe"), [
        iconPath,
        pathJoin(buildPath, "icon.ico"),
        pathJoin(buildPath, "icon.bitmap"),
      ]);
    });

    progress.addTask("正在构建可执行文件...", async () => {
      await resource.extract(
        "windows/ResourceHacker.exe",
        pathJoin(buildPath, "ResourceHacker.exe")
      );

      const versionInfoPath = pathJoin(buildPath, "VERSION_INFO");
      await fs.write(versionInfoPath, versionInfoTemplate(this.state!.config));

      const iconScript = `
-delete ICON,1,0
-delete ICON,2,0
-delete ICON,3,0
-delete ICON,4,0
-delete ICON,5,0
-delete ICON,6,0
-delete ICON,7,0
-addoverwrite ${pathJoin(buildPath, "icon.ico")}, ICONGROUP,1,1033
-addoverwrite ${pathJoin(buildPath, "icon.bitmap")}, RCDATA,ICON_BITMAP,1033
`;

      const script = `
[FILENAMES]
Exe=    ${currentExe}
SaveAs= ${targetExe}
Log=    ${pathJoin(buildPath, "ResourceHacker.log")}
[COMMANDS]
-addoverwrite ${indexesPath}, RCDATA,${indexesKey},1033
-addoverwrite ${dataPath}, RCDATA,${dataKey},1033
${this.state!.config.icon ? iconScript : ""}
`;

      await fs.write(pathJoin(buildPath, "bundle_script.txt"), script);

      await process.exec(pathJoin(buildPath, "ResourceHacker.exe"), [
        "-script",
        pathJoin(buildPath, "bundle_script.txt"),
      ]);
    });

    progress.addTask("正在清理构建环境...", async () => {
      await fs.remove(buildPath);
    });

    await progress.run();
    close();

    return targetExe;
  }

  private async buildMacOsApp(_target?: string) {
    const currentExe = await process.currentExe();
    const file = _target || (await dialog.saveFile(["app"]));

    if (!file) {
      throw new Error("未选择app文件");
    }

    const appPath = file.endsWith(".app") ? file : file + ".app";
    const appContentsPath = pathJoin(appPath, "Contents");
    const appResourcesPath = pathJoin(appContentsPath, "Resources");
    const appMacOSPath = pathJoin(appContentsPath, "MacOS");
    const appExecutablePath = pathJoin(appMacOSPath, this.state!.name);
    const appInfoPlistPath = pathJoin(appContentsPath, "Info.plist");
    const appIconPath = pathJoin(appResourcesPath, "icon.icns");
    const appIconsetPath = pathJoin(appResourcesPath, "icon.iconset");

    const projectResourcePath = this.state!.path;
    const indexesPath = pathJoin(appResourcesPath, indexesKey);
    const dataPath = pathJoin(appResourcesPath, dataKey);

    const [progress, close] = modal.progress(
      "正在构建应用",
      "正在构建应用, 请稍候..."
    );

    progress.addTask("正在创建目录结构...", async () => {
      // make base structure
      await fs.createDir(appPath);
      await fs.createDir(appContentsPath);
      await fs.createDir(appResourcesPath);
      await fs.createDir(appIconsetPath);
      await fs.createDir(appMacOSPath);
    });


    progress.addTask("正在复制可执行文件...", async () => {
      await fs.copy(currentExe, appExecutablePath);
    });

    let fileIndexes: Record<string, [number, number]> = {};
    let buffer = new ArrayBuffer(0);
    progress.addTask("正在构建资源文件...", async () => {
      const [_fileIndex, _buffer] = await packageResource(projectResourcePath);
      fileIndexes = _fileIndex;
      buffer = _buffer;
    });

    progress.addTask("正在压缩资源文件...", async () => {
      const compressedBuffer = pako.deflateRaw(buffer).buffer;
      await Promise.all([
        fs.write(indexesPath, JSON.stringify(fileIndexes, null, 2)),
        fs.write(dataPath, arrayBufferToBase64(compressedBuffer), "base64"),
      ]);
    });

    progress.addTask("正在生成图标...", async () => {
      // create icon
      if (!this.state!.config.icon) {
        return;
      }

      const iconPath = pathJoin(this.state!.path, this.state!.config.icon);

      for (let size of [16, 32, 64, 128, 256, 512, 1024]) {
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

    progress.addTask("正在生成Info.plist配置...", async () => {
      // generate Info.plist
      await fs.write(appInfoPlistPath, plistTemplate(this.state!.config));
    });

    await progress.run();
    close();

    return appPath;
  }
}

