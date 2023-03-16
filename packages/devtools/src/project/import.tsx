import { StateModel } from "@bramblex/state-model";
import { useLocalModel, useModelContext } from "@bramblex/state-model-react";
import { useEffect } from "react";
import { pathJoin, uuid } from "../utils";
import { ProjectModel } from "./model";

const { fs, os, dialog } = TauriLite.api;

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

	clearHistory() {
		this.setState([]);
		fs.remove(this.historyFilePath);
	}
}

export function ImportPage() {
	const history = useLocalModel(() => new HistoryMode());
	const project = useModelContext(ProjectModel);

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

	useEffect(() => {
		const handleDrop = (_: string, { paths }: { paths: string[] }) => {
			const path = paths[0];
			if (path) {
				handlePath(path)
			}
		}

		history.init();

		TauriLite.addEventListener('fileDrop.dropped', handleDrop);
		return () => {
			TauriLite.removeEventListener('fileDrop.dropped', handleDrop);
		}
	}, []);

	return <div>
		<p>未选择项目, 点击下方按钮选择，或将项目拖入程序</p>
		<button onClick={async () => {
			const { home } = await os.dirs();
			const path = await dialog.pickDir(home);
			if (path) {
				handlePath(path);
			}
		}}>选择项目</button>

		<button
			style={{ marginLeft: "6px" }}
			onClick={async () => {
				const { home } = await os.dirs();
				const projectDir = await dialog.saveFile([], home);
				const sep = await os.sep();

				const projectName = projectDir.split(sep).pop();
				fs.createDir(projectDir);

				const files = [
					["tauri_lite.json", JSON.stringify({ name: projectName, uuid: uuid() })],
					["index.html", "<h1>Hello World!</h1><script src='./index.js'></script>"],
					["index.js", "console.log('Hello World!')"]
				];

				for (const [file, content] of files) {
					await fs.write(pathJoin(projectDir, file), content);
				}

				await handlePath(projectDir);
				await project.debug();
			}}>新建项目</button>

		{history.state.length > 0 ? <>
			<hr />
			历史项目:
			<ul>
				{history.state.map((path) => <li className="link" key={path} onClick={() => handlePath(path)}>{path}</li>)}
			</ul>
			<button onClick={() => history.clearHistory()}>清除历史</button>
		</> : null}

	</div>
}