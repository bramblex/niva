---
sidebar_position: 1
---

# Bridge 与 API 调用

`Niva` 是页面调用 Niva 原生 API 的入口。页面可用的传输由 Niva 原生端根据窗口入口、当前 frame 的来源和窗口授权决定；JavaScript 调用体不能自行声明 origin 或窗口身份。

## 选择传输

- **打包的本地 Niva 页面**由本地资源协议加载。macOS 页面 origin 为 `niva://app`，Windows WebView2 映射成 `http://niva.app`；Linux 源码分支也选择 `niva://app`，但尚无 Linux 构建或运行验收。页面连接的 WebSocket endpoint 仍是 `ws://127.0.0.1:<port>/__niva_ws`。原生端为每个窗口生成随机 token，并在握手时分别校验 token、页面 `Origin`、loopback 服务 `Host` 和 token 对应的窗口 ID。
- **显式开发启动**可为确切的 `http://localhost:<port>` 或 `http://127.0.0.1:<port>` 开放该窗口的 WebSocket 凭据，以连接本地开发服务器。普通打包模式不会因 `debug.entry` 自动开放开发服务。
- **同源 iframe**可按浏览器同源规则读取顶层页面的本地桥接凭据，因此也可以使用 WebSocket。每条 frame 连接有独立的调用 ID 空间；某个 frame 断开不会取消另一个 frame 的调用。
- **远端页面或跨源 iframe**没有本地 WebSocket token，走平台 IPC。IPC 每次根据 WebView 提供的 frame 来源 URL 和所属窗口检查授权。默认没有权限；配置见[远端页面权限](./permissions)。

远端 IPC 只接受 unary JSON API，不支持事件流、二进制数据或 `Niva.stream`。IPC 请求与响应各有 256 KiB 上限；页面请求等待回复的超时为 60 秒。`window.open` 和 `webview.baseFileSystemUrl` 在 IPC 路径上始终拒绝。

## 调用 API

```ts
// Niva.call 在本地 WebSocket 页面和已授权 IPC 页面都能发起 unary JSON 调用。
const id = await Niva.call("window.current", []);

// Niva.api 是命名空间代理，使用与具体 API 页面相同的 Promise 接口。
const title = await Niva.api.window.title();
```

原生 handler 的错误会使 Promise reject。`Niva.api` 的简单属性访问通过 `Niva.call` 转发；一些本地页面 API 会在初始化脚本中用流式实现封装：`fs.read/write/append`、`http.request/get/post`、`process.exec` 和 `resource.read`。这些包装需要本地 WebSocket；不能假定同名调用在远端 IPC 下可用。

## WebSocket wire v1

握手成功后，文本帧采用 JSON 对象。浏览器侧发起 `hello`、`call` 和 `cancel`；服务端返回 `result` 和 `event`。

```jsonl
{"t":"hello","wid":0,"v":1}
{"t":"call","id":1,"method":"window.current","args":[]}
{"t":"result","id":1,"code":0,"data":0}
```

`hello.wid` 必须匹配 token 绑定的原生窗口，`v` 必须与 `Niva.bridgeVersion` 相同。结果码中 `0` 表示成功、`-1` 表示 handler 错误、`-2` 表示超时、`-3` 表示队列繁忙；远端 IPC 的 `-4` 表示拒绝或不支持该方法，包括权限拒绝、未知 API 和流式 API。取消、断线和关窗会清理对应调用；取消后不保证收到终局结果。普通 `process.execStream` 子进程会在所属调用被取消时终止并回收，`detached: true` 除外。

二进制帧有 18 字节头：版本、flags、调用 ID 和序列号，后续字节是 payload。flags 的 `0x01` (`START`) / `0x02` (`END`) 标记一组的边界；`0x04` (`STDERR`) 用于标识 `execStream` 的 stderr 子流。`Niva.stream` 的 `onChunk` 按 WebSocket 收帧顺序接收每个非空 payload；`onBlob` 在某个子流结束时返回按序拼接的完整 Blob。`Niva.streamSend` 以相同的分片格式向服务端发送 stdin 或文件数据。

二进制帧格式、子流分组与示例见[流式调用](./stream)。stdio 宿主协议是独立的 NDJSON 格式，不复用这个 WebSocket wire；见[stdio 宿主桥](./stdio)。

## JS wrapper 与原生 handler {#js-wrappers}

`Niva.api` 多数方法直接调用同名 Rust handler。`fs.read/write/append`、`http.request/get/post`、`process.exec` 和 `resource.read` 则由 `initialize_script.js` 在本地 WebSocket 页面封装到对应的 stream handler；这些方法不是远端 IPC 可调用的 unary Rust handler。具体限制见[流式调用](./stream)。NodeCompat 的模块类型与实际范围见[NodeCompat](./node-compat)。

## 运行时状态

这些规则描述当前源码路径，不代表所有系统与 WebView 版本都已完成真机验收。尤其是 Windows frame IPC 和导航时序仍需要在目标 Windows WebView Runtime 上验证。远端 IPC 的 WebView 注册细节见[权限说明](./permissions)。
