# Niva 项目详细文档

> 描述对象：当前工作区**磁盘实测状态**（`main` + 未提交的重构中代码）。
> 关键现状：`crates/niva_macros` 已从磁盘删除；API 系统已是 **smol 全异步**实现；
> 旧 `thread_pool.rs` 已无人引用（死文件）；`async-lab/` 为独立异步实验工程。
> 行号均为实测值，改动后可能漂移，请以文件名为准。

---

## 1. 项目概览

Niva 是一个对标 Tauri 的超轻量跨端桌面应用框架：前端（HTML/CSS/JS，可用 Vue/React）跑在系统原生 Webview（wry）里，
Rust 提供窗口、文件、HTTP、托盘、快捷键、对话框等原生 API，通过自研 WebSocket wire 协议与前端通信。

### 1.1 技术栈

| 层 | 技术 | 证据 |
|---|---|---|
| 后端语言 | Rust 2024 edition | `crates/niva/Cargo.toml:5` |
| 窗口/Webview | `tao 0.37` + `wry 0.57`，菜单 `muda 0.20`，托盘 `tray-icon 0.25`，全局快捷键 `global-hotkey 0.8` | `crates/niva/Cargo.toml:10-16` |
| 异步运行时 | `smol 2` + `async-channel 2`（无 tokio），WS `tungstenite 0.30` | `crates/niva/Cargo.toml:35-38` |
| HTTP/文件/系统 | `ureq 2.6`（blocking，经 `smol::unblock` 跑）、`fs_extra`、`opener`、`rfd 0.11`、`arboard 3.6`（剪贴板）、`directories`、`sys-locale`、`os_info`、`mime_guess`、`flate2`、`png 0.17`、`glob`、`url`、`base64 0.13` | `crates/niva/Cargo.toml:17-43` |
| 平台相关 | macOS `cocoa 0.24 + objc 0.2`；Windows `winapi + windows 0.62` | `crates/niva/Cargo.toml:45-55` |
| 前端 devtools | React 19 + Vite 8 + TS 5.9 + sass，自研 CSS，`@bramblex/state-model` 做状态，`ace-builds` 做 JSON 编辑器，`pako` 做资源压缩，`neverthrow` 做错误处理 | `packages/devtools/package.json` |
| 类型包 | `packages/types/Niva_zh.d.ts`（1423 行，全局 `Niva` 对象声明） | `packages/types/package.json` |

### 1.2 仓库布局

```text
niva/
  crates/niva/            # 主程序：应用框架 + 所有原生 API（二进制名 niva）
    src/main.rs           # 入口（19 行）
    src/app/              # 应用核心
      mod.rs              # NivaApp / 启动 / 事件循环类型 / CLI 参数 / LaunchInfo
      main_exec.rs        # run_on_main：任意线程跳主线程执行并 await 结果
      api_manager/        # API 分发器（smol 异步）+ wire protocol v2
        mod.rs            # 三种注册 + 调度 + 超时/取消
        protocol.rs       # 文本/二进制帧编解码 + 单测
        thread_pool.rs    # 死文件，已无人引用
      http_server/        # 手写 smol HTTP+WebSocket 本地回环服务器
      window_manager/     # 窗口管理：mod / builder / options / url / window
      resource_manager/   # 资源：trait + 文件系统实现 + 内嵌包实现 + 图标工具
      tray_manager/       # 系统托盘        shortcut_manager/ # 全局快捷键
      menu/               # muda 菜单构建（mod + options）
      api/                # 14 个 API 命名空间 + mod.rs 统一入口
      options.rs          # niva.json 顶层结构   assets.rs  # 初始化脚本内嵌
      utils.rs            # 宏与小工具（blocking! / lock! / IdCounter…）
      event_handler.rs    # tao 事件 + 外部事件（菜单/托盘/热键）分发
    assets/               # initialize_script.js（注入每个 webview）+ 默认图标
    build.rs              # Windows winres + 写 version.rs
  crates/icon_creator/    # 小工具：PNG → 多尺寸 .ico（Windows 打包用）
  packages/devtools/      # 图形化开发工具（本身就是一个 Niva 应用，自举打包）
    src/                  # index/app/common/models/pages/modals/i18n/build-scripts/templates
    niva.json             # devtools 自己的项目配置（自举入口）
    public/               # 调试静态资源（含 windows 预编译 exe）
    vite.config.ts        # 端口 3000，输出 build/
  packages/types/         # Niva_zh.d.ts + package.json（file: 链接给 devtools）
  build_MacOS.sh / build_Windows.cmd  # 发版脚本：编 Rust → 自举打包 → zip
  async-lab/              # 独立 workspace 的 smol 实验场（详见其 README）
  docs/                   # 本文档
```

### 1.3 工作区与版本

* Rust workspace：`Cargo.toml`，成员 `crates/niva` + `crates/icon_creator`，`resolver="2"`，release 开 `lto`。
* npm workspaces：根 `package.json`（`packages/devtools` + `packages/types`），脚本只有 `prepare`（husky）与
  `type-checker`（进 devtools 跑 `tsc --noEmit`）。
* 版本：`niva 0.9.9` / `devtools 0.9.9` / `types 0.9.9` 三处一致。

