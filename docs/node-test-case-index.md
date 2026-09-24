# NodeCompat 测试用例索引

2026-09-24，按当前源码、冻结清单和已保存执行报告核对。

## 统计口径

- 扩展清单固定为 Node v22.14.0、commit `5d2feb257bcee090e57900eb51720171a6aa92f3`，共 **58 个未修改的官方测试文件**：48 项纯 JS 适配器契约，10 项 Native/网络模块契约。
- 原有 30 项纯 JS 文件覆盖 path、buffer、events、querystring、string_decoder；新增 18 项扩展 util、assert、url、crypto、zlib、stream、timers。另选 10 项 fs、process、os、child_process、http、https、net、tls、dns、dgram。见下方逐文件映射。
- **适用范围结果：58 pass、0 fail、0 unsupported；另单列 2 处已授权的环境检查点跳过，门禁通过**。统一结果文件：[Node 官方测试报告](node-compat-upstream-results.json)。不得把旧 30 项报告或 Native 初始试跑当成 58 项最终结果。
- 58 是文件数，不是断言数。一个文件可能测试多个 API、遍历大量输入，fuzz 还会按时间循环；没有把 assert 调用数冒充用例数。
- 纯 JS 上游测试由 Node v22.14.0 主机执行，模块路由到 Niva NodeCompat 适配器。Native 选择项仍由 Node 主机加载原始测试和断言，但适配器的 `Niva.callSync`、`Niva.call`、`Niva.stream`/`streamSend` 经独立真实 Niva WebView relay 到 Native；测试文件不是在浏览器内执行。产品 fs/网络/进程操作不由 host fs/net mock 代替。
- 下表列主测试目标；某 API 作为造数据/辅助工具出现，不表示它的完整契约已验收。Windows 条件分支的运行范围取决于实际测试宿主。Native 表中 `https` 与 `dns` 选择为受限案例：HTTPS 只测参数校验、没有握手；DNS 只测参数校验、没有 DNS 报文查询。TLS 才是本组的真实验证握手用例。
- 包内测试 109 项、release WebView 45 项属于另外的测试层；179/179 是入口查找检查，不是 179 项行为都已跑官方测试。

来源：[冻结 manifest](../packages/node-compat/upstream/manifest.json)、[统一逐文件报告](node-compat-upstream-results.json)、[验收规则](node-upstream-conformance.md)。

## 官方文件 → API → 场景

### path：11 个文件

| 官方文件 | 主测试 API | 关键场景 |
|---|---|---|
| [test-path-basename.js](../packages/node-compat/upstream/node-v22.14.0/test/parallel/test-path-basename.js) | basename | 后缀剥离、空串、根目录、分隔符、POSIX/Windows 路径 |
| [test-path-dirname.js](../packages/node-compat/upstream/node-v22.14.0/test/parallel/test-path-dirname.js) | dirname | 根目录、尾部分隔符、相对路径、Windows 盘符/UNC |
| [test-path-extname.js](../packages/node-compat/upstream/node-v22.14.0/test/parallel/test-path-extname.js) | extname | 点文件、多重扩展名、尾部点号、目录分隔符 |
| [test-path-glob.js](../packages/node-compat/upstream/node-v22.14.0/test/parallel/test-path-glob.js) | matchesGlob | 通配符、字符集合/范围/否定、**、两种路径分隔符、非字符串参数 |
| [test-path-isabsolute.js](../packages/node-compat/upstream/node-v22.14.0/test/parallel/test-path-isabsolute.js) | isAbsolute | POSIX 根路径、Windows 盘符和 UNC、相对路径 |
| [test-path-join.js](../packages/node-compat/upstream/node-v22.14.0/test/parallel/test-path-join.js) | join | 多段拼接、空段、点段、连续分隔符、跨平台边界 |
| [test-path-normalize.js](../packages/node-compat/upstream/node-v22.14.0/test/parallel/test-path-normalize.js) | normalize | ./.. 折叠、根目录、重复/尾部分隔符、Windows 特殊根 |
| [test-path-parse-format.js](../packages/node-compat/upstream/node-v22.14.0/test/parallel/test-path-parse-format.js) | parse / format | root/dir/base/name/ext 字段、组合与往返、非法对象和错误消息 |
| [test-path-relative.js](../packages/node-compat/upstream/node-v22.14.0/test/parallel/test-path-relative.js) | relative | 相同路径、上下级、不同根/盘符、UNC |
| [test-path-resolve.js](../packages/node-compat/upstream/node-v22.14.0/test/parallel/test-path-resolve.js) | resolve | 多段绝对化、实时 cwd、cwd 返回空串；Windows 分支含固定子进程 fixture |
| [test-path.js](../packages/node-compat/upstream/node-v22.14.0/test/parallel/test-path.js) | posix / win32、sep / delimiter、公共路径方法 | 默认平台别名、分隔符常量、错误类型和 ERR_INVALID_ARG_TYPE |

