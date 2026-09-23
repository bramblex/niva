# Niva 项目手册

> 本文按 2026-09-23 的源码整理，依赖版本和实现细节以对应清单与源码为准。文中区分源码状态与平台验收：macOS 的手工记录不能替代 Windows 真机验证。

## 1. 项目概览

Niva 是以 Rust 和系统 WebView 构建桌面应用的框架。Rust 管理窗口、资源、HTTP/WS、原生 IPC 和系统 API；前端可使用普通 HTML/JavaScript 或 Vue、React 等工具链。Devtools 是一个由 Niva 自举打包的桌面应用。

### 1.1 技术组成

| 部分 | 当前实现 | 主要位置 |
|---|---|---|
| 桌面窗口与 WebView | Rust 2024、tao、wry | `crates/niva/Cargo.toml`、`crates/niva/src/app/window_manager/` |
| 原生菜单、托盘、快捷键 | muda、tray-icon、global-hotkey | `crates/niva/src/app/{menu,tray_manager,shortcut_manager}/` |
| API 调度与网络 | smol、async-channel、tungstenite、ureq 3 | `crates/niva/src/app/api_manager/`、`http_server/` |
| 窗口通信 | 每窗 WebSocket 与平台 IPC 混合桥接 | `window_manager/builder.rs`、`ipc_macos.rs`、`ipc_windows_frames.rs`、`initialize_script.js` |
| 可选 Node 兼容层 | 独立 `packages/node-compat` 包，按项目配置打包 | `packages/node-compat/`、`crates/niva/src/app/node_compat.rs` |
| 图形化开发工具 | React、Vite、TypeScript | `packages/devtools/` |
| 类型声明 | `packages/types/Niva_zh.d.ts` | `packages/types/` |
| Windows 资源封装 | 自研 `win_packager` | `crates/win_packager/` |

精确依赖版本见 `Cargo.toml`、`package.json` 和各 workspace package manifest；手册不复制版本号，避免清单更新后产生过期版本表。

### 1.2 仓库布局

```text
niva/
  crates/niva/                 # 桌面框架、原生 API、HTTP/WS、平台 IPC、stdio 与 NodeCompat
  crates/win_packager/         # Windows 可执行文件和资源封装工具
  crates/icon_creator/         # 图标转换辅助 crate
  packages/devtools/           # Niva 图形化开发工具及构建脚本
  packages/types/              # TypeScript API 声明
  packages/node-compat/        # 可选 Node 风格模块适配包
  packages/examples/           # 示例前端项目
  examples/                    # stdio 宿主与示例 UI
  docs/                        # 手册、协议、设计和验收记录
  build_MacOS.sh               # macOS 自举构建脚本
  build_Windows.cmd            # Windows 自举构建脚本
```

当前仓库中没有旧的 `async-lab/` 或 `crates/niva_macros/`。API 注册在 Rust 函数中完成；同步工作通过显式 blocking 注册/`blocking!` 处理，没有已删除的属性宏依赖。

## 2. 启动与应用生命周期

入口为 `crates/niva/src/main.rs`，应用组装在 `crates/niva/src/app/mod.rs`：解析命令行参数、读取项目配置与资源、注册 API、初始化窗口/托盘/快捷键管理器、启动本地回环 HTTP+WS 服务，然后进入 tao 事件循环。

主窗口 id 为 `0`。配置窗口、托盘和快捷键在事件循环启动时创建或注册。窗口关闭时清理它拥有的快捷键、托盘及未完成 API 调用；主窗口退出会结束应用。应用级 stdio 桥只服务主窗口。

调试参数包括 `--debug-resource=DIR`、`--debug-config=PATH`、`--debug-entry=URL`、`--debug-devtools=true` 和 `--stdio`。`--project`/`--build` 由 Devtools 前端用于无人值守构建；Rust 也识别 `--build` 的存在，用它禁止打包时误用配置中的 Vite 调试入口。

## 3. 窗口、页面与通信

### 3.1 每窗口的桥接选择

Niva 并非只使用 WebSocket，也并非统一改用 IPC。每个窗口根据启动方式、平台和页面来源选择通信路径：

