# Niva Bridge 合约（给兼容层作者）

本仓库提供传输和原生 API；可选的 NodeCompat 浏览器包位于 `packages/node-compat`。Wire 版本由 `Niva.bridgeVersion` 与 Rust 侧 `WIRE_VERSION` 同步。

## 传输选择

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

仅 WebSocket 支持二进制流：服务端向客户端发送 `fs.readStream`、`http.requestStream`、`resource.readStream` 的 body，以及 `execStream` 的 stdout/stderr；客户端向服务端发送 `fs.writeStream` 文件内容和 `execStream` stdin。JS 的 `onChunk` 在每帧到达时收到一份字节副本；`onBlob` 仍按连接内 `id` 和 STDERR flag 分组、按 `seq` 排序，在 END 后收到完整 Blob。远端 IPC 不支持 stream 或 binary。

## JS 运行时

```js
Niva.call(method, args) -> Promise
Niva.stream(method, args, {onEvent, onChunk, onBlob}) -> {id, promise, cancel}
Niva.streamSend(id, ArrayBuffer|Uint8Array|string, end?) -> bool
Niva.api.<ns>.<method>(...) // 直通代理及流式 API 覆盖
Niva.addEventListener/removeEventListener
Niva.bridgeVersion // number
```

`Niva.call` 根据页面可用传输自动走本地 WS 或远端 IPC。远端 IPC 只允许普通 unary JSON 方法；单次请求和响应最大 256 KiB，流式方法不可用。`Niva.stream` 与 `Niva.streamSend` 在 IPC-only 页面不可用。

`Niva.api` 默认透传 `ns.method`。以下 unary API 在本地 WS 模式由流式实现覆盖，外部兼容层可继续使用同名 Promise 接口：`fs.read/write/append`、`http.get/post/request`、`process.exec`、`resource.read`。原生流式 API 包括 `fs.readStream/writeStream`、`http.requestStream`、`process.execStream`、`resource.readStream`。

## 远端 IPC 授权

- 每个窗口在窗口配置的 `permissions` 中声明精确页面 origin 对可调用方法的授权；key 包含 scheme、host 和 port，值为方法名或 `namespace.*`。未授权默认拒绝。远端授权只适用于 unary JSON IPC。
- Native 从平台 IPC 消息取得调用 frame 的 source URL，并将窗口 ID 与原生窗口绑定；授权不信任调用 payload 自报的 `wid` 或 origin。macOS 使用消息 frame 的 URL；Windows 在回包前再次核对当前页面 origin 与请求 origin。
- `window.open` 和 `webview.baseFileSystemUrl` 即使列入 grant 也会被 IPC 层拒绝。未注册方法、stream handler、授权失败分别以 bridge 结果拒绝；授权错误码为 `-4`。
- 配置与源码路径已实现。2026-09-23 在 macOS WebView 手工验证了本地主 frame 与同源 iframe 的独立 WS 调用、移除子 frame 后主 frame 的流式调用继续完成、跨源顶层页及 iframe 的授权/拒绝 IPC、`connect-src 'none'` 下的 IPC、`__niva_fs` 有无凭据的 200/403，以及将文件 HTML 嵌入 iframe 后其脚本被 sandbox 拦截。Windows 仅通过目标编译检查，仍需真机验证 frame 消息与导航时序。

## 兼容层建议

```text
native bridge → 独立 Node 形封装 → 用户/AI 代码
readStream/writeStream       readFile/writeFile + 过渡同步 existsSync/statSync/readFileSync
execStream                   child_process.exec / spawn 仿真
os.info/dirs                  os.platform/arch/tmpdir/homedir
(无)                          require('path') 纯 JS 实现
```

Bridge 本身采用异步语义。NodeCompat 当前没有同步文件或进程 API；历史文档对同步 API 的窄版与全量版要求冲突，产品范围仍待决定。`require` 只读取已注册白名单模块，未知模块抛错；实际模块清单与限制见 `node-compat-design.md`。

`--stdio` 宿主桥使用独立的 NDJSON stdin/stdout 协议；它不改变上述 WebSocket wire v1。宿主消息只归主窗口（id 0），详见 `stdio-host-design.md`。