### buffer：9 个文件

| 官方文件 | 主测试 API | 关键场景 |
|---|---|---|
| [test-buffer-alloc.js](../packages/node-compat/upstream/node-v22.14.0/test/parallel/test-buffer-alloc.js) | alloc / allocUnsafe / allocUnsafeSlow / SlowBuffer；并触及 from、slice、整数读写等 | 分配、编码/解码、整数边界、内存池属性、参数/offset 错误；包含多个 API 的综合文件 |
| [test-buffer-bytelength.js](../packages/node-compat/upstream/node-v22.14.0/test/parallel/test-buffer-bytelength.js) | byteLength / isEncoding | 字符串多编码字节数、Buffer/ArrayBuffer/TypedArray、编码别名和无效输入 |
| [test-buffer-compare.js](../packages/node-compat/upstream/node-v22.14.0/test/parallel/test-buffer-compare.js) | Buffer.compare / buf.compare | 大小排序、相等、空 Buffer、Uint8Array、非法参数及错误消息 |
| [test-buffer-concat.js](../packages/node-compat/upstream/node-v22.14.0/test/parallel/test-buffer-concat.js) | concat | 列表拼接、显式长度、截断/补零、空列表、错误元素类型 |
| [test-buffer-fill.js](../packages/node-compat/upstream/node-v22.14.0/test/parallel/test-buffer-fill.js) | fill | 数字/字符串/Buffer 填充、编码、start/end 范围、对象转换、伪造 length、底层范围检查 |
| [test-buffer-from.js](../packages/node-compat/upstream/node-v22.14.0/test/parallel/test-buffer-from.js) | from / copyBytesFrom | 字符串、ArrayBuffer、视图、数组/类数组、拷贝与共享语义、offset/length 和无效输入 |
| [test-buffer-indexof.js](../packages/node-compat/upstream/node-v22.14.0/test/parallel/test-buffer-indexof.js) | indexOf / lastIndexOf | 数字/字符串/Buffer 查找、空搜索串、正负偏移、多编码、参数及 receiver 错误 |
| [test-buffer-tojson.js](../packages/node-compat/upstream/node-v22.14.0/test/parallel/test-buffer-tojson.js) | toJSON / from(JSON 形状) | JSON 结构和序列化/重建 |
| [test-buffer-write.js](../packages/node-compat/upstream/node-v22.14.0/test/parallel/test-buffer-write.js) | write；并触及编码专用写入方法 | UTF-8/UTF-16LE/ASCII/Latin1/hex/base64/base64url、写入长度、越界、编码错误、UCS-2 溢出回归 |

### events：3 个文件

| 官方文件 | 主测试 API | 关键场景 |
|---|---|---|
| [test-events-list.js](../packages/node-compat/upstream/node-v22.14.0/test/parallel/test-events-list.js) | EventEmitter / on / removeListener / eventNames | 构造器导出、字符串和 Symbol 事件名、删除后的事件列表 |
| [test-events-listener-count-with-listener.js](../packages/node-compat/upstream/node-v22.14.0/test/parallel/test-events-listener-count-with-listener.js) | listenerCount(event, listener)、on / once / off / removeAllListeners / emit | 按指定函数计数、重复注册、once 执行后移除、准确调用次数 |
| [test-events-once.js](../packages/node-compat/upstream/node-v22.14.0/test/parallel/test-events-once.js) | events.once | 事件参数、error 拒绝与清理、EventTarget、无效选项/信号、提前/事后 abort、阻止事件传播时仍取消、监听器清理 |

