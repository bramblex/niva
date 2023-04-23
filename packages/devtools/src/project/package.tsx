import { useModel, useModelContext } from "@bramblex/state-model-react";
import { useEffect, useState } from "react";
import { pathJoin } from "../utils";
import { ProjectModel } from "./model";
import { OptionsEditor } from "./options-editor";
import { WindowControl } from '../app'
import { ImportLoader } from "./import";
import './package.scss';

export function Icon( {showError = true} ) {
	const { state } = useModel(useModelContext(ProjectModel));

	const [iconSrc, setIconSrc] = useState("");
	const [err, setErr] = useState("");

	useEffect(() => {
		const { fs } = Niva.api;
		const iconPath = pathJoin(state!.path, state!.config.icon);
		fs.read(iconPath, 'base64')
			.then((data: string) => setIconSrc(`data:image/png;base64,${data}`))
			.catch(() => setErr(`❌图标读取失败 "${iconPath})"`));
	}, [state]);

	return <div>
		{iconSrc ? <img style={{ height: '100%', width: '100%' }} alt="" src={iconSrc} /> : (showError ? <p>图标读取中...</p> : null)}
		{showError && err ? <p>{err}</p> : null}
	</div>
}

function ProjectDetails() {
	const project = useModelContext(ProjectModel);
	const { state } = useModel(project);

	if (!state) {
		return null;
	}

	return <div className="project-detail">
		<section className="pd-base">
			<div className="pd-lf">
				<div className="pd-lf__info-container">
					<span>{state.config.icon ? <Icon /> : null}</span>
					<div className="info-container">
						<h3>{state.name}</h3>
						<p>文件位置: {state.path}</p>
						<p>上次编辑: {"0000-000-000-00"}</p>
					</div>
				</div>
				<div>
					<button className="btn btn-primary" onClick={async () => {project.debug()}}>调试</button>
					<button className="btn" onClick={() => project.build()}>构建</button>
				</div>
			</div>
			<div className="pd-rt">
				<button className="btn btn-md btn-info" onClick={async () => {project.refresh()}}><i className="icon-sm icon-refresh"></i>刷新</button>
			</div>
		</section>
		<section className="pd-more">
			<div className="fields-section">
				<h4>基本信息</h4>
				<div className="field-item">
					<span>项目名称</span>
					<span>{state.name}</span>
				</div>
				<div className="field-item">
					<span>uuid</span>
					<span>{state.uuid}</span>
				</div>
			</div>
			<div className="fields-section">
				<h4>调试信息</h4>
				<div className="field-item">
					<span className="field-name">项目名称</span>
					<span>{state.name}</span>
				</div>
			</div>
		</section>
	</div>
}

export function ProjectConfig() {
	const project = useModelContext(ProjectModel);
	const { state } = useModel(project);

	if (!state) {
		return null;
	}

	return <OptionsEditor project={project} close={() => {}}></OptionsEditor>
}

export function Directory() {
	return 		<div>
		<ImportLoader type="directory"></ImportLoader>
	</div>
}

export function ProjectPage() {
	const project = useModelContext(ProjectModel);
	const { state } = useModel(project);
	const [tab, setTab] = useState(0);

	if (!state) {
		return null;
	}

	// const operations = [
	// 	['打开项目', () => project.open()],
	// 	['编辑配置', () => project.edit()],
	// 	['启动调试', () => project.debug()],
	// 	['关闭项目', () => project.close()],
	// ] as const;

	return <div className="project-page">
			<div className="directory">
				{/* <button onClick={() => {console.log('test xxx click')}}>xxxxxx</button> */}
				<Directory></Directory>
			</div>
			<div className="project-info">
				<section className="tabs">
      				<menu className="tabs-menu" role="tablist" aria-label="Project Tabs">
        				<button role="tab" aria-controls="detail-tab" aria-selected={tab === 0} onClick={() => setTab(0)}>项目信息</button>
        				<button role="tab" aria-controls="config-tab" aria-selected={tab === 1} onClick={() => setTab(1)}>项目配置</button>
      				</menu>
      				<article className='tabs-panel' role="tabpanel" id="detail-tab" hidden={tab !== 0}>
        				<ProjectDetails />
      				</article>
      				<article className='tabs-panel' role="tabpanel" id="config-tab" hidden={tab !== 1}>
        				<ProjectConfig />
      				</article>
    			</section>
			</div>

		{/* <fieldset>
			<legend>调试信息</legend>
			<ul>
				<li>调试入口(debugEntry): {state.config.debugEntry || '（无）'}</li>
			</ul>
		</fieldset> */}

		{/* <div style={{ display: 'flex' }}>
			{operations.map(([text, onClick]) =>
				<button key={text} onClick={onClick} style={{ marginRight: "6px" }}>{text}</button>
			)}
			<button className="default" style={{ marginLeft: 'auto' }} onClick={() => project.build()}>构建项目</button>
		</div> */}
	</div>
}