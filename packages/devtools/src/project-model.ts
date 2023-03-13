import { StateModel } from "@bramblex/state-model";
import { generatePlist } from "./plist-template";
import { tryOr, uuid } from "./utils";

interface ProjectState {
  name: string;
  uuid: string;
  path: string;
  config: any;
}

export class ProjectModel extends StateModel<ProjectState | null> {
  private sep!: string;

  constructor() {
    super(null);
  }

  async init(path: string) {
    if (!this.sep) {
      const { sep } = await TauriLite.api.os.sep();
      this.sep = sep;
    }

    // check path is a directory
    const {
      metadata: { isDir },
    } = await tryOr(
      () => TauriLite.api.fs.stat({ path }),
      async () => ({ metadata: { isDir: false } })
    );

    if (!isDir) {
      TauriLite.api.dialog.showMessage({
        title: "导入项目失败",
        content: `'${path}' 不是一个目录, 请选择一个目录`,
      });
      return;
    }

    // check tauri_lite.json exists, if not create it
    const config = await this.loadOrCreateConfig(path);

    this.setState({
      name: config.name,
      uuid: config.uuid,
      path,
      config,
    });
  }

  getConfigPath() {
    return [this.state!.path, "tauri_lite.json"].join(this.sep);
  }

  private async loadOrCreateConfig(path: string) {
    const configPath = [path, "tauri_lite.json"].join(this.sep);
    return await tryOr(
      async () => {
        const { content } = await TauriLite.api.fs.read({ path: configPath });
        const config = JSON.parse(content);
        if (!config.name || !config.uuid) {
          config.name = config.name || "tauri_lite-project";
          config.uuid = config.uuid || uuid();
          await TauriLite.api.fs.write({
            path: configPath,
            content: JSON.stringify(config, null, 2),
          });
        }
        return config;
      },
      async () => {
        const defaultConfig = {
          name: "tauri_lite-project",
          uuid: uuid(),
        };
        await TauriLite.api.fs.write({
          path: configPath,
          content: JSON.stringify(defaultConfig, null, 2),
        });
        return defaultConfig;
      }
    );
  }

  // operations
  open() {
    TauriLite.api.process.open({ uri: this.state!.path });
  }

  close() {
    this.setState(null);
  }

  edit() {
    TauriLite.api.process.open({ uri: this.getConfigPath() });
  }

  async debug() {
    const { exe } = await TauriLite.api.process.currentExe();
    TauriLite.api.process.exec({
      cmd: exe,
      args: ["--work-dir", this.state!.path],
      detached: true,
    });
  }

  async build() {
    const { os } = await TauriLite.api.os.info();
    if (os.toLowerCase() === "macos") {
      return this.buildMacOsApp();
    } else if (os.toLowerCase() === "windows") {
      return this.buildWindowsApp();
    }
  }

  async buildWindowsApp() {
    const { exe } = await TauriLite.api.process.currentExe();
    const { cwd } = await TauriLite.api.process.cwd();
    const { home } = await TauriLite.api.os.dirs();

    const { file } = await TauriLite.api.dialog.saveFile({
      filters: ['exe'],
    });

    const {
      path, name, config,
    } = this.state!;

    const mainExePath = [path, name + '.exe'].join(this.sep);
    const iconPath = [path, config.icon].join(this.sep);

    await TauriLite.api.fs.cp({ from: exe, to: mainExePath });
    await TauriLite.api.process.exec({
      cmd: [cwd, 'appacker.exe'].join(this.sep),
      args: [
        '-s', path,
        '-e', name + '.exe',
        '-i', iconPath,
        '-d', file,
      ],
    });
    await TauriLite.api.fs.rm({ path: mainExePath });
  }

  async buildMacOsApp() {
    const { exe } = await TauriLite.api.process.currentExe();
    const { home } = await TauriLite.api.os.dirs();
    const { file } = await TauriLite.api.dialog.saveFile({
      filters: ['app'],
      startDir: home,
    });

    const appPath = file;
    const appContentsPath = [appPath, "Contents"].join(this.sep);
    const appResourcesPath = [appContentsPath, "Resources"].join(this.sep);
    const appMacOSPath = [appContentsPath, "MacOS"].join(this.sep);
    const executablePath = [appMacOSPath, this.state!.name].join(this.sep);
    const appInfoPlistPath = [appContentsPath, "Info.plist"].join(this.sep);
    const appIconPath = [appResourcesPath, "icon.icns"].join(this.sep);
    const appIconsetPath = [appResourcesPath, "icon.iconset"].join(this.sep);

    // make base structure
    await TauriLite.api.fs.mkDir({ path: appPath });
    await TauriLite.api.fs.mkDir({ path: appContentsPath });
    await TauriLite.api.fs.mkDir({ path: appResourcesPath });
    await TauriLite.api.fs.mkDir({ path: appIconsetPath });

    await TauriLite.api.fs.cp({ from: this.state!.path, to: appMacOSPath });
    await TauriLite.api.fs.cp({ from: exe, to: executablePath });

    // create icon
    let iconPath =
      [this.state!.path, this.state!.config.icon].join(this.sep) ||
      "./logo.png";
    for (let size of [16, 32, 64, 128, 256, 512, 1024]) {
      await TauriLite.api.process.exec({
        cmd: "sips",
        args: [
          "-z", size.toString(), size.toString(), iconPath,
          "--out", [appIconsetPath, `icon_${size}x${size}.png`].join(this.sep),
        ],
      });
    }
    await TauriLite.api.process.exec({
      cmd: "iconutil",
      args: [
        "-c", "icns", appIconsetPath,
        "-o", appIconPath
      ],
    });

    // generate Info.plist
    await TauriLite.api.fs.write({
      path: appInfoPlistPath,
      content: generatePlist(this.state!.config),
    });
  }
}
