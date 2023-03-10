import { useModel, useModelContext } from "@bramblex/state-model-react";
import { Page } from "./app";
import { ProjectModel } from "./project-model";

export function ProjectPage() {
	const project = useModelContext(ProjectModel);
	const { state } = useModel(project);

	if (!state) {
		return null;
	}

	const tableContent = [
		['项目名', state.name],
		['UUID', state.uuid],
		['目录', state.path],
		['配置文件', project.getConfigPath()]
	] as const;

	const operations = [
		['打开项目', () => project.open()],
		['编辑配置', () => project.edit()],
		['启动调试', () => project.debug()],
		['打包App', () => project.build()],
		['关闭项目', () => project.close()]
	] as const;

	return <Page title={state.name}>
		<div className="project-info-container">

			<div className="project-info-table">
				{tableContent.map(([key, value]) => <div key={key} className="project-info-table-field">
					<div className="project-info-table-key">{key}:</div>
					<div className="project-info-table-value">{value}</div>
				</div>)}
			</div>

			<div className="project-operator-group">
				{operations.map(([text, onClick]) =>
					<div key={text} className="project-operator-button" onClick={onClick}>{text}</div>
				)}
			</div>

		</div>
	</Page>
}