---

## 2. Rust 后端（`crates/niva`）

### 2.1 启动流程

入口 `src/main.rs:1-19`：release Windows 去控制台（`windows_subsystem`），
全 crate 禁止“拿着 Mutex 过 `.await`”（`#![deny(clippy::await_holding_lock)]`），
建 `tao EventLoop<NivaEvent>` → `NivaApp::new` → `app.run(event_loop)`。

`NivaApp::new`（`src/app/mod.rs:85-157`），顺序不可换：

1. `NivaArguments::new()`（`mod.rs:226-257`）：手解析 `--key=value`，**只认 4 个键**：
   `--debug-devtools=true`、`--debug-resource=DIR`、`--debug-config=PATH`、`--debug-entry=URL`。
   注意：发版脚本传的 `--project/--build` **不是 Rust 消费的**，是 devtools 前端自己读 `process.args()` 消费的（见 §3.2）。
2. 选资源管理器（`mod.rs:88-91`）：给了 `--debug-resource` 就用 `FileSystemResource` 读目录，否则用 `AppResourceManager` 读内嵌包。
3. `NivaLaunchInfo::new`（`mod.rs:271-320`）：读 `niva.json`（`--debug-config` 文件优先，否则资源包内 `niva.json`），
   按 `std::env::consts::OS` 取同名顶层键（`"macos"/"windows"`）与 base 递归合并（`merge_values`，`utils.rs:127-140`），
   再反序列化为 `NivaOptions`；`--debug-devtools` 强制开 devtools；
   算出 `id_name = {name小写}_{uuid前8}`，data/cache/temp 目录挂到系统目录下（`BaseDirs`）。
4. macOS 激活策略（`mod.rs:95-116`，其中 `default_menu_creation` 目前是 no-op）。
5. `ApiManager::new` + `register_api_instances` + 包 `Arc`（`mod.rs:119-121`）。
6. `WindowManager::new`（`WebContext` 数据目录指向应用 data_dir）、`NivaShortcutManager::new()`、`NivaTrayManager::new()`。
7. 组装 `Arc<NivaApp>`（`mod.rs:130-141`，含 `event_loop.create_proxy()`），回绑 window/api/tray。
8. `NivaHttpServer::start` 起本地回环 HTTP+WS 服务器（`mod.rs:148-151`），端口随机，打印 `http server listening on …`。
9. `EventHandler::install_external_handlers` 转发 muda/托盘/热键事件（`mod.rs:154`）。

`NivaApp::run`（`mod.rs:191-215`）：按配置开主窗口（id 固定 0）→ 注册配置里的快捷键 → 创建配置里的托盘 →
`event_loop.run(|event,target,cf| handler.handle(...))`，主线程从此不再返回。

### 2.2 事件循环与主线程调用

* `NivaEvent`（`mod.rs:44-69`）：`Pin<Box<dyn Fn(&Target,&mut ControlFlow)->Result<()> + Send>>` 的 newtype，
  tao 的 `UserEvent` 通道，跨线程靠 `Send`。
* `EventHandler::handle`（`event_handler.rs:115-134`）：每 tick 置 `ControlFlow::Wait`，分 `WindowEvent` / `UserEvent`（执行回调），
  错误只打日志不崩循环（`try_or_log_err!`）。
* 窗口事件（`event_handler.rs:136-198`）：
  `Destroyed` → 移出 map + `cleanup_window`；`Focused` → mac 切菜单 + 推 `window.focused`；
  `ScaleFactorChanged/ThemeChanged` → 转发；`CloseRequested` → 若开了拦截（`blockCloseRequested`）则推 `window.closeRequested` 让前端决定，
  否则关闭 + 清理，主窗口（id 0）则 `ControlFlow::Exit`。
* 外部事件桥（`event_handler.rs:27-113`）：muda 菜单 / tray-icon / global-hotkey 都不在 tao 线程，
  统一经各自 `set_event_handler` 转成 `send_ipc_event` 推给前端：
  `menu.clicked(id)`、`tray.leftClicked/rightClicked/doubleClicked(tray_id)`、`shortcut.emit(id)`。
  菜单 id 用 `merge_id(window_id,item_id)`（`utils.rs:173-179`）混淆后再 `split_id` 还原。
* `run_on_main`（`main_exec.rs:15-39`）：任意后台线程跳主线程执行闭包并 `await` 结果。
  实现：`async_channel::bounded(1)` + `Mutex<Option<F>>`（把 `FnOnce` 藏成 `Fn` 以满足 `NivaEvent`），
  `proxy.send_event` 投递，主线程执行后 `try_send` 回结果。**禁止在主线程自己调**（必死锁，注释已写明）。

### 2.3 本地回环 HTTP + WebSocket 服务器

旧 `niva://` 自定义协议已彻底移除（iframe 下有坑），改为手写极简 HTTP 服务器（`http_server/mod.rs`，498 行，
注释明说“只要 GET + `Connection: close`，不要 tokio/axum/hyper”）：

* `NivaHttpServer::start`（`http_server/mod.rs:39-64`）：随机 token，`127.0.0.1:0` 随机端口，
  单驱动线程 `niva-http` 跑 `smol::block_on(accept_loop)`，WS pump 丢到 smol unblock 池。
