import { useModel, useModelContext } from "@bramblex/state-model-react";
import { useEffect, useState } from "react";
import { pathJoin } from "../utils";
import { ProjectModel } from "./model";
import { OptionsEditor } from "./options-editor";

export function Icon() {
	const { state } = useModel(useModelContext(ProjectModel));

	const [iconSrc, setIconSrc] = useState("");
	const [err, setErr] = useState("");

	useEffect(() => {
		const { fs } = TauriLite.api;
		const iconPath = pathJoin(state!.path, state!.config.icon);
		fs.read(iconPath, 'base64')
			.then((data: string) => setIconSrc(`data:image/png;base64,${data}`))
			.catch(() => setErr(`❌图标读取失败 "${iconPath})"`));
	}, [state]);

	return <div style={{ marginLeft: '40px' }}>
		{iconSrc ? <img style={{ height: '64px', width: '64px' }} alt="" src={iconSrc} /> : <p>图标读取中...</p>}
		{err ? <p>{err}</p> : null}
	</div>
}

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
				{state.config.icon ? <Icon /> : null}
				<ul >
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