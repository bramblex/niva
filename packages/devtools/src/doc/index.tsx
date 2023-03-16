import { StrictMode, useState } from 'react';
import './style.css'

type DocTreeNodeItem = [string, JSX.Element]
type DocTreeNode = [string, DocTreeNode[], boolean?] | DocTreeNodeItem

const indexNode: DocTreeNodeItem = ['快速开始', <div>Hello</div>];
const doc: DocTreeNode = ['文档', [
	indexNode,
	['项目配置', <div></div>],
	['Api', [
		['fs', [
			['stat', <div>fs.stat</div>],
			['exists', <div>fs.exists</div>],
			['read', <div>fs.read</div>],
			['write', <div>fs.write</div>],
			['copy', <div>fs.copy</div>],
			['move', <div>fs.move</div>],
			['remove', <div>fs.remove</div>],
			['createDir', <div>fs.createDir</div>],
			['createDirAll', <div>fs.createDirAll</div>],
			['readDir', <div>fs.readDir</div>],
		], false],
		['dialog', [
			['showMessage', <div>dialog.showMessage</div>],
			['pickFile', <div>dialog.pickFile</div>],
			['pickFiles', <div>dialog.pickFiles</div>],
			['pickDir', <div>dialog.pickDir</div>],
			['pickDirs', <div>dialog.pickDirs</div>],
			['saveFile', <div>dialog.saveFile</div>],
		], false],
		['http', [
			['request', <div>http.request</div>],
			['get', <div>http.get</div>],
			['post', <div>http.post</div>],
		], false],
		['os', [
			['info', <div>os.info</div>],
			['dirs', <div>os.dirs</div>],
			['sep', <div>os.sep</div>],
			['eol', <div>os.eol</div>],
		], false],
		['process', [
			['pid', <div>process.pid</div>],
			['currentDir', <div>process.currentDir</div>],
			['env', <div>process.env</div>],
			['setCurrentDir', <div>process.setCurrentDir</div>],
			['exit', <div>process.exit</div>],
			['exec', <div>process.exec</div>],
			['open', <div>process.open</div>],
		], false],
		['webview', [
			['isDevtoolsOpen', <div>webview.isDevtoolsOpen</div>],
			['openDevtools', <div>webview.openDevtools</div>],
			['closeDevtools', <div>webview.closeDevtools</div>],
			['setBackgroundColor', <div>webview.setBackgroundColor</div>],
		], false],
		['window', [
			['scaleFactor', <div>window.scaleFactor</div>],
			['innerPosition', <div>window.innerPosition</div>],
			['outerPosition', <div>window.outerPosition</div>],
			['setOuterPosition', <div>window.setOuterPosition</div>],
			['innerSize', <div>window.innerSize</div>],
			['setInnerSize', <div>window.setInnerSize</div>],
			['outerSize', <div>window.outerSize</div>],
			['setMinInnerSize', <div>window.setMinInnerSize</div>],
			['setMaxInnerSize', <div>window.setMaxInnerSize</div>],
			['setTitle', <div>window.setTitle</div>],
			['title', <div>window.title</div>],
			['isVisible', <div>window.isVisible</div>],
			['setVisible', <div>window.setVisible</div>],
			['isFocused', <div>window.isFocused</div>],
			['setFocus', <div>window.setFocus</div>],
			['isResizable', <div>window.isResizable</div>],
			['setResizable', <div>window.setResizable</div>],
			['isMinimizable', <div>window.isMinimizable</div>],
			['setMinimizable', <div>window.setMinimizable</div>],
			['isMaximizable', <div>window.isMaximizable</div>],
			['setMaximizable', <div>window.setMaximizable</div>],
			['isClosable', <div>window.isClosable</div>],
			['setClosable', <div>window.setClosable</div>],
			['isMinimized', <div>window.isMinimized</div>],
			['setMinimized', <div>window.setMinimized</div>],
			['isMaximized', <div>window.isMaximized</div>],
			['setMaximized', <div>window.setMaximized</div>],
			['Decorated', <div>window.Decorated</div>],
			['setDecorated', <div>window.setDecorated</div>],
			['fullscreen', <div>window.fullscreen</div>],
			['setFullscreen', <div>window.setFullscreen</div>],
			['setAlwaysOnTop', <div>window.setAlwaysOnTop</div>],
			['setAlwaysOnBottom', <div>window.setAlwaysOnBottom</div>],
			['requestUserAttention', <div>window.requestUserAttention</div>],
			['setContentProtection', <div>window.setContentProtection</div>],
			['setVisibleOnAllWorkspaces', <div>window.setVisibleOnAllWorkspaces</div>],
			['setCursorIcon', <div>window.setCursorIcon</div>],
			['setCursorPosition', <div>window.setCursorPosition</div>],
			['setCursorGrab', <div>window.setCursorGrab</div>],
			['setCursorVisible', <div>window.setCursorVisible</div>],
			['dragWindow', <div>window.dragWindow</div>],
			['setIgnoreCursorEvents', <div>window.setIgnoreCursorEvents</div>],
		], false],
	]],
	['事件', [
		['window', [
			['focused', <div>window.focused</div>],
			['scaleFactorChanged', <div>window.scaleFactorChanged</div>],
			['themeChanged', <div>window.themeChanged</div>],
		], false],
		['menu', [
			['clicked', <div>menu.clicked</div>],
		], false],
		['fileDrop', [
			['hovered', <div>fileDrop.hovered</div>],
			['dropped', <div>fileDrop.dropped</div>],
			['cancelled', <div>fileDrop.cancelled</div>],
		], false],
		['ipc', [
			['callback', <div>ipc.callback</div>],
		], false]
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
	return <li className='doc-tree-item' onClick={() => onItemClick([name, children])}>{name}</li>
}

export function DocTab() {
	const [item, setItem] = useState<DocTreeNodeItem>(indexNode);
	const [name, content] = item;

	return <div className="doc-tab-container">
		<StrictMode>
			<ul className="tree-view has-collapse-button has-connector has-container doc-tree">
				<DocTree node={doc} onItemClick={item => setItem(item)} />
			</ul>
		</StrictMode>

		<fieldset className='doc-content'>
			<legend>{name}</legend>
			{content}
		</fieldset>
	</div>
}