* `server_info()`（`app/mod.rs:163-167`）给窗口构建提供 `(port, token)`：页面入口 URL 与每窗 WS 地址都从这里拼。
* 静态文件走 `ResourceManager`（debug 目录或内嵌包），缺 entry/资源时返回可读的 HTML 错误页
  （`error_page_html`，`utils.rs:187-204`，wry 与 server 共用）。

### 2.4 窗口系统

* `WindowManager`（`window_manager/mod.rs:20-114`）：`windows: HashMap<u8,Arc<NivaWindow>>` + `id_map: HashMap<WindowId,u8>`，
  id 由 `IdCounter` 分配（u8，上限 255）。`open/get/close/list` 成套；
  `cleanup_window(app, window)`（`mod.rs:107-113`）要求传 `&Arc<App>` 而不是 manager 守卫，避免锁嵌套，
  内容：注销本窗快捷键 + 销毁本窗托盘 + 取消本窗未完成的流式调用。
* `NivaBuilder::build_window`（`builder.rs`）：tao `WindowBuilder` 按 `NivaWindowOptions` 逐项设置
  （标题/主题/图标/尺寸/位置/各开关/全屏/置顶/透明/无边框等）→ 平台 extras（mac 一组 / win 一组）→
  建 muda 菜单并 `attach_menu`（mac `init_for_nsapp`，win `init_for_hwnd`）。
* `NivaBuilder::build_webview`（`builder.rs`）：**无 `with_ipc_handler`、无 custom protocol**。
  注入启动脚本（`__niva_ws_url/__niva_window_id/__niva_token` + `INITIALIZE_SCRIPT`），开 clipboard、
  devtools 开关、`with_accept_first_mouse(true)`、透明背景；导航守卫只放行 server 源与 entry 前缀
  （`url.rs` 取 `scheme://host` 比 `starts_with`）；文件拖放转逻辑坐标后推
  `fileDrop.hovered/dropped/cancelled`（`Over` 故意忽略防刷屏）。
* `NivaWindow`（`window_manager/window.rs:27-191`）：`id/window_id/webview/window`（`Deref<Target=Window>` 透出原生能力）、
  `menu_options/menu_handle`（muda 句柄必须常驻，否则原生菜单消失）、`ws_tx`（本窗 WS 发送端）、`state`（拦截关闭/菜单可见）。
  对外只剩三件事：`send_ws_envelope/send_ws_binary`（WS 未连返回 false 静默丢弃）与
  `send_ipc_event(event, payload)`（包成 `["event",name,payload]`）。`unsafe impl Send+Sync`（`window.rs:61`）。
* 窗口菜单：`build_menu/build_submenu`（`builder.rs`），mac 无配置时给默认菜单（全选/复制/粘贴/剪切/撤销/退出）。

### 2.5 API 系统（三注册 + 分发 + 协议）

统一入口 `api/mod.rs:18-32` 按模块逐个注册。内存类型（`api_manager/mod.rs:24-60`）：
`ApiArguments(Value)`（`get` 整包反序列化 / `optional(size)` 补 null 到定长）、
`ApiRequest(u64 id, String method, args)`（`ok/err` 构造）、`ApiResponse(u64,code,msg,data)`。

三种注册（`api_manager/mod.rs:247-336`）：

| 方法 | 签名形态 | 用途 |
|---|---|---|
| `register_api`（`_with` 可覆超时） | 纯 `async fn`，返回值自动投递 | 一元调用（窗口/系统查询、对话框、剪贴板…） |
| `register_blocking_api` | 同步体，构造上必经 `blocking!` → `smol::unblock` 弹性池 | syscall、子进程 wait、文件 IO、模态框 |
| `register_stream_api` | `async fn(CallContext, Req)->Result<()>`，自驱上下文 | 大文件/长进程/http 流（分片 + 取消） |

分发模型（`api_manager/mod.rs:201-240,422-571`）：`ApiManager` 只在启动期 `&mut` 注册，之后冻结，
`app.api()` 是无锁 `Arc` 克隆。`new` 里起一条 `niva-api` 驱动线程 `smol::block_on(dispatch_rx.recv → spawn(run_job).detach)`，
handler 永不阻塞 driver。`dispatch` 查窗/查表失败直接回错；`dispatch_tx` 是有界队列（默认 64，
`ApiOptions.max_queue`），满了立即回 `code -3 "server busy"`；每个调用登记 `active[(wid,id)]`，
`run_job` 用 `smol::future::or(work, timeout-or-cancelled)` 包起来，超时回 `code -2 "API timeout"`
（默认 30s，`ApiOptions.timeout_ms`；`Some(None)` = 永不超时），结束必关 cancel 通道并摘表，
`cancel_call/cancel_window`（关窗/socket 掉线）静默取消。错误码：`0` ok / `-1` 业务错 / `-2` 超时 / `-3` 忙。

`CallContext`（`api_manager/mod.rs:92-187`）：流式句柄，可廉价克隆。
`push(name,data)` 发事件、`chunk/chunk_stderr` 发二进制分片（首包自动 START）、`respond(result)` 发终态、
`next_chunk/next_chunk_blocking/collect_all` 收上行分片（给 exec stdin 这类交互用）、`cancelled()/is_cancelled()` 协作取消。

