# Niva Bridge 合约（给兼容层作者）

本仓库提供传输和 Niva 原生 API；NodeCompat 是随 Niva 主程序嵌入的 Node 风格浏览器模块层。`nodeCompat: false` 可关闭，模块对象可筛选入口。Wire 版本由 `Niva.bridgeVersion` 与 Rust 侧 `WIRE_VERSION` 同步。

## 传输选择

既定架构包含三条原生调用路径；NodeCompat 的实现难度按复用这些路径评估：

| 路径 | 用途 | 支持边界 |
|---|---|---|
| 双向 WS | 本地可信页面的基础调用、异步结果、事件、二进制和 socket 字节流 | 使用窗口绑定凭据与现有消息/流协议；Node net/dgram/tls 适配器在 JS 上承接这些流。 |
| 同步 XHR | 本地可信页面中必须保留同步 Node 返回形态的选定方法 | `POST /__niva_sync` 需要窗口 token 和精确 `Origin`；Rust 再检查同步方法门禁与 blocking handler。仅限本地可信页面，不进入 IPC。 |
| 平台 IPC | 经窗口授权的跨域简单 unary JSON 调用 | 有限 allowlist；不承担同步接口、socket、双向流、二进制或完整 Node 兼容。单次请求/响应上限 256 KiB。 |

纯 JS/WebView 原语能够完成的逻辑不需要原生通信。“跨域不能用 WS/XHR”指本项目不向外源开放本地 socket 或同步服务的边界。IPC 不是在任意 WS 失败后绕过权限的通用回退；仍按原生识别的 origin 和窗口授权判断。初始化快照和当前 IPC allowlist 的源码事实与逐项 NodeCompat 目标分开记录；完整分层见 [NodeCompat 实现状态](node-compat-implementation.md)，逐项工作见 [Node API 覆盖盘点](node-api-coverage.md)。

固定启动信息经 `Niva.bootstrap` 注入。可信本地顶层页面拿到只读 OS 快照；真实 process 元数据和 `process` 全局只给 main 窗口（id 0）注入/注册，子窗口不获 process 启动载荷，跨域 IPC 不获任何启动快照。动态查询和操作仍由对应通道处理；进程/OS 查询范围见 [实现状态](node-compat-implementation.md)。

Native `net`、`dgram`、`tls` 提供 TCP/UDP/TLS 系统原语；NodeCompat 在 JS 层实现 HTTP/HTTPS 客户端和应用服务器、DNS 协议。Niva 内部启动 HTTP/WS/资源服务另行保留，不能与 Node 应用的 `http.createServer()` 混淆。[分层与迁移依据](node-layering-plan.md)。

- **打包的本地 Niva 页面**由 Wry 异步自定义协议从固定 `niva://app/` 加载。macOS 页面 origin 为 `niva://app`，Windows WebView2 页面 origin 为 `http://niva.app`；Linux 分支在源码中也选择 `niva://app`，但没有 Linux 构建或运行验收。普通 loopback HTTP 静态路由在打包模式关闭。Native 为每个窗口单独生成随机 token，只保存在内存中；启动入口为本地包的窗口，仅在顶层页面 origin 与该窗口可信 origin 相同、且路径不在 `__niva_fs` 下时注入 WS URL、窗口 ID 和 token，因此同源本地页面导航仍满足注入条件。WS 握手同时校验路径、token、精确页面 `Origin` 和本地服务 `Host`；缺失或不匹配返回 403。`hello.wid` 必须与 token 绑定的原生窗口 ID 相符。
- **显式开发启动的本机入口**（`--debug-entry`，或配合 `--debug-config` 的 `debug.entry`）可为精确的 `http://localhost:<port>` / `http://127.0.0.1:<port>` origin 单独开放同一窗口的 WS token，保留 Vite 的流式 API 与 HMR。普通打包启动忽略 `niva.json` 中的 `debug.entry`，不会开放该例外。
- 同源 frame 可按浏览器同源规则读取主 frame 的本地 WS 凭据；跨源 frame 不能读取这些凭据，走远端 IPC 路径。每个连接（包括每个 frame 的连接）拥有独立 call ID 命名空间；断开某个 frame 只清理该连接的调用。事件可广播给该窗口的所有连接。
- **远端页面**没有本地 WS 凭据，只可使用平台 IPC 发送 unary JSON 调用。macOS 的 `window.webkit.messageHandlers.nivaReply.postMessage()` 返回 Promise；Windows 使用 `window.ipc.postMessage()` 发送请求，并通过 WebView message 事件接收回复。
- `window.open` 已从初始化脚本移除。远端 IPC 也拒绝 `window.open` 和 `webview.baseFileSystemUrl`，避免远端页面创建窗口或取得本地文件服务入口。

## WebSocket 文本帧（JSON 对象）

```text
C -> S  {t:"hello", wid, v}                    # 每个新连接首帧；wid 必须匹配 token 绑定窗口，v 必须匹配 bridgeVersion
C -> S  {t:"call", id, method, args}            # id: u64；命名空间按连接隔离
C -> S  {t:"cancel", id}                        # 取消该连接上的 stateful 调用
S -> C  {t:"result", id, code, message, data}   # 终局；0=成功，-1=handler 错，-2=超时，-3=忙，-4=权限拒绝
S -> C  {t:"event", id?, seq, name, data}        # 流事件；无 id 表示窗口广播
```

`seq` 按调用递增（终局 result 不占序号）。超时返回 `-2`；取消、连接断开和关窗会清理对应活动调用，不保证返回终局帧。`process.execStream` 的普通子进程会在取消时终止并回收；其他已进入 `unblock` 的阻塞操作仍可能继续执行到结束。

