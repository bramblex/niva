import { StateModel } from "@bramblex/state-model";
import { HistoryModel } from "./history.model";
import { useModel, useModelContext } from "@bramblex/state-model-react";
import { ProjectModel } from "./project.model";
import { ModalModel } from "./modal.model";
import { checkVersion, pathJoin, pathSplit, tryOrAlert } from "../common/utils";
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
}> {
  constructor() {
    super({} as any);
    this.setState({
      history: new HistoryModel(this),
      modal: new ModalModel(this),
      locale: new LocaleModel(this),
      project: null,
    });
  }

  async init() {
    Niva.addEventListener("window.closeRequested", () =>
      tryOrAlert(this, this.exit())
    );
    const { history, locale, modal } = this.state;
    await Promise.all([history.init(), locale.init()])
    initEndPromise.then(() => {
      checkVersion(modal, locale);
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
    const { modal, locale, project: lastProject } = this.state;

    const result = await this.close();
    if (result.isErr()) {
      return result;
    }

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
        return Err(ErrorCode.PROJECT_CONFIG_NOT_EXISTS, { configPath });
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

    const project = new ProjectModel(this, path);
    const projectInitResult = await project.init();

    this.setState({
      ...this.state,
      project,
    });

    this.state.history.record(project);
    return projectInitResult;
  }

  async close(): Promise<AppResult> {
    const { project } = this.state;
    if (project) {
      const result = await project.dispose();
      if (result.isErr()) {
        return result;
      }
      this.setState({
        ...this.state,
        project: null,
      });
    }
    return Ok(void 0);
  }

  async create(): Promise<AppResult> {
    const { modal } = this.state;
    const path = await modal.showNative<string | null>(() =>
      Niva.api.dialog.saveFile()
    );

    if (!path) {
      return Ok(void 0);
    }

    const newProjectResult = await fromThrowableAsync(async () => {
      await fs.createDirAll(path);
      const projectName = pathSplit(path).pop();
      const files = generateNewProject(projectName || "niva-new-project");
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

    if (modal.state.length > 0) {
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

export function useApp() {
  const app = useModelContext(AppModel);
  return useModel(app);
}

export function useHistory() {
  const { history } = useModelContext(AppModel).state;
  return useModel(history);
}

export function useProject() {
  const { project } = useModelContext(AppModel).state;
  if (!project) {
    throw new Error("No project is open");
  }
  return useModel(project);
}

export function useModal() {
  const { modal } = useModelContext(AppModel).state;
  return useModel(modal);
}

export function useLocale() {
  const { locale } = useModelContext(AppModel).state;
  return useModel(locale);
}