### querystring：4 个文件

| 官方文件 | 主测试 API | 关键场景 |
|---|---|---|
| [test-querystring-escape.js](../packages/node-compat/upstream/node-v22.14.0/test/parallel/test-querystring-escape.js) | escape | 字符转义、Unicode/代理项边界 |
| [test-querystring-maxKeys-non-finite.js](../packages/node-compat/upstream/node-v22.14.0/test/parallel/test-querystring-maxKeys-non-finite.js) | parse({maxKeys}) | 10,000 参数；Infinity/NaN 数值与同名字符串的不同语义 |
| [test-querystring-multichar-separator.js](../packages/node-compat/upstream/node-v22.14.0/test/parallel/test-querystring-multichar-separator.js) | parse / stringify | 多字符分隔符和等号分隔符 |
| [test-querystring.js](../packages/node-compat/upstream/node-v22.14.0/test/parallel/test-querystring.js) | parse / stringify / unescape / unescapeBuffer | 重复键/数组、空与非字符串输入、转义、损坏编码、自定义 codec、键数量等综合行为 |

### string_decoder：3 个文件

| 官方文件 | 主测试 API | 关键场景 |
|---|---|---|
| [test-string-decoder-end.js](../packages/node-compat/upstream/node-v22.14.0/test/parallel/test-string-decoder-end.js) | StringDecoder.write / end | 逐字节与整块一致性、结束残留字节、重复使用、UTF-8/UTF-16LE/base64/base64url 等 |
| [test-string-decoder-fuzz.js](../packages/node-compat/upstream/node-v22.14.0/test/parallel/test-string-decoder-fuzz.js) | write / end | 随机字节和分块；输出与拼接 Buffer 的 toString 对照。随机轮次数随运行时间变化 |
| [test-string-decoder.js](../packages/node-compat/upstream/node-v22.14.0/test/parallel/test-string-decoder.js) | StringDecoder / write / end / text | 多种切块、损坏 UTF-8、代理对、状态、无效编码/参数/this、内存足够时的大字符串限制 |

### 新增纯 JS 契约：18 个文件

这些文件与前述 30 项一样，由 Node v22.14.0 主机加载上游原文；内建模块映射到 Niva JS 适配器。逐文件及逐子测试结果已写入上方链接的统一报告，失败项没有移出分母。