Wire protocol v2（`api_manager/protocol.rs:1-160` + 单测 `162-229`）：

* C→S 文本：`{t:"hello",wid}`（首帧绑窗）、`{t:"call",id:u64,method,args}`、`{t:"cancel",id}`。
* S→C 文本：`{t:"result",id,code,message,data}`、`{t:"event",id?,seq,name,data}`（有 id 归属调用，无则广播）。
* 二进制：18 字节头 `ver(1)+flags+id u64BE+seq u64BE`（`BIN_HEADER_LEN`，`protocol.rs:79-85`），
  flags `START 0x01 / END 0x02 / STDERR 0x04`；`encode_chunk/decode_chunk`（`protocol.rs:113-160`）。

### 2.6 API 清单（14 个命名空间，注册行号为实测）

* `clipboard`（`api/clipboard.rs:13-15`）：`read/write`（U，主线程）。
* `dialog`（`api/dialog.rs:13-19`）：`showMessage/pickFile/pickFiles/pickDir/pickDirs/saveFile`（U，主线程 rfd，取消回 null）。
* `extra`（`api/extra.rs:10-19`）：`getActiveWindowId/focusByWindowId`（B）；mac 独占 `hideApplication/showApplication/hideOtherApplications/setActivationPolicy`（U）。
* `fs`（`api/fs.rs:13-24`）：`stat/exists/copy/move/remove/createDir/createDirAll/readDir/readDirAll`（B）；
  `readStream/writeStream`（S，64K 分片）。**注意：已无一元 `read/write/append`**，前端老签名由注入脚本垫（见 §2.10）。
* `http`（`api/http.rs:8-9`）：仅 `requestStream`（S，ureq 阻塞全程包 `blocking!`，先发 `head{status,headers}` 事件再流 body）。
* `monitor`（`api/monitor.rs:12-16`）：`list/current/primary/fromPoint`（U，直调 tao，逻辑像素）。
* `os`（`api/os.rs:12-17`）：`info{os,arch,version}/dirs/sep/eol/locale`（U）。
* `process`（`api/process.rs:13-23`）：`pid/currentDir/currentExe/env/args/setCurrentDir/exit/version`（U）；
  `open`（B，opener）；`execStream`（S：非 detached 时 stdout/stderr 各一 pump 线程发二进制，stdin 由上行分片喂，终态 `{status}`；detached 直接回 pid）。
  **已无一元 `exec`**，老签名由注入脚本垫。
* `resource`（`api/resource.rs:9-12`）：`exists/extract`（B）；`readStream`（S）。一元 `read` 由注入脚本垫。
* `shortcut`（`api/shortcut.rs:10-14`）：`register/unregister/unregisterAll/list`（U，主线程）。
* `tray`（`api/tray.rs:13-18`）：`create/destroy/destroyAll/list/update`（U，主线程）。
* `webview`（`api/webview.rs:10-15`）：`isDevtoolsOpen/openDevtools/closeDevtools/baseUrl/baseFileSystemUrl`（U）。
* `window`（`api/window.rs:34-91`）：52 个一元调用（U）：`current/open/close/list/sendMessage`、菜单四件套、
  几何（内外位置尺寸/最大最小/标题/显隐/聚焦/各开关/最小化/最大化/`Decorated`注意大写 D/全屏/置顶置底/attention/防截屏/全space可见）、
  光标（30+ 枚举 `setCursorIcon`、位置/抓取/显隐）、`dragWindow`（主线程）/`setIgnoreCursorEvents/theme/blockCloseRequested`。
  跨窗统一缺省当前窗。事件（非 API）：`window.focused/scaleFactorChanged/themeChanged/closeRequested/message`。
* `windowExtra`（`api/window_extra.rs:34-62`，条件编译）：Windows `setEnable/setTaskbarIcon(B)/theme/resetDeadKeys/beginResizeDrag/setSkipTaskbar/setUndecoratedShadow`；
  macOS `simpleFullscreen…/hasShadow…/isDocumentEdited…/allowsAutomaticWindowTabbing/tabbingIdentifier` 等。

（U = `register_api`，B = `register_blocking_api`，S = `register_stream_api`）

### 2.7 资源管理

trait（`resource_manager/mod.rs:21-26`）：`exists/load/extract/load_icon`，`Send+Sync+Debug`。两种实现：

* `FileSystemResource`（`mod.rs:28-80`）：`--debug-resource` 目录直读，开发调试用。
* `AppResourceManager`（`mod.rs:82-167`）：发版内嵌包。mac 从 `current_exe/../Resources/RESOURCE_INDEXES + RESOURCE_DATA` 读文件
  （`mod.rs:97-115`）；Windows 从 `RT_RCDATA` 的同名资源读（`mod.rs:117-131`，经 `win_utils.rs` 的 Find/Load/LockResource）。
  包格式：`INDEXES = JSON {path:[offset,length]}`，`DATA = deflate(文件拼接)`，`load = data[offset..offset+len]`，
  与前端打包脚本的键名严格对应（见 §3.5）。`load_icon` 只接受 PNG，带缓存（`mod.rs:66-79/153-166`），
  解码分发三种图标类型（`image_utils.rs`）。
