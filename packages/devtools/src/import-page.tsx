import { useModelContext } from "@bramblex/state-model-react";
import classNames from "classnames";
import { useEffect, useState } from "react";
import { Page } from "./app";
import { ProjectModel } from "./project-model";
import { tryOr } from "./utils";

export function ImportPage() {
	const [dropHover, setDropHover] = useState(false);
	const project = useModelContext(ProjectModel);

	useEffect(() => {
		const handleHover = () => setDropHover(true);
		const handleCancel = () => setDropHover(false);

		const handleDrop = (_: string, { paths }: { paths: string[] }) => {
			const path = paths[0];
			if (path) {
				project.init(path);
			}
			setDropHover(false);
		}

		TauriLite.addEventListener('fileDrop.hover', handleHover);
		TauriLite.addEventListener('fileDrop.drop', handleDrop);
		TauriLite.addEventListener('fileDrop.cancel', handleCancel);
		return () => {
			TauriLite.removeEventListener('fileDrop.hover', handleHover);
			TauriLite.removeEventListener('fileDrop.drop', handleDrop);
			TauriLite.removeEventListener('fileDrop.cancel', handleCancel);
		}
	}, []);

	return <Page title="导入 Tauri Lite 项目">
		<div className='project-picker-container'>
			<div className={classNames('project-picker', { 'drop-hover': dropHover })} onClick={async () => {
				const { home } = await TauriLite.api.os.dirs();
				const { dir } = await tryOr(
					() => TauriLite.api.dialog.pickDir({ startDir: home }),
					async () => ({ dir: null }));
				if (dir) {
					project.init(dir);
				}
			}}>
				<div className='project-picker-icon'>+</div>
				<div className='project-picker-text'>点击选择项目目录或将目录拖拽到此处</div>
			</div>
		</div>
	</Page>
}