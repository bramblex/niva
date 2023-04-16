import classNames from 'classnames';
import { StateModel } from "@bramblex/state-model";
import { useLocalModel, useModelContext } from "@bramblex/state-model-react";
import { useEffect } from "react";
import { pathJoin, uuid } from "../utils";
import { ProjectModel } from "./model";
import { Icon } from './package';
import './import.scss';

const { fs, os, dialog } = Niva.api;

class HistoryMode extends StateModel<string[]> {
	private historyFilePath!: string;

	constructor() {
		super([])
	}

	async init() {
		const { data } = await os.dirs();
		this.historyFilePath = pathJoin(data, 'history.json');
		this.readHistory();
	}

	async writeHistory(path: string) {
		this.setState([
			path,
			...this.state,
		].filter((item, index, array) => array.indexOf(item) === index).slice(0, 10));
		await fs.write(this.historyFilePath, JSON.stringify(this.state));
	}

	async removeHistory(path: string) {
		this.setState(this.state.filter(item => item !== path));
		await fs.write(this.historyFilePath, JSON.stringify(this.state));
	}

	async readHistory() {
		if (!await fs.exists(this.historyFilePath)) {
			await fs.write(this.historyFilePath, "[]");
		}
		const content = JSON.parse(await fs.read(this.historyFilePath));
		this.setState(content);
	}

	search() {
		
	}

	clearHistory() {
		this.setState([]);
		fs.remove(this.historyFilePath);
	}
}

interface ImportLoaderProps { 
    type: string;
}

export function ImportLoader(props: ImportLoaderProps) {
	const history = useLocalModel(() => new HistoryMode());
	const project = useModelContext(ProjectModel);
	const { type } = props;

	async function handlePath(path: string) {
		try {
			await project.init(path);
			if (project.state) {
				history.writeHistory(project.state.path)
			}
		} catch (e) {
			history.removeHistory(path);
		}
	}

	async function newProject() {
		const { home } = await os.dirs();
		const projectDir = await dialog.saveFile([], home);

		if (!projectDir) {
			return;
		}

		const sep = await os.sep();

		const projectName = projectDir.split(sep).pop();
		fs.createDir(projectDir);

		const files = [
			["niva.json", JSON.stringify({ name: projectName, uuid: uuid() })],
			["index.html", "<h1>Hello World!</h1><script src='./index.js'></script>"],
			["index.js", "console.log('Hello World!')"]
		];

		for (const [file, content] of files) {
			await fs.write(pathJoin(projectDir, file), content);
		}

		await handlePath(projectDir);
		await project.debug();
	}

	async function selectProject() {
		const { home } = await os.dirs();
		const path = await dialog.pickDir(home);
		if (path) {
			handlePath(path);
		}
	}

	useEffect(() => {
		const handleDrop = (_: string, { paths }: { paths: string[] }) => {
			const path = paths[0];
			if (path) {
				handlePath(path)
			}
		}

		history.init();

		Niva.addEventListener('fileDrop.dropped', handleDrop);
		return () => {
			Niva.removeEventListener('fileDrop.dropped', handleDrop);
		}
	}, []);

	const pageImport = <div className="file-uploader">
		<div className="file-uploader__tips">
			<i className="icon-md icon-plus"></i>
			点击或拖拽文件到此处上传
		</div>
		<div className="file-uploader__btns">
			<button className="btn btn-bg btn-primary" onClick={async () => {
				selectProject()
			}}><i className="icon-sm icon-folder"></i>选择项目</button>

			<button className="btn btn-bg"
				style={{ marginLeft: "6px" }}
				onClick={async () => {
					newProject()
				}}><i className="icon-sm icon-plus"></i>新建项目</button>
		</div>
	</div>

	const historyRenderList = ['xxxxx', 'xxx2'].map(i => ({
		name: 'Niva',
		icon: 'icon.png',
		path: i // "/Users/karenlin/workspace/niva/packages/example"
	})) // TODO: 搜索的时候换搜索结果列表

	const directoryImport = <div className="file-uploader-dir">
		<div className="search-bar">
			<div className="search-input">
				<input placeholder="搜索项目"></input>
				<i className="icon-sm icon-search"></i>
			</div>
			<div className="btn-containers">
				<div><span className="text-btn" onClick={async () => newProject()}><i className="icon-sm icon-plus-primary"></i>新建项目</span></div>
				<div><span className="text-btn"  onClick={async () => selectProject()}><i className="icon-sm icon-folder-primary"></i>打开项目</span></div>
			</div>
		</div>
		<div className="history">
			<span className="text-btn clear-history" onClick={async () => history.clearHistory()}>浏览历史<i className="icon-sm icon-delete"></i></span>
			{historyRenderList.length > 0 ? 
				<div className="history-list">
					{historyRenderList.map((item) => 
						<div className={classNames("history-item", {active: item.path === project?.state?.path})} key={item.path} onClick={() => handlePath(item.path)}>
							<div className="picon">{item.icon ? <Icon /> : null}</div>
							<div className="pinfo">
								<h4>{item.name}</h4>
								<span>{item.path}</span>
							</div>
						</div>
					)}
				</div> : null}
		</div>
	</div>

	return type === 'page' ? pageImport : directoryImport
}

export function ImportPage() {
	return (<div className="import-page"><ImportLoader type="page"/></div>)
}