| 模块 | 官方文件 | 主测试 API | 关键场景 |
|---|---|---|---|
| `util` | [test-util-format.js](../packages/node-compat/upstream/node-v22.14.0/test/parallel/test-util-format.js) | format / inspect | 常见格式占位符、对象/数组额外参数、循环引用。 |
| `util` | [test-util-promisify.js](../packages/node-compat/upstream/node-v22.14.0/test/parallel/test-util-promisify.js) | promisify | callback 转 Promise、custom promisify symbol、调用顺序与参数错误；含 Node 内部辅助导入。 |
| `util` | [test-util-callbackify.js](../packages/node-compat/upstream/node-v22.14.0/test/parallel/test-util-callbackify.js) | callbackify | Promise 转 callback、多种结果值、异步时序及错误拒绝。 |
| `util` | [test-util-deprecate.js](../packages/node-compat/upstream/node-v22.14.0/test/parallel/test-util-deprecate.js) | deprecate | 重复调用只警告一次，保留包装函数行为并验证 warning。 |
| `assert` | [test-assert.js](../packages/node-compat/upstream/node-v22.14.0/test/parallel/test-assert.js) | assert / strictEqual / deepEqual / throws | loose/strict 相等、深比较、异常匹配、错误消息。 |
| `assert` | [test-assert-async.js](../packages/node-compat/upstream/node-v22.14.0/test/parallel/test-assert-async.js) | rejects / doesNotReject | Promise 与 thenable 成功/失败、拒绝匹配、并发断言收集。 |
| `assert` | [test-assert-deep.js](../packages/node-compat/upstream/node-v22.14.0/test/parallel/test-assert-deep.js) | deepStrictEqual / AssertionError | 深比较差异、错误对象属性与格式化消息；包含 Node `node:test` 和 util 辅助。 |
| `assert` | [test-assert-fail.js](../packages/node-compat/upstream/node-v22.14.0/test/parallel/test-assert-fail.js) | fail | 有/无参数时的默认消息、operator、actual/expected 与错误属性。 |
| `url` | [test-url-fileurltopath.js](../packages/node-compat/upstream/node-v22.14.0/test/parallel/test-url-fileurltopath.js) | fileURLToPath | 非法输入类型、非 file scheme、路径编码和平台路径规则。 |
| `url` | [test-url-pathtofileurl.js](../packages/node-compat/upstream/node-v22.14.0/test/parallel/test-url-pathtofileurl.js) | pathToFileURL | 相对/绝对路径、编码、尾部分隔符与平台分支。 |
| `crypto` | [test-crypto-pbkdf2.js](../packages/node-compat/upstream/node-v22.14.0/test/parallel/test-crypto-pbkdf2.js) | pbkdf2 / pbkdf2Sync | 同步与 callback 派生结果一致、多个输入/算法及参数错误。 |
| `crypto` | [test-crypto-randomuuid.js](../packages/node-compat/upstream/node-v22.14.0/test/parallel/test-crypto-randomuuid.js) | randomUUID | UUID v4 形状、不同调用值、options 与 buffer 参数。 |
| `zlib` | [test-zlib-from-gzip-with-trailing-garbage.js](../packages/node-compat/upstream/node-v22.14.0/test/parallel/test-zlib-from-gzip-with-trailing-garbage.js) | gunzip / gunzipSync | 连续 gzip member 与尾部零字节的解压行为。 |
| `zlib` | [test-zlib-kmaxlength-rangeerror.js](../packages/node-compat/upstream/node-v22.14.0/test/parallel/test-zlib-kmaxlength-rangeerror.js) | gunzip / gunzipSync | 临时降低 Buffer 最大长度，校验输出拼接越界时抛 RangeError。 |
| `stream` | [test-stream-pipeline-with-empty-string.js](../packages/node-compat/upstream/node-v22.14.0/test/parallel/test-stream-pipeline-with-empty-string.js) | pipeline / PassThrough | 字符串源与 objectMode 转换流完成、callback 只调用一次。 |
| `stream` | [test-stream-pipeline-async-iterator.js](../packages/node-compat/upstream/node-v22.14.0/test/parallel/test-stream-pipeline-async-iterator.js) | pipeline / Readable async iterator | 错误源、async iterator 消费、destroy 与错误传播。 |
| `timers` | [test-timers.js](../packages/node-compat/upstream/node-v22.14.0/test/parallel/test-timers.js) | set/clearTimeout、set/clearInterval、set/clearImmediate | 回调顺序、取消、重复调度与句柄 ref/unref 相关约束。 |
| `timers` | [test-timers-promises-scheduler.js](../packages/node-compat/upstream/node-v22.14.0/test/parallel/test-timers-promises-scheduler.js) | scheduler.yield / timers/promises.setTimeout | scheduler 让出事件循环、Promise 定时器与进程事件顺序。 |

### Native/网络 relay 契约：10 个文件

下列测试文件保持上游字节不变，由 Node host 加载和运行；Native 产品操作经 Niva WebView relay 执行，不能用 host `fs`/socket 假对象代替。**本次已执行，10 pass / 0 fail**；`https` 与 `dns` 两个样例只覆盖验证/参数边界，不宣称完成 TLS 握手或 DNS 报文查询。