* 图标注意：`assets.rs:2` 的 `DEFAULT_LOGO` 已注释，当前无二进制内默认图标。

### 2.8 托盘 / 快捷键 / 菜单

* `NivaTrayManager`（`tray_manager/mod.rs:38-100` 实测前段）：`trays: HashMap<tray_id,(window_id,menu_ids,ArcMut<TrayIcon>)>`，
  `create`（`77-93`，建图标 + 记录菜单 id 集）/`get/destroy/destroy_all/list/update`，
  所有权按 window 校验；`get_window_id_by_menu_id/by_tray_id`（`58-75`）供事件反查；图标 id 用 `merge_id(wid,tid)` 字符串化。
* `NivaShortcutManager`（`shortcut_manager/mod.rs:17-125` 全读）：`GlobalHotKeyManager` +
  `shortcuts: hotkey_id(u32)→(wid,action_id,HotKey)` + `index: (wid,action_id)→hotkey_id` 双索引；
  `register_with_options`（启动用）/`register_with_id`（重 id、重 accelerator 都报错）/`register`（自动找空闲 u8，
  `wrapping_add` 转一圈）/`unregister/unregister_all/list`。`new` 里 `expect`（`29`），无快捷键环境会崩，见 §8。
* 菜单（`menu/options.rs:1-45` + `menu/mod.rs`）：`MenuItemOption` 三态 `#[serde(tag="type")]`
 （`Native{label}` / `Item{id,label,enabled,selected,icon,accelerator}` / `Menu{label,enabled,children}`），
  id 混淆 `merge_id`，accelerator 只在 mac 解析，`selected` 进 CheckMenuItem，icon 只在 mac 解 png（否则回退普通项），
  出口 `build_menu`（托盘）/`build_submenu`（窗口根菜单）。

### 2.9 工具宏与小件

`utils.rs`：`ArcMut/arc/arc_mut`（`1-17`）；`IdCounter`（`19-39`，成功分支不自增——§8 已知问题）；
`unsafe_impl_sync_send`（`41-47`）；`set_property(_some)`（`49-68`，builder 专用）；
`lock`（返回 Result，`70-77`）/`lock_force`（unwrap，`79-84`）；`logical(_try)`（`86-102`，物理→逻辑像素）；
`log/log_err/log_if_err`（`104-125`，裸 `println!`，无分级）；`try_or_log_err!`（`151-161`）；
`merge_values`（`127-140`，平台配置合并）；`url_join/merge_id/split_id`（`163-179`）；
`html_escape/error_page_html`（`181-204`）；`blocking!`（`217-222`，`smol::unblock` 包同步体，注释立规矩：driver 线程与异步体内禁直接阻塞）。

### 2.10 注入脚本（前端真正的“标准库”）

`assets/initialize_script.js`（486 行）经 `assets.rs:1 include_str!` 编进二进制，每个 webview 初始化时执行：

* 事件总线（`17-60`）：`add/remove/removeAll/__emit__`，`emit` 用 `setTimeout 0` 异步派发，支持 `event / prefix.* / *` 三级匹配。
* WS wire（`73-287`）：读预置的 `__niva_ws_url/__niva_window_id/__niva_token`，首帧 `{t:hello}`，
  `Niva.call → streamCall().promise`，`Niva.stream → {promise,cancel}`（cancel 发 `{t:cancel}` 并清二进制队列），
  `streamSend(id,bytes,end)` 组 18 字节帧；二进制按 stderr 分组、seq 排序、END 合成一个 `Blob` 回 `onBlob`；
  `result.code!=0 → reject(message)`；断线 1s 重连。`Niva.api` 是双层 `Proxy`（`299-321`）：
  `Niva.api.ns.method(...args) → Niva.call("ns.method", args)`。
* **兼容垫片**（`289-462`，保持老签名可用，devtools 与用户项目都在用）：
  `fs.read/write/append` ← `fs.readStream/writeStream`（`370-397`，utf8/base64 双编码）；
  `http.get/post/request` ← `http.requestStream`（`399-425`，拼 `head` + body）；
  `process.exec` ← `process.execStream`（`427-450`，detached 直透，否则拼 stdout/stderr 文本）；
  `resource.read` ← `resource.readStream`（`452-462`）。
* 收尾（`464-486`）：挂 `Niva.call/stream/streamSend`，`delete window.close/open`，挂 `window.Niva`。

### 2.11 icon_creator 与 build.rs

* `crates/icon_creator/src/main.rs:13-32`：`args[1]=源图 args[2]=目标.ico`（无参数校验），
  `image::open` → 16/24/32/48/64/128/256 `resize_exact(Lanczos3)` → 写 `ico::IconDir`。
  依赖与主 crate 不同解码栈（`ico 0.3 + image 0.24`）。
* `crates/niva/build.rs:1-14`：Windows 用 `winres` 编译资源；全平台 `build-version` 写 `version.rs`
 （供 `process.version`，`api/process.rs:1 include!`）。

---

## 3. 前端 devtools（`packages/devtools`）

