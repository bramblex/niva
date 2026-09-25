# NodeCompat 实施与验收记录

> 历史基线说明（2026-09-25）：本页保留统一 runtime 重构前的范围与验收记录。文中的“当前”、模块/API 数量、通过数、体积和旧配置仅适用于对应历史快照，不代表 `codex/architecture-implementation` 的验证结果。新实现进度及重新验收证据见[架构实施台账](architecture-implementation-plan.md)；原始 JSON 证据保持不变。

2026-09-24，基于 `3787257` 的未提交工作区。机器证据见 [JSON 报告](node-api-implementation-evidence.json)。

## 当前结果

- 已接入 **22 个模块入口、179 个目标 API 入口**。入口计数来自真实 WebView 对固定清单的逐项查找，实例方法按实际对象或 prototype 检查。
- debug HTTP、打包 `niva://` 两条路径均完成真实调用验证。首版 release 连续两次通过 34 项检查；本轮官方测试扩展后的 release 通过 **45 项布尔检查**。
- macOS arm64 release 主程序为 **2,772,208 bytes（2.772 MB）**，距离严格小于 3,300,000 bytes 的上限还剩 **527,792 bytes**。
- **官方契约适用范围门禁通过**：固定 Node v22.14.0 的 58 个原始文件，58 pass、0 fail、0 unsupported，另有用户授权的 2 处环境检查点跳过；[完整结果](node-compat-upstream-results.json)记录每个文件及源码校验。范围扩展到 22 个模块族的选定用例；不代表全部 179 项调用契约通过。详见[模块/API/用例索引](node-test-case-index.md)。

[原覆盖盘点](node-api-coverage.md)的 46 基础支持 / 49 部分兼容 / 84 缺失是实施前快照，仍保留供对照。本轮没有仅凭入口或 smoke 通过就把这些状态升级为“Node 已兼容”。

## 模块与 API 入口

| 模块 | 目标 API 数 | 当前实现分层 |
|---|---:|---|
| `fs` | 40 | Native 文件操作 + JS callback/Promise/流 |
| `path` | 16 | JS；cwd 每次同步查询 |
| `events` | 11 | JS |
| `process` | 17 | main 启动数据 + JS/Native 操作 |
| `http` | 4 | JS HTTP/1.x + Native TCP |
| `https` | 3 | JS HTTP/1.x + Native TLS |
| `util` | 6 | JS |
| `os` | 15 | 静态启动注入；动态 Native/XHR |
| `crypto` | 8 | JS 库/Web Crypto；timingSafeEqual 用 Native |
| `child_process` | 7 | Native 子进程 + JS 流与接口 |
| `stream` | 7 | readable-stream |
| `url` | 4 | 浏览器 URL + JS |
| `buffer` | 14 | buffer 及适配 |
| `querystring` | 2 | JS |
| `net` | 3 | Native TCP + JS Duplex |
| `tls` | 2 | Native 系统 TLS + JS |
| `dns` | 3 | JS dns-packet/UDP/TCP；系统 lookup 用 Native |
| `dgram` | 1 | Native UDP + JS |
| `zlib` | 4 | fflate |
| `assert` | 5 | JS |
| `string_decoder` | 1 | 纯 JS 库 |
| `timers` | 6 | 浏览器定时器 + JS 句柄 |
| 合计 | **179** | **22 个模块族** |

具体调用契约仍需按[官方验收规则](node-upstream-conformance.md)继续对齐。

## 已落地的分层与加载

Native 提供文件、进程、系统信息、TCP/UDP/TLS。HTTP/HTTPS 客户端和应用服务器、DNS 报文处理在 JS；`dns.lookup` 使用系统 resolver，记录查询使用 UDP，并在截断时退回 TCP。原 `ureq` / `http.requestStream` / `Niva.api.http` 应用客户端路径已移除；Niva 内部资源、同步调用和 WS 服务继续保留。

可信本地页面使用 WS 传递事件和二进制；同步调用使用鉴权 XHR，由 Rust 校验 token、精确 Origin、Host、请求大小及可同步执行的方法。socket/文件句柄绑定窗口和 WS 连接，断连清理资源。Socket 接收采用 256 KiB credit 窗口；仅订阅 `onChunk` 的调用不保留历史 Blob 分片。WS 断开会拒绝旧连接的 pending calls。

静态 `os` 数据在启动时注入；动态 cwd、CPU、空闲内存、网卡和 uptime 每次查询。真实 `process` 仅注入 main 窗口可信顶层；子框架不继承 process 数据。

NodeCompat 默认启用，并作为压缩 JS、ESM wrapper、索引和 loader 内嵌进主程序。`nodeCompat: false` 关闭加载；对象配置筛选模块并控制 import map。未知模块名使配置校验失败。`Niva.require` 与 `Niva.import` 复用已注册对象，classic/ESM 初始化保持幂等。Devtools 不再额外暂存一份 NodeCompat 资源，版本请求已改用 `https` 模块。

## 验证

| 检查 | 本次结果 |
|---|---|
| `cargo fmt --all -- --check` | 通过 |
| `cargo check --workspace` | 通过 |
| `cargo clippy --workspace --all-targets` | 通过；存在 warning，未采用 `-D warnings` |
| `cargo test --workspace` | 通过：Niva 120 项、packager 10 项 |
| Windows `x86_64-pc-windows-msvc` target check | 通过；编译证据 |
| NodeCompat JS tests | 109/109 |
| Devtools bridge/API/fixture 检查 | 20/20 |
| Devtools TypeScript/Vite | 通过 |
| 网站 typecheck/build | 通过 |
| 上游 harness 自检 | 27/27 |
| 选定官方原测试文件 | **58 pass / 0 fail / 0 unsupported；2 处环境跳过** |
| release 打包 WebView fixture | **最新 45/45，入口 179/179** |