| 模块 | 官方文件 | 主测试 API | 关键场景及限制 |
|---|---|---|---|
| `fs` | [test-fs-promises-writefile-typedarray.js](../packages/node-compat/upstream/node-v22.14.0/test/parallel/test-fs-promises-writefile-typedarray.js) | `fs/promises.writeFile` / `readFile` | 对临时文件写入 Uint8Array/Uint16Array/Uint32Array 并读回；经过真实 Native 文件系统。使用上游 tmpdir helper。 |
| `process` | [test-process-chdir.js](../packages/node-compat/upstream/node-v22.14.0/test/parallel/test-process-chdir.js) | `process.chdir` / `cwd` | 切换到临时目录、验证 cwd 和无效参数；测试只对 worker_threads 的 main-thread guard 提供 test shim，须在隔离 Niva 进程运行。 |
| `os` | [test-os-eol.js](../packages/node-compat/upstream/node-v22.14.0/test/parallel/test-os-eol.js) | `os.EOL` | 校验平台换行符和只读属性；这是静态 OS 字段测试，不覆盖动态 Native OS 查询。 |
| `child_process` | [test-child-process-execfile.js](../packages/node-compat/upstream/node-v22.14.0/test/parallel/test-child-process-execfile.js) | `execFile` / `execFileSync` / `ChildProcess.kill` | 子进程退出码、callback 错误/输出、shell/env、AbortSignal；需允许 Native 启动固定 Node v22.14.0 子进程 fixture。只在该测试上下文明确映射 fixture execPath，不表示产品 `process.execPath` 被替换或验收。 |
| `http` | [test-http-client-get-url.js](../packages/node-compat/upstream/node-v22.14.0/test/parallel/test-http-client-get-url.js) | `createServer` / `listen` / `get` / request end | 本地 HTTP server 检查 GET 路径；client 分别传 URL 字符串、`url.parse()` 结果和 `URL` 对象。经过真实 Native loopback TCP。 |
| `https` | [test-https-options-boolean-check.js](../packages/node-compat/upstream/node-v22.14.0/test/parallel/test-https-options-boolean-check.js) | `https.createServer` | 用入库 PEM fixture 验证 key/cert/CA 输入类型和错误；不 listen、不握手，故不计作 Native TLS 验收。 |
| `net` | [test-net-server-close.js](../packages/node-compat/upstream/node-v22.14.0/test/parallel/test-net-server-close.js) | `createServer` / `listen` / `close` / `connect` / `Socket.destroy` | 本地 TCP server 接两个连接，关闭时销毁 sockets 并等待 socket/server close 事件。经过真实 Native TCP 生命周期。 |
| `tls` | [test-tls-connect-no-host.js](../packages/node-compat/upstream/node-v22.14.0/test/parallel/test-tls-connect-no-host.js) | `tls.createServer` / `tls.connect` | 本地 server/client 使用 fixture 证书；client 省略 host、提供 CA，验证默认 localhost 名称、授权状态、数据与关闭。经 Native TLS 握手。 |
| `dns` | [test-dns-resolvens-typeerror.js](../packages/node-compat/upstream/node-v22.14.0/test/parallel/test-dns-resolvens-typeerror.js) | `dns.resolveNs`（通用 `resolve` 的 RR 子方法） | 非法 hostname/callback 的同步类型错误；不发 DNS 请求，不验证 Native UDP/TCP 查询。依赖的 `resolveNs` 不在 3 项高频清单内。 |
| `dgram` | [test-dgram-udp4.js](../packages/node-compat/upstream/node-v22.14.0/test/parallel/test-dgram-udp4.js) | `createSocket` / `bind` / `send` / `close` | IPv4 loopback UDP echo；检查 payload、来源地址/端口与 close 事件。经过真实 Native UDP datagram。 |

Native relay 的原始 fixture 由冻结 manifest 校验；运行器中的 helper 映射在报告 infrastructure 字段明示。执行结果已写入统一的 [58 文件逐项报告](node-compat-upstream-results.json)。

## 代表性官方断言

