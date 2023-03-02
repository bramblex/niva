# Tauri Lite

一个基于 Tauri WRY 跨端 Webview 库的轻量级的跨端应用开发框架。

![screenshot](https://github.com/bramblex/tauri-lite/raw/main/examples/normal/screenshot.png)

## 目标

- 跨平台 - 支持 Macos, Windows。
- 轻量级
  - 不依赖 Node.js，Chromium，Electron 等其他三方以来，将可执行文件拖进项目即可使用。
  - 可执行文件 < 3M (Tauri 6M+, Electron 100M+)
- 开发效率 - 与 Web 开发一致，不需要额外学习 NodeJS，Electron 或者 Rust。

## TODO
- [ ] 用户自定义菜单栏 & 快捷键支持
- [ ] 窗口功能支持
  - [ ] 窗口事件支持
  - [ ] 窗口操作 API 支持
  - [ ] 窗口 icon 支持
- [ ] 项目打包脚本支持
  - [ ] MacOS
  - [ ] Windows

## Usage

1. 下载或者编译 Tauri Lite.
2. 将 target/{release, debug}/tauri-lite 拖进 web 前端项目的目标目录，比如本项目的 example 目录。
3. 确保项目目录下有 index.html 文件，如果需要更多配置可以使用 tauri-lite.json。
4. 双击 tauri-lite 即可打开应用程序。

## Config
Tauri Lite 可以通过 tauri-lite.json 提供项目和窗口的配置。

### Project config
项目基本配置，绝大部分配置只有在打包发布时候才有用。除了 name 字段外，其他字段都是可选的。

| 字段          | 类型   | 默认值 | 描述                    |
| ------------- | ------ | ------ | ----------------------- |
| name          | string |        | 应用名称                |
| icon          | string |        | 应用图标，窗口图标(WIP) |
| version       | string |        | 应用版本                |
| author        | string |        | 应用作者                |
| description   | string |        | 应用描述                |
| copyright     | string |        | 应用版权                |
| website       | string |        | 应用网站                |
| website_label | string |        | 应用网站标签            |

### Window & Webview config
窗口和 Webview 配置

| 字段                      | 类型             | 默认值           | 描述                                         |
| ------------------------- | ---------------- | ---------------- | -------------------------------------------- |
| entry                     | string           | index.html       | 入口文件                                     |
| title                     | string           | 默认取 name 字段 | 窗口标题                                     |
| devtools                  | bool             | false            | 是否启用调试工具                             |
| background_color          | [u8, u8, u8, u8] | [255,255,255,1]  | 背景颜色 [R, G, B, A], Alpha 只能设置 0 和 1 |
| theme                     | light \| dark    | light            | 窗口主题                                     |
| size                      | [f64, f64]       | [800, 600]       | 窗口大小                                     |
| min_size                  | [f64, f64]       |                  | 窗口最小大小                                 |
| max_size                  | [f64, f64]       |                  | 窗口最大大小                                 |
| position                  | [f64, f64]       | [0, 0]           | 窗口位置                                     |
| resizable                 | bool             | true             | 是否可调整大小                               |
| minimizable               | bool             | true             | 是否可最小化                                 |
| maximizable               | bool             | true             | 是否可最大化                                 |
| closable                  | bool             | true             | 是否可关闭                                   |
| fullscreen                | bool             | true             | 是否默认全屏                                 |
| maximized                 | bool             | false            | 是否默认最大化                               |
| visible                   | bool             | true             | 是否默认可见                                 |
| focused                   | bool             | false            | 是否默认聚焦                                 |
| content_protection        | bool             | false            | 是否启用内容保护                             |
| transparent               | bool             | false            | 窗口透明，需配合 background_color 使用       |
| decorations               | bool             | true             | 是否显示窗口装饰(窗口边框)                   |
| always_on_top             | bool             | false            | 是否总是显示在最上层                         |
| always_on_button          | bool             | false            | 是否总是显示在最底层                         |
| visible_on_all_workspaces | bool             | false            | 是否在所有工作区显示                         |
| menu                      | Menu[]           |                  | 用户菜单栏配置(WIP)                          |

### Menu
WIP: 用户菜单栏配置

### Runtime config
运行时配置
| 字段    | 类型 | 默认值 | 描述             |
| ------- | ---- | ------ | ---------------- |
| workers | u32  | 5      | 运行时线程池大小 |


## API
Tauri lite 提供了一些基础的 API，可以在前端代码中直接使用，所有的 Api 都返回 Promise。
```js
// webview with Proxy support 
TauriLite.api.fs.ls({ path: './' })
//            ↑  ↑
//    namespace  method
  .then(console.log)
  .catch(console.error)

// webview without Proxy support
TauriLite.call('fs.ls', { path: './'})
//              ↑  ↑
//      namespace  method
  .then(console.log)
  .catch(console.error)
```

### File System
文件系统相关 api

| API    | 参数类型                        | 返回类型             | 描述              |
| ------ | ------------------------------- | -------------------- | ----------------- |
| stat   | {path: string}                  | {metadata: MetaData} | 获取文件信息      |
| exists | {path: string}                  | {exists: bool}       | 判断文件是否存在  |
| read   | {path: string}                  | {content: string}    | 读取文件内容      |
| write  | {path: string, content: string} |                      | 写入文件内容      |
| append | {path: string, content: string} |                      | 追加文件内容(WIP) |
| mv     | {path: string, newPath: string} |                      | 移动文件          |
| cp     | {path: string, newPath: string} |                      | 复制文件          |
| rm     | {path: string}                  |                      | 删除文件          |
| ls     | {path: Option<string>}          | {files: string[]}    | 列出目录下的文件  |
| mkDir  | {path: string}                  |                      | 创建目录          |
| rmDir  | {path: string}                  |                      | 删除目录          |
| link   | {path: string, newPath: string} |                      | 创建链接(WIP)     |

```ts
// MetaData 类型
interface MetaData {
  isFile: bool,
  isDir: bool,
  isSymlink: bool,
  size: number,
  accessed: number,
  modified: number,
  created: number,
}
```

### Http
HTTP 网络请求相关 api

| API      | 参数类型 | 返回类型 | 描述          |
| -------- | -------- | -------- | ------------- |
| request  | Request  | Response | 发送请求      |
| download |          |          | 下载文件(WIP) |

```ts
// Request 类型
interface Request {
  url: string,
  method: 'GET' | 'POST' | 'PUT' | 'DELETE' | 'PATCH' | 'HEAD' | 'OPTIONS',
  headers?: { [key: string]: string },
  body?: string,
}

// Response 类型
interface Response {
  status: number,
  headers: { [key: string]: string },
  body: string,
}
```

### OS
系统相关 api

| API  | 参数类型 | 返回类型 | 描述             |
| ---- | -------- | -------- | ---------------- |
| info |          | Info     | 获取操作系统信息 |
| dirs |          | Dirs     | 获取系统目录     |

```ts
// Info 类型
interface Info {
  os: string,
  arch: string,
  version: string,
}

interface Dirs {
  home: string,
  temp: string,
  audio: string,
  desktop: string,
  document: string,
  download: string,
  font: string,
  picture: string,
  public: string,
  template: string,
  video: string,
}
```

### Process
进程相关 api

| API   | 参数类型       | 返回类型           | 描述           |
| ----- | -------------- | ------------------ | -------------- |
| pid   |                | {pid: number}      | 获取进程 id    |
| cwd   |                | {cwd: string}      | 获取当前目录   |
| chDir | {path: string} |                    | 切换当前目录   |
| env   |                | {[string]: string} | 获取环境变量   |
| exit  |                |                    | 退出进程       |
| exec  | ExecOptions    | ExecResult         | 执行命令或程序 |

```ts
// ExecOptions 类型
interface ExecOptions {
  command: string, // 命令或程序
  args?: string[], // 命令或程序参数
  cwd?: string, // 工作目录
  env?: { [key: string]: string }, // 环境变量
  detached?: bool, // 是否分离进程
}

interface ExecResult {
  pid: number, // 进程 id(仅在 detached 为 true 时有效)

  // 以下字段仅在 detached 为 false 时有效
  status: number, // 进程退出状态
  stdout: string, // 标准输出
  stderr: string, // 错误输出
}
```

## Bundle
将程序打包成单个可执行文件方案

- windows - https://github.com/SerGreen/Appacker
- macos - https://github.com/burtonageo/cargo-bundle
