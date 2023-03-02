# Tauri Lite
## 设计草稿

* TauriLite
	* Environment 初始化并且管理当前的运行环境
	* StaticServer 静态服务，加载静态 web 文件
	* MainWindow 封装并且传递多线程
		* Window 
		* Webview 
			* eval 代理 webview 的 js 代码执行，避免
			* ipc handler 
		* EventLoop 主循环，传递事件
	* Api 实现 Api
		* File System
		* Http
		* OS
		* Process
		* Window

## API
### File System
- [x] stat(path) -> Stat
- [x] exists(path) -> boolean

- [x] read(path) -> string
- [x] write(path, data: string) -> void
- [ ] append(path, data: string) -> void

- [x] mv(path, newPath) -> void
- [x] cp(path, newPath) -> void
- [x] rm(path) -> void

- [x] ls(path) -> string[]
- [x] mkDir(path) -> void
- [x] rmDir(path) -> void

- [ ] link(path, newPath) -> void

### Http
- [x] request(url, options?) -> string
- [ ] download(url, path, options?) -> void

### OS
- [x] info() -> string
- [x] dirs() -> string

### Process
- [x] exec(command | file, args?, options?) -> string
- [x] pid() -> number
- [x] cwd() -> string
- [x] chDir() -> void
- [x] env() -> Env
- [x] exit() -> !