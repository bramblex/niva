import quickStart1 from './screenshots/quick-start-1.png'
import quickStart2 from './screenshots/quick-start-2.png'
import quickStart3 from './screenshots/quick-start-3.png'
import quickStart4 from './screenshots/quick-start-4.png'
import quickStart5 from './screenshots/quick-start-5.png'
import quickStart6 from './screenshots/quick-start-6.png'

export function QuickStart() {
	return <div className='quick-start'>
		<p>
			<p>
				<b>1. 新建或者打开一个 TauriLite 项目</b>
			</p>
			<p>
				<img alt={quickStart1} src={quickStart1} />
			</p>
		</p>
		<p>
			<p>
				<b>2. 调试项目页面</b>
			</p>
			<p>
				<img alt={quickStart2} src={quickStart2} />
			</p>
			<p>点击打开调试项目</p>
			<p>
				<img alt={quickStart3} src={quickStart3} />
			</p>
			<p>右键可打开调试面板</p>
		</p>
		<p>
			<p>
				<b>3. 构建项目成为一个可执行文件</b>
			</p>
			<p>
				<img alt={quickStart4} src={quickStart4} />
			</p>
			<p>点击构建项目，选择可执行文件生成的位置和名字</p>
			<p>
				<img alt={quickStart5} src={quickStart5} />
			</p>
			<p>等待构建完成</p>
			<p>
				<img alt={quickStart6} src={quickStart6} />
			</p>
			<p>构建完成后会自动代码可执行文件生成的位置</p>
		</p>
	</div>
}