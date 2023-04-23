import { useEffect, useRef, useState } from 'react';
import { DialogComponentProps } from "../modal";
import { ProjectModel } from "./model";

import AceEditor from 'react-ace';
import ace from "ace-builds";

import 'ace-builds/src-noconflict/mode-json';
import 'ace-builds/src-noconflict/theme-github';
import 'ace-builds/src-noconflict/ext-language_tools'

import { tryOrAlertAsync, withCtx, withCtxP } from '../utils';

import './options-editor.scss';

// eslint-disable-next-line import/no-webpack-loader-syntax
const jsonWorkerUrl = require("file-loader!ace-builds/src-noconflict/worker-json").default;
ace.config.setModuleUrl("ace/mode/json_worker", jsonWorkerUrl);


const { fs } = Niva.api;

interface OptionsEditorProps extends DialogComponentProps {
	project: ProjectModel;
}

export function OptionsEditor({ close, project }: OptionsEditorProps) {
	const [value, setValue] = useState('');

	useEffect(() => {
		tryOrAlertAsync(async () => {
			const configPath = project.state!.configPath;
			const content = await withCtxP(fs.read(configPath), '读取配置文件失败');
			setValue(content as string);
		}).catch(close);
	}, []);

	return <div className="options-editor">
		{/* <div className="title-bar">
			<div className="title-bar-text" id="dialog-title">编辑配置</div>
			<div className="title-bar-controls">
				<button aria-label="Close" onClick={() => {
					close();
				}}></button>
			</div>
		</div> */}

		<div className="options-editor-body">
			<AceEditor
				mode="json"
				theme="github"
				name="options-editor"
				height='100%'
				width='100%'
				value={value}
				onChange={setValue}
				editorProps={{
					$blockScrolling: true
				}}
			/>
		</div>

		<footer style={{ textAlign: "right" }}>
			<button className='btn btn-md' style={{ marginRight: '6px' }} onClick={() => {
				
			}}>重制</button>
			<button className="btn btn-md btn-primary" onClick={() =>
				tryOrAlertAsync(async () => {
					withCtx(() => JSON.parse(value), '配置文件格式错误');
					await withCtxP(fs.write(project.state!.configPath, value), '保存配置文件失败');
					close();
					project.init(project.state!.path);
				})
			}>确认</button>
		</footer>
	</div >
}