import classNames from 'classnames';
import { StateModel } from "@bramblex/state-model";
import { useLocalModel, useModelContext } from "@bramblex/state-model-react";
import { useState, useEffect, useReducer, useRef } from "react";
import { pathJoin, uuid } from "../utils";
import { ProjectModel } from "./model";
import { Icon } from './package';
import { modal } from "../modal";
import './import.scss';

const { fs, os, dialog } = Niva.api;

interface ImportLoaderProps { 
    type: string;
}

interface FileState {
	isHover: boolean;
	error: string;
}

interface HistoryState {
	path: string;
	name: string;
	icon: string;
	hlHTML?: string;
}

class HistoryMode extends StateModel<HistoryState[]> {
	private historyFilePath!: string;

	constructor() {
		super([])
	}

	async init() {
		const { data } = await os.dirs();
		this.historyFilePath = pathJoin(data, 'history.json');
		this.readHistory();
	}

	async writeHistory(historyData: HistoryState) {
		this.setState([
			historyData,
			...this.state,
		].filter((item, index, array) => array.findIndex(i => i.path === item.path) === index).slice(0, 10));
		await fs.write(this.historyFilePath, JSON.stringify(this.state));
	}

	async removeHistory(path: string) {
		this.setState(this.state.filter(item => item.path !== path));
		await fs.write(this.historyFilePath, JSON.stringify(this.state));
	}

	async readHistory() {
		if (!await fs.exists(this.historyFilePath)) {
			await fs.write(this.historyFilePath, "[]");
		}
		const content = JSON.parse(await fs.read(this.historyFilePath));
		this.setState(content);
	}

	searchHistory(keyword: string) {
		if (!keyword) {
			return this.state.map(item => {
				item.hlHTML = '';
				return item;
			})
		}
		return this.state
			.filter(item => item.name.toLocaleLowerCase().includes(keyword.toLocaleLowerCase()))
			.map(item => {
				item.hlHTML = item.name.replace(new RegExp(`(${keyword})`, 'ig'), `<span class="keyword-match">$1</span>`);
				return item;
			})
	}

	clearHistory() {
		this.setState([]);
		fs.remove(this.historyFilePath);
	}
}

export function ImportLoader(props: ImportLoaderProps) {
	const history = useLocalModel(() => new HistoryMode());
	const project = useModelContext(ProjectModel);

	const fileUploaderRef = useRef<HTMLDivElement|null>(null);
	const [keyword, setKeyword] = useState('');
	const { type } = props;

	const reducer = (state: FileState, newState: FileState) => {
		return {...state, ...newState}
	}
	const fileState: FileState = {
		isHover: false,
		error: ''
	}
	const [{ isHover, error }, fileDispatch] = useReducer(reducer, fileState)

	async function handlePath(path: string) {
		try {
			const error = await project.init(path);
			if (error) {
				fileDispatch({error, isHover: false})
				return
			}
			if (project.state) {
				history.writeHistory({
					path: project.state.path,
					name: project.state.name,
					icon: project.state?.config?.icon || ""
				})
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
		const handleDrop = (_: string, { paths, position }: { paths: string[], position: number[] }) => {
			if (!isHover) {
				return
			}
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

	const pageImport = <div className={classNames("file-uploader", {active: isHover, error: !isHover && !!error})}
			ref={fileUploaderRef}
			onDragEnter={async () => { fileDispatch({ isHover: true, error: '' }) }}
			onDragOver={async () => { fileDispatch({ isHover: true, error: '' }) }}
			onDragLeave={async () => { fileDispatch({ isHover: false, error: '' }) }}>
		<div className="file-uploader__tips">
			<i className="icon-md icon-plus"></i>
			{error || "点击或拖拽文件到此处上传"}
		</div>
		<div className="file-uploader__btns">
			<button className="btn btn-bg btn-primary" onClick={async () => {
				selectProject()
			}}><i className="icon-sm icon-folder"></i>选择项目</button>

			<button className="btn btn-bg"
				style={{ marginLeft: "6px" }}
				onClick={async () => {
					newProject()
				}}><i className="icon-sm icon-plus-black"></i>新建项目</button>
		</div>
	</div>

	const historyList = history.searchHistory(keyword)

	const directoryImport = <div className="file-uploader-dir">
		<div className="search-bar">
			<div className="search-input">
				<input placeholder="搜索项目" onChange={async (e) => setKeyword(e.target.value)}></input>
				<i className="icon-sm icon-search"></i>
			</div>
			<div className="btn-containers">
				<div><span className="text-btn" onClick={async () => newProject()}><i className="icon-sm icon-plus-primary"></i>新建项目</span></div>
				<div><span className="text-btn"  onClick={async () => selectProject()}><i className="icon-sm icon-folder-primary"></i>打开项目</span></div>
			</div>
		</div>
		<div className="history">
			<span className="text-btn clear-history" onClick={async () => history.clearHistory()}>浏览历史<i className="icon-sm icon-delete"></i></span>
			{historyList.length > 0 ? 
				<div className="history-list">
					{historyList.map((item) => 
						<div className={classNames("history-item", {active: item.path === project?.state?.path})} key={item.path}>
							<div className="picon">{item.icon ? <Icon showError={false}/> : null}</div>
							<div className="pinfo" onClick={() => handlePath(item.path)}>
								{item.hlHTML ? <h4 dangerouslySetInnerHTML={{__html: item.hlHTML}}></h4> : <h4>{item.name}</h4>}
								<span>{item.path}</span>
							</div>
							<i className="icon-sm icon-delete" onClick={async () => {
								const ok = await modal.confirm(
									"提示",
									"确认删除？"
								);
								if (ok) {
									history.removeHistory(item.path);
								}
							}}></i>
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