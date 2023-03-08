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

    // check tauri-lite.json exists, if not create it
    const config = await this.loadOrCreateConfig(path);

    this.setState({
      name: config.name,
      uuid: config.uuid,
      path,
      config,
    });
  }

  getConfigPath() {
    return [this.state!.path, "tauri-lite.json"].join(this.sep);
  }

  private async loadOrCreateConfig(path: string) {
    const configPath = [path, "tauri-lite.json"].join(this.sep);
    return await tryOr(
      async () => {
        const { content } = await TauriLite.api.fs.read({ path: configPath });
        const config = JSON.parse(content);
        if (!config.name || !config.uuid) {
          config.name = config.name || "tauri-lite-project";
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
          name: "tauri-lite-project",
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
    const { exe } = await TauriLite.api.process.currentExe();
    const { home } = await TauriLite.api.os.dirs();
    const { file } = await TauriLite.api.dialog.saveFile({
      startDir: home,
    });

    const appPath = file;
    const appContentsPath = [appPath, "Contents"].join(this.sep);
    const appResourcesPath = [appContentsPath, "Resources"].join(this.sep);
    const appMacOSPath = [appContentsPath, "MacOS"].join(this.sep);
    const executablePath = [appMacOSPath, this.state!.name].join(this.sep);
    const appInfoPlistPath = [appContentsPath, "Info.plist"].join(this.sep);

    await TauriLite.api.fs.mkDir({ path: appPath });
    await TauriLite.api.fs.mkDir({ path: appContentsPath });
    await TauriLite.api.fs.mkDir({ path: appResourcesPath });
    await TauriLite.api.fs.cp({ from: this.state!.path, to: appMacOSPath });
    await TauriLite.api.fs.cp({ from: exe, to: executablePath });
    await TauriLite.api.fs.write({
      path: appInfoPlistPath,
      content: generatePlist(this.state!.config),
    });
  }
}
