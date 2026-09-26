# Niva Bridge 合约

> 2026-09-26 更新：普通异步调用固定走IPC/`evaluate_script`；流式重载操作按操作创建时的可用状态选WS优化桥或稳定IPC Channel。WS不可用时同一操作经IPC Channel承载，不再把它称为“IPC fallback”。当前路由已接线，资源生命周期固定沿用创建时的bridge；全量平台验收仍在进行中。历史NodeCompat报告不能代替本次验收。

所有页面API实现在`Niva`对象上。Node模块接口（例如`Niva.fs`）和Niva窗口接口（例如`Niva.window`）共用同一套 API/Stream handler；传输选择属于低层 bridge，不暴露给 JS/Rust 高层 API。`Niva.bridge`是低层入口，`Niva.stream`表示Node stream模块。当前代码与本节描述的是协议契约；macOS、Windows真实 WebView端到端验收仍须单独记录。

## 两类API与桥接路径

对外API分为异步接口与同步兼容接口。它们不是三种可供调用方选择的传输：JS/Rust高层API不暴露也不要求调用方指定桥接方式。

| API类别 | 稳定路径 | 可选性能路径 | 用途 |
|---|---|---|---|
| 异步API | 平台IPC调用Rust；Native→JS使用 `evaluate_script` | 已建立并认证的WebSocket bridge | 普通异步调用走IPC。大量文件/网络数据、二进制数据及流式 `child_process` stdio 等优先走WS；WS无法建立或不可用时，同一操作经IPC/`evaluate_script`完成 |
| 同步兼容API | 同步XHR | 无 | 仅供Node兼容所需的同步方法使用；每个方法首次调用时必须发出warning。不得把普通异步操作放到同步XHR上 |

IPC/`evaluate_script`是必须可用的稳定异步桥；WS是可选优化路径，不构成单独的公开API类别。这里的“可用”指通道可达且操作不因WS不可用而丢失；IPC性能低于WS是可接受的。新流在创建时选择路由；该stream创建的文件句柄、socket和子进程控制操作沿用同一bridge。WS在创建时不可用就使用IPC Channel；已走WS的资源遇到连接断开则明确失败，不迁移或重放，避免重复副作用。`GET /__niva_fs/...`是独立的文件资源接口，不属于异步bridge。

`process.chdir`等改变进程状态的操作使用异步API；这里有意采用`Promise<void>`，不同于Node原生同步签名。同步XHR仅覆盖语义本身确需同步返回的Node兼容入口，且每个公开兼容方法首次调用发出warning。

```ts
Niva.bridge.call(method, args) // Promise
Niva.bridge.callSync(method, args) // 同步值或抛错
Niva.bridge.stream(method, args, {onEvent, onChunk, onBlob}) // {id,promise,cancel}
Niva.bridge.streamSend(id, data, end?)
```

JS显式cancel必须结束对应Promise并释放本地状态。Native取消不等于回滚已发生的副作用；已进入不可中断系统调用的操作必须如实说明完成边界。

## 来源与鉴权

- 普通本地资源使用由应用UUID派生的Wry协议origin：macOS/Linux为`niva-<uuid>://app`，Windows WebView2为`http://niva-<uuid>.app`（UUID去掉连字符并转小写）。同一应用在重启和多个窗口间保持origin稳定，不同应用使用不同origin；`niva://app`仅是配置入口别名。origin隔离存储，不代替窗口token或API授权。`--resource`/`--config`本身不授予调试权限。
- 显式`--debug-entry`，或显式开发开关启用的本机debug.entry，可为精确localhost/127.0.0.1开发origin授予窗口凭据；普通启动忽略调试入口配置。
- Native为窗口生成随机内存token。WS与同步XHR检查token、窗口绑定、精确Origin及本地Host；`hello.wid`不能换成别的窗口。凭据不得进入资源文件、日志或外部输入配置。
- 本地 IPC Channel 在 Rust 侧校验 token、真实 frame/source URL、窗口、session、call 与随机 channel capability；channel capability绑定对应本地流的窗口/session/call/frame上下文，不能仅因WS失败放行。
- 普通远端页面及跨源frame没有该凭据。IPC从平台消息取得真实source URL，通过窗口`permissions`的精确origin和方法授权；payload自报origin/wid不作为依据。未授权默认拒绝。
- opaque来源、`__niva_fs`文件服务页面不得因通道切换获得完整Native能力。IPC禁止`window.open`和`webview.baseFileSystemUrl`等扩权入口，也禁止把任意Unary handler直接当作允许的IPC接口。
- 同源frame可依浏览器同源规则访问凭据，但每个realm/连接的任务与资源仍独立。销毁一个iframe不清理仍存活的兄弟frame。

## Wire v1 与 Channel

```text
C -> S {t:"hello",wid,v:1}
C -> S {t:"call",id,method,args}
C -> S {t:"cancel",id}
S -> C {t:"result",id,code,message,data}
S -> C {t:"event",id?,seq,name,data}
```

`id`按连接隔离，活动ID不可重复覆盖。每个新流在创建时选择已建立的WS或稳定IPC Channel；由该流创建的文件句柄、socket、子进程等资源在生命周期内沿用同一bridge，不能在后续控制调用时换owner。WS连接丢失时该连接拥有的资源明确失败，不跨bridge重放或迁移。重复ID的协议错误作为无call id的`bridge.protocolError`事件报告，不能冒充原请求的终局。终局只有一次；超时/取消后的迟到结果不得完成新的调用。

