# Tauri Lite
一个基于 Tauri WRY 跨端 Webview 库的轻量级的跨端应用开发框架。

## 目标
* 跨平台 - 支持 Macos, Windows。
* 轻量级 
	- 不依赖 Node.js，Chromium，Electron 等其他三方以来，将可执行文件拖进项目即可使用。
	- 可执行文件 < 3M (Tauri 6M+, Electron 100M+)
* 开发效率 - 与 Web 开发一致，不需要额外学习 NodeJS，Electron 或者 Rust。

## Usage
1. 下载或者编译 Tauri Lite.
2. 将 target/{release, debug}/tauri-lite 拖进 web 前端项目的目标目录，比如本项目的 example 目录。
3. 在上面目录中新建 tauri-lite.config 配置文件, name 字段为项目名称必填，如 example/tauri-lite.config。
4. 在 tauri-lite.config 用 entry 指定入口 html 文件，如果不填默认是 index.html。
5. 双击 tauri-lite 即可打开应用程序。

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

## Bundle
* windows - https://github.com/SerGreen/Appacker
* macos - https://github.com/burtonageo/cargo-bundle
