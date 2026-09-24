# Node 兼容层（NodeCompat）设计与当前边界

> 状态：NodeCompat 已作为浏览器 JS 资源嵌入 Niva 主程序，并默认启用。它提供 22 个 Node 风格模块的有限子集，不是 Node.js 运行时。逐 API 支持范围以[覆盖盘点](node-api-coverage.md)为准。

## 1. 目标与范围

NodeCompat 让页面和打包后的纯 JS 库可以使用一组选定的 Node 风格 API，例如 `node:fs/promises`、`node:path` 和 `node:buffer`。依赖在应用构建时解析为浏览器资源；Niva 主程序内嵌兼容层资源，运行应用不需要 Node runtime。实际兼容性仍取决于库使用的完整 API、选项和依赖链。

当前目标为 **22 个模块族、179 项 API**：`path`、`os`、`fs`、`child_process`、`events`、`util`、`querystring`、`buffer`、`url`、`crypto`、`zlib`、`http`、`https`、`assert`、`stream`、`process`、`net`、`dgram`、`tls`、`dns`、`string_decoder`、`timers`。目标清单不表示其中每项 API 都已实现；状态和限制见[覆盖盘点](node-api-coverage.md)。不提供完整 Node/npm 加载器、任意运行时 `require()`、原生 addon 或完整事件循环。

默认启用 NodeCompat。`nodeCompat: false` 可关闭；对象配置可用 `modules` 选择模块子集、用 `importmap` 控制 HTML 文档中的裸名 importmap 注入。未填写 `modules` 时启用全部 22 个模块。相关资源从主程序嵌入的资源索引读取，不再要求 Devtools 重复暂存 NodeCompat 文件。

Node 专有行为按能力分层：Native `net`、`dgram`、`tls` 提供系统 socket 与 TLS 基础；HTTP/HTTPS 应用模块和 DNS 协议层在 JS 实现。Niva 内部启动用 HTTP/WS/资源服务继续由 Native 提供，它与应用调用 `http.createServer()` 是不同服务。HTTP/HTTPS 的迁移已删除旧 `Niva.api.http`/ureq 客户端；Node HTTP server 关闭清理仍有待修复项。DNS 的 `lookup` 与 `resolve` 语义不同，逐项完成情况以源码及 API 清单为准。[网络分层及测量依据](node-layering-plan.md)。

实现优先使用 JS、浏览器原语和无需 Node runtime 的纯 JS 库。Native 仅承担必须访问系统资源或浏览器无法满足目标契约的部分。纯 JS 算法、压缩和协议解析不会仅因潜在性能优势而下沉 Native。

## 2. 加载、配置与启动数据

Native 构建阶段将已检查入库的 runtime、vendor、模块 wrapper 和 classic 入口生成嵌入资源索引及压缩数据。Cargo 构建不需要运行 npm。Classic 入口注册已选模块；ESM 映射仅包含选择的模块及其资源。`Niva.require()` 从已注册的同步表读取模块；`Niva.import()` 优先复用已注册对象，未注册时使用当前资源映射动态导入。未知模块会抛错。

```json
"nodeCompat": {
  "modules": ["fs", "path", "os", "child_process"],
  "importmap": true
}
```

未指定 `nodeCompat` 或设为 `true` 时默认启用全部模块；`false` 关闭；对象配置省略 `modules` 时选全部模块。未知模块配置会被拒绝。`importmap: false` 仅关闭文档导航时的裸名映射注入，不关闭所选模块资源。静态 ESM 裸名仍须在模块解析前有 importmap，或由应用打包工具将其解析掉。

启动数据通过 `Niva.bootstrap` 提供。可信本地顶层页面可读取静态 OS 快照；真实 `process` 对象及其启动元数据仅注入 main 窗口（id 0）的可信顶层页面。子窗口不会获得 process 启动载荷。动态 OS/进程数据或操作由运行时桥接 API 查询，不应当作启动快照，也不应缓存为实时值。启动脚本只向顶层文档注入；跨域 IPC 不提供启动快照。

NodeCompat 与 `Niva.api` 是两个接口层：`Niva.api` 暴露 Niva 专有原生功能，NodeCompat 暴露 Node 风格模块。文件系统等底层能力可共用相同 Native handler，但不要把模块方法等同于 `Niva.api` 的公共命名空间。

## 3. 传输和权限边界

- 本地可信页面使用双向 WebSocket 承载普通异步调用、事件、二进制流及 socket 字节流。
- 本地可信页面的 `Niva.callSync()` 使用同步 XHR；Native 校验窗口 token、精确 origin、loopback Host、请求长度和同步 handler 白名单。它只适用于被允许的同步方法，不承载 UI、socket 或流操作。
- 跨域页面只能在 Native 授权后使用有限 unary JSON IPC；不支持同步调用、二进制、socket、双向流或完整 Node API。

进程等敏感能力需服从窗口和页面权限，不因 NodeCompat 默认启用而扩大授权。WS、同步 XHR 和 IPC 的当前鉴权细节见 [bridge 合约](bridge.md) 与 [权限设计](permission-design.md)。

## 4. 实现与验收状态

当前实现与验收以[实施记录](node-compat-implementation.md)和机器证据为准：22 个模块、179 个目标入口可加载；最新 release 打包 WebView 通过 45 项检查，macOS arm64 主程序 2,772,208 bytes。选定 Node 官方契约 58 个文件的适用检查通过，另有 2 处经用户授权的引擎差异跳过；Windows 仅 target check，尚未真机或 release 体积验收。

这些检查分别覆盖 JS 组件、Native 单测和有限 macOS WebView 路径，不能合并成 179 项 Node 兼容验收。HTTP/HTTPS 客户端与服务端真实网络路径仍有集成工作，特别是 server close 清理；尚未完成完整目标功能的 release 主程序体积测量，也未确认小于 3,300,000 bytes 的发布门禁。完整第三方库和官方 Node 测试子集仍须逐项验收。[实施状态与证据](node-compat-implementation.md)。

## 5. 浏览器与平台限制

- Web Crypto、Compression Streams、WebView fetch/network 和 CSP 行为依赖目标 WebView。浏览器能 import 某个 wrapper 不表示其功能在每个平台可用。
- 文件与进程二进制流需要本地 WebSocket；远端 IPC 页面没有对应能力。
- 不提供完整 CommonJS/npm runtime、Node 原生扩展、任意文件系统模块加载、未列入目标的 Node builtin 或完整 Node 事件循环。
- `process` 受 main 窗口限制；不要在其他窗口假设该全局存在。
- macOS 的有限 WebView 路径已有验证；Windows target check 不等于 Windows 真机支持。按平台记录完整功能及发布体积验收。

## 6. Node 库兼容验收

兼容验收采用[官方测试子集规则](node-upstream-conformance.md)：固定上游版本，以 API 签名、选项和平台为契约；纳入子集的原断言失败必须阻断。未覆盖 API 或只有项目自有 smoke 的条目不能标为通过官方 Node 测试。

第三方库验收还应检查完整依赖链、打包输出是否残留运行时 Node 内置导入或 `.node` addon，并在目标 WebView 运行代表性功能，验证回调/同步返回、Buffer、错误和资源清理。仅能打包、仅能 import、或者只通过合成协议样例，都不足以宣称完整 Node 模块兼容。