常用code：0成功、-1执行错误、-2超时、-3容量/流入站拒绝、-4权限或通道拒绝。`seq`用于流事件/分片顺序，result不占流序号。入站队列满不能静默丢块，应终止对应调用。

两条异步桥共用18-byte wire frame：`[version=1][flags][id u64 BE][seq u64 BE]`，随后payload；flags为START=1、END=2、STDERR=4。二进制payload最大16 KiB。WS上传输原始二进制帧；IPC Channel在传输边界对完整wire frame做Base64，JS/Rust API handler仍只处理原始字节。Native→JS IPC帧/事件经 `evaluate_script` 投递，JS→Native channelSend及ACK经原生IPC传递。两种传输均要求严格序号、有界队列、ACK/背压、取消与session租约。普通异步调用使用稳定IPC；重载/流式操作在创建时优先选已建立的WS，否则使用同语义IPC Channel。该IPC二进制编码会产生Base64 CPU及临时字符串开销，不代表零拷贝或性能更快。

高层`http.requestText`/`https.requestText`是Rust ureq提供的一次性Native请求。Node `http.request/get`使用Rust `ureq`和平台TLS connector执行HTTP/HTTPS流式I/O，JS只适配`ClientRequest`/`IncomingMessage`对象；页面与Rust之间的数据使用创建该请求时选定的WS或IPC Channel。`createServer`则由JS处理HTTP协议并复用Native TCP/TLS。远端授权页面只可使用精确origin grant允许的一次性 unary IPC，不开放Channel、流或二进制。

## IPC文本接口与限额

| 高层能力 | Native RPC | 说明 |
|---|---|---|
| UTF-8文件读取 | `fs.readText` | Node readFile仅显式UTF-8分支可映射到此文本接口；默认Buffer返回不支持 |
| 文本写入/追加 | `fs.writeText`、`fs.appendText` | string可用；不能用Base64伪装二进制 |
| 文件/目录操作 | `fs.node`精确op白名单 | stat/lstat/readdir/access/realpath/mkdir/rename/copyFile/rm/unlink/cp；仍检查参数与响应上限 |
| HTTP/HTTPS文本 | `http.requestText` | 返回statusCode/statusMessage/headers/body；允许内网、外网、localhost，保留正常TLS验证 |
| 命令/程序执行 | `process.execText`、`process.execFileText` | `Niva.child_process`上的一次性扩展，返回stdout/stderr/status/signal；不能冒充ChildProcess流对象 |

IPC控制请求JSON最多256KiB，响应JSON最多8MiB（容纳文本转义开销）。IPC channel二进制使用不超过16 KiB payload的wire frame，受有界队列与ACK/背压控制。HTTP body最多1MiB；exec stdout+stderr总计最多64KiB；网络/exec默认10秒、最多30秒，调用选项只可收紧。读取阶段实施限额，不先无限缓冲。文本接口不是二进制编码隧道。

Node `http.request/get`通过统一的`http.requestStream` handler，由Rust ureq负责HTTP/HTTPS协议、TLS校验和流式I/O；JS仅适配ClientRequest/IncomingMessage对象。页面与Rust之间的数据优先经WS优化桥，WS不可用时使用IPC Channel，上传消费与响应读取都有逐块确认，取消会关闭Native请求。`createServer`继续由JS处理HTTP协议并复用Native TCP/TLS。HTTP是否可访问某地址与IPC是否有调用权限是不同检查；已授权应用拥有与普通Node程序相同的内外网访问方向。

## IPC会话与失联

调用envelope包含`sessionId`与本地场景的token；sessionId每realm生成，只用于生命周期，不能替代Native鉴权。只有存在在途IPC时才发送一次性JSON心跳：

```text
C -> S {t:"heartbeat",sessionId,token?}
S -> C {t:"heartbeatAck",sessionId,expiresInMs:3000}
S -> C {t:"heartbeatError",sessionId,code:"ERR_NIVA_SESSION_EXPIRED",message}
```

当前策略为1秒心跳、3秒租约。过期后失败该session的在途任务、关闭资源并终止其exec进程；迟到心跳不复活旧任务。正常零active状态不等于已过期。页面JS阻塞或后台计时器节流也可能被视为失联，必须明确报错、不重放操作。

WS按连接断开清理。原生窗口销毁/可观测导航及时清理；macOS无法依赖公开API稳定识别所有iframe销毁，因此单iframe IPC失联最坏按租约加调度延迟确认，不能宣称unload/GC即时通知。Native资源需要RAII或独立监督者兜底，不能只把cleanup写在可能被取消的await之后。JS资源对象显式close/dispose为主，FinalizationRegistry只作可用时的兜底。

## Node接口与stdio

基础`Niva` API始终存在；`injectCommonJs`和`injectEsm`独立、默认关闭，分别控制Node全局/模块环境和浏览器import map。Node内置CJS/ESM接口引用同一Niva实现；用户覆盖自行负责。CommonJS文件通过同步`module.resolve/load`由Native解析读取；浏览器执行并管理缓存/循环。不支持.node，也不自建ESM引擎。

`Niva.os.info`是启动静态数据，动态系统信息按相应接口查询。真实`Niva.process`及其stdio仅供可信主窗口；child_process的流式stdio优先使用WS优化桥，WS不可用时走IPC Channel（IPC二进制帧用Base64），并保留有界队列、ACK/背压和取消语义。shell/Python宿主直接使用普通管道，协议由应用自己定义。Niva不再提供`api.host`、`--stdio`或内置NDJSON，框架日志写独立文件。stdin EOF只结束输入流，不自动退出UI。见[stdio宿主说明](stdio-host-design.md)。
