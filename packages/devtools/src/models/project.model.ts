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
import { signMacOsApp } from "../build-scripts/sign-macos";
import { signWindowsApp } from "../build-scripts/sign-windows";

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
    return this.resolveUnsavedChanges();
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
      fs.write(this.state.configPath, content)
    );
    if (saveResult.isErr()) {
      return Err(ErrorCode.SAVE_CONFIG_FAILED, {
        reason: saveResult.error,
      });
    }

    this.state.editor.update({ content, isEdit: false });
    return Ok(void 0);
  }

  async build(target?: string): Promise<AppResult> {
    const prepareResult = await this.prepareForAction();
    if (prepareResult.isErr()) {
      return prepareResult;
    }

    const { modal, locale } = this.app.state;
    let phase: "build" | "sign" | "open-output" = "build";
    let platform: "macos" | "windows";
    let appPath: string;

    try {
      const { os: osType } = await os.info();
      const [progress, close] = modal.progress(locale.t("BUILDING_APP"));
      try {
        const buildParams = {
          project: this,
          progress,
          file: null as string | null,
        };

        if (osType.toLowerCase().replace(/\s/g, "") === "macos") {
          platform = "macos";
          buildParams.file =
            target || (await Niva.api.dialog.saveFile(["app"]));
          if (!buildParams.file) {
            return Ok(void 0);
          }
          appPath = await buildMacOsApp(buildParams);
        } else if (osType.toLowerCase() === "windows") {
          platform = "windows";
          buildParams.file =
            target || (await Niva.api.dialog.saveFile(["exe"]));
          if (!buildParams.file) {
            return Ok(void 0);
          }
          appPath = await buildWindowsApp(buildParams);
        } else {
          throw new Error(`${locale.t("UNSUPPORTED_OS")}"${osType}"`);
        }

        await progress.run();
      } finally {
        close();
      }

      phase = "sign";
      await this.signIfConfigured(platform, appPath);

      // A caller-supplied target is the non-interactive CLI path. It must be
      // able to await a definitive build result without a success prompt.
      if (!target) {
        phase = "open-output";
        const shouldOpen = await modal.confirm(
          locale.t("BUILD_SUCCESS"),
          locale.t("BUILD_SUCCESS_MESSAGE")
        );
        if (shouldOpen) {
          await process.open(dirname(appPath));
        }
      }

      return Ok(void 0);
    } catch (error) {
      return Err(ErrorCode.UNKNOWN, {
        phase,
        reason: error instanceof Error ? error.message : String(error),
      });
    }
  }

  /**
   * One-stop signing after a successful build. The `sign` section is
   * optional; secrets never live in niva.json (keychain profile on macOS,
   * env vars for passwords — see sign-*.ts).
   */
  private async signIfConfigured(
    platform: "macos" | "windows",
    appPath: string
  ): Promise<void> {
    const { modal, locale } = this.app.state;
    const sign = this.state.config.sign;
    if (platform === "macos" && sign?.macos) {
      const [progress, close] = modal.progress(locale.t("SIGNING_APP"));
      try {
        await signMacOsApp({ project: this, progress, appPath });
        await progress.run();
      } finally {
        close();
      }
    } else if (platform === "windows" && sign?.windows) {
      const [progress, close] = modal.progress(locale.t("SIGNING_APP"));
      try {
        await signWindowsApp({ project: this, progress, exePath: appPath });
        await progress.run();
      } finally {
        close();
      }
    }
  }

  async debug(): Promise<AppResult> {
    const prepareResult = await this.prepareForAction();
    if (prepareResult.isErr()) {
      return prepareResult;
    }

    const { path, configPath, config } = this.state;
    const resource = pathJoin(path, config?.debug?.resource);
    const entry = config?.debug?.entry || "";

    if (!(await fs.exists(resource))) {
      return Err(ErrorCode.DEBUG_RESOURCE_NOT_FOUND, { resource });
    }

    return fromThrowableAsync(async () => {
      const exe = await process.currentExe();
      await process.exec(
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
