# Niva Bridge 合约

> 当前状态（2026-10）：API 控制面/纯数据面分离与 Wire v2 已进入当前源码。真实 macOS WebView 的 custom-protocol WS 与 CSP 限制 IPC 两 lane smoke 均通过，包含 CommonJS globals、流 attach、双向数据、同源 iframe session 和 parent 后续 unary；完整证据及未覆盖平台见[Bridge v2 验收记录](bridge-v2-validation.md)。后文标明的历史快照保留当时行为，不定义当前协议。

## 当前协议

当前源码按 API 控制面与纯数据面分工；内部 ticket 和 IPC envelope 细节以 Native/runtime 实现为准。保留现有 Native API 语义与安全边界：

- **同步兼容 API**：保留同步 XHR，只供 Node 兼容中确需同步返回的方法使用，并按公开兼容方法发出首调 warning。普通异步操作不得改用同步 XHR。
- **异步 API 控制面**：API 创建消息使用 `t:"api_call"`；API 创建及后续控制统一走 IPC。method/args、API dispatch、权限检查和副作用提交都属于此控制面；不得经 WS dispatch API。
- **Native 到 JS 的小结果**：由统一 Wry `evaluate_script`调用 `__niva_ipc_reply({sessionId,rid,sourceOrigin,response})` 回传。创建流成功后，先经此回调返回 `channelOpened` 与 capability，再选定一个数据 lane：IPC 发 `channelAttach` 并收到 `channelAttached`，或 WS 发 ticket `attach` 并收到 `attached`。所选 lane 确认后才传输数据；同一流不会连续 attach 两次。
- **纯双向数据面**：只允许已鉴权的 WebSocket 优化路径，或稳定的 Wry IPC/`evaluate_script`路径。WS 使用 `{t:"hello",wid,v:2,sessionId}`；attach 为 `{t:"attach",id,sessionId,capability}`，Native 回复 `{t:"attached",id,sessionId}` 或 `{t:"attachError",id,sessionId,...}`。WS 此后只处理 attach、data、ack、cancel，不承载 method/args、API dispatch 或新的 API 副作用。文本数据消息为 `{t:"channelData",id,sessionId,seq,frame}`；二进制数据使用18-byte header `[version=2][flags][id u64 BE][seq u64 BE]`，其中 `id` 是 channel id，`seq` 是该 channel 的序号。确认消息为 `{t:"ack",id,sessionId,seq}`，取消为 `{t:"cancel",id,sessionId}`；ACK `seq` 表示该 channel 已确认的序号，与旧 Wire v1 语义不可混用。IPC 数据路径遵循同一 channel、序号、ACK 与取消语义。
- **资源 owner**：Native 资源固定归属创建它的 IPC session；数据 transport 独立选择，可用 WS 或稳定 IPC，不因 attach、断线或数据通道选择而改变 owner。关闭 session 时清理其资源与流。
- **页面 session 生命周期**：`session_close` 是经 IPC 发送的单向生命周期控制消息，不属于 API。持久化页面恢复时的匹配条件、旧请求取消、资源处置和新 session 行为见[BFCache 与页面生命周期](#bfcache-与页面生命周期)。
- **流与切换**：保留严格 seq、ACK、背压、有界队列、cancel、session lease 和资源归属。WS 建立失败时可用稳定 IPC 数据路径；已开始流断线后的切换不得承诺无损，也不得重放可能已提交的 API 副作用。若实现切换，必须能区分仅数据传输恢复与 API 重试，并对不确定状态明确失败。
- **授权与 frame 边界**：Rust 侧使用来源 URL/origin、token、窗口、session、call 与流凭证鉴权；切换数据通道不能扩权。Wry 不提供可供本协议依赖的真实 Native child-frame ID；同源 iframe 通过父页 relay 和 JS session 映射路由 reply，Native 仍校验来源 URL/origin、token、session 与 ticket。顶层本地文档与同源 iframe 可通过统一 Wry IPC/eval 路径使用 bridge。远端顶层页面仍受精确 origin grant 与 unary 权限边界约束，不因本地传输可用而获得流或二进制权限。跨源 iframe 一律 fail-closed，即使它自身配置了 origin grant 也拒绝 API bridge：Wry `evaluate_script`只定向主文档，无法安全定向跨源子 frame，且不得让 parent 观察 child 的秘密；不为此增加平台专用适配。
- **平台抽象**：统一使用 Wry IPC 与 `evaluate_script` 的稳定机制；现有 WKWebView reply handler 与 WebView2 专用 IPC/reply 通道已移除，不保留也不新增平台专用 reply 协议。

流创建后，IPC 与 WS attach 是二选一的 route，不是连续步骤。`api_call`参数、`rid`分配和内部 ticket 的其余字段由实现定义，但不能改变控制面/数据面边界：

```text
JS -> Native  Wry IPC {t:"api_call", ...method/args...}
Native -> JS  Wry evaluate_script: __niva_ipc_reply({sessionId,rid,sourceOrigin,response})
                 response: channelOpened + capability (流创建成功时)
二选一 IPC 数据 lane：
JS -> Native  Wry IPC channelAttach（带上面返回的 channel id/capability）
Native -> JS  Wry evaluate_script channelAttached
或 WS 数据 lane：
JS -> Native WS   {t:"hello",wid,v:2,sessionId}
JS -> Native WS   {t:"attach",id,sessionId,capability}
Native -> JS     {t:"attached",id,sessionId} | {t:"attachError",id,sessionId,...}
JS <-> Native    {t:"channelData",id,sessionId,seq,frame} | 18-byte binary frame, version=2
JS <-> Native    {t:"ack",id,sessionId,seq}
JS -> Native     {t:"cancel",id,sessionId}
```

最终 debug binary 的真实 macOS WebView smoke 已验证两种数据 lane、API IPC、CommonJS globals、ticket/attach、双向流数据、同源 iframe 独立 session 与 iframe 后 top unary；Node HTTP compatibility、BFCache 和各 lane 的证据范围见[Bridge v2 验收记录](bridge-v2-validation.md)。源码行为不自动证明未运行的远端 grant 矩阵、跨源 iframe、Windows/macOS其他版本或完整资源生命周期。WS 建立失败的 IPC lane 有真实 macOS smoke 覆盖；活跃流断线无损迁移不作保证，副作用不重放。旧报告、macOS 编译和 Windows target check 均不能替代目标平台真机证据。

`docs/release-0.10.0-beta.1.md`中的旧报告只证明各自记录的源码/runtime 快照；当前源码和本轮验收数据见[Bridge v2 验收记录](bridge-v2-validation.md)。

所有页面API实现在`Niva`对象上。Node模块接口（例如`Niva.fs`）和Niva窗口接口（例如`Niva.window`）共用一套 API/Stream handler；传输选择属于低层 bridge，不暴露给调用方。`Niva.bridge`是低层入口，`Niva.stream`表示Node stream模块。下文明确标为历史实现快照的内容只记录旧路由与当时源码观察；权限限制按其明示的远端/本地边界理解，不得泛化为所有本地 `api_call` 的限制。当前 macOS 两 lane WebView smoke 已通过，Windows 真机及其他未列目标仍开放。

## BFCache 与页面生命周期

浏览器的 Back/Forward Cache（BFCache，前进/后退缓存）可能把离开页面时的整个 document
暂存在内存中。命中 BFCache 返回时，浏览器恢复原来的 DOM、JavaScript realm、模块对象和事件
监听器，并继续执行被冻结的页面；这不同于 reload，后者会创建新 document 并重新运行 bootstrap。

旧 runtime 在 `pagehide` 时将 bridge session 永久标记为失效；BFCache 恢复原 JavaScript
对象后仍持有已失效 session，Native API 调用因此失败。现在生命周期按以下顺序处理：

1. `pagehide` 发生时，runtime 取消旧 session 的在途调用并发送不等待回复的 IPC `session_close`，
   携带原 session ID、来源上下文及适用的 token。Native 校验完整 session key（窗口、来源、
   session 及 frame/generation），关闭并 tombstone 旧 session；这不是可重用的普通 API 请求。
2. 只有 `pagehide` 与后续 `pageshow` 都带 `persisted=true`，恢复页面来源仍匹配原来源、frame 仍受
   支持且能够解析到 IPC host 时，runtime 才在原 realm 中生成新的密码学随机 session ID。Native
   不依赖 Wry 未提供的真实子 frame ID；frame 支持边界仍按当前来源与 relay 合约执行。
3. 新 session 建立后，旧调用保持取消状态，旧 session 的迟到回复按 session ID/epoch 丢弃；API
   不自动重放。普通 Native 句柄属于旧 session，恢复后应重新创建。session lease 已过期时，
   后续 `pageshow` 不能复活它。

BFCache 保留页面 realm，因此模块缓存、页面监听器和 canonical `process.stdin`、`stdout`、
`stderr` 对象身份可以延续。旧 stdin Native 流会取消；页面恢复后 runtime 为其建立新输入流。
用于 CSP/模块执行的 `runtimeConfig.nonce` 保持不变；它不是新的 session ID（session ID 自身
由一个密码学随机 nonce 生成）。

stdout/stderr 只有在旧写入以 `ERR_NIVA_SESSION_EXPIRED` 结束、所有受跟踪的旧写入回调均已结算
后，才可能恢复原 stream 对象。恢复要求 writer 已进入 `destroyed/closed/closeEmitted` 终态，
没有活动写入或 write callback、队列为空，且用户没有显式调用 `destroy()` 或 `end()`（也未进入
`ending/ended/finished` 状态）。此时才清理 pinned `readable-stream@4.7.0` 因缓冲错误回调留下的
`pendingcb` 计数，并恢复同一 canonical stream。普通写入错误或任何条件不满足时都不恢复；
旧 Promise 仍按原错误结束，不会重放写入或 API。

当前实现与验证范围见[Bridge v2 验收记录](bridge-v2-validation.md)。以上行为有当前 runtime 测试与 macOS WebView 证据；Windows 真机与完整跨平台页面生命周期验收仍开放。

## 两类API与桥接路径

对外API分为异步接口与同步兼容接口。它们不是三种可供调用方选择的传输：JS/Rust高层API不暴露也不要求调用方指定桥接方式。

| API类别 | 稳定路径 | 可选性能路径 | 用途 |
|---|---|---|---|
| 异步API | IPC `t:"api_call"`创建和控制；Native→JS小结果/流凭证使用Wry `evaluate_script`回调 | 已建立并认证的WebSocket仅承载纯双向数据 | method/args与API dispatch不走WS；数据WS不可用时可使用Wry IPC/`evaluate_script`数据路径 |
| 同步兼容API | 同步XHR | 无 | 仅供Node兼容所需的同步方法使用；每个方法首次调用时必须发出warning。不得把普通异步操作放到同步XHR上 |

IPC `api_call`是异步API的唯一创建/控制面；Wry IPC/`evaluate_script`是纯数据的稳定路径，WS只作纯数据优化。创建流并取得凭证后才开始传数据。WS在建连阶段不可用时可选稳定数据路径；对已开始流不承诺断线无损迁移。API副作用不得因换数据通道而重放；无法确认状态时明确失败。`GET /__niva_fs/...`是独立文件资源接口，不属于异步bridge。

`process.chdir`等改变进程状态的操作使用异步API；这里有意采用`Promise<void>`，不同于Node原生同步签名。同步XHR仅覆盖语义本身确需同步返回的Node兼容入口，且每个公开兼容方法首次调用发出warning。

Node `http.request/get`的 HTTP/HTTPS 流式I/O由Rust `ureq`与平台TLS connector执行，JS适配Node请求/响应对象。若终态 TCP socket 的 read-timeout setter 返回 `InvalidInput`，Native 以非阻塞 `peek` 仅探测数据或 EOF；探测到可读后恢复阻塞模式并继续读取，`WouldBlock` 保留原错误，不消费数据、不重试请求或执行无界阻塞读取。

```ts
Niva.bridge.call(method, args) // Promise
Niva.bridge.callSync(method, args) // 同步值或抛错
Niva.bridge.stream(method, args, {onEvent, onChunk, onBlob}) // {id,promise,cancel}
Niva.bridge.streamSend(id, data, end?)
```

JS显式cancel必须结束对应Promise并释放本地状态。Native取消不等于回滚已发生的副作用；已进入不可中断系统调用的操作必须如实说明完成边界。

## 当前来源与鉴权（历史观察另见下文）

- 普通本地资源使用由应用UUID派生的Wry协议origin：macOS/Linux为`niva-<uuid>://app`，Windows WebView2为`http://niva-<uuid>.app`（UUID去掉连字符并转小写）。同一应用在重启和多个窗口间保持origin稳定，不同应用使用不同origin；`niva://app`仅是配置入口别名。origin隔离存储，不代替窗口token或API授权。`--resource`/`--config`本身不授予调试权限。
- 显式`--debug-entry`，或显式开发开关启用的本机debug.entry，可为精确localhost/127.0.0.1开发origin授予窗口凭据；普通启动忽略调试入口配置。
- Native为窗口生成随机内存token。WS与同步XHR检查token、窗口绑定、精确Origin及本地Host；`hello.wid`不能换成别的窗口。凭据不得进入资源文件、日志或外部输入配置。
- 本地 IPC Channel 在 Rust 侧校验 token、source URL/top-origin、窗口、session、call 与随机 channel capability；同源 child 的回复由父页按 JS session 路由。Wry 不提供真实 Native frame ID；channel capability绑定对应流的窗口/session/call上下文，不能仅因WS失败放行。
- 普通远端页面及跨源frame没有该凭据。IPC从平台消息取得真实source URL，通过窗口`permissions`的精确origin和方法授权；payload自报origin/wid不作为依据。未授权默认拒绝。
- opaque来源、`__niva_fs`文件服务页面不得因通道切换获得完整Native能力。**远端 IPC**禁止`window.open`和`webview.baseFileSystemUrl`等扩权入口，也禁止把任意Unary handler直接当作允许的IPC接口；这是远端授权边界，不是对可信本地 `api_call` 的统一限制。
- 同源frame可依浏览器同源规则访问凭据，但每个realm/连接的任务与资源仍独立。销毁一个iframe不清理仍存活的兄弟frame。

## 历史快照：旧 Wire v1 与 Channel（不定义当前 API 控制路由）

以下帧只记录旧 wire snapshot，不表示允许当前 WS 承载 API `call`。当前协议移除 WS `call`；API 创建和控制统一由 Wry IPC `api_call`完成，WS hello 与 binary header 使用 v2，ACK `seq` 为 channel 序号。

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

API快照：高层`http.requestText`/`https.requestText`是Rust ureq提供的一次性Native请求。Node `http.request/get`使用Rust `ureq`和平台TLS connector执行HTTP/HTTPS流式I/O，JS只适配`ClientRequest`/`IncomingMessage`对象；旧路由为页面与Rust之间的数据使用创建该请求时选定的WS或IPC Channel。`createServer`则由JS处理HTTP协议并复用Native TCP/TLS。远端顶层授权页面只可使用精确origin grant允许的一次性 unary API，不开放Channel、流或二进制；跨源iframe即使有grant也按新目标fail-closed。

## 已记录的 IPC 文本接口与限额（实现快照）

| 高层能力 | Native RPC | 说明 |
|---|---|---|
| UTF-8文件读取 | `fs.readText` | Node readFile仅显式UTF-8分支可映射到此文本接口；默认Buffer返回不支持 |
| 文本写入/追加 | `fs.writeText`、`fs.appendText` | string可用；不能用Base64伪装二进制 |
| 文件/目录操作 | `fs.node`精确op白名单 | stat/lstat/readdir/access/realpath/mkdir/rename/copyFile/rm/unlink/cp；仍检查参数与响应上限 |
| HTTP/HTTPS文本 | `http.requestText` | 返回statusCode/statusMessage/headers/body；允许内网、外网、localhost，保留正常TLS验证 |
| 命令/程序执行 | `process.execText`、`process.execFileText` | `Niva.child_process`上的一次性扩展，返回stdout/stderr/status/signal；不能冒充ChildProcess流对象 |

IPC控制请求JSON最多256KiB，响应JSON最多8MiB（容纳文本转义开销）。IPC channel二进制使用不超过16 KiB payload的wire frame，受有界队列与ACK/背压控制。HTTP body最多1MiB；exec stdout+stderr总计最多64KiB；网络/exec默认10秒、最多30秒，调用选项只可收紧。读取阶段实施限额，不先无限缓冲。文本接口不是二进制编码隧道。

历史实现路由：Node `http.request/get`通过统一的`http.requestStream` handler，由Rust ureq负责HTTP/HTTPS协议、TLS校验和流式I/O；JS仅适配ClientRequest/IncomingMessage对象。页面与Rust之间的数据曾优先经WS优化桥，WS不可用时使用IPC Channel。当前控制/数据面关系以本页[当前协议](#当前协议)为准。`createServer`继续由JS处理HTTP协议并复用Native TCP/TLS。HTTP是否可访问某地址与IPC是否有调用权限是不同检查；已授权应用拥有与普通Node程序相同的内外网访问方向。

## 历史实现快照：IPC会话与失联

调用envelope包含`sessionId`与本地场景的token；sessionId每realm生成，只用于生命周期，不能替代Native鉴权。只有存在在途IPC时才发送一次性JSON心跳：

```text
C -> S {t:"heartbeat",sessionId,token?}
S -> C {t:"heartbeatAck",sessionId,expiresInMs:3000}
S -> C {t:"heartbeatError",sessionId,code:"ERR_NIVA_SESSION_EXPIRED",message}
```

当前策略为1秒心跳、3秒租约。过期后失败该session的在途任务、关闭资源并终止其exec进程；迟到心跳不复活旧任务。正常零active状态不等于已过期。页面JS阻塞或后台计时器节流也可能被视为失联，必须明确报错、不重放操作。

WS按连接断开清理。原生窗口销毁/可观测导航及时清理；macOS无法依赖公开API稳定识别所有iframe销毁，因此单iframe IPC失联最坏按租约加调度延迟确认，不能宣称unload/GC即时通知。Native资源需要RAII或独立监督者兜底，不能只把cleanup写在可能被取消的await之后。JS资源对象显式close/dispose为主，FinalizationRegistry只作可用时的兜底。

## Node接口与stdio（桥接路由为历史实现快照）

基础`Niva` API始终存在；`injectCommonJs`和`injectEsm`独立、默认关闭，分别控制Node全局/模块环境和浏览器import map。Node内置CJS/ESM接口引用同一Niva实现；用户覆盖自行负责。CommonJS文件通过同步`module.resolve/load`由Native解析读取；浏览器执行并管理缓存/循环。不支持.node，也不自建ESM引擎。

`Niva.os.info`是启动静态数据，动态系统信息按相应接口查询。真实`Niva.process`及其stdio仅供可信主窗口。旧路由中child_process流式stdio优先用WS，WS不可用时走IPC Channel（IPC二进制帧用Base64）；当前资源owner固定为创建时IPC session，数据transport独立。shell/Python宿主直接使用普通管道，协议由应用自己定义。Niva不再提供`api.host`、`--stdio`或内置NDJSON，框架日志写独立文件。stdin EOF只结束输入流，不自动退出UI。见[stdio宿主说明](stdio-host-design.md)。
