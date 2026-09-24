# NodeCompat HTTP 与 HTTPS

HTTP 与 HTTPS 是 NodeCompat 模块，不属于 `Niva.api`。应用可使用 Node 风格模块接口：

```js
const http = Niva.require("http");
const https = Niva.require("https");

const request = https.get("https://example.com/", (response) => {
  console.log(response.statusCode, response.headers);
});
request.on("error", console.error);
```

ESM 项目也可从 `node:http`、`node:https` 或 NodeCompat 包的 `http`、`https` 子路径导入。具体可用方法与参数以当前模块实现和 [NodeCompat 模块说明](./node-compat)为准。

## HTTP 模块

当前模块提供 `request()`、`get()`、`createServer()`、`ClientRequest`、`IncomingMessage`、`ServerResponse`、`METHODS` 和 `STATUS_CODES`。HTTPS 提供相同的客户端与服务端接口，并在 Native TLS 层验证证书链与主机名；不支持关闭 TLS 校验。

HTTP 报文解析、请求/响应对象和服务器逻辑运行在 JS，TCP/TLS 字节流由 Native socket 层提供。应用 `http.createServer()` 创建的服务器与 Niva 内部用于加载页面、资源服务及 bridge WebSocket 的 Native HTTP 服务无关。

旧版 `Niva.api.http.request/get/post` 和 `Niva.stream("http.requestStream", ...)` 已移除，不应继续使用。对简单 JSON API 可直接使用浏览器 `fetch`；需要 Node 风格 socket 流、HTTP server 或 Node 客户端请求对象时使用 NodeCompat 模块。

## 当前限制

- macOS 系统 TLS 后端在结束写端时关闭整个 TLS 会话；TLS 不接受 `allowHalfOpen: true`，TCP 支持半关闭。
- NodeCompat HTTP/HTTPS 是 Node API 子集，不承诺所有 Agent、连接复用、upgrade、100-continue、超时和错误语义。
- 已在 macOS 实际验证 HTTP/HTTPS 收发、关闭与 TLS CA/主机名验证；这不代表全部 Node HTTP 行为通过官方契约测试。
- 真实 socket 依赖本地可信页面的 WebSocket bridge；远端 IPC 不提供 socket 或流。
- 当前 macOS 核心 WebView 和 Native socket/TLS 有有限验证，Windows 只有 target 编译检查，尚无真机结果。
- NodeCompat 默认随 Niva 主程序内嵌并启用，可用 `nodeCompat: false` 关闭或在对象配置中筛选模块。

应用开发者使用时请按目标平台验证请求/响应生命周期与 TLS 行为。仓库的 `docs/node-compat-implementation.md` 记录测试证据和未完成门禁。
