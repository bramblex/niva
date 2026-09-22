# Niva Bridge 合约（给兼容层作者：如下一代 node-like 封装）

本仓库只提供**传输 + 原生 API 直通**；所有"好用"的形状（Node 形、`require`、
`path` 实现等）由独立仓库基于本合约实现。本合约变更遵循 semver 式的
wire 版本（见 `Niva.bridgeVersion`，与 Rust 侧 `WIRE_VERSION` 同步）。

## 传输

- 每个窗口一个 WebSocket，URL 由 native 在初始化脚本前注入：
  `window.__niva_ws_url` / `window.__niva_window_id` / `window.__niva_token`。
- token 每启动随机生成，只存在内存，不接受外部传入；缺失/错误直接 403。
- 断线自动重连（1s），发送队列在 open 后 flush；二进制另有独立队列。

## 文本帧（JSON 对象）

```text
C -> S  {t:"hello", wid, v}                 # 首帧绑定窗口，v 必须等于 bridgeVersion
C -> S  {t:"call", id, method, args}        # id: u64，客户端自增
C -> S  {t:"cancel", id}                    # 终止 stateful 调用
S -> C  {t:"result", id, code, message, data}  # 终局；code 0=ok -1=handler错 -2=超时 -3=忙
S -> C  {t:"event", id?, seq, name, data}   # 流推送；id 缺席为全局广播
```

seq 按调用单调递增（result 不占序号）。超时/取消/关窗语义：
超时回 `-2`，cancel 与关窗静默清理（不回包），超时不杀底层阻塞调用。

## 二进制帧（18B 头 + payload，WS 边界即帧边界，无需长度字段）

```text
[ver=1][flags][id u64 BE][seq u64 BE][payload...]
flags: 0x01 START / 0x02 END / 0x04 STDERR（exec 子流）
```

- S→C：`fs.readStream` / `http.requestStream` / `resource.readStream` 的 body
  分片、`execStream` 的 stdout（flag 0）/stderr（flag 0x04）；
  按 `id` + flag 分组、seq 排序、END flush 成一个 Blob。
- C→S：`fs.writeStream` 的文件内容、`execStream` 的 stdin；
  同一 id 的 seq 自增，END 表示结束（关闭 stdin / 落盘提交）。

## JS 运行时（初始化脚本注入）

```js
Niva.call(method, args) -> Promise                      // 一元调用
Niva.stream(method, args, {onEvent, onBlob}) -> {id, promise, cancel}
Niva.streamSend(id, ArrayBuffer|Uint8Array|string, end?) -> bool
Niva.api.<ns>.<method>(...)  // 直通代理 + 高层覆盖（见下）
Niva.addEventListener/removeEventListener(after `*` 通配符)
Niva.bridgeVersion // number
```

`Niva.api` 代理默认透传 `ns.method`；以下方法被流式原生实现覆盖，
签名与旧 unary 版本逐字节兼容：`fs.read/write/append`、`http.get/post/request`、
`process.exec`、`resource.read`。新增原生流式方法直接可用：
`fs.readStream/writeStream`、`http.requestStream`、`process.execStream`、
`resource.readStream`。

## 安全模型（已决策）

- 传输只听 loopback；密钥只有一个（启动时随机 token，只存内存，
  不接受外部传入），WS 与 HTTP 共用。
- 写操作与 API 全走 WS（`?token=` 鉴权）；静态文件读走 HTTP session-cookie：
  entry URL 自带 `?niva_token={token}` 首验并种 `HttpOnly` cookie，
  之后凭 cookie；无身份 → 403。详见 `docs/http-auth-plan.md`。
- 豁免：`/__niva_compat/*` 公开（无秘密）+ CORS 头，供 dev-server 页面跨域加载。
- 高危面：`/__niva_fs/` 读任意本地文件，必须在 session 鉴权内。
- 不做：UA 标记（破坏页面兼容）、自定义 header（wry 无通道）、读路径身份豁免。

## 给兼容层的建议分层

```text
native 流式原语（本仓） → Node 形封装（独立仓） → 用户/AI 代码
   readStream/writeStream      readFile/writeFile + 过渡同步 existsSync/statSync/readFileSync
   execStream                  child_process.exec / spawn 仿真
   os.info/dirs                os.platform/arch/tmpdir/homedir
   (无)                        require('path') 纯 JS 实现
```

注意：bridge 只有异步语义；同步只给小文件读三件套开过渡口子
（`existsSync/statSync/readFileSync`，同步 XHR + 60s 超时 + 每次 warning，
`execSync`/写同步永不给），详见 `docs/node-api-frequency.md §6.4`；
`require` 垫片只注册白名单（`fs`/`path`/`os`/`child_process`），
未知模块保持抛错。
