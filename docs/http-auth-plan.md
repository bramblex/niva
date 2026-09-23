# HTTP 资源鉴权：现状与后续边界

> 状态：原 session-cookie 方案已被当前按窗口 scoped file-token 实现取代；打包普通
> 静态资源现由 `niva://app/` 自定义协议提供，不再经过普通 HTTP 静态路由。loopback
> 服务继续承载 WS、`__niva_fs` 和显式调试模式下的静态/NodeCompat HTTP 路由。不要将
> 下文旧设计视为待实现要求或当前协议；源码见 `http_server`、`custom_protocol`、
> `window_manager/builder.rs` 与 `docs/bridge.md`。

## 1. 当前实现

- Native 为每个窗口生成独立随机 token，并将 `webview.baseFileSystemUrl` 指向
  `http://127.0.0.1:<port>/__niva_fs/<window-token>/`。
- 服务端在读取文件前确认 token 对应现存窗口；无效或缺失凭据返回 403。
- 该路径有意允许持凭据窗口读取任意本机普通文件，与只读项目资源根目录的
  `FileSystemResource` 边界不同；请求路径按 URL 百分号编码解码，单次响应上限 32 MiB。
- 成功的文件响应带 `Content-Security-Policy: sandbox` 与
  `X-Content-Type-Options: nosniff`。
- 文件响应只在请求 `Origin` 与 token 所属窗口的 `trusted_ws_origin` 精确相同时返回
  `Access-Control-Allow-Origin`。macOS smoke 验证 `niva://app` 页面通过
  `webview.baseFileSystemUrl()` 跨源 fetch 读取成功；带有效 token 但 Origin 不匹配时
  返回 HTTP 200 但没有 ACAO，缺失/无效 token 返回 403。
- `FileSystemResource` 对开发资源目录执行解码、路径组件与 canonical 根目录约束。
- 打包模式的普通 HTML、JS、CSS 等资源由 `niva://app/` 异步协议提供。该处理器只读
  包内资源、限制并发/超时/响应大小，拒绝所有 `__niva_*` 内部路径；启用 NodeCompat
  时仅放行通过模块 allowlist 的 `__niva_compat/*` 文件。
- 显式调试启动仍允许 loopback HTTP 静态资源与 NodeCompat 资源。NodeCompat HTTP 路由
  使用 `Access-Control-Allow-Origin: *`，且没有窗口 token 鉴权；普通静态 HTTP 路由也
  不在 `__niva_fs` 鉴权范围。不要把 `__niva_fs` 已校验表述成所有 HTTP 路由已认证。
- 打包 HTML 文档导航响应中的 CSP meta 会合并当前 WS 与 `__niva_fs` HTTP endpoint 到 `connect-src`。
  当前协议响应不能读取或修改宿主环境另外添加的 CSP response header；这类 header 仍
  可能阻止 WS 或文件 fetch。

## 2. 被取代的 session-cookie 草案

以下是历史设计，不是现行实现：

- 复用 WS 启动 token，经 entry query 首验后设置 `niva_session` HttpOnly cookie。
- 其后要求所有静态 HTTP 请求携带 cookie，仅对 `/__niva_compat/*` 豁免。
- 以普通静态资源、404、`__niva_fs` 均统一拦截为目标。

现行方案采用每窗口文件 URL 凭据保护 `__niva_fs`，并不实现上述全 HTTP cookie
会话流程。不要追加 entry query、cookie 或旧方案中的豁免规则，除非后续明确决定
并重新设计各 HTTP 路由的威胁模型。

## 3. 仍需确认的事项

- 确认打包 `niva://` 资源与显式调试 HTTP 静态资源的预期暴露范围；后者只在调试参数
  打开时服务，不能误作生产鉴权。WS 的 `Origin`/`Host` 与 API 权限 grant 不能替代
  HTTP 路由授权。
- 确认本机其他进程及浏览器对调试模式 loopback 静态/NodeCompat 内容的威胁边界。若
  未来增加通用 HTTP 鉴权，设计需与当前窗口 token、固定打包 origin 和开发 entry
  协调，不能直接照搬旧 cookie 草案。
- Windows WebView2 的实际 `http://niva.app` origin、iframe/CORS 请求及宿主 CSP header
  行为仍需 Windows 真机验证；macOS smoke 和 Windows target check 不能替代该验收。
- 发布门禁仍要求 Windows 真机、CI 和 roadmap 中其他 P0 完成；本文件更新不构成
  v1.0 验收。
