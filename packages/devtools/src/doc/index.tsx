import React from 'react';
import { useEffect, useState } from 'react';
import { getCurrentDir, getHome, pathJoin, tempWith } from '../utils';
import { Options } from './options';
import { QuickStart } from './quick-start';
import './style.css'

type DocTreeNodeItem = [string, JSX.Element]
type DocTreeNode = [string, DocTreeNode[], boolean?] | DocTreeNodeItem

function ApiCase(props: { method: string, args: any[], desc?: string }) {
	const { method: _method, args, desc } = props;
	const [namespace, method] = _method.split('.');

	const [result, setResult] = useState('');

	return <div>
		<p><b>测试用例{desc ? `(${desc})` : ''}:</b></p>
		<pre>{_method}({args.map((arg, i) => JSON.stringify(arg)).join(',')})</pre>
		<p>
			<button onClick={() => {
				TauriLite.api[namespace][method](...args)
					.then((res: any) => setResult(JSON.stringify(res, null, 2)))
					.catch((err: any) => setResult(err.toString()))
			}}>测试</button>
		</p>
		<p>测试结果:</p>
		<pre className='example-code'>{result}</pre>
	</div>
}

function ApiExample(props: { cases: [string, any[]?, string?][] }) {
	const { cases } = props;
	useEffect(() => {
		return () => {
			TauriLite.api.process.setCurrentDir(getCurrentDir());
			// clear temp dir
			TauriLite.api.fs.readDir(tempWith(''))
				.then((list: { name: string }[]) => list.map(item => TauriLite.api.fs.remove(tempWith(item.name))));
		}
	}, []);
	return <>
		{cases.map(([method, args, desc], i) => <ApiCase key={i} method={method} args={args || []} desc={desc} />)}
	</>
}

function DragExample() {
	return <div>
		<p><b>测试用例</b></p>
		<pre>onMouseDown(() ={'>'} window.dragWindow())</pre>
		<p>拖动下方方块:</p>
		<div
			style={{ width: '100px', height: '100px', background: 'red' }}
			onMouseDown={() => TauriLite.api.window.dragWindow()}>
		</div>
	</div>
}

function EventExample(props: { event: string, exampleData: any }) {
	const { event, exampleData } = props;
	return <div>
		<p><b>示例代码:</b></p>
		<pre>TauriLite.addListener({JSON.stringify(event)}, (event, data) ={'>'} {'{'}...{'}'})</pre>
		<p>示例 data 结构</p>
		<pre>{JSON.stringify(exampleData, null, 2)}</pre>
	</div>
}