- 受信任的本地页面在支持且满足条件时使用该窗口专属 WebSocket；窗口管理器为每个窗口生成独立凭据，调用和事件使用 `initialize_script.js` 中的 wire 协议。
- **打包本地页**由 Wry 异步自定义协议从 `niva://app/` 加载；macOS origin 是 `niva://app`，Windows WebView2 origin 是 `http://niva.app`。WebSocket 与 `__niva_fs` 仍经动态 `127.0.0.1:<port>` 服务；打包模式普通 HTTP 静态路由关闭。
- macOS 通过 WKWebView 消息处理器传递 IPC；Windows 通过 WebView2 WebMessage 与 frame 处理器传递 IPC。跨源页面/iframe 等需要 IPC 的上下文可走平台 IPC 路径。
- 显式 debug 启动可加载跨端口开发入口并授予该窗口桥接；带 `--debug-resource` 的开发启动继续通过 loopback HTTP 加载静态资源。普通打包启动忽略配置中的 debug entry。每个原生 API 调用仍需在 Rust 侧做窗口、来源与授权校验。
- WebSocket 断开、事件转发和页面来源边界应以 `docs/bridge.md`、`docs/security.md` 及对应平台代码为准。

当前验收记录：macOS 常规 WebView 手工验证覆盖本地主 frame 与同源 iframe 的 WS、跨源顶层页与 iframe 的 IPC，以及拒绝/授权、CSP、文件 URL 凭据场景。2026-09-23 另以临时 app 实测固定协议 origin `niva://app`、主页面/同源 iframe 的 WS 原生调用、静态 JS 资源，以及普通 loopback 静态请求 404。通过 `webview.baseFileSystemUrl()` 在该页 `fetch` 文件成功；有效 token 配非匹配 Origin 时服务端不返回 ACAO，无效 token 返回 403。另用只选 `path/fs/assert/stream` 的临时包验证静态 `import 'path'`、`Niva.import('fs/promises')`、`require('assert/strict')` 和未选 `child_process.js` 资源 404。未测 NodeCompat 全模块语义、WS 二进制流、存储迁移或性能。Windows 已有限实测 WebView2 打包页、同源 iframe、文件 URL 和部分 NodeCompat；详见 `docs/windows-validation-2026-09-23.md`。远端 IPC 与完整行为矩阵仍待验收。

### 3.2 API 调度与协议

API 名称在 `crates/niva/src/app/api/` 注册，`ApiManager` 将请求按窗口和方法分派。异步 API 直接运行；可能阻塞的调用应走 blocking 执行路径；长任务可采用流式接口并支持取消。公开签名以 `packages/types/Niva_zh.d.ts` 和 `docs/bridge.md` 为准，避免依赖手册中的旧 API 数量或行号。

WebSocket wire 协议和二进制帧格式以 `docs/bridge.md` 与 `crates/niva/src/app/api_manager/protocol.rs` 为准。IPC 使用各平台消息封装，不能将 WS 文本帧格式误认为平台 IPC 的传输格式。

### 3.3 stdio Host Bridge

`--stdio` 显式启用子进程宿主模式。`crates/niva/src/app/stdio.rs` 在 stdin/stdout 上收发 NDJSON：宿主发 `msg` 帧给主窗口，主窗口可通过 `Niva.api.host.send` 回发；WebSocket 主窗口握手后输出一次 `ready`。stdout 留给协议帧，诊断写 stderr。输入行有 64 MiB 上限，EOF 或管道写失败请求应用退出。接口与限制见 `docs/stdio-host-design.md`，可运行示例位于 `examples/stdio_host.py` 和 `examples/stdio-host/`。

验收边界：roadmap 记录了 macOS Python 宿主往返、坏帧恢复、EOF/BrokenPipe 退出的实际验证；Windows Python 宿主的管道继承、坏帧恢复及 EOF 退出已真机验证；BrokenPipe 等边界仍待测。

## 4. 资源、配置与 NodeCompat

资源由 `ResourceManager` 抽象提供。调试时可从文件系统目录读取；打包应用从平台资源容器加载，索引记录资源路径及其压缩数据区间。macOS 使用 `.app/Contents/Resources`；Windows 使用可执行文件资源。打包格式与读取实现见 `crates/niva/src/app/resource_manager/` 以及平台构建脚本。

`niva.json` 同时包含应用元数据、窗口/托盘/快捷键/API 设置和平台覆盖。项目配置字段与类型以 `packages/types/Niva_zh.d.ts`、`crates/niva/src/app/options.rs` 和 Devtools 配置编辑器为准。`nodeCompat` 是显式 opt-in：可选 `true` 或模块/importmap 配置；默认关闭时不把适配文件加入应用资源。

启用 NodeCompat 后，Devtools 按允许的模块集合暂存并打包 `packages/node-compat` 文件。运行时仅在符合条件的 HTML 文档导航响应中注入脚本和 importmap；打包模式下脚本和被 allowlist 的 ESM 资源通过 `niva://app/` 提供，文件系统 debug 模式沿用 loopback HTTP 路由。importmap 合并遵循实现中的用户映射优先规则。模块清单、配置格式和限制见 `docs/node-compat-design.md` 与 `packages/node-compat/README.md`。它不提供完整 Node.js 运行时或任意 npm 包兼容；真实 WebView smoke 覆盖了少量选中模块，不代表完整模块/API 验收。

