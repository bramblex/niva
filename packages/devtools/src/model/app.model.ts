import { StateModel } from "@bramblex/state-model";
import { HistoryModel } from "./history.model";
import { useModel, useModelContext } from "@bramblex/state-model-react";
import { ProjectModel } from "./project.model";
import { DialogModel } from "./dialog.model";

export class AppModel extends StateModel<{
	history: HistoryModel,
	dialog: DialogModel,
	project: ProjectModel | null
}> {

	constructor() {
		super({
			history: new HistoryModel(),
			dialog: new DialogModel(),
			project: null
		});
	}

	async init() {
		Niva.addEventListener('window.closeRequested', () => this.exit());
	}

	async exit() {
		const { project, dialog } = this.state;
		if (dialog.state.length > 0) {
			return;
		}
		if (project) {
			await project.dispose();
		}
		Niva.api.window.close();
	}
}

export function useAppModel() {
	const app = useModelContext(AppModel);
	return useModel(app);
}

export function useHistoryModel() {
	const { history } = useModelContext(AppModel).state;
	return useModel(history);
}

export function useProjectModel() {
	const { project } = useModelContext(AppModel).state;
	if (!project) {
		throw new Error("No project is open");
	}
	return useModel(project);
}