const indexNode: DocTreeNodeItem = ['快速开始', <QuickStart />];
const createDoc: () => DocTreeNode = () => ['文档', [
	indexNode,
	['项目配置', <Options />],
	['Api', [
		['fs', [
			['stat', <ApiExample cases={[['fs.stat', ['index.html']], ['fs.stat', ['cannot-found']]]} />],
			['exists', <ApiExample cases={[['fs.exists', ['index.html']], ['fs.exists', ['cannot-found']]]} />],
			['read', <ApiExample cases={[
				['fs.read', ['index.html']],
				['fs.read', ['logo.png', 'base64']]
			]} />],
			['write', <ApiExample cases={[
				['fs.write', [tempWith("test.txt"), "hello world"]],
				['fs.read', [tempWith("test.txt")]],
				['fs.write', [tempWith("test.png"), "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8BQDwAEhQGAhKmMIQAAAABJRU5ErkJggg==", 'base64']],
				['process.open', [tempWith("test.png")]],
			]} />],
			['copy',
				<ApiExample cases={[
					['fs.write', [tempWith("test.txt"), "hello world"]],
					['fs.copy', [tempWith("test.txt"), tempWith("test-copy.txt")]],
					['fs.createDir', [tempWith("test-dir")]],
					['fs.copy', [tempWith("test-dir"), tempWith("test-dir-copy"), { copyInside: true }]],
					['fs.readDir', [tempWith("")]],
				]} />
			],
			['move',
				<ApiExample cases={[
					['fs.write', [tempWith("test.txt"), "hello world"]],
					['fs.move', [tempWith("test.txt"), tempWith("test-move.txt")]],
					['fs.createDir', [tempWith("test-dir")]],
					['fs.move', [tempWith("test-dir"), tempWith("test-dir-move"), { copyInside: true }]],
					['fs.readDir', [tempWith("")]],
				]} />
			],
			['remove',
				<ApiExample cases={[
					['fs.write', [tempWith("test.txt"), "hello world"]],
					['fs.remove', [tempWith("test.txt")]],
					['fs.createDir', [tempWith("test-dir")]],
					['fs.remove', [tempWith("test-dir")]],
					['fs.readDir', [tempWith("")]],
				]} />
			],
			['createDir',
				<ApiExample cases={[
					['fs.createDir', [tempWith("test-dir")]],
					['fs.readDir', [tempWith("")]],
				]} />
			],
			['createDirAll',
				<ApiExample cases={[
					['fs.createDirAll', [tempWith(pathJoin('test-dir', 'test-dir2', 'test-dir3'))]],
					['fs.readDir', [tempWith("")]],
					['fs.readDir', [tempWith("test-dir")]],
					['fs.readDir', [tempWith(pathJoin("test-dir", "test-dir2"))]],
				]} />
			],
			['readDir',
				<ApiExample cases={[
					['fs.write', [tempWith("test.txt"), "hello world"]],
					['fs.readDir', [tempWith("")]],
				]} />
			],
		], false],
		['dialog', [
			['showMessage',
				<ApiExample cases={[
					['dialog.showMessage', ["hello", "world", "info"]],
					['dialog.showMessage', ["hello", "world", "warning"]],
					['dialog.showMessage', ["hello", "world", "error"]],
				]} />
			],
			['pickFile',
				<ApiExample cases={[
					['dialog.pickFile'],
					['dialog.pickFile', [['png', 'jpg']]],
					['dialog.pickFile', [['png', 'jpg'], getHome()]],
				]} />
			],
			['pickFiles',
				<ApiExample cases={[
					['dialog.pickFiles'],
					['dialog.pickFiles', [['png', 'jpg']]],
					['dialog.pickFiles', [['png', 'jpg'], getHome()]],
				]} />
			],
			['pickDir',
				<ApiExample cases={[
					['dialog.pickDir'],
					['dialog.pickDir', [getHome()]],
				]} />
			],
			['pickDirs',
				<ApiExample cases={[
					['dialog.pickDirs'],
					['dialog.pickDirs', [getHome()]],
				]} />
			],
			['saveFile',
				<ApiExample cases={[
					['dialog.saveFile'],
					['dialog.saveFile', [['png', 'jpg']]],
					['dialog.saveFile', [['png', 'jpg'], getHome()]],
				]} />
			],
		], false],
		['http', [
			['request',
				<ApiExample cases={[
					['http.request', [{
						method: 'GET',
						url: 'https://httpbin.org/anything',
						headers: {
							"x-test-header": "test",
						},
					}]],
					['http.request', [{
						method: 'POST',
						url: 'https://httpbin.org/anything',
						headers: {
							"x-test-header": "test",
						},
						body: '{ "message": "hello world" }',
					}]],
				]} />
			],
			['get',
				<ApiExample cases={[
					['http.get', ['https://httpbin.org/anything', {
						"x-test-header": "test",
					}]],
				]} />
			],
			['post',
				<ApiExample cases={[
					['http.post', ['https://httpbin.org/anything', '{ "message": "hello world" }', {
						"x-test-header": "test",
					}]],
				]} />
			],
		], false],
		['os', [
			['info', <ApiExample cases={[['os.info'],]} />],
			['dirs', <ApiExample cases={[['os.dirs'],]} />],
			['sep', <ApiExample cases={[['os.sep'],]} />],
			['eol', <ApiExample cases={[['os.eol'],]} />],
		], false],
		['process', [
			['pid', <ApiExample cases={[['process.pid'],]} />],
			['currentDir', <ApiExample cases={[['process.currentDir'],]} />],
			['env', <ApiExample cases={[['process.env'],]} />],
			['setCurrentDir',
				<ApiExample cases={[
					['process.setCurrentDir', [getHome()]],
					['process.currentDir', []]
				]} />
			],
			['exit', <ApiExample cases={[['process.exit'],]} />],
			['exec',
				<ApiExample cases={[
					['process.exec', ['echo', ["hello world"]], 'Mac OS'],
					['process.exec', ['help', []], 'Windows'],
				]} />
			],
			['open', <ApiExample cases={[
				['process.open', ["https://github.com/bramblex/tauri_lite"]],
				['fs.write', [tempWith("test.txt"), "hello world"]],
				['process.open', [tempWith("test.txt")]],
			]} />
			],
		], false],
		['webview', [
			['isDevtoolsOpen', <ApiExample cases={[['webview.isDevtoolsOpen'],]} />],
			['openDevtools', <ApiExample cases={[['webview.openDevtools'],]} />],
			['closeDevtools', <ApiExample cases={[['webview.closeDevtools'],]} />
			],
		], false],
		['window', [
			['scaleFactor', <ApiExample cases={[['window.scaleFactor']]} />],
			['innerPosition', <ApiExample cases={[['window.innerPosition']]} />],
			['outerPosition', <ApiExample cases={[['window.outerPosition']]} />],
			['setOuterPosition', <ApiExample cases={[['window.setOuterPosition', [[0, 0]]]]} />],
			['innerSize', <ApiExample cases={[['window.innerSize']]} />],
			['setInnerSize',
				<ApiExample cases={[
					['window.setInnerSize', [[1600, 1200]]],
					['window.setInnerSize', [[800, 600]]],
				]} />
			],
			['outerSize', <ApiExample cases={[['window.outerSize']]} />],
			['setMinInnerSize', <ApiExample cases={[['window.setMinInnerSize', [[800, 600]]]]} />],
			['setMaxInnerSize', <ApiExample cases={[['window.setMaxInnerSize', [[1600, 1200]]]]} />],
			['setTitle', <ApiExample cases={[['window.setTitle', ["hello world"]]]} />],
			['title', <ApiExample cases={[['window.title']]} />],
			['isVisible', <ApiExample cases={[['window.isVisible']]} />],
			['setVisible', <ApiExample cases={[
				['window.setVisible', [true]],
				['window.setVisible', [false]],
			]} />],
			['isFocused', <ApiExample cases={[['window.isFocused']]} />],
			['setFocus', <ApiExample cases={[['window.setFocus']]} />],
			['isResizable', <ApiExample cases={[['window.isResizable']]} />],
			['setResizable', <ApiExample cases={[
				['window.setResizable', [true]],
				['window.setResizable', [false]]
			]} />],
			['isMinimizable', <ApiExample cases={[['window.isMinimizable']]} />],
			['setMinimizable', <ApiExample cases={[
				['window.setMinimizable', [true]],
				['window.setMinimizable', [false]]
			]} />],
			['isMaximizable', <ApiExample cases={[['window.isMaximizable']]} />],
			['setMaximizable', <ApiExample cases={[
				['window.setMaximizable', [true]],
				['window.setMaximizable', [false]]
			]} />],
			['isClosable', <ApiExample cases={[['window.isClosable']]} />],
			['setClosable', <ApiExample cases={[
				['window.setClosable', [true]],
				['window.setClosable', [false]]
			]} />],
			['isMinimized', <ApiExample cases={[['window.isMinimized']]} />],
			['setMinimized', <ApiExample cases={[
				['window.setMinimized', [true]],
				['window.setMinimized', [false]]
			]} />],
			['isMaximized', <ApiExample cases={[['window.isMaximized']]} />],
			['setMaximized', <ApiExample cases={[
				['window.setMaximized', [true]],
				['window.setMaximized', [false]]
			]} />],
			['Decorated', <ApiExample cases={[['window.Decorated']]} />],
			['setDecorated', <ApiExample cases={[
				['window.setDecorated', [true]],
				['window.setDecorated', [false]]
			]} />],
			['fullscreen', <ApiExample cases={[['window.fullscreen']]} />],
			['setFullscreen', <ApiExample cases={[
				['window.setFullscreen', [true]],
				['window.setFullscreen', [false]]
			]} />],
			['setAlwaysOnTop', <ApiExample cases={[
				['window.setAlwaysOnTop', [true]],
				['window.setAlwaysOnTop', [false]]
			]} />],
			['setAlwaysOnBottom', <ApiExample cases={[
				['window.setAlwaysOnBottom', [true]],
				['window.setAlwaysOnBottom', [false]]
			]} />],
			['requestUserAttention', <ApiExample cases={[
				['window.requestUserAttention'],
				['window.requestUserAttention', ["informational"]],
				['window.requestUserAttention', ["critical"]]
			]} />],
			['setContentProtection', <ApiExample cases={[
				['window.setContentProtection', [true]],
				['window.setContentProtection', [false]]
			]} />],
			['setVisibleOnAllWorkspaces', <ApiExample cases={[
				['window.setVisibleOnAllWorkspaces', [true]],
				['window.setVisibleOnAllWorkspaces', [false]]
			]} />],
			['setCursorIcon', <ApiExample cases={[
				['window.setCursorIcon', ["default"]],
				['window.setCursorIcon', ["pointer"]],
				['window.setCursorIcon', ["crosshair"]],
				['window.setCursorIcon', ["copy"]],
			]} />],
			['setCursorPosition', <ApiExample cases={[
				['window.setCursorPosition', [[0, 0]]],
			]} />],
			['setCursorGrab', <ApiExample cases={[
				['window.setCursorGrab', [true]],
				['window.setCursorGrab', [false]]
			]} />],
			['setCursorVisible', <ApiExample cases={[
				['window.setCursorVisible', [true]],
				['window.setCursorVisible', [false]]
			]} />],
			['dragWindow', <DragExample />],
			['setIgnoreCursorEvents', <ApiExample cases={[
				['window.setIgnoreCursorEvents', [true]],
				['window.setIgnoreCursorEvents', [false]]
			]} />],
		], false],
	]],
	['事件', [
		['window', [
			['focused', <EventExample event='window.focused' exampleData={{ focused: true }} />],
			['scaleFactorChanged',
				<EventExample
					event='window.scaleFactorChanged'
					exampleData={{ scaleFactor: 2, newInnerSize: { with: 800, height: 600 } }}
				/>],
			['themeChanged',
				<EventExample
					event='window.themeChanged'
					exampleData={{ theme: 'dark' }}
				/>],
		], false],
		['menu', [
			['clicked',
				<EventExample
					event='menu.clicked'
					exampleData={{ menuId: 124 }}
				/>
			],
		], false],
		['fileDrop', [
			['hovered',
				<EventExample
					event='fileDrop.hovered'
					exampleData={{ paths: ['/xxx/xxx/xx.txt'], position: [0, 0] }}
				/>
			],
			['dropped',
				<EventExample
					event='fileDrop.hovered'
					exampleData={{ paths: ['/xxx/xxx/xx.txt'], position: [0, 0] }}
				/>
			],
			['cancelled',
				<EventExample
					event='fileDrop.hovered'
					exampleData={null}
				/>
			],
		], false],
	]]
]]


