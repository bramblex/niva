# Origin 权限与远端 IPC

本文记录当前远端 API 授权边界与 bridge 的来源校验。源码已实现 IPC `t:"api_call"`控制、Wry `evaluate_script`/`__niva_ipc_reply({sessionId,rid,sourceOrigin,response})`回包、流 ticket/attach 与 WS Wire v2；WK reply handler 和 WebView2 专用 IPC/reply 已移除。真实 macOS 双 lane smoke 及未覆盖项见[Bridge v2 验收记录](bridge-v2-validation.md)；完整控制/数据面契约见[Bridge 合约](bridge.md)。

候选验收结果与尚未覆盖的平台见[0.10.0-beta.1 候选记录](release-0.10.0-beta.1.md)。文末 2026-09-23/26 的 smoke 是对应日期的历史观察，不代表当前候选已完成端到端授权验收。

## 授权配置

每个窗口可在窗口配置的 `permissions` 中列出允许调用原生 unary JSON API 的页面 origin 和方法：

```jsonc
{
  "entry": "https://app.example.com/",
  "permissions": {
    "https://app.example.com": ["window.title", "clipboard.*"],
    "https://tools.example.com:8443": ["dialog.showMessage"]
  }
}
```

- key 必须是精确的 `http` 或 `https` origin，包含 scheme、host 和有效端口；路径、query、fragment、用户名密码均不接受。协议和端口参与匹配。没有子域通配或自由正则。
- grant 支持 `namespace.method` 和 `namespace.*`。没有匹配 grant 时默认拒绝。
- `permissions` 中的精确 origin grant 面向远端顶层页面的受限 unary JSON IPC；可信本地顶层文档和同源 iframe 使用经 token 认证的稳定 IPC。跨源 iframe 即使单独配置了 origin grant 也 fail-closed，不提供 API bridge：Wry `evaluate_script`只定向主文档，不能安全定向跨源子 frame，也不能让 parent 观察 child 的秘密。远端顶层页面仍不得开放流或二进制 Channel；数据通道切换不得改变 Rust 授权结果。macOS 同源 iframe 的 session reply 已由真实 WebView smoke 覆盖；跨源 iframe 仍按源码边界拒绝，单独的平台威胁矩阵保持开放。

## 当前 Native 校验边界

远端顶层页面没有本地 WS token，只能在精确 origin grant 下调用 unary JSON IPC。来源从 Wry IPC 请求对应的 URI/source origin 与 Native 当前窗口上下文校验，不取信请求体自报 origin/wid。Rust 校验窗口权限、session、call 与流 ticket；WS 和数据路径切换不能改变权限结果。Wry 不提供本协议可依赖的真实 Native child-frame ID：同源 child 经父页 relay，runtime 用 JS session 将 reply 送回对应 child；跨源 iframe 一律拒绝，不能借 parent relay 暴露秘密。

- Rust 根据窗口自己的 `WindowPermissions`，用 source URL 的精确 origin 检查完整方法名或 `namespace.*`。调用不能通过伪造 `wid` 切换窗口。
- **远端 IPC**拒绝 `window.open` 与 `webview.baseFileSystemUrl`，即使配置 grant 也不开放；这是远端授权限制，不是本地 API 的统一限制。远端顶层仅开放精确 grant 授权的 unary `api_call`，不开放流、Channel 或二进制帧。
- IPC 控制请求最多 256 KiB，编码后的响应最多 8 MiB。文本 handler 另有较小的接口限额，例如 HTTP 响应 body 最多 1 MiB、`execText`/`execFileText` 的 stdout 与 stderr 合计最多 64 KiB；其他文本接口限额见[Bridge 合约](bridge.md)。权限拒绝返回 bridge 错误码 `-4`。

### 已移除的历史平台 adapter

旧快照中 macOS 通过 `WKScriptMessage`/`nivaReply.postMessage()`回复，Windows 通过 WebView2 IPC handler 和 message event 回包。这些平台专用 reply 通道已从当前源码移除；不得据此描述当前实现。当前两平台目标统一使用 Wry IPC 与 Wry `evaluate_script`；Windows 尚无真机验证。

## 传输边界

当前 Native 小结果及流凭证经 Wry `evaluate_script`投递。创建流取得 ticket 后，数据 lane 只能二选一：IPC `channelAttach`/`channelAttached`，或 WS `hello`/`attach`/`attached`；不是先后连续两次 attach。