React 19 + Vite 8 + TS 5.9（`package.json:6-33`），无 CRA/antd/路由/Redux。
脚本：`start=vite` / `build=tsc --noEmit && vite build` / `preview`。
`vite.config.ts`：端口固定 3000（注释写明兼容 `niva.json` 的 `debug.entry`），输出 `build/`。
类型经 `types: file:../types` 本地链接 + `src/index.tsx:4 import "types"` 获得全局 `Niva`。

### 3.1 入口与窗口壳

`src/index.tsx:1-41`：禁右键、禁 `Ctrl+R`；`"*"事件与`Niva.call` 双日志通道；
`Niva.api.window.blockCloseRequested(true)` 拦截关闭；`envReady` 后才 `createRoot.render`。

`src/app.tsx:1-347`：

* `WindowControl`（`31-172`）：自绘 mac（红黄绿）/Windows 三键。mac 关闭走 `app.exit()`；
  mac 最大化写的是 `setMaximized(!true)`（`139`，恒 false，疑似笔误；Windows 分支 `157` 正确）。
* `Titlebar`（`175-201`）：非按钮/链接区域按下调 `window.dragWindow()`。
* `WindowFrame`（`203-296`）：订阅 `window.focused` 切 active 态；状态栏显示 `os.info + process.version`，
  点中英切换、文档/Github 外链（`process.open`）、单击复制版本、双击开 devtools。
* `App`（`298-347`）：`window.app` 单例缓存；`parseArgs(await process.args())` 支持无人值守：
  `--project=` 打开项目，`--build=` 直接构建然后 `window.close()`——发版脚本的自举全靠它；
  无参则恢复最近项目；按历史是否为空切 `ImportPage/ProjectPage`（`342`）。

### 3.2 数据模型（`src/models/`）

* `AppModel`（`app.model.ts:18-245`）：根 store（history/modal/project/locale）。
  `init`（`34-43`）监听 `window.closeRequested → exit()`，history/locale 并行初始化，首屏后检查更新；
  `openWithPicker/open`（`45-139`）：先关旧项目 → 并行查存在性 → 无 `niva.json` 则按 `package.json` 启发式
  （`react-scripts→react` / `vite→vueVite` / `vue→vue` / 否则 `simple`，`100-108`）生成配置写盘 → `ProjectModel.init`；
  `create`（`156-183`，用 `dialog.saveFile` 选位置 + 三件套写盘）；`exit`（`185-199`，有模态栈则拒绝，
  配合 `blockCloseRequested` 形成“拦截-确认-真关”闭环）。
* `ProjectModel`（`project.model.ts:17-243` 全读）：`init`（`65-105`，`fs.read` + `validateConfig`，
  icon 经 `fileSystemUrl(pathJoin(path, debug.resource, icon))` 解析）；`validateConfig`（`228-242`）
  只强校验 `name && uuid`；`dispose/refresh/save`（`107-149`，脏检查 + 未保存确认）；
  `build(target?)`（`151-193`，按 `os.info` 分发 mac/win，`modal.progress` 跑构建脚本，成功可一键打开产物目录）；
  `debug()`（`195-222`）：取自身 `currentExe`，`process.exec(detached:true)` 拉起子进程，
  参数即 `--debug-config/--debug-resource/--debug-devtools/--debug-entry`；
  `open()`（`224-226`）调系统文件管理器。
* `HistoryModel/LocaleModel/ModalModel`（`history.model.ts` 99 行 / `locale.model.ts` 54 行 / `modal.model.tsx` 130 行）：
  历史持久化 + 最近项目；`os.locale()` 以 CN 结尾切中文，`t(key,{{var}})` 模板；
  `show/showNative/alert/confirm/progress` 弹窗栈，`showNative` 先挂空遮罩防 Webview 焦点竞态，
  `ProgressModel.addTask/run` 串行进度机（构建脚本用）。

### 3.3 通用工具（`src/common/utils.ts:1-208` 全读）

模块顶并行预取 `baseFileSystemUrl/sep/dirs/currentDir`（`12-19`），`envReady`（`21-23`）门控渲染；
`tryOrAlert`（`38-54`，Result 转弹窗）；路径三件套 `pathJoin/pathSplit/dirname` + `urlJoin/fileSystemUrl` +
`tempDirWith/dataDirWith/getHome/getCurrentDir`（`65-103`）；`createPromise`（`109-116`）；
`parseArgs`（`119-128`，`--k=v`，`split("=")` 只取首个等号后）；`isAbsolutePath`（`130-132`，只认 `/` 与大写盘符）；
`parseVersion`（`149-158`，非数字 strip，补零到 4 段，供 plist/version-info）；
`checkVersion`（`176-191`，`http.get` GitHub latest 对 `process.version`，命中弹确认窗跳官网）；
`runCmd`（`193-208`，`process.exec` 包一层，非 0 抛错，Windows 打包调 ResourceHacker 用）。

### 3.4 页面与弹窗（结构）

`pages/import/` 空历史落地页（拖拽导入高亮 + 新建/打开）；
`pages/project/` 工作区：`index`（目录栏 + 详情 + 全局拖拽遮罩）、`list`（历史搜索/高亮/删除）、
`info`（信息/配置双 Tab + 脏标记 `*`）、`details`（Logo/名称/路径/uuid/调试与构建入口）、
`logo.tsx` 与 `icon.tsx` 是完全重复的 Logo 组件（后者疑似死文件）、
`config-editor/`（Ace JSON 编辑，Ctrl/Cmd+S 保存，RESET 丢弃重载）；
`modals/`（Alert/Confirm/Progress 三件套 + 栈式渲染）；`i18n/`（`en_US` 54 键源 + 同构 `zh_CN`）。

### 3.5 构建脚本（`src/build-scripts/`，打包核心）

* `base.ts:7-81` 全读：`indexesKey=RESOURCE_INDEXES`、`dataKey=RESOURCE_DATA`（与 Rust 资源管理器键名严格对应）；
  `packageResource`（`readDirAll` 扁平遍历）+ `appendResource`（`fs.read(base64)` → 拼 ArrayBuffer，
  记 `[offset,length]`）+ base64/ArrayBuffer 互转与拼接。
* `build-macos.ts:13-111` 全读（6 个 task）：建 `*.app/Contents/{Resources/icon.iconset,MacOS}` →
  拷当前 exe 作模板 → 首包 `niva.json` 再包 `build.resource` 全量 → `pako.deflateRaw` 压成
  `RESOURCE_INDEXES(JSON)` + `RESOURCE_DATA(base64)` → 有 icon 才 `sips` 5 尺寸 + `iconutil` 合 icns →
  `plistTemplate` 写 `Info.plist`。止于 `.app`，不做 dmg/zip。
* `build-windows.ts`（125 行）：备料（`tempDirWith(<name>_<uuid前8>)`，与 Rust `id_name` 呼应）→ 打包压缩同 mac →
  `resource.extract("windows/icon_creator.exe")` + 跑它生成多尺寸 ico →
  `versionInfoTemplate` 写版本信息 + 拼 ResourceHacker 脚本（exe 换壳：INDEXES/DATA 进 RCDATA，图标进 ICONGROUP）→
  清理。注意：用的 exe 是 `public/windows/` 里随前端构建进包的**预编译二进制**，不是本次 cargo 产物。
* `templates/`：`config-template.ts`（四档脚手架 `simple/vueVite/vue/react`，端口 5173/8080/3000）、
  `new-project-template.ts`（8 行：niva.json + Hello World 两件套）、
  `macos-plist-template.ts`（42 行）、`windows-version-info-template.ts`（34 行）。

---

## 4. 类型包（`packages/types`）

`Niva_zh.d.ts` 1423 行，头三行即 `eslint-disable + prettier-ignore + @ts-nocheck`（`1-3`），
`declare global { const Niva: NivaObj }`（`5-8`）。`NivaObj` 含事件四件套 + `api` 下 14 个命名空间
（与 §2.6 的 Rust 注册名一一对应，另保留老一元签名 `fs.read/write/append、http.request/get/post、
process.exec、resource.read`——运行时由注入脚本垫到流式实现，见 §2.10）：
clipboard（`391-397`）、dialog（`408-432`）、extra（`465` 起）、fs（含 `read/write/readDir/readDirAll 523-597`）、
http（含 `request/get/post` 与 `requestStream`，`606-651`）、monitor（`662-672` 起）、os、process
（含 `exec 798` 与 `execStream 811`）、resource（含 `read 842` 与 `readStream 846`）、shortcut、tray、webview、
window（约 50 方法，含 `requestUserAttention 1227` 等）、windowExtra（平台专属）。
配置类型：`NivaOptions`（`95-121`，含递归 `macos/windows` 覆盖）、窗口/托盘/快捷键/菜单/事件表。
`package.json`（`name:types version:0.9.9 private main:./Niva_zh.d.ts`），README 只有简介无用法，**以 d.ts 本体为准**。

---

## 5. niva.json 配置

以 `packages/devtools/niva.json` 为实例（name/uuid 必填——`project.model.ts:238` 只认这两项）：

```jsonc
{
  "name": "NivaDevtools", "uuid": "9bef098a-…", "icon": "icon.png", // 仅 png
  "version": "0.9.9",              // Rust 不认识，仅展示/模板用
  "debug": { "entry": "http://localhost:3000", "resource": "public" },
  "build": { "resource": "build" },// Rust 不认识，仅构建脚本用
  "window": { "title","icon","size/minSize/maxSize/position","resizable/minimizable/…",
              "fullscreen/maximized/visible/transparent/decorations",
              "alwaysOnTop/alwaysOnBottom/visibleOnAllWorkspaces/focused/contentProtection",
              "theme/devtools/entry","menu": […] },
  "tray": { "icon","title","tooltip","menu": […] },
  "shortcuts": [{ "accelerator": "Ctrl+N", "id": 1 }],
  "api": { "timeoutMs": 30000, "maxQueue": 64 },
  "macos": { "window": { … } },    // 平台覆盖：按 OS 名取键与 base 递归合并
  "windows": { "window": { … } }   // devtools 实例：mac 无边框四件套 / win 无边框+阴影
}
```

mac 窗口还有：`movableByWindowBackground/titleBarTransparent/titleHidden/titleBarButtonsHidden/
fullSizeContentView/resizeIncrements/disallowHiDpi/hasShadow/automaticWindowTabbing/tabbingIdentifier`；
win 还有：`parentWindow/ownerWindow/taskbarIcon/skipTaskbar/undecoratedShadow`。
mac 应用级：`activationPolicy/defaultMenuCreation/activateIgnoringOtherApps`（`options.rs:10-14`）。

---

## 6. 构建与发布链

### 6.1 `build_MacOS.sh:1-33`

`git describe` 取版本 → `yarn && devtools build`（删 `build/windows` 免进 mac 包）→
双 target release 编译（`RUSTFLAGS="-l framework=WebKit" MACOSX_DEPLOYMENT_TARGET=11.0`，
对应 `rust-toolchain.toml` 的双 darwin target）→ **用刚编出的 x86_64 niva 给自己打包**
（`--debug-resource/--debug-config/--project/--build=dist/x86_64/NivaDevtools.app`，
`--project/--build` 由 devtools 前端消费做无人值守构建）→ 拷包骨架换 aarch64 Mach-O →
两个 `zip`。产物：`dist/{x86_64,aarch64}/NivaDevtools.app` + 两 zip。

### 6.2 `build_Windows.cmd`

同构单 target 版：`yarn + devtools build`（保留 `build/windows`）→ `cargo build --release` →
`niva.exe --debug-resource --debug-config --project --build=dist\NivaDevtools.exe` →
`Compress-Archive` 成 zip。产物：`dist\NivaDevtools.exe` + zip。

### 6.3 周边配置

* `.vscode/launch.json` 多组 lldb（含 `--debug-entry=http://localhost:3000` 本地调试）；
  `tasks.json` 只有 `build:devtools`；`settings.json` 只有 cSpell。