function DocTree({ node, onItemClick }: { node: DocTreeNode, onItemClick: (node: DocTreeNodeItem) => any }): JSX.Element {
	const [name, children, open] = node;
	if (Array.isArray(children)) {
		return <details open={open == null ? true : open}>
			<summary>{name}</summary>
			<ul>
				{children.map((node, i) => <DocTree key={i} node={node} onItemClick={onItemClick} />)}
			</ul>
		</details>
	}
	return <li >
		<span className='doc-tree-item' onClick={() => onItemClick([name, children])}>{name}</span>
	</li>
}

export function DocTab() {
	const [item, setItem] = useState<DocTreeNodeItem>(indexNode);
	const [name, content] = item;
	const [doc, setDoc] = useState<DocTreeNode | null>(null);

	useEffect(() => {
		setTimeout(() => {
			setDoc(createDoc());
		}, 300);
	}, []);

	return doc && <div className="doc-tab-container">
		<ul className="tree-view has-collapse-button has-connector has-container doc-tree">
			<DocTree node={doc} onItemClick={item => setItem(item)} />
		</ul>
		<fieldset className='doc-content'>
			<legend>{name}</legend>
			<React.Fragment key={Math.random()}>
				{content}
			</React.Fragment>
		</fieldset>
	</div>
}