## 5. Devtools 与构建

Devtools 使用 Vite，开发服务器固定端口 `3000`（`strictPort`），与 `packages/devtools/niva.json` 中的调试入口一致；生产构建输出到 `packages/devtools/build`。`npm run build --workspace=packages/devtools` 执行 TypeScript 检查与 Vite 构建，prebuild 脚本会准备 NodeCompat 资源。

Devtools 构建流程位于 `packages/devtools/src/build-scripts/`：macOS 生成 `.app` 目录结构、内嵌资源并生成 plist/图标；Windows 调用随 Devtools 资源发布的 `win_packager.exe`，封装资源和版本信息，可按项目配置处理图标及 NodeCompat 资源。Windows 当前构建路径已迁移到自研 `crates/win_packager`；`ResourceHacker.exe` 不再是活动实现或依赖，旧的 `icon_creator.exe`/ResourceHacker 两阶段脚本也不应再描述为现行流程。打包器行为和命令见 `crates/win_packager/README.md`。

仓库根目录脚本负责编译 Niva 并启动 Devtools 自举构建。平台发布产物与签名配置以 `build_MacOS.sh`、`build_Windows.cmd`、Devtools signing scripts 和 `docs/roadmap.md` 中的实际记录为准。源码或 target check 通过不能替代目标平台真机验收。

## 6. 开发与检查

典型本地开发方式：先在 `packages/devtools` 启动 Vite，再运行 Niva 指向 Devtools 配置和本地入口：

```sh
npm run start --workspace=packages/devtools
cargo run -p niva -- \
  --debug-resource=packages/devtools/public \
  --debug-config=packages/devtools/niva.json \
  --debug-entry=http://localhost:3000 --debug-devtools=true
```

Windows target 编译检查与真机运行是不同的证据。文档或发布记录应明确说明所运行的命令、目标平台和是否使用真实设备；状态汇总见 `docs/roadmap.md`。CI workflow 文件已存在，但 roadmap 当前要求留下 GitHub 上实际运行成功的记录。

## 7. 窗口、菜单、托盘与快捷键状态

`docs/window-tray-menu-plan.md` 跟踪窗口/托盘/菜单整治。当前源码包含三个 P0 修复：快捷键管理器初始化不再因 `expect` 直接 panic、Windows owner 使用独立的 `owner_window` 配置字段、窗口菜单 set/hide/show 操作经主线程执行。源码状态不等于平台验收完成：Windows 已有限验证 owner、菜单点击及快捷键；macOS 菜单和更广行为矩阵仍待完成。

方案中 P1 的菜单原生项日志处理、跨平台快捷键/菜单图标和 PNG 缩放需要分别按计划文档中源码状态核对；特别是 Windows 菜单快捷键消息循环限制仍需作为平台限制处理。托盘、菜单、快捷键的行为验收要在受影响平台实际操作。

## 8. 发布边界与当前未完成项

- Windows target check 只证明交叉编译检查覆盖通过，不证明 Windows 上窗口、菜单、IPC、stdio 或打包流程可运行。
- macOS 手工桥接与 stdio 验证有 roadmap 记录；Devtools Vite UI/HMR 的完整实际操作验收仍待补。
- 固定打包 origin 与普通 HTTP 静态路由隔离已有源码和有限的 macOS smoke 证据；Windows WebView2 已有有限真机 smoke；严格 CSP、response-header 环境和其他 v1.0 门禁仍未完成。不要因本文描述实现存在而宣称 v1.0 发布门禁已关闭。
- 本轮文档/版本字段核对后的 macOS 双架构 Devtools 候选标记为 `v0.9.10-18-gf3f9036-dirty`；其中 Niva release 裸二进制为 arm64 2,835,264 字节、x86_64 3,175,264 字节。两份 zip 完整性、Mach-O 架构和 `Info.plist` 的应用版本 `0.9.9.0` 已核对。这是带未提交改动的本机候选，不等于签名后的发布包；Windows 默认 unwind 裸二进制为 3,877,376 字节；macOS/Windows 共享 release profile 现使用 panic=abort，Windows 裸二进制为 2,450,432 字节，详见 Windows 验证记录。较早的本机 arm64 `target/release/niva` 为 2,904,944 字节，不与双架构产物混用。

最新状态以 `docs/roadmap.md`、专题设计文档和对应平台验收证据为准。本手册描述架构与使用路径，不是平台验收清单。
