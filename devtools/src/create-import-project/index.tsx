import style from './style.module.css'

export function CreateImportProject() {

	return <div className={style.emptyPage}>
		<div className={style.title}>新建或导入 Tauri Lite 项目</div>
		<div className={style.fileSelector}>
			<div className={style.fileSelectorIcon}>+</div>
			<div className={style.fileSelectorText}>点击选择项目目录或将目录拖拽到此处</div>
		</div>
	</div>
}