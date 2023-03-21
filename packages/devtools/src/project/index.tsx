import './style.css'

import { useLocalModel, useModel, useModelContext, useModelProvider } from "@bramblex/state-model-react";
import { ImportPage } from "./import";
import { ProjectModel } from "./model";
import { ProjectPage } from "./package";
import { useEffect } from 'react';
import { modal } from '../modal';
import { isAbsolutePath, parseArgs, pathJoin, resolvePath, tryOrAlert } from '../utils';

export function ProjectApp() {
	const project = useLocalModel(() => new ProjectModel());
	const Provider = useModelProvider(ProjectModel);

	useEffect(() => {
		tryOrAlert(async () => {
			const { process } = TauriLite.api;
			const args = parseArgs(await process.args());
			if (args.project) {
				const projectPath = await resolvePath(args.project);
				await project.init(projectPath);

				if (args.build) {
					const buildTarget = await resolvePath(args.build);
					await project.build(buildTarget);
					await process.exit();
				}
			}
		});
	}, []);

	return <Provider value={project}>
		{project.state ? <ProjectPage /> : <ImportPage />}
	</Provider>
}

export function ProjectTab() {
	return <ProjectApp />
}