* `.husky/pre-commit` 跑根 `type-checker`（`tsc --noEmit`）；`.npmrc:1` 键名拼写为 `regestry`（少个 `i`），实际不生效。

---

## 7. 协议与格式速查

* JS→Rust（WS 文本）：`{t:"hello",wid}` / `{t:"call",id,method,args}` / `{t:"cancel",id}`。
* Rust→JS（WS 文本）：`{t:"result",id,code,message,data}`（0/-1/-2/-3）/ `{t:"event",id?,seq,name,data}`。
* 二进制帧：`[ver=1][flags:START|END|STDERR][id u64BE][seq u64BE][payload]`，18 字节头。
* 资源包：`RESOURCE_INDEXES = JSON{path:[offset,length]}` + `RESOURCE_DATA = deflate(拼接)`；
  mac 放 `Contents/Resources/` 两文件，win 进 `RCDATA` 同名资源；首包永远是 `niva.json`。
* 前端事件名：`window.focused/scaleFactorChanged/themeChanged/closeRequested/message`、
  `menu.clicked`、`tray.left/right/doubleClicked`、`shortcut.emit`、`fileDrop.hovered/dropped/cancelled`。

---

## 8. 现状已知问题（实测）

1. `utils.rs:28-38 IdCounter::next` 成功分支不自增（且 `0..u8::MAX` 取不到 255，debug 下 `u8` 加法可溢出 panic）。
2. `process.exec/fs.read/write/http.get` 等老一元签名 Rust 侧已无注册，全靠 `initialize_script.js:369-462` 垫片；
   若有人绕过注入脚本直连 WS 调这些名字会得 `api not found`。
