import { StateModel } from "@bramblex/state-model";
import { spawn } from "node:child_process";
import { fs, fileExists, process } from "../common/node";
import { runPackager, hostBuildTarget, requireTargetSuccess, type PackagerTargetResult } from "../build-scripts/packager";
import { AppModel } from "./app.model";
import { fileSystemUrl, pathJoin } from "../common/utils";
import { openExternal } from "../common/niva";
import {
  AppResult,
  Err,
  Ok,
  fromThrowable,
  fromThrowableAsync,
} from "../common/result";
import { ErrorCode } from "../common/error";

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
    this.update({
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
    // Keep accidental direct reloads (including future reset call sites) from
    // replacing an active editor buffer without the unsaved-changes flow.
    if (
      this.app.state.project === this &&
      this.state.editor.state.isEdit
    ) {
      return Err(ErrorCode.PROJECT_HAS_UNSAVED_CHANGE);
    }

    return this.loadConfig();
  }

  private async loadConfig(): Promise<AppResult> {
    const { path, configPath } = this.state;

    const loadResult = await fromThrowableAsync(async () => {
      const configContent = await fs.readFile(configPath, "utf8");
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

    this.update({
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
    const hadEdits = this.state.editor.state.isEdit;
    const result = await this.resolveUnsavedChanges();
    if (result.isErr() || !hadEdits) return result;

    const wasSaved = !this.state.editor.state.isEdit;
    const reloadResult = await this.loadConfig();
    if (reloadResult.isErr()) return reloadResult;
    if (wasSaved) await this.app.state.history.record(this);
    return Ok(void 0);
  }

  async refresh() {
    const result = await this.resolveUnsavedChanges();
    if (result.isErr()) {
      return result;
    }

    const loadResult = await this.loadConfig();
    if (loadResult.isErr()) {
      return loadResult;
    }
    await this.app.state.history.record(this);
    return Ok(void 0);
  }

  /** Reloads the persisted config after an explicit save/discard decision. */
  async reset(): Promise<AppResult> {
    return this.refresh();
  }

  async save(): Promise<AppResult> {
    if (!this.state.editor.state.isEdit) {
      return Ok(void 0);
    }

    const saveResult = await this.persistEdits();
    if (saveResult.isErr()) {
      return saveResult;
    }
    return this.refresh();
  }

  private async resolveUnsavedChanges(): Promise<AppResult> {
    const { editor } = this.state;
    if (!editor.state.isEdit) {
      return Ok(void 0);
    }

    const { modal } = this.app.state;
    const decision = await modal.confirmUnsaved();
    if (decision === "cancel") {
      return Err(ErrorCode.PROJECT_HAS_UNSAVED_CHANGE);
    }
    if (decision === "save") {
      return this.persistEdits();
    }

    // Discard is intentionally deferred until a successful reload replaces
    // the editor model. If disk loading fails, the user's draft is preserved.
    return Ok(void 0);
  }

  private async prepareForAction(): Promise<AppResult> {
    const hadUnsavedChanges = this.state.editor.state.isEdit;
    if (!hadUnsavedChanges) {
      return Ok(void 0);
    }

    const result = await this.resolveUnsavedChanges();
    if (result.isErr()) {
      return result;
    }

    // Both saving and discarding affect the editor's in-memory view of the
    // project config. Reload before actions that consume config.state.
    return this.loadConfig();
  }

  private async persistEdits(): Promise<AppResult> {
    const { isEdit, content } = this.state.editor.state;
    if (!isEdit) {
      return Ok(void 0);
    }

    const validateResult = ProjectModel.validateConfig(content);
    if (validateResult.isErr()) {
      return Err(ErrorCode.SAVE_CONFIG_VALIDATE_FAILED, { content });
    }

    const saveResult = await fromThrowableAsync(async () =>
      fs.writeFile(this.state.configPath, content, "utf8")
    );
    if (saveResult.isErr()) {
      return Err(ErrorCode.SAVE_CONFIG_FAILED, {
        reason: saveResult.error,
      });
    }

    this.state.editor.update({ content, isEdit: false });
    return Ok(void 0);
  }

  async build(outputDirectory = process.cwd()): Promise<AppResult<PackagerTargetResult>> {
    const prepareResult = await this.prepareForAction();
    if (prepareResult.isErr()) {
      return Err(prepareResult.error.code, prepareResult.error.extra);
    }
    return fromThrowableAsync(async () => {
      const kitDirectory = window.localStorage.getItem("niva-devtools-packager-kit");
      if (!kitDirectory) throw new Error("Select a packaging kit before running a CLI build.");
      const target = hostBuildTarget();
      const execution = await runPackager(this, kitDirectory, outputDirectory, [target]);
      return requireTargetSuccess(execution, target);
    });
  }

  async debug(): Promise<AppResult> {
    const prepareResult = await this.prepareForAction();
    if (prepareResult.isErr()) {
      return prepareResult;
    }

    const { path, configPath, config } = this.state;
    const resource = pathJoin(path, config?.debug?.resource);
    const entry = config?.debug?.entry || "";

    if (!(await fileExists(resource))) {
      return Err(ErrorCode.DEBUG_RESOURCE_NOT_FOUND, { resource });
    }

    return fromThrowableAsync(async () => {
      const child = spawn(
        process.execPath,
        [
          `--config=${configPath}`,
          `--resource=${resource}`,
          "--debug-devtools=true",
          ...(entry ? [`--debug-entry=${entry}`] : []),
        ],
        { detached: true },
      );
      await new Promise<void>((resolve, reject) => {
        let settled = false;
        child.once("spawn", () => {
          if (settled) return;
          settled = true;
          child.unref();
          resolve();
        });
        child.once("error", (error: Error) => {
          if (settled) return;
          settled = true;
          reject(error);
        });
        child.once("close", (code: number | null, signal: string | null) => {
          if (settled) return;
          settled = true;
          if (code === 0 || (code === null && signal === null)) resolve();
          else reject(new Error(`Niva debug process exited (${signal || code}).`));
        });
      });
    });
  }

  open(): Promise<AppResult> {
    return fromThrowableAsync(() => openExternal(this.state.path));
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
