import { useLocalModel, useModel, useModelContext, useModelProvider } from "@bramblex/state-model-react";
import { ImportPage } from "./import";
import { ProjectModel } from "./model";
import { ProjectPage } from "./package";

export function ProjectApp() {
	const project = useLocalModel(() => new ProjectModel());
	const Provider = useModelProvider(ProjectModel);
	return <Provider value={project}>
		<AppInner />
	</Provider>
}

function AppInner() {
	const project = useModelContext(ProjectModel);
	const { state } = useModel(project);
	return state ? <ProjectPage /> : <ImportPage />;
}

export function ProjectTab() {
	return <ProjectApp />
}