3. `app.tsx:139` mac 最大化 `setMaximized(!true)` 恒 false；`pages/project/icon.tsx` 与 `logo.tsx` 完全重复。
4. `shortcut_manager/mod.rs:29` `expect` 建全局热键管理器；`main.rs` 外 `win_utils/from_wstr` 无界读、
   `fs.readDir` 系 `unwrap` 等历史坑仍在（见早前评估）。
5. `.npmrc:1` 拼写、`thread_pool.rs` 死文件、`niva_macros` 已删但发版注释/文档多处仍提旧宏（`#[niva_api]` 已不存在，
   现行写法是三注册函数 + `blocking!`）。

---

## 9. 本地开发工作流

```bash
# 前端 devtools 调试（需先起 vite，见 vite.config.ts 端口 3000）
cargo run -- --debug-resource=packages/devtools/public \
  --debug-config=packages/devtools/niva.json \
  --debug-entry=http://localhost:3000 --debug-devtools=true
# 无人值守构建（给脚本用的也是这套）
niva --debug-resource=…/build --debug-config=…/niva.json --project=… --build=…/xxx.app
# 类型检查
npm run type-checker
# smol 实验
cd async-lab && cargo run
```

---

## 10. Windows 打包迁移（计划，已定）

Windows 打包从 `ResourceHacker.exe + icon_creator.exe` 迁移到自研
`crates/win_packager`（备料 + PNG→ICO + 注入一次完成，对外唯一 Windows
打包器）。计划、对照表、验收标准见 `crates/win_packager/README.md` §8，
实现 stub 在 `crates/win_packager/src/windows_impl.rs`。
关系到本手册 §2.11（icon_creator）、§3.5（build-windows.ts）、
§6.2（build_Windows.cmd）的描述，迁移完成后更新。