真实 fixture 覆盖同步二进制读写、callback、FileHandle、文件流/watch、子进程同步与异步执行/kill、hash/timingSafeEqual、600 KB HTTP 往返与关闭、有效 CA/SAN 的 HTTPS 与自然关闭、PKCS#1 RSA 私钥归一化后的 HTTPS、UDP echo、DNS A/TTL/TXT、UDP 截断转同端口 TCP、TCP 收到 FIN 后继续回复、OS 动态查询，以及 ESM/CommonJS 对象身份。

运行入口：

```sh
npm test --workspace=packages/node-compat
cargo build --release -p niva
python3 examples/node-compat-integration/run.py target/release/niva --packaged
```

fixture 使用临时 CA 和文件，以及仅作用于子进程的 HOME/TMPDIR。进度日志默认关闭，`--trace` 可开启诊断；断言和最终结果始终保留。最终 release SHA-256：

```text
ff5c9ffe219c1fb21813a5f07d2b2eb658515872f3d2b5d7ae97c237111191a3
```

产物为 `target/release/niva`，包含 Native、内嵌 classic/ESM/vendor JS 及加载索引。实测平台 macOS 26.6.2 / arm64，Rust 1.98.1；不是业务资源包、安装包或签名后的分发产物。macOS/Windows 构建脚本及 CI 的主程序上限统一为 3,300,000 bytes。

## 明确限制与后续门禁

- 当前选定的 30 个官方文件全部通过；它们不覆盖全部 Node API 或全部 179 项契约，入口清单未缩减。
- macOS 系统 TLS 结束写端会结束会话；TLS `allowHalfOpen: true` 明确返回 `ENOTSUP`。TCP 支持半关闭。
- TCP `setNoDelay` / `setKeepAlive` 尚未接原生选项，明确返回 `ENOTSUP`；`ref` / `unref` 不控制 Niva 进程存活。
- HTTP 目前使用关闭连接的策略，完整 Agent、连接池、upgrade 和全部选项语义不在本次已验证范围。
- UDP broadcast/multicast、部分文件选项等仍有明确不支持项；不能把模块入口数量扩展成所有方法/选项的承诺。
- `child_process.fork` 已排除。`--stdio` 模式的 stdin/stdout 保留给宿主 NDJSON 协议。
- Windows 没有真机或最终 release 体积数据；远端 CI 未运行确认，未做发布、签名或提交推送。

## 分工与记录范围

Luna Fast 完成纯 JS vendor、内嵌资源构建、Devtools 资源改造和文档核对；Luna Deep 完成 socket、OS/DNS 系统查询、JS 基础模块与网络适配，并复核文件/进程资源清理。主线程负责同步桥接、文件/进程/HTTP 集成、跨模块修复和最终 release 验收。

工作区同期新增的上游测试门禁和相关材料已保留；它们的失败状态在本报告中如实列出。本次未提交、推送或发布。

## 初始 30 文件契约修复（历史）

Luna 分工修复 path/querystring、Buffer/StringDecoder、events；主线程补齐忠实的测试辅助设施并集成验证。官方文件/fixture 的 SHA-256 和 30 文件分母未变。自己的旧回归断言中，querystring 自定义 decoder 的 `+` 输入应为 `%20`、Buffer kMaxLength 应跟随 Node 基准；这两处已纠正。

运行器显式提供测试基础设施：预期错误对象、EventTarget 内省、OS cwd/env/内存容量、固定 fixture 子进程。被测模块与 crypto 辅助调用均路由 Niva，buffer binding 范围检查使用 Niva 共享实现；没有用宿主 Node 的被测 API 代替。

新增 WebView 回归验证 EventEmitter 可构造、abort 抵抗 stopImmediatePropagation、main process.cwd 覆盖、querystring decoder 输入、Buffer 边界以及 UTF-8/base64url StringDecoder。模块选择资源闭包也补齐了 Buffer 依赖，并通过专门检查。原全仓 Rust 门禁记录保留；该历史阶段未修改 Rust 实现，另重跑了 6 项内嵌模块资源测试和最终 release 构建。

## 8 项失败的后续修复

本次已修复其中 6 个原始文件：`test-assert.js`、zlib 容量边界、child_process execFile、HTTP URL 输入、HTTPS 选项验证、TLS no-host 验证。assert 调用表达式通过本页源码与 Acorn 恢复；zlib 在实际产品注册表中首次加载才初始化。HTTP 旧式 URL 使用浏览器可用的 `url@0.11.4`。

macOS 显式 CA 路径在认证完成前暂停 Secure Transport。只有 peer DER 与显式信任证书完全相同才使用固定证书的 Basic X509 策略；有效期、DNS/IP/SAN/CN、KU/EKU、关键扩展及握手签名仍被验证，普通 CA 链继续走 SSL 策略。显式 CA 替换系统根；不同 servername、自定义验证回调或未实现的 TLS 策略选项会被拒绝，避免静默忽略安全要求。没有修改系统信任库。

`util-format` 和 `assert-deep` 各有一处引擎专用检查按用户要求跳过；原文件和 58 项文件分母保留，其他检查继续执行。默认门禁已通过，无豁免诊断结果仍单列 56/58。[边界与复现](node-webview-engine-boundaries.md)。
