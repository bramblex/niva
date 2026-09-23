# Origin 权限与远端 IPC

本文记录当前远端 API 授权契约。源码已实现配置解析、原生 IPC 调用校验和平台请求/回复路径；这不等同于浏览器或目标操作系统运行时验收。

## 授权配置

每个窗口可在窗口配置的 `permissions` 中列出允许调用原生 unary JSON API 的页面 origin 和方法：

```jsonc
{
  "entry": "https://app.example.com/",
  "permissions": {
    "https://app.example.com": ["window.title", "clipboard.*"],
    "https://tools.example.com:8443": ["dialog.alert"]
  }
}
```

- key 必须是精确的 `http` 或 `https` origin，包含 scheme、host 和有效端口；路径、query、fragment、用户名密码均不接受。协议和端口参与匹配。没有子域通配或自由正则。
- grant 支持 `namespace.method` 和 `namespace.*`。没有匹配 grant 时默认拒绝。
- `permissions` 授权远端 IPC；同源本地 Niva 页面使用经 token 认证的 WS bridge。

## 原生校验路径

远端页面没有本地 WS token，只能调用 unary JSON IPC。调用来源由 WebView 提供，而不是从 JS 请求体读取：

- macOS 从 `WKScriptMessage` 的发送 frame request 取得 source URL，并绑定创建该 handler 的原生窗口 ID。`nivaReply.postMessage()` 返回 Promise 结果。
- Windows IPC handler 同样把原生窗口 ID 和 request URI 交给 Rust 授权；响应投递前在主线程确认当前 WebView origin 仍等于请求 origin。前端通过 `window.ipc.postMessage()` 发起调用，通过 WebView message 事件收取响应。
- Rust 根据窗口自己的 `WindowPermissions`，用 source URL 的精确 origin 检查完整方法名或 `namespace.*`。调用不能通过伪造 `wid` 切换窗口。
- 远端 IPC 拒绝 `window.open` 与 `webview.baseFileSystemUrl`，即使配置 grant 也不开放。stream handler 和二进制数据没有 IPC 表示，不能经远端 IPC 授权或调用。
- IPC 只接受 call 消息；请求体和编码后的响应最大 256 KiB。权限拒绝返回 bridge 错误码 `-4`。

## 传输边界

打包本地页面由 Wry 异步自定义协议从 `niva://app/` 加载：macOS 精确页面 origin 是 `niva://app`，Windows WebView2 实际页面 origin 是 `http://niva.app`。本地 WS 仍连接动态 `ws://127.0.0.1:<port>`，握手同时校验 loopback 服务 `Host`、对应窗口 token 和该窗口的平台精确页面 `Origin`；token 对应的窗口必须仍然存在。token 每窗单独随机生成并保存在内存。hello 帧中的 `wid` 必须等于 token 绑定的窗口 ID。每个 frame 的 WS 连接拥有独立调用 ID 空间；跨源 frame 不能读取主 frame 的本地凭据，因此只能使用自己的远端 IPC 来源与权限。

显式开发启动可把本机 Vite 入口的精确 `http://localhost:<port>` 或 `http://127.0.0.1:<port>` 作为该窗口的 WS 来源；握手仍逐窗校验 token、精确 `Origin` 和 Niva 服务 `Host`。带 `--debug-resource` 的文件系统资源调试也保留 loopback 静态服务。打包模式关闭普通 HTTP 静态路由，改从 `niva://app/` 读取包内静态文件；WS 和按窗口 token 鉴权的 `__niva_fs` 仍通过动态 loopback 服务。普通打包启动忽略嵌入配置的 `debug.entry`，不会因其指向本机端口而授予完整 bridge。

若页面直接 `fetch` `webview.baseFileSystemUrl` 返回的 `__niva_fs` URL，打包 origin 与 loopback HTTP 是跨源。协议 CSP 会允许当前 WS 与 HTTP endpoint；文件路由先校验窗口 token，再仅对与该 token 所属窗口 `trusted_ws_origin` 精确匹配的 `Origin` 返回 `Access-Control-Allow-Origin`。macOS 临时 app smoke 已验证 `niva://app` 页带 token fetch 成功；无效 token 返回 403，非匹配 Origin 即使带有效 token 也不返回 CORS allow header。Windows 尚未真机验收。

远端页面及跨源 frame 的 bridge 能力由原生来源 URL 和每窗 grant 限定，不会因拿到主窗口的 `wid` 而继承本地 WS 身份。HTTP `__niva_fs` 的认证与调试模式下普通 HTTP 静态路由的暴露属于另一条路径，不由本篇的 IPC grant 替代；包内普通资源由自定义协议按资源路径读取。

## 尚需实际验收

代码路径和类型定义描述了预期契约，但 Windows 尚无真机验收。2026-09-23 的 macOS 临时 app smoke 实际观察到 `niva://app`，主页面与同源 iframe 分别通过 WS 调用 `window.current` 和 `fs.createDir`；主页面通过 `webview.baseFileSystemUrl()` 对 `__niva_fs` 做跨源 fetch 成功，非法 token 和非匹配 Origin 的行为也已探测。另一临时包仅选 `path/fs/assert/stream`，验证了部分 NodeCompat import/require 路径和未选模块 404。smoke 没有覆盖跨源 frame、窗口间 token 隔离、二进制流、重启存储、完整 NodeCompat 语义或导航/关窗时序。macOS IPC 授权/拒绝的既有测试证据见 [`bridge.md`](bridge.md)；Windows target check 不能替代 WebView2 真机验证。HTTP/资源服务的鉴权与路径安全也应按各自文档和独立验收记录判断。