## 二进制帧（18 字节头 + payload）

```text
[ver=1][flags][id u64 BE][seq u64 BE][payload...]
flags: 0x01 START / 0x02 END / 0x04 STDERR（exec 子流）
```

仅 WebSocket 支持二进制流：服务端向客户端发送 `fs.readStream`、`resource.readStream`、子进程 stdout/stderr，以及 `socket.*` TCP/TLS/UDP 收流；客户端向服务端发送 `fs.writeStream`、子进程 stdin、socket 写入和 UDP datagram。Node HTTP/HTTPS 的报文解析在 JS 层，字节读写走 socket stream。`Niva.stream` 的 `onChunk` 按帧收到字节，`onBlob` 在 END 后收到分组后的完整数据。远端 IPC 不支持 stream 或 binary。

## JS 运行时

```js
Niva.call(method, args) -> Promise
Niva.callSync(method, args) -> any
Niva.stream(method, args, {onEvent, onChunk, onBlob}) -> {id, promise, cancel}
Niva.streamSend(id, ArrayBuffer|Uint8Array|string, end?) -> bool
Niva.bootstrap // 本地启动快照；process 仅 main 窗口
Niva.api.<ns>.<method>(...) // Niva 专有原生能力代理
Niva.addEventListener/removeEventListener
Niva.bridgeVersion // number
```

`Niva.call` 根据页面可用传输自动走本地 WS 或获准的远端 IPC。`Niva.stream` 与 `Niva.streamSend` 仅本地 WS 可用。`Niva.callSync` 只在本地可信页面发送同步 XHR；服务端验证窗口 token、精确 origin、loopback Host、请求长度和同步 handler，不为跨域创建同步 IPC。它只适用于同步契约，不能调用 UI、socket 或流式 API。

`Niva.api` 只代表 Niva 专有原生 API。Node 网络模块使用 `Niva.require("http")` / `Niva.require("https")` 或对应 NodeCompat import；HTTP/HTTPS 不属于 `Niva.api` 的公共声明。旧版 `Niva.api.http` 与 `http.requestStream` 已退出当前 API/HTTP 实现路径，不能作为新调用入口。文件、进程、资源的既有流仍由 `Niva.stream` 支持。

同步 XHR 的 Rust 端只接受有确定 `Content-Length`、且不超过 16 MiB 的请求体。它按当前同步方法门禁与已注册 blocking handler 执行；非同步 handler、UI 方法及 connection-owned socket 操作会被拒绝。远端 IPC 仍只有经 permission grant 授权的 unary JSON 方法，大小限制为每请求/响应 256 KiB。

## 远端 IPC 授权

- 每个窗口在窗口配置的 `permissions` 中声明精确页面 origin 对可调用方法的授权；key 包含 scheme、host 和 port，值为方法名或 `namespace.*`。未授权默认拒绝。远端授权只适用于 unary JSON IPC。
- Native 从平台 IPC 消息取得调用 frame 的 source URL，并将窗口 ID 与原生窗口绑定；授权不信任调用 payload 自报的 `wid` 或 origin。macOS 使用消息 frame 的 URL；Windows 在回包前再次核对当前页面 origin 与请求 origin。
- `window.open` 和 `webview.baseFileSystemUrl` 即使列入 grant 也会被 IPC 层拒绝。未注册方法、stream handler、授权失败分别以 bridge 结果拒绝；授权错误码为 `-4`。
- 当前源码的授权与路由以 Rust 检查为准。2026-09-24 的验证包括 NodeCompat 包测试 69/69、macOS WebView 核心 15 项在显式 debug HTTP 和打包 `niva://` 下通过（包含同步 XHR、文件句柄/流、watch、同步子进程取消回收及 crypto）；Native OS/bootstrap 和 socket/TLS 测试也有对应单测。旧 Native HTTP 客户端和应用 API 已移除，NodeCompat HTTP/HTTPS 客户端及应用服务器已有实现；完整真实网络集成与 server close 清理仍待验收。Windows 通过目标编译检查但没有真机验证；全部目标集成后的 release 主程序体积尚未测量。

## 兼容层建议

```text
native bridge → 独立 Node 形封装 → 用户/AI 代码
readStream/writeStream       readFile/writeFile + 过渡同步 existsSync/statSync/readFileSync
execStream                   child_process.exec / spawn 仿真
os.info/dirs                  os.platform/arch/tmpdir/homedir
(无)                          require('path') 纯 JS 实现
```

`require` 只读取已注册的 22 个 NodeCompat 模块及其子路径，未知模块抛错。默认启用内嵌 NodeCompat，可用 `nodeCompat: false` 关闭，也可用 `nodeCompat.modules` 选子集；详情见 [实现状态](node-compat-implementation.md)。

`--stdio` 宿主桥使用独立的 NDJSON stdin/stdout 协议；它不改变上述 WebSocket wire v1。宿主消息只归主窗口（id 0），详见 `stdio-host-design.md`。

## 页面模块的延迟初始化

`Niva.registerModule(id, implementation)` 注册已初始化值；`Niva.registerModuleFactory(id, factory)` 在第一次 `Niva.require` 或 `Niva.import` 时调用 factory 并缓存返回值，失败不缓存。后续注册可替换该名称。它是页面内 JS 注册表，不产生 Native 请求，也不更改 wire 版本。

NodeCompat zlib 的工厂在首次加载时读取 Buffer 的容量上限，CommonJS 的 `zlib`/`node:zlib` 与 ESM wrapper 共享同一实例。注册模块不会提前触发工厂；ESM import 按通常的模块求值时序初始化。
