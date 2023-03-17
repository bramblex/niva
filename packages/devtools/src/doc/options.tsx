import options1 from './screenshots/options-1.png';
import options2 from './screenshots/options-2.png';


type OptionField = [string]

const options: Record<string, Record<string, OptionField>> = {
	'基本选项': {
		name: ['项目名, 必填'],
		uuid: ['UUID, 不可修改'],
	},
	'调试选项': {
		debugEntry: ['调试入口, vue / react 等项目调试时使用，如 http://localhost:3000'],
	},
	'项目选项（仅构建时使用）': {
		icon: ['图标'],
		version: ['版本'],
		author: ['作者'],
		description: ['描述'],
		copyright: ['版权'],
		license: ['许可证'],
		website: ['网站'],
	},
	'窗口选项': {
		entry: ['入口文件, 不填则默认为 index.html'],
		backgroundColor: ['背景颜色 RGBA, 例如 [255, 255, 255, 1]'],
		devtools: ['是否启用开发者工具'],
		title: ['窗口标题'],
		theme: ['窗口主题'],
		size: ['窗口大小, 不填则默认为 [800, 600]'],
		minSize: ['窗口最小大小'],
		maxSize: ['窗口最大大小'],
		position: ['窗口位置'],
		resizable: ['是否可调整窗口大小'],
		minimizable: ['是否可最小化窗口'],
		maximizable: ['是否可最大化窗口'],
		closable: ['是否可关闭窗口'],
		fullscreen: ['是否全屏显示'],
		maximized: ['是否最大化窗口'],
		visible: ['是否可见'],
		transparent: ['是否透明'],
		decorations: ['是否显示窗口装饰'],
		alwaysOnTop: ['是否总在最前面'],
		alwaysOnBottom: ['是否总在最后面'],
		visibleOnAllWorkspaces: ['是否在所有工作区可见'],
		focused: ['是否聚焦于窗口'],
		contentProtection: ['是否启用内容保护'],
		menu: ['窗口菜单选项, 详见下方'],
	},
};



export function Options() {
	return <div>
		{Object.entries(options).map(([group, fields]) => <fieldset>
			<legend>{group}</legend>
			<ul>
				{Object.entries(fields).map(([name, [label]]) => <li>{name}: {label}</li>)}
			</ul>
		</fieldset>)}
		<fieldset>
			<legend>窗口菜单选项</legend>
			窗口选项必须是一个数组: 
			<ul>
				<li>[string, number] 为一个菜单项</li>
				<li>[string, [ ... ]] 为一个子菜单</li>
				<li>"---" 为一个分隔符</li>
			</ul>
			<p>
				<img alt="" src={options1} />
			</p>
			<p>
				<img alt="" src={options2} />
			</p>
		</fieldset>
	</div>
}