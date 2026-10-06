# Niva 项目手册

本文概述当前源码结构、开发入口和打包方式。当前候选为 0.10.0-beta.1；候选验收仍有发布阻断，平台状态、检查结果和未完成项以[候选验收记录](release-0.10.0-beta.1.md)为准。本手册不把源码存在或跨平台编译通过当作目标平台真机验收。

## 1. 项目结构

Niva 使用 Rust 和系统 WebView 运行桌面应用，不随业务应用附带 Chromium 或 Node.js。Rust 宿主管理窗口、资源、原生 API 和桥接；页面侧统一使用 TypeScript runtime。图形化工具与命令行共用 `niva-packager` 打包核心。

| 部分 | 当前实现 | 主要位置 |
|---|---|---|
| 桌面宿主、原生 API、资源与桥接 | Rust、tao、wry | `crates/niva/` |
| 页面 runtime 与 Node 风格适配器 | TypeScript | `packages/runtime/` |
| 公开 API 类型 | 从 runtime 合约生成 | `packages/types/` |
| 图形化开发工具 | React、Vite、TypeScript | `packages/devtools/` |
| GUI/CLI 共用打包器 | Rust 独立 CLI | `crates/niva_packager/` |
| Windows 可执行文件资源支持 | Rust | `crates/win_packager/` |

```text
crates/niva/             桌面运行时、原生 API、窗口与系统集成
crates/niva_packager/    使用预编译 Niva runtime 构建业务应用
crates/win_packager/     Windows 可执行文件与资源封装
packages/runtime/        页面侧统一 TypeScript runtime
packages/types/          公开 TypeScript 类型与生成脚本
packages/devtools/       图形化项目管理、调试和构建工具
examples/                示例项目
docs/                    bridge、打包、设计与验收说明
```

## 2. 页面 API、runtime 与类型