- `path.resolve()`：把 `process.cwd()` 覆盖为空串后，结果应是 `.`。
- `path.win32.matchesGlob('foo\\bar\\baz', 'foo/**')` 应为 `true`。
- `Buffer.compare(Buffer.alloc(0), Buffer.alloc(1))` 应为 `-1`。
- `Buffer.alloc(9).write('foo', -1)` 应抛 `RangeError`，code 为 `ERR_OUT_OF_RANGE`。
- `querystring.parse` 解析 10,000 个参数：`maxKeys: NaN/Infinity` 保留全部；字符串 `'NaN'/'Infinity'` 使用默认 1,000 上限。
- `events.once` 的 signal 提前或随后 abort 时拒绝为 `AbortError`；其他监听器调用 `stopImmediatePropagation()` 仍须完成取消。
- `StringDecoder` 接收 UTF-8 `F0 B8 41`，跨不同分块也应输出 `\ufffdA`。

这些是文件中的代表性场景，完整参数矩阵和断言以原文件为准。

## 扩展 17 个模块：自有测试与 WebView

这些模块现在均已有上方列明的官方文件；本节另列自有测试与 WebView 证据，不混入官方通过数。下表列主入口和代表场景，并非每个方法或每种选项都已有独立用例。测试链接是主要文件；多个模块还由其他回归文件交叉覆盖。

真实集成入口：[WebView fixture](../examples/node-compat-integration/index.html)；已保存结果：[机器证据](node-api-implementation-evidence.json)。

