import { StateModel } from "@bramblex/state-model";
import { AppModel } from "./app.model";
import { fileSystemUrl, pathJoin } from "../common/utils";
import { AppResult, Err, Ok, fromThrowableAsync } from "../common/result";
import { ErrorCode } from "../common/error";

const { fs } = Niva.api;

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
      isEdit: false,
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

    const validateResult = this.validateConfig(loadResult.value);

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
        (await modal.confirm(
          locale.getTranslation("WARNING"),
          locale.getTranslation("UNSAVED")
        )) === false
      ) {
        return this.save();
      }
      return Err(ErrorCode.PROJECT_HAS_UNSAVED_CHANGE, {
        path: this.state.path,
      });
    }
    return Ok(void 0);
  }

  async save(): Promise<AppResult> {
    // @TODO:
    const result = fromThrowableAsync(async () => {});
    return this.init();
  }

  async build(): Promise<AppResult> {
    return Ok(void 0);
  }

  async debug(): Promise<AppResult> {
    return Ok(void 0);
  }

  open(): Promise<AppResult> {
    return fromThrowableAsync(() => Niva.api.process.open(this.state.path));
  }

  private validateConfig(config: any): AppResult<any> {
    if (config && config.name && config.uuid) {
      return Ok(config);
    }
    return Err(ErrorCode.PROJECT_CONFIG_VALIDATE_FAILED);
  }
}
