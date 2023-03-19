import { StateModel } from "@bramblex/state-model";
import { generatePlist } from "./template";
import {
  arrayBufferToBase64,
  base64ToArrayBuffer,
  concatArrayBuffers,
  dirname,
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

const { os, fs, process, dialog, resource } = TauriLite.api;

interface ProjectState {
  name: string;
  uuid: string;
  path: string;
  configPath: string;
  config: any;
}

export class ProjectModel extends StateModel<ProjectState | null> {
  constructor() {
    super(null);
  }

  init(path: string) {
    return tryOrAlertAsync(async () => {
      // check path is a directory
      const { isDir } = await tryOrP(fs.stat(path), { isDir: false });

      if (!isDir) {
        modal.alert("错误", `'${path}' 不是一个目录, 请选择一个目录`);
        return;
      }

      // check tauri_lite.json exists, if not create it
      const configPath = pathJoin(path, "tauri_lite.json");
      const config = await this.loadOrCreateConfig(configPath);

      this.setState({
        name: config.name,
        uuid: config.uuid,
        path,
        configPath,
        config,
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
          name: "tauri_lite_project",
          uuid: uuid(),
        };
        await withCtxP(
          fs.write(configPath, JSON.stringify(defaultConfig, null, 2)),
          '创建项目配置文件 "tauri_lite.json" 失败'
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

  debug() {
    const projectPath = this.state!.path;
    const debugEntry = this.state!.config.debugEntry;

    return tryOrAlertAsync(async () => {
      const exe = await process.currentExe();
      process.exec(
        exe,
        [
          "--resource-dir",
          projectPath,
          "--devtools",
          "true",
          ...(debugEntry ? ["--debug-entry", debugEntry] : []),
        ],
        { detached: true }
      );
    });
  }

  build() {
    return tryOrAlertAsync(async () => {
      const { os: osType } = await os.info();

      let appPath: string;
      if (osType.toLowerCase().replace(/\s/g, "") === "macos") {
        appPath = await this.buildMacOsApp();
      } else if (osType.toLowerCase() === "windows") {
        appPath = await this.buildWindowsApp();
      } else {
        throw new Error(`不支持当前操作系统 "${osType}"`);
      }

      if (
        await modal.confirm("构建成功", `应用 ${this.state!.name} 已成功构建`)
      ) {
        process.open(dirname(appPath));
      }
    });
  }

  private async buildWindowsApp(): Promise<string> {
    const currentExe = await process.currentExe();
    const file = await dialog.saveFile(["exe"]);
    if (!file) {
      throw new Error("未选择exe文件");
    }
    const targetExe = file.endsWith(".exe") ? file : file + ".exe";

    const resourcePath = this.state!.path;
    const buildPath = tempWith(this.state!.name);
    const indexesKey = "TAURI_LITE_RESOURCE_INDEXES";
    const indexesPath = pathJoin(buildPath, indexesKey);
    const dataKey = "TAURI_LITE_RESOURCE_DATA";
    const dataPath = pathJoin(buildPath, dataKey);

    await fs.createDirAll(buildPath);

    const fileIndexes: Record<string, [number, number]> = {}
    let buffer = new ArrayBuffer(0);

    for (const name of await fs.readDirAll(resourcePath)) {
      const filePath = pathJoin(resourcePath, name);
      const fileKey = name.replace(/\\/g, "/");
      const fileBuffer = base64ToArrayBuffer(await fs.read(filePath, 'base64'));
      fileIndexes[fileKey] = [buffer.byteLength, fileBuffer.byteLength];
      buffer = concatArrayBuffers(buffer, fileBuffer);
    }

    await Promise.all([
      fs.write(indexesPath, JSON.stringify(fileIndexes, null, 2)),
      fs.write(dataPath, arrayBufferToBase64(buffer), 'base64'),
      resource.extract("ResourceHacker.exe", pathJoin(buildPath, "ResourceHacker.exe")),
    ])

    await fs.write(pathJoin(buildPath, "bundle_script.txt"), `
[FILENAMES]
Exe=    ${currentExe}
SaveAs= ${targetExe}
Log=    ${pathJoin(buildPath, "ResourceHacker.log")}
[COMMANDS]
-addoverwrite ${indexesPath}, RCDATA,${indexesKey},1033
-addoverwrite ${dataPath}, RCDATA,${dataKey},1033
`);

    await process.exec(
      pathJoin(buildPath, "ResourceHacker.exe"),
      ['-script', pathJoin(buildPath, "bundle_script.txt")]
    )

    return targetExe;
  }

  private async buildMacOsApp() {
    const exe = await process.currentExe();
    const file = await dialog.saveFile(["app"]);
    const currentDir = await process.currentDir();

    if (!file) {
      throw new Error("未选择app文件");
    }

    const appPath = file.endsWith(".app") ? file : file + ".app";
    const appContentsPath = pathJoin(appPath, "Contents");
    const appResourcesPath = pathJoin(appContentsPath, "Resources");
    const appMacOSPath = pathJoin(appContentsPath, "MacOS");
    const executablePath = pathJoin(appMacOSPath, this.state!.name);
    const appInfoPlistPath = pathJoin(appContentsPath, "Info.plist");
    const appIconPath = pathJoin(appResourcesPath, "icon.icns");
    const appIconsetPath = pathJoin(appResourcesPath, "icon.iconset");

    const [progress, close] = modal.progress("正在构建应用", "正在构建应用, 请稍候...");

    progress.addTask("正在创建目录结构...", async () => {
      // make base structure
      await fs.createDir(appPath);
      await fs.createDir(appContentsPath);
      await fs.createDir(appResourcesPath);
      await fs.createDir(appIconsetPath);
      await fs.createDir(appMacOSPath);
    });

    progress.addTask("正在复制项目文件...", async () => {
      await fs.copy(this.state!.path, appResourcesPath, {
        contentOnly: true,
      });
      await fs.copy(exe, executablePath);
    });

    progress.addTask("正在生成图标...", async () => {
      // create icon
      const iconPath =
        pathJoin(this.state!.path, this.state!.config.icon) ||
        pathJoin(currentDir, "logo.png");

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
      await fs.write(appInfoPlistPath, generatePlist(this.state!.config));
    });

    await progress.run();
    close();

    return appPath;
  }
}
