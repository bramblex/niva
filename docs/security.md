# Niva 安全评估（2026-09-23，以当前源码为准）

## 0. 信任模型

**能执行原生 bridge 调用的 JS 拥有该窗口被授予的能力。** `exec`、文件访问等 API
本身影响很大，核心边界是哪些 frame 能获得 bridge 身份和哪些 API grant。

本文描述源码实现，不代表所有平台已通过真机验收或 v1.0 发布门禁已关闭。

## 1. 当前实现

### 本地 bridge 与远端 IPC

- 打包本地页面由 Wry 异步自定义协议从 `niva://app/` 加载；macOS 页面 origin 为
  `niva://app`，Windows WebView2 页面 origin 为 `http://niva.app`。非 Windows 源码分支
  也选择 `niva://app`，但 Linux 尚无构建或运行验收。窗口以本地包启动时才建立可信
  origin；初始化脚本只在顶层文档的当前 origin 精确匹配该值、且路径不在 `__niva_fs`
  下时注入按窗口生成且仅存内存的 WS token。同源页面导航仍满足 origin 条件。WS 握手
  继续校验精确页面 `Origin`、loopback 服务 `Host`、路径和 token，并将 hello 窗口 ID
  绑定到该 token。
- 远端页面和 frame 通过原生 IPC 来源 URL 按窗口 grant 授权。grant 使用精确
  HTTP(S) origin 与 API 方法规则；未授权默认拒绝。远端 IPC 不提供流式调用，且
  `window.open`、`webview.baseFileSystemUrl` 不允许经 grant 开放。
- 源码具备 macOS 与 Windows frame 级 IPC 路径；Windows 真机关键流程仍待验证。
- 显式开发启动中的本机 Vite 页仍使用精确 loopback HTTP origin。打包模式的普通
  HTTP 静态和 NodeCompat 路由关闭；`/__niva_ws` 与按窗口 token 验证的
  `/__niva_fs/<window-token>/...` 仍由动态 loopback HTTP 服务提供。
- `niva://` 协议处理器只读取本应用资源；拒绝 `__niva_*` 内部路径，NodeCompat
  仅在启用且 asset 通过模块 allowlist 时可读。异步队列为 4 个工作线程、32 个等待
  请求；超时 15 秒，响应上限 32 MiB。该限制不构成性能测试结果。

### HTTP 文件访问与资源路径

- `GET /__niva_fs/<window-token>/...` 会校验 token 是否仍绑定一个现存窗口；拒绝
  缺失或无效凭据。持凭据窗口可读任意本机普通文件，响应上限 32 MiB；成功响应带 `Content-Security-Policy: sandbox` 和
  `X-Content-Type-Options: nosniff`。
- 持有效窗口 token 时，文件路由只对与该 token 所属窗口 `trusted_ws_origin` 精确匹配
  的 `Origin` 返回 `Access-Control-Allow-Origin`。非匹配 Origin 即使持有效 URL token
  仍可得到 HTTP 资源状态，但响应不带 CORS allow header，浏览器脚本不能读取响应；
  缺失/无效 token 返回 403。macOS smoke 实测了精确 `niva://app` 下的 fetch、非匹配
  Origin 无 ACAO 和无效 token 403；Windows 仍待真机验证。
- 该授权只覆盖 `__niva_fs` 文件读取路径。打包模式不再通过普通 HTTP 路由提供项目
  静态资源；显式调试启动仍启用普通静态与 NodeCompat HTTP 路由，其中 NodeCompat
  资源带 `Access-Control-Allow-Origin: *`，这些调试路由没有窗口 token 鉴权。不要据此
  声称所有 HTTP 路由都已鉴权。
- 新项目模板提供可编辑的严格 CSP meta（`default-src 'self'`，外部 `index.js` 可运行），
  默认不启用 NodeCompat。打包协议页与显式 `debug-resource` 的 HTML 文档导航只改写页面
  自带的 enforcing CSP meta：保留原 source list，并把当前 loopback WS/HTTP origin 加到
  `connect-src`；HTTP origin 也会加到已有或继承 `default-src` 的 `img-src`、`media-src`、
  `font-src` 和 `style-src`，供 token-scoped `webview.baseFileSystemUrl()` 使用。
- 页面启用 NodeCompat 且有 CSP meta 时，运行时会把 Niva 注入块移动到 head 内最后一条
  CSP meta 之后，并仅给该注入块中的 importmap/classic adapter 脚本添加每次导航新生成的
  nonce；不会添加 `unsafe-inline`，也不会给 Niva 注入块之外的作者 importmap 加 nonce。
  注入标记或预期标签格式不完整、或作者可执行脚本夹在注入块与 CSP meta 之间时，文档
  返回 500。没有 CSP meta 的已有项目不会被强制新增策略；项目自己的 meta CSP 会按上述
  规则补连接与资源源。
- 外部 Vite 页面由项目自行设置 CSP。若 WebView 宿主、代理或其他层另加 CSP response
  header，当前代码无法读取或修改该 header；它与页面 meta policy 叠加后仍可能阻止请求或
  NodeCompat 脚本。
- `FileSystemResource` 对路径做百分号解码、拒绝绝对路径和非普通路径组件，并在
  解析后校验文件仍位于 canonical 资源根目录内；符号链接逃逸也会被拒绝。

## 2. 剩余风险与验收边界

- bridge 权限隔离不等于远端内容可信。仅授予确有需要的 API；不要在有原生权限的
  frame 执行未经信任的网络脚本。`http.requestStream` 现在只接受 HTTP(S) 且拒绝
  userinfo、代理、非公开网络解析地址；每次重定向重新校验，唯一 loopback 例外是
  当前 Niva 服务的精确 `127.0.0.1:<port>`。该 IP 范围策略无法识别企业网络把
  公网编号地址私有路由的情况，依赖升级时需复核 ureq resolver 接口。
- 显式调试模式下普通 HTTP 静态资源与 NodeCompat 路由不在 `__niva_fs` token 校验范围
  内；仅有 macOS smoke 证明打包模式 `/index.html` 返回 404，其他平台和调试路由仍需
  按预期暴露范围核查。不要把 WS token 或 IPC grant 当成通用 HTTP 鉴权。
- macOS 临时 app smoke（2026-09-23）观察到 `niva://app`，主页面和同源 iframe 均通过
  WS 原生调用，普通 loopback 静态请求返回 404，匿名 `__niva_fs` 返回 403。另一个仅选
  `path/fs/assert/stream` 的临时包验证了部分 NodeCompat ESM、Promise 与 allowlist
  路径；这不代表完整模块语义已验收。上述结果均不是 Windows、二进制流、存储迁移、
  异步负载或性能验收证据。Wry 异步处理只避免在协议回调同步读取资源。
- 仓库已有 `.github/workflows/ci.yml`，但尚无远端成功运行记录；Windows 目标编译
  检查不能代替 Windows 真机验证。在 Windows 真机、CI 和其他 v1.0 P0 完成前，
  不得标记 v1.0 门禁通过。

## 3. 发布门禁跟踪

v1.0 的未完成项目和权威状态见 [`roadmap.md`](roadmap.md)。至少仍须完成：

- Windows 真机关键流程验证。
- CI 执行 Rust 检查与测试、TypeScript 检查及 Vite 构建。
- `window-tray-menu-plan.md` 所列三个 P0。
- 其他 roadmap v1.0 门禁项的证据验收。

源码中已有的 token、来源校验或路径约束不自动关闭发布门禁；关闭项须有对应提交、
检查结果或真机记录。
