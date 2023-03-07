import classNames from 'classnames';
import { useEffect, useState } from 'react';
import { dialog } from '../dialog';
import style from './style.module.css'

export function ImportProject() {
	const [dropHover, setDropHover] = useState(false);
	const [dir, setDir] = useState<string | null>(null);

	const pickProject = async () => {
		const { homeDir } = await TauriLite.api.os.dirs();
		const { dir } = await TauriLite.api.dialog.pickDir({ homeDir });
		setDir(dir);
		dialog.message('导入项目', `导入项目 ${dir}`);
	}

	const handleProject = async (path: string) => {
		const { metadata } = await TauriLite.api.fs.stat({ path });
		if (!metadata.isDir) {
			dialog.message('导入项目', `导入项目 ${path} 失败，不是一个目录`);
			return;
		}
		setDir(path);
		dialog.message('导入项目', `导入项目 ${path}`);
	}

	useEffect(() => {
		const handleHover = () => {
			setDropHover(true);
		}
		const handleDrop = (_: string, { paths }: { paths: string[] }) => {
			handleProject(paths[0])
			setDropHover(false);
		}
		const handleCancel = () => {
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

	return <div className={style.importProjectPage}>
		<div className={style.title}>导入 Tauri Lite 项目</div>
		<div className={classNames({ [style.fileSelector]: true, [style.dropHover]: dropHover })} onClick={pickProject}>
			<div className={style.fileSelectorIcon}>+</div>
			<div className={style.fileSelectorText}>点击选择项目目录或将目录拖拽到此处</div>
		</div>
	</div>
}