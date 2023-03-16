import { useModel, useModelContext } from "@bramblex/state-model-react";
import { ProjectModel } from "./model";

export function ProjectPage() {
	const project = useModelContext(ProjectModel);
	const { state } = useModel(project);

	if (!state) {
		return null;
	}

	const operations = [
		['打开项目', () => project.open()],
		['编辑配置', () => project.edit()],
		['启动调试', () => project.debug()],
		['关闭项目', () => project.close()],
	] as const;

	return <div>
		<fieldset>
			<legend>项目信息</legend>
			<ul>
				<li>项目名(name): {state.name}</li>
				<li>UUID(uuid): {state.uuid}</li>
				<li>项目目录: {state.path} </li>
				<li>配置文件: {state.configPath} </li>
			</ul>
		</fieldset>

		<fieldset>
			<legend>调试信息</legend>
			<ul>
				<li>调试入口(debugEntry): {state.config.debugEntry || '（无）'}</li>
			</ul>
		</fieldset>

		<div style={{ display: 'flex' }}>
			{operations.map(([text, onClick]) =>
				<button key={text} onClick={onClick} style={{ marginRight: "6px" }}>{text}</button>
			)}
			<button className="default" style={{ marginLeft: 'auto' }} onClick={() => project.build()}>构建项目</button>
		</div>

	</div>
}