回复格式为 `__niva_ipc_reply({sessionId,rid,sourceOrigin,response})`；顶层和同源 iframe 共用此路径。流创建后返回 `channelOpened`/capability，然后在 IPC `channelAttach`/`channelAttached` 与 WS `hello`/`attach`/`attached` 中选一个 route 确认后才传数据。WS hello 必须携带 `{t:"hello",wid,v:2,sessionId}`；attach 使用 `{t:"attach",id,sessionId,capability}`，确认后 WS 只处理数据、ACK和cancel，不承载 API method/args。跨源 iframe 不可被安全定向，必须拒绝，不得借 parent 转发暴露 child 数据。Native 资源固定归属创建时的 IPC session，WS/IPC 只决定数据 transport，不改变 owner。

打包本地页面由 Wry 异步自定义协议从 `niva-<uuid>://app/` 加载：应用UUID去掉连字符并转小写，同一应用重启和多窗口使用相同origin，不同应用使用不同origin；Windows 当前设计映射为 `http://niva-<uuid>.app`，但未作真机验收。`niva://app/`只作为配置入口别名。该origin边界用于Web存储隔离，不是调用授权。可信本地页面的 API IPC 以请求来源 URL/top-origin、窗口凭据、session 与 call/ticket 校验；同源 child 通过父页 relay 和 JS session 路由 reply，不依赖真实 Native frame ID。WS 握手校验 loopback 服务 `Host`、对应窗口 token 和该窗口的精确页面 `Origin`，WS 不可用时流数据可经 IPC Channel。token 每窗单独随机生成并保存在内存，token 绑定的窗口必须仍然存在。hello 帧中的 `wid` 必须等于 token 绑定的窗口 ID。

显式开发启动可通过 `--debug-entry=http://localhost:<port>` 或 `--debug-entry=http://127.0.0.1:<port>` 指定本机 Vite 入口；也可由 Devtools 显式启动开关授权配置中的 `debug.entry`。`--resource` 只选择文件系统资源目录，`--config` 只选择配置文件；二者本身不授权调试入口。显式本机调试入口使用该窗口的精确 WS 来源，握手仍逐窗校验 token、精确 `Origin` 和 Niva 服务 `Host`；普通 HTTP 静态路由也仅在显式调试入口下开放。打包模式关闭普通 HTTP 静态路由，改从当前应用UUID对应的Wry协议读取包内静态文件；WS 和按窗口 token 鉴权的 `__niva_fs` 仍通过动态 loopback 服务。普通打包启动忽略嵌入配置的 `debug.entry`，不会因其指向本机端口而授予完整 bridge。

若页面直接 `fetch` `webview.baseFileSystemUrl` 返回的 `__niva_fs` URL，打包 origin 与 loopback HTTP 是跨源。协议 CSP 会允许当前 WS 与 HTTP endpoint；文件路由先校验窗口 token，再仅对与该 token 所属窗口 `trusted_ws_origin` 精确匹配的 `Origin` 返回 `Access-Control-Allow-Origin`。2026-09-23 固定origin的macOS临时app smoke已验证 `niva://app` 页带token fetch成功；无效token返回403，非匹配Origin即使带有效token也不返回CORS allow header。当前UUID origin及Windows仍未在真机验收。

远端页面及跨源 frame 的 bridge 能力由来源 URL/top-origin、每窗 grant 和 session 路由限定，不会因拿到主窗口的 `wid` 而继承本地 token。跨源 iframe 即使配置 grant 也 fail-closed，因为不能安全地将 Wry eval 定向至 child。HTTP `__niva_fs` 的认证与调试模式下普通 HTTP 静态路由的暴露属于另一条路径，不由本篇的 IPC grant 替代；包内普通资源由自定义协议按资源路径读取。

## 尚需实际验收

2026-10-06 的真实 macOS bridge-route smoke 已覆盖 custom-protocol WS 与严格 CSP IPC 两 lane、同源 iframe session relay、frame 后 top unary、Node CommonJS 和二进制流；远端精确 origin grant IPC 又通过25项真实 WebView 检查，含HTTP本地请求、HTTPS有效证书、限额、拒绝与lease expiry。各自报告和源码 fingerprint 见[Bridge v2 验收记录](bridge-v2-validation.md)。这些结果不构成完整远端权限矩阵；跨源 iframe 真机拒绝路径、Windows UUID origin/WebView2 和完整导航/关窗资源矩阵仍开放。2026-09-23 的 macOS 临时 app smoke 对固定 `niva://app`、`__niva_fs` token/CORS 的观察，以及 2026-09-26 的 localStorage/NodeCompat 临时包记录，仍是各自旧源码快照的历史证据；它们不替代当前协议验收。HTTP/资源服务鉴权与路径安全也应按各自文档和独立验收记录判断。
