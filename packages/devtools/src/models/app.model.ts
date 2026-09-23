import { StateModel } from "@bramblex/state-model";
import { HistoryModel } from "./history.model";
import {useModel} from "../common/state";
import { createContext, useContext } from "react";
import { ProjectModel } from "./project.model";
import { ModalModel } from "./modal.model";
import { checkVersion, pathJoin, tryOrAlert } from "../common/utils";
import { Err, Ok, AppResult, fromThrowableAsync } from "../common/result";

import { ErrorCode } from "../common/error";
import { ConfigType, generateConfig } from "../templates/config-template";
import { LocaleModel } from "./locale.model";
import { generateNewProject } from "../templates/new-project-template";
import { initEndPromise } from "../app";

const { fs } = Niva.api;

export class AppModel extends StateModel<{
  history: HistoryModel;
  modal: ModalModel;
  project: ProjectModel | null;
  locale: LocaleModel;
  availableVersion: string | null;
}> {
  constructor() {
    super({} as any);
    this.update({
      history: new HistoryModel(this),
      modal: new ModalModel(this),
      locale: new LocaleModel(this),
      project: null,
      availableVersion: null,
    });
  }

  async init() {
    Niva.addEventListener("window.closeRequested", () =>
      tryOrAlert(this, this.exit())
    );
    const { history, locale } = this.state;
    await Promise.all([history.init(), locale.init()])
    initEndPromise.then(async () => {
      const availableVersion = await checkVersion();
      if (availableVersion) {
        this.update({ ...this.state, availableVersion });
      }
    })
  }

  async openWithPicker(): Promise<AppResult> {
    const { modal } = this.state;
    const path = await modal.showNative<string | null>(() =>
      Niva.api.dialog.pickDir()
    );

    if (path) {
      return this.open(path);
    }
    return Ok(void 0);
  }

  async open(path: string): Promise<AppResult> {
    const { modal, locale } = this.state;

    // first check if the path is a valid project path
    const configPath = pathJoin(path, "niva.json");
    const packageJsonPath = pathJoin(path, "package.json");
    const [isExists, isConfigExists, isPackageJsonExists] = await Promise.all([
      fs.exists(path),
      fs.exists(configPath),
      fs.exists(packageJsonPath),
    ]);

    if (!isExists) {
      return Err(ErrorCode.PROJECT_PATH_NOT_EXISTS, { path });
    }

    const { isDir } = await fs.stat(path);
    if (!isDir) {
      return Err(ErrorCode.PROJECT_PATH_IS_NOT_DIR, { path });
    }

    if (!isConfigExists) {
      if (
        !(await modal.confirm(
          locale.t("WARNING"),
          locale.t("PROJECT_CREATE_CONFIG_WHERE_NOT_FOUND")
        ))
      ) {
        // Declining optional config creation is a user cancellation, not an
        // invalid project error. In particular, keep the currently open
        // project untouched.
        return Ok(void 0);
      }

      let projectName = path.split(/\/|\\/).pop() as string;
      const configType: AppResult<ConfigType> = isPackageJsonExists
        ? await fromThrowableAsync<ConfigType>(async () => {
          const packageJson = JSON.parse(await fs.read(packageJsonPath));
          if (packageJson?.name) {
            projectName = packageJson.name;
          }
          if (packageJson.dependencies["react-scripts"]) {
            return "react";
          } else if (packageJson.devDependencies["vite"]) {
            return "vueVite";
          } else if (packageJson.dependencies["vue"]) {
            return "vue";
          } else {
            return "simple";
          }
        })
        : Ok("simple");

      const configContent = generateConfig(
        configType.unwrapOr("simple"),
        projectName
      );

      const createConfigFileResult = await fromThrowableAsync(async () => {
        await fs.write(configPath, JSON.stringify(configContent, null, 2));
      });

      if (createConfigFileResult.isErr()) {
        return Err(ErrorCode.PROJECT_CONFIG_CRATE_FAILED, {
          configPath,
          reason: createConfigFileResult.error,
        });
      }
    }

    const { project: currentProject } = this.state;
    if (currentProject?.state.path === path) {
      // Reopening the active path should reload the same model after resolving
      // its edits, so a just-saved config cannot be replaced by a stale
      // candidate loaded before the save.
      return currentProject.refresh();
    }

    const project = new ProjectModel(this, path);
    const projectInitResult = await project.init();
    if (projectInitResult.isErr()) {
      return projectInitResult;
    }

    if (currentProject) {
      const disposeResult = await currentProject.dispose();
      if (disposeResult.isErr()) {
        return disposeResult;
      }
    }

    this.update({
      ...this.state,
      project,
    });

    await this.state.history.record(project);
    return Ok(void 0);
  }

  async close(): Promise<AppResult> {
    const { project } = this.state;
    if (project) {
      const result = await project.dispose();
      if (result.isErr()) {
        return result;
      }
      this.update({
        ...this.state,
        project: null,
      });
    }
    return Ok(void 0);
  }

  async create(): Promise<AppResult> {
    const { modal } = this.state;
    const requestedName = await modal.promptProjectName();
    if (requestedName === null) {
      return Ok(void 0);
    }

    const projectName = requestedName.trim();
    if (
      !projectName ||
      projectName === "." ||
      projectName === ".." ||
      /[\\/]/.test(projectName)
    ) {
      return Err(ErrorCode.PROJECT_CREATE_FAILED, {
        reason: "Project name must be a single non-empty directory name.",
      });
    }

    const parentPath = await modal.showNative<string | null>(() =>
      Niva.api.dialog.pickDir()
    );
    if (!parentPath) {
      return Ok(void 0);
    }

    const path = pathJoin(parentPath, projectName);
    if (await fs.exists(path)) {
      return Err(ErrorCode.PROJECT_ALREADY_EXISTS, { path });
    }

    // Resolve the current editor before writing a new project. Cancelling the
    // switch must not leave an unused project folder behind.
    if (this.state.project) {
      const leaveResult = await this.state.project.refresh();
      if (leaveResult.isErr()) {
        return leaveResult;
      }
    }

    // createDir is atomic and fails if another item appears after the
    // preflight check, so existing project data can never be overwritten.
    const createDirectoryResult = await fromThrowableAsync(() =>
      fs.createDir(path)
    );
    if (createDirectoryResult.isErr()) {
      if (await fs.exists(path)) {
        return Err(ErrorCode.PROJECT_ALREADY_EXISTS, { path });
      }
      return Err(ErrorCode.PROJECT_CREATE_FAILED, {
        path,
        reason: createDirectoryResult.error,
      });
    }

    const newProjectResult = await fromThrowableAsync(async () => {
      const files = generateNewProject(projectName);
      await Promise.all(
        files.map(([name, content]) => fs.write(pathJoin(path, name), content))
      );
    });

    if (newProjectResult.isErr()) {
      return Err(ErrorCode.PROJECT_CREATE_FAILED, {
        path,
        reason: newProjectResult.error,
      });
    }

    return this.open(path);
  }

  async exit(): Promise<AppResult> {
    const { modal } = this.state;

    if (modal.state.modals.length > 0) {
      return Err(ErrorCode.APP_EXIT_PREVENTED_BY_DIALOG);
    }

    const result = await this.close();
    if (result.isErr()) {
      return result;
    }

    Niva.api.window.close();
    return Ok(void 0);
  }
}

const AppModelContext = createContext<AppModel | null>(null);

export const AppModelProvider = AppModelContext.Provider;

function useAppModel(): AppModel {
  const app = useContext(AppModelContext);
  if (!app) {
    throw new Error("AppModel is not provided");
  }
  return app;
}

export function useApp() {
  const app = useAppModel();
  useModel(app);
  return app;
}

export function useHistory() {
  const { history } = useAppModel().state;
  useModel(history);
  return history;
}

export function useProject() {
  const { project } = useAppModel().state;
  if (!project) {
    throw new Error("No project is open");
  }
  useModel(project);
  return project;
}

export function useModal() {
  const { modal } = useAppModel().state;
  useModel(modal);
  return modal;
}

export function useLocale() {
  const { locale } = useAppModel().state;
  useModel(locale);
  return locale;
}
