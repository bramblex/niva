import { StateModel } from "@bramblex/state-model";
import { AppModel } from "./app.model";
import { dirname, fileSystemUrl, pathJoin } from "../common/utils";
import {
  AppResult,
  Err,
  Ok,
  fromThrowable,
  fromThrowableAsync,
} from "../common/result";
import { ErrorCode } from "../common/error";
import { buildMacOsApp } from "../build-scripts/build-macos";
import { buildWindowsApp } from "../build-scripts/build-windows";

const { fs, process, os } = Niva.api;

interface ProjectEditorModelState {
  content: string;
  isEdit: boolean;
}

export class ProjectEditorModel extends StateModel<ProjectEditorModelState> {
  constructor(content: string) {
    super({
      content: content,
      isEdit: false,
    });
  }

  setContent(content: string) {
    this.setState({
      content,
      isEdit: true,
    });
  }
}

interface ProjectModelState {
  path: string;
  configPath: string;

  icon: string | null;
  name: string;
  uuid: string;
  config: any;

  editor: ProjectEditorModel;
}

export class ProjectModel extends StateModel<ProjectModelState> {
  constructor(readonly app: AppModel, readonly path: string) {
    super({
      path,
      configPath: pathJoin(path, "niva.json"),

      icon: null,
      name: "",
      uuid: "",

      config: {},
      editor: new ProjectEditorModel(""),
    });
  }

  async init(): Promise<AppResult> {
    const { path, configPath } = this.state;

    const loadResult = await fromThrowableAsync(async () => {
      const configContent = await fs.read(configPath);
      const config = JSON.parse(configContent);
      config.__rawContent__ = configContent;
      return config;
    });

    if (loadResult.isErr()) {
      return Err(ErrorCode.PROJECT_LOAD_CONFIG_FAILED, {
        path,
        configPath,
        reason: loadResult.error,
      });
    }

    const validateResult = ProjectModel.validateConfig(loadResult.value);

    if (validateResult.isErr()) {
      return validateResult;
    }

    const config = validateResult.value;

    this.setState({
      ...this.state,

      icon: config.icon
        ? fileSystemUrl(pathJoin(path, config.debug?.resource, config.icon))
        : null,
      name: config.name,
      uuid: config.uuid,

      config,
      editor: new ProjectEditorModel(config.__rawContent__),
    });

    return Ok(void 0);
  }

  async dispose(): Promise<AppResult> {
    const { modal, locale } = this.app.state;
    const { isEdit } = this.state.editor.state;
    if (isEdit) {
      if (
        (await modal.confirm(locale.t("WARNING"), locale.t("UNSAVED"))) === true
      ) {
        return this.save();
      }
    }
    return Ok(void 0);
  }

  async refresh() {
    const result = await this.dispose();
    if (result.isErr()) {
      return result;
    }
    await this.app.state.history.record(this);
    return this.init();
  }

  async save(): Promise<AppResult> {
    const { isEdit, content } = this.state.editor.state;
    if (!isEdit) {
      return Ok(void 0);
    }
    const validateResult = ProjectModel.validateConfig(content);
    if (validateResult.isErr()) {
      return Err(ErrorCode.SAVE_CONFIG_VALIDATE_FAILED, { content });
    }
    const saveResult = await fromThrowableAsync(async () =>
      fs.write(this.state.configPath, content)
    );
    
    if (saveResult.isErr()) {
      return Err(ErrorCode.SAVE_CONFIG_FAILED, {
        reason: saveResult.error,
      });
    }
    this.state.editor.setState({content, isEdit: false})
    return this.refresh();
  }

  async build(target?: string): Promise<AppResult> {
    const { modal, locale } = this.app.state;

    return await fromThrowableAsync(async () => {
      const { os: osType } = await os.info();

      let appPath: string;
      if (osType.toLowerCase().replace(/\s/g, "") === "macos") {
        appPath = await buildMacOsApp(this, target);
      } else if (osType.toLowerCase() === "windows") {
        appPath = await buildWindowsApp(this, target);
      } else {
        throw new Error(`${locale.t("UNSUPPORTED_OS")}"${osType}"`);
      }

      modal
        .confirm(locale.t("BUILD_SUCCESS"), locale.t("BUILD_SUCCESS_MESSAGE"))
        .then((ok) => ok && process.open(dirname(appPath)));
    });
  }

  async debug(): Promise<AppResult> {
    const { path, configPath, config } = this.state;
    const resource = pathJoin(path, config?.debug?.resource);
    const entry = config?.debug?.entry || "";

    return fromThrowableAsync(async () => {
      const exe = await process.currentExe();
      process.exec(
        exe,
        [
          `--debug-config=${configPath}`,
          `--debug-resource=${resource}`,
          "--debug-devtools=true",
          ...(entry ? [`--debug-entry=${entry}`] : []),
        ],
        { detached: true }
      );
    });
  }

  open(): Promise<AppResult> {
    return fromThrowableAsync(() => process.open(this.state.path));
  }

  private static validateConfig(rawConfig: any): AppResult<any> {
    let config = rawConfig;
    if (typeof config === "string") {
      const configResult = fromThrowable(() => JSON.parse(config));
      if (configResult.isErr()) {
        return Err(ErrorCode.PROJECT_CONFIG_VALIDATE_FAILED);
      }
      config = configResult.value;
    }

    if (config && config.name && config.uuid) {
      return Ok(config);
    }
    return Err(ErrorCode.PROJECT_CONFIG_VALIDATE_FAILED);
  }
}
