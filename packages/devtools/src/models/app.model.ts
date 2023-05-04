import { StateModel } from "@bramblex/state-model";
import { HistoryModel } from "./history.model";
import { useModel, useModelContext } from "@bramblex/state-model-react";
import { ProjectModel } from "./project.model";
import { ModalModel } from "./modal.model";
import { pathJoin } from "../common/utils";
import { Err, Ok, AppResult, fromThrowable, fromThrowableAsync } from "../common/result";

import { ErrorCode } from "../common/error";
import { ConfigType, generateConfig } from "../templates/config-template";
import { LocaleModel } from "./locale.model";
import { generateNewProject } from "../templates/new-project-template";

const {
	fs,
} = Niva.api;

export class AppModel extends StateModel<{
	history: HistoryModel,
	dialog: ModalModel,
	project: ProjectModel | null,
	locale: LocaleModel,
}> {

	constructor() {
		super({} as any);
		this.setState({
			history: new HistoryModel(this),
			dialog: new ModalModel(this),
			locale: new LocaleModel(this),
			project: null
		})
	}

	async init() {
		Niva.addEventListener('window.closeRequested', () => this.exit());
		const { history } = this.state;
		await history.init();

		const recently = history.recently();
		if (recently) {
			await this.open(recently);
		}
	}

	async openWithPicker(): Promise<AppResult> {
		const pathResult = await fromThrowableAsync(async () => {
			return Niva.api.dialog.pickDir();
		});
		if (pathResult.isErr()) {
			return pathResult;
		}
		const path = pathResult.value;
		if (path) {
			return this.open(path);
		}
		return Ok(void 0);
	}

	async open(path: string): Promise<AppResult> {
		const { dialog } = this.state;

		// first check if the path is a valid project path
		const configPath = pathJoin(path, 'niva.json');
		const packageJsonPath = pathJoin(path, 'package.json');
		const [
			isExists,
			isConfigExists,
			isPackageJsonExists,
		] = await Promise.all([
			fs.isDir(path),
			fs.exists(configPath),
			fs.exists(packageJsonPath),
		]);

		if (!isExists) {
			return Err(ErrorCode.PROJECT_PATH_NOT_EXISTS, { path });
		}

		if (!isConfigExists) {
			if (!await dialog.confirm('Warning', '')) {
				return Err(ErrorCode.PROJECT_PATH_NOT_EXISTS, { configPath });
			}


			let projectName = path.split(/\/|\\/).pop() as string;
			const configType: AppResult<ConfigType>
				= isPackageJsonExists ? await fromThrowableAsync<ConfigType>(async () => {
					const packageJson = JSON.parse(await fs.read(packageJsonPath));
					if (packageJson?.name) {
						projectName = packageJson.name;
					}
					if (packageJson.dependencies["react-scripts"]) {
						return "react"
					} else if (packageJson.devDependencies["vite"]) {
						return "vueVite"
					} else if (packageJson.dependencies["vue"]) {
						return "vue"
					} else {
						return "simple"
					}
				}) : Ok('simple');

			const configContent = generateConfig(configType.unwrapOr('simple'), projectName);

			const createConfigFileResult = await fromThrowableAsync(async () => {
				await fs.write(configPath, JSON.stringify(configContent, null, 2));
			});

			if (createConfigFileResult.isErr()) {
				return Err(ErrorCode.PROJECT_CONFIG_CRATE_FAILED, { configPath, reason: createConfigFileResult.error });
			}
		}

		const project = new ProjectModel(this, path);
		const projectInitResult = await project.init();

		this.setState({
			...this.state,
			project
		});

		return projectInitResult;
	}

	async create(): Promise<AppResult> {
		const pathResult = await fromThrowableAsync(async () => {
			return Niva.api.dialog.saveFile();
		});

		if (pathResult.isErr()) {
			return pathResult
		}

		const path = pathResult.value;

		if (!path) {
			return Ok(void 0)
		}

		const { project } = this.state;
		if (project) {
			await project.dispose()
		}

		const newProjectResult = await fromThrowableAsync(async () => {
			await fs.makeDirAll(path);
			const files = generateNewProject("niva-new-project");
			await Promise.all(files.map(([name, content]) => fs.write(pathJoin(path, name), content)))
		});

		if (newProjectResult.isErr()) {
			return Err(ErrorCode.PROJECT_CREATE_FAILED, { path, reason: newProjectResult.error })
		}

		return this.open(path);
	}

	async exit(): Promise<AppResult> {
		const { project, dialog } = this.state;

		if (dialog.state.length > 0) {
			return Err(ErrorCode.APP_EXIT_PREVENTED_BY_DIALOG);
		}

		if (project) {
			const projectDisposeResult = await project.dispose();
			if (projectDisposeResult.isErr()) {
				return projectDisposeResult;
			}
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

export function useLocale() {
	const { locale } = useModelContext(AppModel).state;
	return useModel(locale);
}