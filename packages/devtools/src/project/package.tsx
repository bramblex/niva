import { useModel, useModelContext } from "@bramblex/state-model-react";
import { useEffect, useState } from "react";
import { pathJoin } from "../utils";
import { ProjectModel } from "./model";
import { OptionsEditor } from "./options-editor";
import { ImportLoader } from "./import";
import { useTranslation } from 'react-i18next'
import './package.scss';

export function Icon( {showError = true} ) {
	const { t } = useTranslation()
	const { state } = useModel(useModelContext(ProjectModel));

	const [iconSrc, setIconSrc] = useState("");
	const [err, setErr] = useState("");

	useEffect(() => {
		const { fs } = Niva.api;
		const iconPath = pathJoin(state!.path, state!.config.icon);
		fs.read(iconPath, 'base64')
			.then((data: string) => setIconSrc(`data:image/png;base64,${data}`))
			.catch(() => setErr(`❌${t('iconfail')} "${iconPath})"`));
	}, [state]);

	return <div>
		{iconSrc ? <img style={{ height: '100%', width: '100%' }} alt="" src={iconSrc} /> : (showError ? <p>{t('iconloading')}...</p> : null)}
		{showError && err ? <p>{err}</p> : null}
	</div>
}

function ProjectDetails() {
	const { t } = useTranslation()
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
						<p>{t('projectpath')}: {state.path}</p>
						<p>{t('lastedit')}: {"0000-000-000-00"}</p>
					</div>
				</div>
				<div>
					<button className="btn btn-primary" onClick={async () => {project.debug()}}>{t('debug')}</button>
					<button className="btn" onClick={() => project.build()}>{t('build')}</button>
				</div>
			</div>
			<div className="pd-rt">
				<button className="btn btn-md btn-info" onClick={async () => {project.refresh()}}><i className="icon-sm icon-refresh"></i>{t('refresh')}</button>
			</div>
		</section>
		<section className="pd-more">
			<div className="fields-section">
				<h4>{t('basic')}</h4>
				<div className="field-item">
					<span>{t('projectname')}</span>
					<span>{state.name}</span>
				</div>
				<div className="field-item">
					<span>uuid</span>
					<span>{state.uuid}</span>
				</div>
			</div>
			<div className="fields-section">
				<h4>{t('debuginfo')}</h4>
				<div className="field-item">
					<span className="field-name">{t('projectname')}</span>
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
	const { t } = useTranslation()
	const project = useModelContext(ProjectModel);
	const { state } = useModel(project);
	const [tab, setTab] = useState(0);

	if (!state) {
		return null;
	}

	return <div className="project-page">
			<div className="directory">
				{/* <button onClick={() => {console.log('test xxx click')}}>xxxxxx</button> */}
				<Directory></Directory>
			</div>
			<div className="project-info">
				<section className="tabs">
      				<menu className="tabs-menu" role="tablist" aria-label="Project Tabs" onMouseDownCapture={(ev) => {
    					const t = ev.target as HTMLElement;
    					if (t.tagName !== 'BUTTON') {
      						Niva.api.window.dragWindow();
    					}
  					}}>
        				<button role="tab" aria-controls="detail-tab" aria-selected={tab === 0} onClick={() => setTab(0)}>{t('projectinfo')}</button>
        				<button role="tab" aria-controls="config-tab" aria-selected={tab === 1} onClick={() => setTab(1)}>{t('projectcfg')}</button>
      				</menu>
      				<article className='tabs-panel' role="tabpanel" id="detail-tab" hidden={tab !== 0}>
        				<ProjectDetails />
      				</article>
      				<article className='tabs-panel' role="tabpanel" id="config-tab" hidden={tab !== 1}>
        				<ProjectConfig />
      				</article>
    			</section>
			</div>
	</div>
}