| 模块 | API / 对象 | 主测试文件 | 已测场景及证据层次 |
|---|---|---|---|
| `fs` | readFile/writeFile/appendFile 及 Sync/promises；stat、existsSync、copyFile、FileHandle.read/close、读写流、watch | [bridge.test.js](../packages/node-compat/test/bridge.test.js) | mock 验证编码、回调、错误和参数；WebView 验证真实读写、句柄、流、watch。并非 fs 全部选项。 |
| `process` | pid / cwd；启动数据 | [bridge.test.js](../packages/node-compat/test/bridge.test.js) | WebView 核对 main pid 与 bootstrap、cwd 查询/覆盖；其他 process API 没有同等逐项行为检查。 |
| `os` | platform/arch/EOL、freemem；hostname/totalmem/userInfo/cpus/uptime/networkInterfaces | [bridge.test.js](../packages/node-compat/test/bridge.test.js) | mock 验证静态值不发请求、动态值每次查询；WebView 实际读取主机数据。 |
| `child_process` | spawn / kill、exec / execFile / execFileSync、stdin/stdout/stderr | [bridge.test.js](../packages/node-compat/test/bridge.test.js) | mock 验证 PID、事件、分片 UTF-8、信号和非零退出；WebView 实际启动 shell、捕获输出、终止子进程。 |
| `http` | request / get / createServer / listen / close、消息流与头 | [http-sockets.test.js](../packages/node-compat/test/http-sockets.test.js) | Node-host 与 Node HTTP 互通，检查 chunked/trailers、CL/TE 冲突及注入拒绝；WebView 真 Native socket 完成 600KB echo/关闭。 |
| `https` | get / createServer / listen / close | [network-bridge.test.js](../packages/node-compat/test/network-bridge.test.js) | WebView 使用临时 CA/SAN 证书握手、读取响应、验证自然 socket 关闭与 server.close。 |
| `net` | Socket/connect、write/end、Server listen/accept/close、读写确认、timeout、allowHalfOpen | [network-bridge.test.js](../packages/node-compat/test/network-bridge.test.js) | mock 验证协议、读写计数/确认和超时；WebView 测 FIN 后服务器继续回复，HTTP/DNS 也经过 TCP。 |
| `tls` | connect / createServer、CA/identity、secureConnect | [network-bridge.test.js](../packages/node-compat/test/network-bridge.test.js) | mock 验证参数与明确不支持项；WebView 通过 HTTPS 验证真实 TLS。错误主机名另有 Rust 底层测试，不属于该 WebView fixture。 |
| `dns` | lookup、Resolver/setServers/getServers、resolve/resolve4 等记录查询 | [network-bridge.test.js](../packages/node-compat/test/network-bridge.test.js) | mock 验证记录编解码、ID/来源校验、TC 回退、系统 lookup 路由；WebView 真 UDP 查询 A+TTL/TXT，AAAA 经截断回退同端口 TCP。 |
| `dgram` | createSocket / bind / send / close、message/address | [network-bridge.test.js](../packages/node-compat/test/network-bridge.test.js) | mock 验证数据报边界、来源、credit 和关闭；WebView 真实 UDP echo/关闭及 DNS UDP。 |
| `crypto` | createHash/createHmac、randomBytes/randomUUID、pbkdf2/pbkdf2Sync、scrypt、timingSafeEqual | [crypto-zlib.test.js](../packages/node-compat/test/crypto-zlib.test.js) | Node-host 对照算法/返回值/回调/错误；WebView 直接验证 SHA-256 已知值和 Native timingSafeEqual。 |
| `zlib` | gzip/gunzip 及 Sync | [crypto-zlib.test.js](../packages/node-compat/test/crypto-zlib.test.js) | Node-host 压缩往返、Node zlib 对照、level 和错误；WebView 验证首次加载容量上限及 CommonJS/ESM 对象身份。 |
| `stream` | Readable.from、Writable、PassThrough、pipeline、finished、promises.pipeline | [completed-js-core.test.js](../packages/node-compat/test/completed-js-core.test.js) | Node-host 验证返回值、成功/错误和 destroy；WebView 经 fs/net 间接使用，没有独立 pipeline 契约检查。 |
| `util` | format / inspect / promisify / callbackify / isDeepStrictEqual / deprecate | [events-util.test.js](../packages/node-compat/test/events-util.test.js) | Node-host 验证格式、深度/颜色、循环引用、receiver、错误、异步顺序、Map/Set/TypedArray；没有专门 WebView 行为检查。 |
| `url` | URL / URLSearchParams、pathToFileURL / fileURLToPath | [buffer-querystring-url.test.js](../packages/node-compat/test/buffer-querystring-url.test.js) | Node-host 对照 URL 参数和 POSIX/Windows drive/UNC 路径；HTTP 使用 URL 字符串不单独算 url 模块验收。 |
| `assert` | 可调用 assert、strict/deep 比较、throws/rejects/doesNotReject、AssertionError | [http-assert.test.js](../packages/node-compat/test/http-assert.test.js) | Node-host 验证断言结果、错误类型及 strict alias；WebView 验证 assert.ok 从本页源码恢复调用表达式。 |
| `timers` | set/clearTimeout、set/clearInterval、set/clearImmediate、句柄 ref/unref/refresh | [completed-js-core.test.js](../packages/node-compat/test/completed-js-core.test.js) | Node-host 验证句柄和取消；fixture 使用浏览器全局定时器不算 NodeCompat timers 模块行为验收。 |

## 容易混淆的边界

- 官方 30 文件只覆盖 **path / buffer / events / querystring / string_decoder** 的选定契约。
- 90 个包内测试包括 mock、Node 对照和 Node socket 互通，不能统一视为真实 Native 验证。
- 40 个 release WebView 布尔检查包括业务行为、生命周期和模块身份检查。当前 `zlib/util/url/assert/timers` 没有专门 WebView 行为断言；`stream` 主要由文件/网络路径间接覆盖。
- 179/179 的扫描只检查接口可访问性。官方文件数、包内 test 数、WebView 检查数和 API 数单位不同，不能相加成为“覆盖 API 总数”。
- 本索引由主线程核对官方文件，Luna Fast 核对其余模块的自有/集成测试；本次只整理清单，没有新增执行测试或扩大验收范围。

用户授权的[两处环境豁免](../packages/node-compat/upstream/environment-exclusions.json)只针对确切调用位置，不跳过整个文件或子测试。[原始无豁免结果](node-upstream-unfiltered-results.json)单独保留为 56/58。