Native bootstrap 总会创建 `Niva`，原生 API 和 Node 风格适配器都是它的属性。`injectCommonJs` 和 `injectEsm` 是独立配置，默认均为 `false`。前者启用 CommonJS 全局及加载器；当前支持 JavaScript/JSON 文件、缓存和循环依赖，不支持原生 addon、`require(ESM)` 或 `require.extensions`。后者为可信本地页面注入 Node 内置模块的 ESM import map；远端页面需要自行配置映射或打包相关依赖。两种模式使用同一 runtime，不代表完整 Node.js 兼容。[Runtime 说明](../packages/runtime/README.md#optional-node-globals)列出模块边界和限制。

`packages/runtime/src/contracts.ts` 是公开 runtime 合约的来源，`packages/types/` 从该合约生成声明。类型包根入口面向浏览器页面，只添加 `Niva` 和 `NivaOptions`，默认不提供 Node 全局或 `node:*` ambient 模块。`@types/node` 是可选 peer；浏览器消费者不会因此安装或自动发现 Node 声明。TypeScript 程序应按用途独立选择声明模式：启用 `injectCommonJs` 的 Niva 页面使用 `@niva/types/commonjs`；运行于 Node 的工具使用 `@niva/types/node`、安装 `@types/node` 并设置 `types: ["node"]`。运行时注入开关与声明模式是两项独立选择，完整说明见[types README](../packages/types/README.md)。Native API 与模块兼容 API 的桥接和权限边界见[Bridge 合约](bridge.md)。

Bridge 当前源码已实现 API 控制面/纯数据面分离：异步 API 以 IPC `t:"api_call"`创建/控制，Native 小结果和流 `channelOpened`/capability 经 Wry `evaluate_script`调用 `__niva_ipc_reply({sessionId,rid,sourceOrigin,response})`返回；流拿到凭证后才 attach。WS v2 仅承载 attach/data/ack/cancel，hello 与18字节 binary header 的 version 均为2；Native 资源 owner 固定为创建时 IPC session，数据 transport 独立。顶层与同源 iframe 共用 Wry IPC/eval；远端顶层页面仅有精确 origin grant 下的 unary API，跨源 iframe 即使有 grant 也 fail-closed。现有 WK reply handler 与 WebView2 专用 IPC/reply 已移除，统一使用 Wry IPC 与 `evaluate_script`。真实 macOS 两 lane WebView smoke 已通过；Windows target check 和真机/CI 状态仍开放，完整证据见[Bridge v2 验收记录](bridge-v2-validation.md)及[Bridge 合约](bridge.md)。

runtime 构建会生成 Native 所需的 bootstrap、ESM facade 和类型声明。Cargo 不执行 npm 构建；Native 构建会检查生成资产及其哈希，缺失或过期时需要先重建 runtime。[Runtime README](../packages/runtime/README.md#build-and-verification)说明了生成和检查流程。

## 3. 启动、资源与调试参数

应用入口为 `crates/niva/src/main.rs`，应用组装和参数解析位于 `crates/niva/src/app/mod.rs`。相关命令行参数为：

- `--resource=DIR`：读取业务资源的目录。
- `--config=PATH`：读取 `niva.json` 配置。
- `--debug-entry=URL`：显式指定本次调试页面入口。
- `--debug-devtools[=true|false]`：启用配置中的调试入口；显式 `--debug-entry` 优先使用命令行入口。
- `--build`：构建模式，不使用配置中的调试入口。

先完成第 6 节中的依赖安装和 `packages/runtime` 构建，生成 Native 要嵌入的页面 runtime。然后在两个终端中分别启动 Devtools 与 Niva；路径相对于仓库根目录。第一条命令会持续运行 Devtools Vite 服务：

```sh
npm run start --workspace=packages/devtools
```

在第二个终端运行：

```sh
cargo run -p niva -- \
  --resource=packages/devtools/public \
  --config=packages/devtools/niva.json \
  --debug-entry=http://localhost:3000 \
  --debug-devtools=true
```

`--debug-resource`、`--debug-config` 和 `--stdio` 已移除，当前入口会拒绝这些参数。`--resource` 与 `--config` 本身不授予调试页面权限；显式调试入口的来源限制与权限边界见[Bridge 合约](bridge.md#来源与鉴权)。

打包应用使用应用 UUID 派生的本地资源 origin；文件系统调试资源由 `ResourceManager` 读取。资源存储布局、路径和访问限制以当前实现及[打包说明](packager-usage.md)为准。

## 4. 作为子进程运行与标准流

Shell、Python 等宿主程序可以直接启动 Niva。主窗口通过 `Niva.process.stdin`、`stdout` 和 `stderr` 收发字节；启用 CommonJS 注入后，也可通过 `process` 全局或 `require('process')` 使用。子窗口不提供宿主 process 标准流。

没有专用 stdio 启动开关，也没有 Native 强制的 NDJSON、ready 消息或请求 ID；消息格式由应用双方约定。stdin EOF 只结束输入流，不自动退出窗口。stdout/stderr 的管道错误由应用处理。Niva 框架日志写入独立文件；日志初始化或轮转失败时不会改写应用的标准流。WebView 底层组件的直接系统输出仍需按平台实际验证。

页面示例、管道拆包和回压说明见[普通 process 标准流宿主说明](stdio-host-design.md)。旧 `api.host`、`--stdio` 和框架内置 NDJSON 协议不属于当前接口。

## 5. Devtools、打包器与 build kit

Devtools 通过 Niva 原生 API 完成项目管理和构建操作；普通浏览器不能替代 Niva 宿主完成这些操作。界面通过 `niva-packager` CLI 调用统一 Rust 打包核心。CLI 用户也可直接调用同一个打包器。发布的 build kit 包含宿主打包器、预编译 Niva runtime、版本与哈希清单及许可材料；业务应用打包不要求用户安装 Node.js 或 Rust。[跨平台打包说明](packager-usage.md)包含 kit 布局、CLI 参数、签名和资源布局细节。

```sh
./niva-packager build \
  --manifest ./manifest.json \
  --config /path/to/project/niva.json \
  --resource-dir /path/to/project/dist \
  --output-dir /path/to/output \
  --resource-layout embedded \
  --target windows-x86_64 \
  --target macos-aarch64 \
  --target macos-x86_64
```

`embedded` 是默认资源布局：Windows 输出单个 `.exe`，macOS 输出包含 `.app` 的 ZIP。`external` 为业务资源保留独立目录，适合需要按需读取的大型资源。每个预编译 runtime 都必须通过对应平台的体积门禁：macOS 严格小于 3,300,000 bytes，Windows 严格小于 3,500,000 bytes；这不等于整个业务应用或 build kit 的大小，也不表示所有候选目标已通过。

## 6. 构建与检查

从仓库根目录按顺序安装依赖、生成 runtime，再构建 Native 程序和 Devtools：

```sh
npm ci
npm run build --workspace=packages/runtime
cargo build --release -p niva -p niva-packager
npm run build --workspace=packages/devtools
```

`packages/devtools` 的 build 包含 TypeScript 检查和 Vite 构建。涉及公开类型时，可运行 `npm run typecheck --workspace=packages/types`。Rust 改动的格式、检查、Clippy 和测试命令，以及其他仓库门禁，以根目录 `AGENTS.md` 为准。候选级检查结果不能由本手册中的命令列表推定；当前已执行和未通过项目见[0.10.0-beta.1 候选验收记录](release-0.10.0-beta.1.md)。

## 7. 平台验收状态

0.10.0-beta.1 尚未发布，候选记录仍列有发布阻断项。Windows MSVC target check 只证明交叉编译检查结果，不能代替 Windows 真机运行；macOS 记录也只适用于明确列出的目标和操作。窗口、托盘、菜单、快捷键、WebView、资源和打包行为的验收必须保留各平台各自的证据。当前逐项状态以[候选验收记录](release-0.10.0-beta.1.md)和[路线图](roadmap.md)为准。
