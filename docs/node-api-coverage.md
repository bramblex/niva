# Node.js 模块与常用 API 覆盖盘点

> 2026-09-24 · 源码基线 `3787257` · 用于选择后续覆盖范围，尚未承诺全部实现。
> 本页逐项计数保留为实施前基线；当前代码已进入实现阶段，新增入口与实测结果见[实施状态](node-compat-implementation.md)。这些盘点状态也不是[Node 官方契约门禁](node-upstream-conformance.md)的验收结果。

浏览器第三方补充：[纯 JS 实现方案与 crypto 逐项映射](node-browser-js-options.md)。现成 Web 兼容库可覆盖的范围按低难度评估，原 Native 路线保留为备选；候选库不改变当前实现状态。

打包决策：全部 179 项目标功能的 Native、NodeCompat JS 及内嵌开销合计满足 **3.3 MB（3,300,000 bytes）**门禁时，直接内置 NodeCompat。[全范围预算](node-api-size-estimates.md#6-全部目标-native--js-内嵌后的体积条件)中已量化部分约 2.52–3.18 MB（网络协议改为 JS 后的预算），尚非完整 release 实测；此前约 2.40 MB 只是局部库样本。

开发体验目标：Node 风格 API 作为默认入口，兼容依赖已覆盖能力的 Node 纯 JS 库；npm/CJS 依赖在构建时处理，Niva 专有能力保留扩展接口。该目标不把现有 179 项接口或所有 Node 库自动标为已兼容，详见 [设计与库验收](node-compat-design.md)。

## 1. 统计结论与口径

当前目标为 **22 个 Node 模块族、179 项 API 契约**。基线时 NodeCompat 已有 **15/22 个模块入口**，7 个未注册；已有入口均为子集。API 层面：**46 项基础支持、49 项部分兼容、84 项缺失**。

| 范围 | API 数 | 基础支持 | 部分兼容 | 缺失 | 基础支持占比 |
|---|---:|---:|---:|---:|---:|
| 当前目标 | 179 | 46 | 49 | 84 | 25.7% |
| 其中高频档 | 95 | 32 | 31 | 32 | 33.7% |

这里的百分比是**清单条目覆盖率**，不是 npm 包兼容率、真实调用流量占比，也不是全部 Node API 覆盖率。部分兼容不折算成半个或一个已支持项。

- **基础支持**：所列常用调用的基本入参、返回形态和主要行为已实现；未声称通过 Node 全量一致性测试，详细限制见逐项说明。
- **部分兼容**：有相近实现，但同步/异步、callback、返回对象、常用选项或流行为存在已知差异，需要修改调用方或继续补齐。
- **缺失**：没有对应 Node 入口或实质行为；底层已有 `Niva.api.*` 能力不能直接计作 Node API 已实现。恒定返回 false 的 kill 等占位实现也算缺失。
- 一个独立调用契约计一项；属性和类构造器同样计项。`fs` callback、`fs/promises`、`fs.*Sync` 分开，因为已有代码的调用方式不同。`fs/promises` 等子路径归入同一模块族；`node:` 前缀、ESM/CommonJS 别名不重复计数。
- `path.posix/win32` 各按一个变体入口计项，不再逐个复制其方法；`createHash` 包含常用 update/digest 链路；类的所有方法没有自动加入分母。每一行的边界以列出的 API 为准。
- Browser 提供的 `console`、timers、fetch 等另外说明。浏览器全局存在不等于 `require("node:console")` 等模块入口已实现，也不等于 Node 的完整运行时语义。

### 当前范围决策

移出目标：`perf_hooks`、`console`、`http2`、`tty`、`v8`、`vm`、`sqlite`、`test`、`cluster`、`worker_threads`、`readline`。`console` 直接使用浏览器原生；`perf_hooks` 不做 Node 兼容模块。`child_process.fork` 也已放弃，不计入目标；不引入 Node 宿主。移除 `node:test` 不表示删除项目测试，移除 `worker_threads` 不限制纯 JS 库内部使用浏览器 Worker。

### 启动数据注入

| 对象/内容 | 目标方案 | 边界 |
|---|---|---|
| process 固定字段 | 启动时收集 env/argv/argv0/platform/arch/pid/execPath/version/versions，同步读 JS 快照 | **仅 main 窗口（id 0）的可信顶层页面注入/注册**；全局、require 与 ESM 入口一致范围。版本字段使用真实元数据，不伪造 Node/V8。 |
| process.cwd | 每次同步 XHR 查询当前目录 | chdir 是实际 Native 操作；后续查询直接得到当前值，不维护 cwd 缓存。 |
| process.env/argv | 按应用启动配置固定注入 | 本方案不增加动态覆盖层，不修改用户全局环境。 |
| os 固定/会话基础信息 | EOL、平台、架构、目录、系统版本、hostname、totalmem、userInfo 启动注入 | 获准本地页面读取快照；跨域不自动获得这些宿主信息。新原生字段只需在启动收集时补齐。 |
| os.uptime | 每次同步 XHR 查询系统当前值 | 不在 JS 中推算。 |
| os.cpus / freemem / networkInterfaces | 每次同步 XHR 查询当前值 | 动态信息不做缓存、推算或定时刷新。 |
| process 操作及事件 | chdir、exit、stdio、nextTick、on 保留各自调用/JS实现 | 同样仅在 main 对象中提供；注入数据不能替代操作能力。 |

静态信息在启动时固定注入，动态信息每次用同步 XHR 查询。公共 Native 收集/序列化/定向注入只计一次；静态字段 getter 不做 XHR。path/url 需要当前 cwd 时使用内部 XHR 查询，不在子窗口注册真实 process 对象。依赖库内部的浏览器 process shim 保持模块私有，不夹带主窗口进程数据。

上述是目标设计；当前源码未实现的 process 注入和 os 同步 getter 仍保留原实现状态。注入范围不等于改变同源页面可访问关系，也不声称现有 Niva.api.process.* 已加 main 守卫。

网络分层最新决策：[Native 基座与 JS 协议层评估](node-layering-plan.md)。

## 2. 评估模型：Native 能力、JS 封装、依赖增量

频率沿用 [早期调研](node-api-frequency.md) 的模块方向；“高/中/场景”是定性工程档位，没有可复算的 npm/GitHub 使用频率样本。官方文档只用于核对 API 契约。本轮按用户确认的三通路及 Native/JS/依赖分层重评成本。

### JS 优先的实现原则

默认使用 JS、浏览器原语和可在 Web 运行的第三方纯 JS 库。只有真实系统能力无法由浏览器提供、选定 API 行为无法在 JS 层满足，或目标 WebView 的实际性能测试不达标时，才使用/扩展 Native。现有 Native 能力也不意味着普通 JS 逻辑应默认下沉。

Node HTTP/HTTPS 客户端及应用服务器统一在 JS 实现 HTTP/1.1，复用 Native net/tls 字节流；DNS resolve 复用 dgram/net。fetch 可供普通 Web 请求独立使用，不承担 Node socket 协议语义。真实文件、OS 信息、进程启动、TCP/UDP 等仍需要系统层。没有性能基准时，不预设摘要、压缩、流对象等要改成 Native。

### 三条既定原生调用路径

| 路径 | 用途 | 本次估算前提 |
|---|---|---|
| 双向 WS | 本地异步请求、事件、二进制、双向流 | 基础设施复用；有现成 Native handler 时不重复计算协议建设成本。 |
| 同步 XHR | 本地同步返回和 Sync API | 既定方案，同步接线统一为低；具体 Native 后端是否存在另评。 |
| 跨域 IPC | 无法使用本地 WS/XHR 的外源页面降级 | 只做少量、简单、明确授权的 unary JSON API；不要求与本地全量接口对等。 |

纯 JS/WebView 原语不是第四种原生传输，它们不需要过桥。跨域限制指 Niva 对本地服务的开放边界；不把 WS 失败当作绕过授权的 IPC 自动回退。

逐项表中的“本地/跨域”是**目标调用路径及边界**，不是当前实现已通过的证明：缺失的接口也需要规划路径。当前目标实现状态为 46/49/84；不能把“可用 IPC 实现”计作“已实现”。IPC 候选只代表建议可纳入的简单子集，实际 grant 仍需按窗口、精确 origin 与方法配置。

当前源码的 IPC 只接受 unary JSON，单次请求/响应上限 256 KiB，拒绝 stream handler；`window.open`、`webview.baseFileSystemUrl` 即使授权也拒绝。跨域不提供原生二进制流、通用服务器推送或同步返回。获准的简单系统查询可考虑异步 IPC 形式；真实 process 对象只在 main 可信顶层注入，不向其他窗口或跨域暴露。[Rust IPC 检查](../crates/niva/src/app/api_manager/mod.rs#L281)、[JS 路由](../crates/niva/assets/initialize_script.js#L320)。

当前 Rust 实现接受经授权的已注册 Unary handler，并没有另一个硬编码的“简单 API”白名单。本表的只读 IPC 候选是产品范围建议；不把它写成当前代码已经施加的额外限制。同步 XHR 按既定方案估算；本分支源码中的具体同步入口接线仍单独记录，不因重评自动变成已交付。

### 难度与工作层

| 难度 | 判断依据 | 例子 |
|---|---|---|
| 低 | Native 已有能力仅需封装，或已有不依赖 Node 的浏览器纯 JS 第三方实现 | 现有文件操作接 Sync XHR、进程/OS 已有信息同步返回 |
| 中 | 扩展已有 Native 或补小型原生方法；复杂 JS 语义 | Stats 字段、真实路径、Node 流适配和 Buffer/事件边界 |
| 高 | Native 缺少整类系统能力，需新增后端及资源生命周期 | TCP/UDP、服务端 socket 等 |
| 很高 | 需要引入大型第三方库、执行引擎或完整运行时 | 若未来确需大型Native依赖，必须记录选型与体积；当前不引入Node宿主 |

整体难度不由函数名或同步/异步决定。同步 XHR 接线始终低；真正需要新增 Native 后端或大型依赖的接口仍单独列明其成本。有现成浏览器纯 JS 库能够覆盖的范围统一按低；缺少现成实现而需自行补复杂 JS 语义时才评中。已有第三方依赖能够复用时，不重复计算为新增大型依赖。未选型/未测量的体积只标风险，不虚构新增字节；最终仍按仓库小于 3.3 MB 的完整 release 主程序目标实测。

Native 状态为“具备/部分具备/缺失/不需要”：它描述满足该 Node 条目所需的原生能力；Native 具备不等于 NodeCompat 入口已经提供。“不需要”表示可在 JS/WebView 层处理，不意味着该 Node API 已实现。模块汇总和下表只估算尚未基础支持的条目；属性、同步接口及子路径的计数口径不变。

### 频率 × 剩余难度

| 频率 | 低难度 | 中难度 | 高难度 | 很高难度 | 待补齐合计 |
|---|---:|---:|---:|---:|---:|
| 高 | 29 | 31 | 3 | 0 | 63 |
| 中 | 23 | 39 | 7 | 0 | 69 |
| 场景 | 0 | 0 | 1 | 0 | 1 |

### 尚待补齐的工作层

同一底层改动可能覆盖多行，以下是目标条目数，不是独立开发任务数或工期；共享前置能力不能拆开承诺。

| 主工作层 | 高频 | 中频 | 场景 | 合计 |
|---|---:|---:|---:|---:|
| JS封装 | 29 | 23 | 0 | 52 |
| JS语义 | 16 | 17 | 0 | 33 |
| 扩展Native | 17 | 23 | 0 | 40 |
| 新增Native | 1 | 6 | 1 | 8 |
| 运行时工程 | 0 | 0 | 0 | 0 |

完整 `nativeWork/jsWork/dependencyReason` 及每项路由说明保存于 [机器可读清单](node-api-inventory.json)。这使得“只补 JS”与“必须改 Native”的决策可以单独筛选。

## 3. 模块总表

体积为**预算增量**，Native 非零区间未实测；JS 为 minify + raw DEFLATE 后的资源预算，和 Native 代码分开。共享依赖不能逐行相加；HTTP/HTTPS/DNS 采用 JS 协议层，socket 基座计一次；fork已移除。[逐模块工作依据、整体难度及库实测](node-api-size-estimates.md)。

| 模块族 | 频率 | 常见场景 | 入口 | API 数 | 基础支持 | 部分 | 缺失 | 补齐难度 | Native Δ KiB | JS 压缩 Δ KiB |
|---|---|---|---|---:|---:|---:|---:|---|---:|---:|
| [`fs`](#module-fs) | 高 | 文件、配置、构建产物 | 已有子集 | 40 | 0 | 21 | 19 | 低—高 | 24–180 | 2–8 |
| [`path`](#module-path) | 高 | 路径拼接与跨平台路径 | 已有子集 | 16 | 15 | 1 | 0 | 中 | 0 | 1–5 |
| [`events`](#module-events) | 高 | 事件机制与依赖库基础 | 已有子集 | 11 | 8 | 1 | 2 | 低—中 | 0 | 1–4 |
| [`process`](#module-process) | 高 | 环境、CLI 参数、进程生命周期 | 未注册 | 17 | 0 | 0 | 17 | 低—高 | 0–20 | 2–6 |
| [`http`](#module-http) | 高 | HTTP 客户端、Node 服务端 | 已有子集 | 4 | 0 | 2 | 2 | 中 | 0 | 8–20 |
| [`https`](#module-https) | 高 | HTTPS 客户端、TLS 服务端 | 已有子集 | 3 | 0 | 2 | 1 | 中 | 0 | 2–6 |
| [`util`](#module-util) | 高 | 格式化、调试、异步适配 | 已有子集 | 6 | 3 | 3 | 0 | 低—中 | 0 | 1–6 |
| [`os`](#module-os) | 高 | 平台、用户目录、系统信息 | 已有子集 | 15 | 1 | 4 | 10 | 低—中 | 8–80 | 1–3 |
| [`crypto`](#module-crypto) | 高 | 摘要、随机数、签名与密钥派生 | 已有子集 | 8 | 0 | 3 | 5 | 低—中 | 0–8 | 9–14 |
| [`child_process`](#module-child_process) | 高 | CLI、工具链、外部程序 | 已有子集 | 7 | 0 | 2 | 5 | 中 | 0–32 | 2–8 |
| [`stream`](#module-stream) | 高 | 库依赖、大文件与网络数据 | 已有子集 | 7 | 0 | 2 | 5 | 低 | 0 | 35–42 |
| [`url`](#module-url) | 高 | URL 解析、参数、file URL | 已有子集 | 4 | 4 | 0 | 0 | 已具备（本表） | 0 | 0 |
| [`buffer`](#module-buffer) | 高 | 二进制数据和编码 | 已有子集 | 14 | 9 | 5 | 0 | 低 | 0 | 9–12；共用 stream 时 0–2 |
| [`querystring`](#module-querystring) | 中 | 既有 Node 包的查询参数 | 已有子集 | 2 | 2 | 0 | 0 | 已具备（本表） | 0 | 0 |
| [`net`](#module-net) | 中 | TCP/IPC、数据库客户端依赖 | 未注册 | 3 | 0 | 0 | 3 | 高 | 16–120 | 3–10 |
| [`tls`](#module-tls) | 中 | TLS 连接与证书配置 | 未注册 | 2 | 0 | 0 | 2 | 高 | 8–72 | 2–6 |
| [`dns`](#module-dns) | 中 | 域名查询与网络工具 | 未注册 | 3 | 0 | 0 | 3 | 中 | 0–24 | 10–14 |
| [`dgram`](#module-dgram) | 场景 | UDP、发现协议 | 未注册 | 1 | 0 | 0 | 1 | 高 | 8–48 | 2–5 |
| [`zlib`](#module-zlib) | 中 | 压缩、解压、网络内容 | 已有子集 | 4 | 0 | 2 | 2 | 低 | 0 | 6–9 |
| [`assert`](#module-assert) | 中 | 断言与测试依赖 | 已有子集 | 5 | 4 | 1 | 0 | 中 | 0 | 0–2 |
| [`string_decoder`](#module-string_decoder) | 中 | 分片字节解码 | 未注册 | 1 | 0 | 0 | 1 | 低 | 0 | 10–12；共用 stream 时 0–1 |
| [`timers`](#module-timers) | 高 | 定时调度 | 未注册 | 6 | 0 | 0 | 6 | 低—中 | 0 | 1–4 |

本次按用户决策从原 33 模块/214 API 候选中移出 11 个模块、34 项 API。它们不再计为缺失或待补齐；另移出 child_process.fork 1 项，当前为 179 项；当前目标也不是 Node 全部内置模块目录。模块未列出的 options、重载和类成员仍需在具体实现时界定。

## 4. 逐项 API 清单

“状态”是当前 NodeCompat 契约实现；“Native”是底层能力；“工作层/难度”是剩余工作；“路线”是本地目标与跨域边界，四者分别记录。纯 JS 目标及 IPC 候选不表示缺失接口已经可用。具体 Native 工作、JS 工作和选型条件也保存在 JSON 对应条目。

<a id="module-fs"></a>

### fs

Node 参考：[官方 fs 文档](https://nodejs.org/api/fs.html)。

| API | 频率 | 当前状态 | Native 能力 | 工作层 / 难度 | 本地 → 跨域目标 | 差距与依赖依据 | 源码 |
|---|---|---|---|---|---|---|---|
| `fs.createReadStream` | 高 | 缺失 | 部分具备 | 扩展Native / 中 | WS → 不纳入IPC | 没有 Readable 文件流入口；底层 readStream 是 Niva bridge 协议，不是 Node ReadStream。 Native：已有 WS 分块读写和取消；完整 flags/偏移选项及原生读取节奏控制仍需扩展。 JS：将现有分块传输接到 Node Readable/Writable，补事件、关闭、错误和背压。 | [fs.js:180](../packages/node-compat/src/runtime/fs.js#L180)；[fs.rs:224](../crates/niva/src/app/api/fs.rs#L224)；[fs.rs:272](../crates/niva/src/app/api/fs.rs#L272)；[mod.rs:97](../crates/niva/src/app/api_manager/mod.rs#L97) |
| `fs.createWriteStream` | 高 | 缺失 | 部分具备 | 扩展Native / 中 | WS → 不纳入IPC | 没有 Writable 文件流入口；底层 writeStream 协议不是 Node WriteStream。 Native：已有 WS 分块读写和取消；完整 flags/偏移选项及原生读取节奏控制仍需扩展。 JS：将现有分块传输接到 Node Readable/Writable，补事件、关闭、错误和背压。 | [fs.js:180](../packages/node-compat/src/runtime/fs.js#L180)；[fs.rs:224](../crates/niva/src/app/api/fs.rs#L224)；[fs.rs:272](../crates/niva/src/app/api/fs.rs#L272)；[mod.rs:97](../crates/niva/src/app/api_manager/mod.rs#L97) |
| `fs.existsSync` | 高 | 缺失 | 具备 | JS封装 / 低 | XHR → 不提供同步 | 当前 fs 入口未导出此方法；沿用既定同步 XHR 方案接入原生文件操作，剩余工作为参数、结果及错误映射。 Native：无；fs.exists 已注册为 unary Native API。 JS：用给定 syncXHR 将布尔结果作为同步返回并映射错误。 | [fs.js:8](../packages/node-compat/src/fs.js#L8)；[fs.rs:45](../crates/niva/src/app/api/fs.rs#L45) |
| `fs.mkdir` | 高 | 部分兼容 | 部分具备 | JS封装 / 低 | WS → 不纳入IPC | 名字存在，但导出的是 Promise 实现；没有 Node 回调签名/回调调用，调用方必须 await。 Native：Native 有 createDir/createDirAll，但不接收权限 mode。 JS：选择 createDir 或 createDirAll；补回调/Promise/同步入口与 Node recursive 返回约定。 | [fs.js:12](../packages/node-compat/src/fs.js#L12)；[fs.rs:125](../crates/niva/src/app/api/fs.rs#L125)；[fs.rs:13](../crates/niva/src/app/api/fs.rs#L13) |
| `fs.mkdirSync` | 高 | 缺失 | 部分具备 | JS封装 / 低 | XHR → 不提供同步 | 当前 fs 入口未导出此方法；沿用既定同步 XHR 方案接入原生文件操作，剩余工作为参数、结果及错误映射。 Native：Native 有 createDir/createDirAll，但不接收权限 mode。 JS：选择 createDir 或 createDirAll；补回调/Promise/同步入口与 Node recursive 返回约定。 | [fs.js:8](../packages/node-compat/src/fs.js#L8)；[fs.rs:125](../crates/niva/src/app/api/fs.rs#L125)；[fs.rs:13](../crates/niva/src/app/api/fs.rs#L13) |
| `fs.readFile` | 高 | 部分兼容 | 部分具备 | 扩展Native / 中 | WS → 不纳入IPC | 名字存在，但导出的是 Promise 实现；没有 Node 回调签名/回调调用，调用方必须 await。 Native：现有 WS readStream/writeStream 已提供核心读写；需为 Node flag/mode 等选项扩展 Native 参数和文件打开方式。 JS：复用已有流读写及 Buffer 编码；补 callback/Promise 契约、AbortSignal 和错误映射。 | [fs.js:9](../packages/node-compat/src/fs.js#L9)；[fs.rs:224](../crates/niva/src/app/api/fs.rs#L224)；[initialize_script.js:505](../crates/niva/assets/initialize_script.js#L505) |
| `fs.readFileSync` | 高 | 缺失 | 部分具备 | 扩展Native / 中 | XHR → 不提供同步 | 当前 fs 入口未导出此方法；沿用既定同步 XHR 方案接入原生文件操作，剩余工作为参数、结果及错误映射。 Native：已有原始二进制 readStream，但无有界 unary 文件读取接口供同步 transport 一次返回。 JS：保留分块读路径给 async API；sync 形态用有大小上限的 unary read，处理 encoding/Buffer/callback。 | [fs.js:8](../packages/node-compat/src/fs.js#L8)；[fs.rs:224](../crates/niva/src/app/api/fs.rs#L224)；[initialize_script.js:505](../crates/niva/assets/initialize_script.js#L505) |
| `fs.readdir` | 高 | 部分兼容 | 部分具备 | JS封装 / 低 | WS → 只读IPC候选 | 名字存在，但导出的是 Promise 实现；没有 Node 回调签名/回调调用，调用方必须 await。 Native：返回 immediate entry names；不带 type/encoding/recursive 元数据。 JS：构造 names 或 withFileTypes/Dirent 结果；需要时对每项复用 stat。 | [fs.js:13](../packages/node-compat/src/fs.js#L13)；[fs.rs:145](../crates/niva/src/app/api/fs.rs#L145) |
| `fs.readdirSync` | 高 | 缺失 | 部分具备 | JS封装 / 低 | XHR → 不提供同步 | 当前 fs 入口未导出此方法；沿用既定同步 XHR 方案接入原生文件操作，剩余工作为参数、结果及错误映射。 Native：返回 immediate entry names；不带 type/encoding/recursive 元数据。 JS：构造 names 或 withFileTypes/Dirent 结果；需要时对每项复用 stat。 | [fs.js:8](../packages/node-compat/src/fs.js#L8)；[fs.rs:145](../crates/niva/src/app/api/fs.rs#L145) |
| `fs.stat` | 高 | 部分兼容 | 部分具备 | 扩展Native / 中 | WS → 只读IPC候选 | 名字存在，但导出的是 Promise 实现；没有 Node 回调签名/回调调用，调用方必须 await。 Native：std::fs::metadata 只发 isDir/isFile/isSymlink/size/timestamps；没有 Node Stats 的 dev/mode/ino 等字段或 bigint 形式。 JS：把现有字段映射成 Stats；补 Node 字段前须扩 Native metadata。 | [fs.js:14](../packages/node-compat/src/fs.js#L14)；[fs.rs:27](../crates/niva/src/app/api/fs.rs#L27) |
| `fs.statSync` | 高 | 缺失 | 部分具备 | 扩展Native / 中 | XHR → 不提供同步 | 当前 fs 入口未导出此方法；沿用既定同步 XHR 方案接入原生文件操作，剩余工作为参数、结果及错误映射。需补 Stats 字段。 Native：std::fs::metadata 只发 isDir/isFile/isSymlink/size/timestamps；没有 Node Stats 的 dev/mode/ino 等字段或 bigint 形式。 JS：把现有字段映射成 Stats；补 Node 字段前须扩 Native metadata。 | [fs.js:8](../packages/node-compat/src/fs.js#L8)；[fs.rs:27](../crates/niva/src/app/api/fs.rs#L27) |
| `fs.watch` | 高 | 缺失 | 缺失 | 新增Native / 高 | WS → 不纳入IPC | 没有 FSWatcher 或 watch 入口；单次文件 bridge 无法提供变更事件。 Native：没有文件监视 handler 或 watcher 生命周期。 JS：增加跨平台 watcher/事件/关闭/取消接口并封装 FSWatcher。 依赖：待选型测量；当前 Cargo.toml 未直接声明 watcher crate；先选型并测 release 体积。 | [fs.js:180](../packages/node-compat/src/runtime/fs.js#L180)；[fs.rs:13](../crates/niva/src/app/api/fs.rs#L13) |
| `fs.writeFile` | 高 | 部分兼容 | 部分具备 | 扩展Native / 中 | WS → 不纳入IPC | 名字存在，但导出的是 Promise 实现；没有 Node 回调签名/回调调用，调用方必须 await。 Native：现有 WS readStream/writeStream 已提供核心读写；需为 Node flag/mode 等选项扩展 Native 参数和文件打开方式。 JS：复用已有流读写及 Buffer 编码；补 callback/Promise 契约、AbortSignal 和错误映射。 | [fs.js:10](../packages/node-compat/src/fs.js#L10)；[fs.rs:272](../crates/niva/src/app/api/fs.rs#L272)；[initialize_script.js:516](../crates/niva/assets/initialize_script.js#L516) |
| `fs.writeFileSync` | 高 | 缺失 | 部分具备 | 扩展Native / 中 | XHR → 不提供同步 | 当前 fs 入口未导出此方法；沿用既定同步 XHR 方案接入原生文件操作，剩余工作为参数、结果及错误映射。 Native：已有 WS 流式写/追加，但没有同步 XHR 可用的 unary write/append endpoint。 JS：async API 继续映射 writeStream；sync API 增加有界 unary body 并统一编码/回调/Promise。 | [fs.js:8](../packages/node-compat/src/fs.js#L8)；[fs.rs:272](../crates/niva/src/app/api/fs.rs#L272)；[initialize_script.js:516](../crates/niva/assets/initialize_script.js#L516) |
| `fs/promises.mkdir` | 高 | 部分兼容 | 部分具备 | JS封装 / 低 | WS → 不纳入IPC | 支持 recursive；mode 被明确拒绝，recursive 创建时也只返回桥接结果而非 Node 的首个创建路径。 Native：Native 有 createDir/createDirAll，但不接收权限 mode。 JS：选择 createDir 或 createDirAll；补回调/Promise/同步入口与 Node recursive 返回约定。 | [fs.js:185](../packages/node-compat/src/runtime/fs.js#L185)；[fs.rs:125](../crates/niva/src/app/api/fs.rs#L125)；[fs.rs:13](../crates/niva/src/app/api/fs.rs#L13) |
| `fs/promises.readFile` | 高 | 部分兼容 | 部分具备 | 扩展Native / 中 | WS → 不纳入IPC | 返回 Buffer/string 与常用 encoding 可用；flag、signal 等选项没有映射。 Native：现有 WS readStream/writeStream 已提供核心读写；需为 Node flag/mode 等选项扩展 Native 参数和文件打开方式。 JS：复用已有流读写及 Buffer 编码；补 callback/Promise 契约、AbortSignal 和错误映射。 | [fs.js:144](../packages/node-compat/src/runtime/fs.js#L144)；[fs.rs:224](../crates/niva/src/app/api/fs.rs#L224)；[initialize_script.js:505](../crates/niva/assets/initialize_script.js#L505) |
| `fs/promises.readdir` | 高 | 部分兼容 | 部分具备 | JS封装 / 低 | WS → 只读IPC候选 | 支持名称与 withFileTypes；Dirent 是近似对象，encoding、buffer、recursive 等选项缺失。 Native：返回 immediate entry names；不带 type/encoding/recursive 元数据。 JS：构造 names 或 withFileTypes/Dirent 结果；需要时对每项复用 stat。 | [fs.js:190](../packages/node-compat/src/runtime/fs.js#L190)；[fs.rs:145](../crates/niva/src/app/api/fs.rs#L145) |
| `fs/promises.stat` | 高 | 部分兼容 | 部分具备 | 扩展Native / 中 | WS → 只读IPC候选 | Stats 只含 size、时间、路径和类型判断；bigint 明确 ENOTSUP，dev/mode/ino 等字段缺失。 Native：std::fs::metadata 只发 isDir/isFile/isSymlink/size/timestamps；没有 Node Stats 的 dev/mode/ino 等字段或 bigint 形式。 JS：把现有字段映射成 Stats；补 Node 字段前须扩 Native metadata。 | [fs.js:93](../packages/node-compat/src/runtime/fs.js#L93)；[fs.rs:27](../crates/niva/src/app/api/fs.rs#L27) |
| `fs/promises.writeFile` | 高 | 部分兼容 | 部分具备 | 扩展Native / 中 | WS → 不纳入IPC | Promise 写入可用；mode 会 ENOTSUP，flag、signal、flush 等未实现。 Native：现有 WS readStream/writeStream 已提供核心读写；需为 Node flag/mode 等选项扩展 Native 参数和文件打开方式。 JS：复用已有流读写及 Buffer 编码；补 callback/Promise 契约、AbortSignal 和错误映射。 | [fs.js:162](../packages/node-compat/src/runtime/fs.js#L162)；[fs.rs:272](../crates/niva/src/app/api/fs.rs#L272)；[initialize_script.js:516](../crates/niva/assets/initialize_script.js#L516) |
| `fs.access` | 中 | 部分兼容 | 部分具备 | 扩展Native / 中 | WS → 只读IPC候选 | 名字存在，但导出的是 Promise 实现；没有 Node 回调签名/回调调用，调用方必须 await。 Native：只有存在性查询；没有 R_OK/W_OK/X_OK 权限检查。 JS：将 mode=F_OK 映射 exists；其余 mode 需要 Native 权限检查。 | [fs.js:15](../packages/node-compat/src/fs.js#L15)；[fs.rs:45](../crates/niva/src/app/api/fs.rs#L45)；[fs.rs:13](../crates/niva/src/app/api/fs.rs#L13) |
| `fs.appendFile` | 中 | 部分兼容 | 部分具备 | 扩展Native / 中 | WS → 不纳入IPC | 名字存在，但导出的是 Promise 实现；没有 Node 回调签名/回调调用，调用方必须 await。 Native：现有 WS readStream/writeStream 已提供核心读写；需为 Node flag/mode 等选项扩展 Native 参数和文件打开方式。 JS：复用已有流读写及 Buffer 编码；补 callback/Promise 契约、AbortSignal 和错误映射。 | [fs.js:11](../packages/node-compat/src/fs.js#L11)；[fs.rs:272](../crates/niva/src/app/api/fs.rs#L272)；[initialize_script.js:516](../crates/niva/assets/initialize_script.js#L516) |
| `fs.appendFileSync` | 中 | 缺失 | 部分具备 | 扩展Native / 中 | XHR → 不提供同步 | 当前 fs 入口未导出此方法；沿用既定同步 XHR 方案接入原生文件操作，剩余工作为参数、结果及错误映射。 Native：已有 WS 流式写/追加，但没有同步 XHR 可用的 unary write/append endpoint。 JS：async API 继续映射 writeStream；sync API 增加有界 unary body 并统一编码/回调/Promise。 | [fs.js:8](../packages/node-compat/src/fs.js#L8)；[fs.rs:272](../crates/niva/src/app/api/fs.rs#L272)；[initialize_script.js:516](../crates/niva/assets/initialize_script.js#L516) |
| `fs.copyFile` | 中 | 部分兼容 | 部分具备 | JS封装 / 低 | WS → 不纳入IPC | 名字存在，但导出的是 Promise 实现；没有 Node 回调签名/回调调用，调用方必须 await。 Native：copy 可复制文件/目录并接受 Niva CopyOptions；未提供 Node copyFile clone flags 等完整语义。 JS：对源类型与 COPYFILE_EXCL 做 Node 参数/错误包装。 | [fs.js:19](../packages/node-compat/src/fs.js#L19)；[fs.rs:97](../crates/niva/src/app/api/fs.rs#L97) |
| `fs.copyFileSync` | 中 | 缺失 | 部分具备 | JS封装 / 低 | XHR → 不提供同步 | 当前 fs 入口未导出此方法；沿用既定同步 XHR 方案接入原生文件操作，剩余工作为参数、结果及错误映射。 Native：copy 可复制文件/目录并接受 Niva CopyOptions；未提供 Node copyFile clone flags 等完整语义。 JS：对源类型与 COPYFILE_EXCL 做 Node 参数/错误包装。 | [fs.js:8](../packages/node-compat/src/fs.js#L8)；[fs.rs:97](../crates/niva/src/app/api/fs.rs#L97) |
| `fs.realpathSync` | 中 | 缺失 | 缺失 | 扩展Native / 中 | XHR → 不提供同步 | 当前 fs 入口未导出此方法；沿用既定同步 XHR 方案接入原生文件操作，剩余工作为参数、结果及错误映射。需补真实路径解析。 Native：未注册 realpath Native API；现有 fs handlers 中没有 canonicalize 结果。 JS：增加受控 canonical path 查询；同时保证解析结果遵守既有文件路径权限边界。 | [fs.js:8](../packages/node-compat/src/fs.js#L8)；[fs.rs:13](../crates/niva/src/app/api/fs.rs#L13) |
| `fs.rename` | 中 | 部分兼容 | 部分具备 | JS语义 / 中 | WS → 不纳入IPC | 名字存在，但导出的是 Promise 实现；没有 Node 回调签名/回调调用，调用方必须 await。 Native：move 存在，但 CopyOptions 的 contentOnly/overwrite 可以造成目录合并等 rename 不同语义。 JS：将目标存在和文件/目录边界映射成 Node rename 错误；调用 move 时限制选项。 | [fs.js:16](../packages/node-compat/src/fs.js#L16)；[fs.rs:79](../crates/niva/src/app/api/fs.rs#L79)；[fs.rs:54](../crates/niva/src/app/api/fs.rs#L54) |
| `fs.renameSync` | 中 | 缺失 | 部分具备 | JS语义 / 中 | XHR → 不提供同步 | 当前 fs 入口未导出此方法；沿用既定同步 XHR 方案接入原生文件操作，剩余工作为参数、结果及错误映射。需对齐 rename 语义，不能直接把目录合并当重命名。 Native：move 存在，但 CopyOptions 的 contentOnly/overwrite 可以造成目录合并等 rename 不同语义。 JS：将目标存在和文件/目录边界映射成 Node rename 错误；调用 move 时限制选项。 | [fs.js:8](../packages/node-compat/src/fs.js#L8)；[fs.rs:79](../crates/niva/src/app/api/fs.rs#L79)；[fs.rs:54](../crates/niva/src/app/api/fs.rs#L54) |
| `fs.rm` | 中 | 部分兼容 | 部分具备 | JS语义 / 中 | WS → 不纳入IPC | 名字存在，但导出的是 Promise 实现；没有 Node 回调签名/回调调用，调用方必须 await。 Native：remove_path 是统一删除原语；Node force/recursive/maxRetries/retryDelay 的选项语义没有 Native 参数。 JS：在 JS 层做存在/类型检查、force/recursive 与错误映射；若需重试再扩 Native。 | [fs.js:17](../packages/node-compat/src/fs.js#L17)；[fs.rs:116](../crates/niva/src/app/api/fs.rs#L116) |
| `fs.rmSync` | 中 | 缺失 | 部分具备 | JS语义 / 中 | XHR → 不提供同步 | 当前 fs 入口未导出此方法；沿用既定同步 XHR 方案接入原生文件操作，剩余工作为参数、结果及错误映射。 Native：remove_path 是统一删除原语；Node force/recursive/maxRetries/retryDelay 的选项语义没有 Native 参数。 JS：在 JS 层做存在/类型检查、force/recursive 与错误映射；若需重试再扩 Native。 | [fs.js:8](../packages/node-compat/src/fs.js#L8)；[fs.rs:116](../crates/niva/src/app/api/fs.rs#L116) |
| `fs.unlinkSync` | 中 | 缺失 | 部分具备 | JS语义 / 低 | XHR → 不提供同步 | 当前 fs 入口未导出此方法；沿用既定同步 XHR 方案接入原生文件操作，剩余工作为参数、结果及错误映射。 Native：remove_path 可删文件，也可递归删除目录；没有 file-only unlink 错误边界。 JS：映射文件删除并在目录输入时返回 Node unlink 错误。 | [fs.js:8](../packages/node-compat/src/fs.js#L8)；[fs.rs:116](../crates/niva/src/app/api/fs.rs#L116) |
| `fs/promises.access` | 中 | 部分兼容 | 部分具备 | 扩展Native / 中 | WS → 只读IPC候选 | 只验证路径存在；R_OK/W_OK/X_OK 等非零 mode 会 ENOTSUP，不能验证权限。 Native：只有存在性查询；没有 R_OK/W_OK/X_OK 权限检查。 JS：将 mode=F_OK 映射 exists；其余 mode 需要 Native 权限检查。 | [fs.js:202](../packages/node-compat/src/runtime/fs.js#L202)；[fs.rs:45](../crates/niva/src/app/api/fs.rs#L45)；[fs.rs:13](../crates/niva/src/app/api/fs.rs#L13) |
| `fs/promises.appendFile` | 中 | 部分兼容 | 部分具备 | 扩展Native / 中 | WS → 不纳入IPC | Promise 追加可用；mode 会 ENOTSUP，flag、signal 等选项未实现。 Native：现有 WS readStream/writeStream 已提供核心读写；需为 Node flag/mode 等选项扩展 Native 参数和文件打开方式。 JS：复用已有流读写及 Buffer 编码；补 callback/Promise 契约、AbortSignal 和错误映射。 | [fs.js:162](../packages/node-compat/src/runtime/fs.js#L162)；[fs.rs:272](../crates/niva/src/app/api/fs.rs#L272)；[initialize_script.js:516](../crates/niva/assets/initialize_script.js#L516) |
| `fs/promises.copyFile` | 中 | 部分兼容 | 部分具备 | JS封装 / 低 | WS → 不纳入IPC | 基本复制和 COPYFILE_EXCL 可用；克隆标志明确 ENOTSUP，mode 其余语义未覆盖。 Native：copy 可复制文件/目录并接受 Niva CopyOptions；未提供 Node copyFile clone flags 等完整语义。 JS：对源类型与 COPYFILE_EXCL 做 Node 参数/错误包装。 | [fs.js:243](../packages/node-compat/src/runtime/fs.js#L243)；[fs.rs:97](../crates/niva/src/app/api/fs.rs#L97) |
| `fs/promises.cp` | 中 | 部分兼容 | 部分具备 | JS封装 / 中 | WS → 不纳入IPC | 支持基础文件/recursive 目录复制；多项常用 Node 选项明确拒绝或不生效，且仅依赖 stat 区分类型。 Native：copy 能复制文件/目录；option 命名与 Node cp 不同，过滤器/时间戳/符号链接等未全部具备。 JS：映射 recursive/force 等 common options，拒绝或补齐其余选项。 | [fs.js:227](../packages/node-compat/src/runtime/fs.js#L227)；[fs.rs:97](../crates/niva/src/app/api/fs.rs#L97) |
| `fs/promises.lstat` | 中 | 缺失 | 缺失 | 扩展Native / 中 | WS → 只读IPC候选 | 没有 NodeCompat lstat 入口；底层 stat 使用跟随符号链接的 metadata，不能作为 lstat 替代。 Native：stat 使用跟随符号链接的 metadata；没有 lstat handler。 JS：注册 symlink_metadata 结果并映射 Node Stats。 | [fs.js:180](../packages/node-compat/src/runtime/fs.js#L180)；[fs.rs:27](../crates/niva/src/app/api/fs.rs#L27) |
| `fs/promises.open` | 中 | 缺失 | 部分具备 | 新增Native / 高 | WS → 不纳入IPC | 没有 FileHandle/open 入口或可供 read/write/stat/close 的句柄对象。 Native：File::open/OpenOptions 只存在于单次流 handler 内；没有跨调用 FileHandle ID/close/read/write 状态。 JS：设计稳定句柄标识与 FileHandle 方法/close 生命周期。 | [fs.js:180](../packages/node-compat/src/runtime/fs.js#L180)；[fs.rs:224](../crates/niva/src/app/api/fs.rs#L224)；[fs.rs:272](../crates/niva/src/app/api/fs.rs#L272) |
| `fs/promises.realpath` | 中 | 缺失 | 缺失 | 扩展Native / 中 | WS → 只读IPC候选 | promises 对象没有 realpath；没有规范化/真实路径 API。 Native：未注册 realpath Native API；现有 fs handlers 中没有 canonicalize 结果。 JS：增加受控 canonical path 查询；同时保证解析结果遵守既有文件路径权限边界。 | [fs.js:180](../packages/node-compat/src/runtime/fs.js#L180)；[fs.rs:13](../crates/niva/src/app/api/fs.rs#L13) |
| `fs/promises.rename` | 中 | 部分兼容 | 部分具备 | JS语义 / 中 | WS → 不纳入IPC | 映射到 move 且强制 contentOnly/overwrite；目录目标已存在时可能合并内容，语义不同于 rename。 Native：move 存在，但 CopyOptions 的 contentOnly/overwrite 可以造成目录合并等 rename 不同语义。 JS：将目标存在和文件/目录边界映射成 Node rename 错误；调用 move 时限制选项。 | [fs.js:208](../packages/node-compat/src/runtime/fs.js#L208)；[fs.rs:79](../crates/niva/src/app/api/fs.rs#L79)；[fs.rs:54](../crates/niva/src/app/api/fs.rs#L54) |
| `fs/promises.rm` | 中 | 部分兼容 | 部分具备 | JS语义 / 中 | WS → 不纳入IPC | 支持基本 force/recursive；缺少 maxRetries/retryDelay，stat 跟随链接也会影响目录分支判断。 Native：remove_path 是统一删除原语；Node force/recursive/maxRetries/retryDelay 的选项语义没有 Native 参数。 JS：在 JS 层做存在/类型检查、force/recursive 与错误映射；若需重试再扩 Native。 | [fs.js:211](../packages/node-compat/src/runtime/fs.js#L211)；[fs.rs:116](../crates/niva/src/app/api/fs.rs#L116) |
| `fs/promises.unlink` | 中 | 缺失 | 部分具备 | JS语义 / 低 | WS → 不纳入IPC | promises 对象没有 unlink；底层 remove 能力不等于 NodeCompat 的 unlink 入口。 Native：remove_path 可删文件，也可递归删除目录；没有 file-only unlink 错误边界。 JS：映射文件删除并在目录输入时返回 Node unlink 错误。 | [fs.js:180](../packages/node-compat/src/runtime/fs.js#L180)；[fs.rs:116](../crates/niva/src/app/api/fs.rs#L116) |

<a id="module-path"></a>

### path

Node 参考：[官方 path 文档](https://nodejs.org/api/path.html)。

| API | 频率 | 当前状态 | Native 能力 | 工作层 / 难度 | 本地 → 跨域目标 | 差距与依赖依据 | 源码 |
|---|---|---|---|---|---|---|---|
| `path.basename` | 高 | 基础支持 | 不需要 | JS语义 / 已具备 | JS → 纯JS目标 | 常见字符串路径调用及返回形状已实现，并有 POSIX/Windows 对照用例。 | [path.js:247](../packages/node-compat/src/runtime/path.js#L247) |
| `path.delimiter` | 高 | 基础支持 | 不需要 | JS语义 / 已具备 | JS → 纯JS目标 | 按默认平台路径模块提供 PATH 列表分隔符。 | [path.js:26](../packages/node-compat/src/runtime/path.js#L26) |
| `path.dirname` | 高 | 基础支持 | 不需要 | JS语义 / 已具备 | JS → 纯JS目标 | 常见字符串路径调用及返回形状已实现，并有 POSIX/Windows 对照用例。 | [path.js:262](../packages/node-compat/src/runtime/path.js#L262) |
| `path.extname` | 高 | 基础支持 | 不需要 | JS语义 / 已具备 | JS → 纯JS目标 | 常见字符串路径调用及返回形状已实现，并有 POSIX/Windows 对照用例。 | [path.js:302](../packages/node-compat/src/runtime/path.js#L302) |
| `path.format` | 高 | 基础支持 | 不需要 | JS语义 / 已具备 | JS → 纯JS目标 | 常见字符串路径调用及返回形状已实现，并有 POSIX/Windows 对照用例。 | [path.js:337](../packages/node-compat/src/runtime/path.js#L337) |
| `path.isAbsolute` | 高 | 基础支持 | 不需要 | JS语义 / 已具备 | JS → 纯JS目标 | 常见字符串路径调用及返回形状已实现，并有 POSIX/Windows 对照用例。 | [path.js:219](../packages/node-compat/src/runtime/path.js#L219) |
| `path.join` | 高 | 基础支持 | 不需要 | JS语义 / 已具备 | JS → 纯JS目标 | 常见字符串路径调用及返回形状已实现，并有 POSIX/Windows 对照用例。 | [path.js:225](../packages/node-compat/src/runtime/path.js#L225) |
| `path.normalize` | 高 | 基础支持 | 不需要 | JS语义 / 已具备 | JS → 纯JS目标 | 常见字符串路径调用及返回形状已实现，并有 POSIX/Windows 对照用例。 | [path.js:124](../packages/node-compat/src/runtime/path.js#L124) |
| `path.parse` | 高 | 基础支持 | 不需要 | JS语义 / 已具备 | JS → 纯JS目标 | 常见字符串路径调用及返回形状已实现，并有 POSIX/Windows 对照用例。 | [path.js:310](../packages/node-compat/src/runtime/path.js#L310) |
| `path.posix` | 高 | 基础支持 | 不需要 | JS语义 / 已具备 | JS + 动态cwd XHR → 纯JS目标 | 独立构造并导出 POSIX 路径对象。 目标接线：涉及相对路径解析时内部 XHR 读取当前 cwd，再执行 JS 路径运算。 | [path.js:433](../packages/node-compat/src/runtime/path.js#L433) |
| `path.relative` | 高 | 基础支持 | 不需要 | JS语义 / 已具备 | JS + 动态cwd XHR → 纯JS目标 | 常见字符串路径调用及返回形状已实现，并有 POSIX/Windows 对照用例。 目标接线：涉及相对路径解析时内部 XHR 读取当前 cwd，再执行 JS 路径运算。 | [path.js:352](../packages/node-compat/src/runtime/path.js#L352) |
| `path.resolve` | 高 | 基础支持 | 部分具备 | JS封装 / 已具备 | JS + 动态cwd XHR → 纯JS目标 | 同步路径解析已实现；依赖 registerNodeCompat 完成 cwd 初始化（或显式 setCwd），未初始化时默认 /。 目标接线：涉及相对路径解析时内部 XHR 读取当前 cwd，再执行 JS 路径运算。 | [path.js:153](../packages/node-compat/src/runtime/path.js#L153)；[registration.js:82](../packages/node-compat/src/runtime/registration.js#L82)；[process.rs:14](../crates/niva/src/app/api/process.rs#L14) |
| `path.sep` | 高 | 基础支持 | 不需要 | JS语义 / 已具备 | JS → 纯JS目标 | 按默认平台路径模块提供分隔符常量。 | [path.js:25](../packages/node-compat/src/runtime/path.js#L25) |
| `path.win32` | 高 | 基础支持 | 不需要 | JS语义 / 已具备 | JS + 动态cwd XHR → 纯JS目标 | 独立构造并导出 Windows 路径对象，含盘符和 UNC 常见形态。 目标接线：涉及相对路径解析时内部 XHR 读取当前 cwd，再执行 JS 路径运算。 | [path.js:434](../packages/node-compat/src/runtime/path.js#L434) |
| `path.matchesGlob` | 中 | 部分兼容 | 不需要 | JS语义 / 中 | JS → 纯JS目标 | 实现星号、双星号、问号和基础字符类；不是完整 glob 语法，复杂模式语义会有差异。 Native：路径规范化、平台变体和当前目录状态均可由现有 JS 实现；resolve 的 cwd 种子已复用 process.currentDir。 JS：补齐 Node glob grammar/平台路径边界；不需要 Native。 | [path.js:385](../packages/node-compat/src/runtime/path.js#L385) |
| `path.toNamespacedPath` | 中 | 基础支持 | 不需要 | JS语义 / 已具备 | JS → 纯JS目标 | POSIX 原样返回，Windows 路径会解析并生成设备命名空间前缀。 | [path.js:374](../packages/node-compat/src/runtime/path.js#L374) |

<a id="module-events"></a>

### events

Node 参考：[官方 events 文档](https://nodejs.org/api/events.html)。

| API | 频率 | 当前状态 | Native 能力 | 工作层 / 难度 | 本地 → 跨域目标 | 差距与依赖依据 | 源码 |
|---|---|---|---|---|---|---|---|
| `events.EventEmitter` | 高 | 部分兼容 | 不需要 | JS语义 / 中 | JS → 纯JS目标 | 普通构造与事件发射可用；captureRejections 选项明确拒绝。 Native：事件分发不需要原生能力。 JS：支持或按 Node 规则实现 captureRejections 构造选项。 | [events.js:7](../packages/node-compat/src/runtime/events.js#L7) |
| `events.EventEmitter.emit` | 高 | 基础支持 | 不需要 | JS语义 / 已具备 | JS → 纯JS目标 | 常见监听、发射、移除或查询返回语义均有实现和针对性用例。 | [events.js:37](../packages/node-compat/src/runtime/events.js#L37) |
| `events.EventEmitter.off` | 高 | 基础支持 | 不需要 | JS语义 / 已具备 | JS → 纯JS目标 | 常见监听、发射、移除或查询返回语义均有实现和针对性用例。 | [events.js:77](../packages/node-compat/src/runtime/events.js#L77) |
| `events.EventEmitter.on` | 高 | 基础支持 | 不需要 | JS语义 / 已具备 | JS → 纯JS目标 | 常见监听、发射、移除或查询返回语义均有实现和针对性用例。 | [events.js:56](../packages/node-compat/src/runtime/events.js#L56) |
| `events.EventEmitter.once` | 高 | 基础支持 | 不需要 | JS语义 / 已具备 | JS → 纯JS目标 | 常见监听、发射、移除或查询返回语义均有实现和针对性用例。 | [events.js:59](../packages/node-compat/src/runtime/events.js#L59) |
| `events.EventEmitter.removeAllListeners` | 高 | 基础支持 | 不需要 | JS语义 / 已具备 | JS → 纯JS目标 | 常见监听、发射、移除或查询返回语义均有实现和针对性用例。 | [events.js:79](../packages/node-compat/src/runtime/events.js#L79) |
| `events.EventEmitter.removeListener` | 高 | 基础支持 | 不需要 | JS语义 / 已具备 | JS → 纯JS目标 | 常见监听、发射、移除或查询返回语义均有实现和针对性用例。 | [events.js:62](../packages/node-compat/src/runtime/events.js#L62) |
| `events.EventEmitter.listenerCount` | 中 | 基础支持 | 不需要 | JS语义 / 已具备 | JS → 纯JS目标 | 常见监听、发射、移除或查询返回语义均有实现和针对性用例。 | [events.js:105](../packages/node-compat/src/runtime/events.js#L105) |
| `events.EventEmitter.listeners` | 中 | 基础支持 | 不需要 | JS语义 / 已具备 | JS → 纯JS目标 | 常见监听、发射、移除或查询返回语义均有实现和针对性用例。 | [events.js:97](../packages/node-compat/src/runtime/events.js#L97) |
| `events.on` | 中 | 缺失 | 不需要 | JS语义 / 中 | JS → 纯JS目标 | 模块对象没有顶层 on 异步迭代器 helper。 Native：事件分发不需要原生能力。 JS：实现 EventEmitter 转 AsyncIterator、AbortSignal 和 return() 清理。 | [events.js:147](../packages/node-compat/src/runtime/events.js#L147) |
| `events.once` | 中 | 缺失 | 不需要 | JS语义 / 低 | JS → 纯JS目标 | 模块对象只导出 EventEmitter、errorMonitor 和 defaultMaxListeners；没有顶层 once Promise helper。 Native：事件分发不需要原生能力。 JS：实现一次性 Promise 监听及 error 事件竞速处理。 | [events.js:147](../packages/node-compat/src/runtime/events.js#L147) |

<a id="module-process"></a>

### process

Node 参考：[官方 process 文档](https://nodejs.org/api/process.html)。

| API | 频率 | 当前状态 | Native 能力 | 工作层 / 难度 | 本地 → 跨域目标 | 差距与依赖依据 | 源码 |
|---|---|---|---|---|---|---|---|
| `process.arch` | 高 | 缺失 | 具备 | JS封装 / 低 | 启动注入 → JS读取（main） → 不注入 process | NodeCompat 没有注册 process 模块或安装 global process；Niva.api.process 的同名原生能力不能计作 Node 入口。 Native：应用启动时收集现有进程/运行时信息并定向序列化到 main；不为每次读取调用 XHR。 JS：以同步属性暴露启动快照；env/argv 的页面内修改与宿主原始参数分开。 | [registration.js:6](../packages/node-compat/src/runtime/registration.js#L6)；[os.rs:13](../crates/niva/src/app/api/os.rs#L13)；[os.rs:21](../crates/niva/src/app/api/os.rs#L21) |
| `process.argv` | 高 | 缺失 | 部分具备 | JS封装 / 低 | 启动注入 → JS读取（main） → 不注入 process | NodeCompat 没有注册 process 模块或安装 global process；Niva.api.process 的同名原生能力不能计作 Node 入口。 Native：应用启动时收集现有进程/运行时信息并定向序列化到 main；不为每次读取调用 XHR。 JS：同步暴露启动参数。 | [registration.js:6](../packages/node-compat/src/runtime/registration.js#L6)；[process.rs:12](../crates/niva/src/app/api/process.rs#L12)；[process.rs:69](../crates/niva/src/app/api/process.rs#L69) |
| `process.cwd` | 高 | 缺失 | 具备 | JS封装 / 低 | XHR实时查询（main） → 不注入 process | NodeCompat 没有注册 process 模块或安装 global process；Niva.api.process 的同名原生能力不能计作 Node 入口。 Native：复用已有 process.currentDir，每次通过既定同步 XHR 返回当前目录。 JS：cwd() 调用同步 XHR 并直接返回结果。 | [registration.js:6](../packages/node-compat/src/runtime/registration.js#L6)；[process.rs:12](../crates/niva/src/app/api/process.rs#L12)；[process.rs:33](../crates/niva/src/app/api/process.rs#L33) |
| `process.env` | 高 | 缺失 | 部分具备 | JS封装 / 低 | 启动注入 → JS读取（main） → 不注入 process | NodeCompat 没有注册 process 模块或安装 global process；Niva.api.process 的同名原生能力不能计作 Node 入口。 Native：应用启动时收集现有进程/运行时信息并定向序列化到 main；不为每次读取调用 XHR。 JS：同步暴露启动时收集的环境配置数据，不增加动态覆盖层。 | [registration.js:6](../packages/node-compat/src/runtime/registration.js#L6)；[process.rs:12](../crates/niva/src/app/api/process.rs#L12)；[process.rs:57](../crates/niva/src/app/api/process.rs#L57) |
| `process.exit` | 高 | 缺失 | 部分具备 | 扩展Native / 中 | XHR → 不注入 process | NodeCompat 没有注册 process 模块或安装 global process；Niva.api.process 的同名原生能力不能计作 Node 入口。 Native：Native 可退出应用事件循环，但 handler 没有读取 Node exitCode 参数。 JS：传递 exit code 并定义异步回调/exit event 时序。 | [registration.js:6](../packages/node-compat/src/runtime/registration.js#L6)；[process.rs:12](../crates/niva/src/app/api/process.rs#L12)；[process.rs:91](../crates/niva/src/app/api/process.rs#L91) |
| `process.nextTick` | 高 | 缺失 | 不需要 | JS语义 / 中 | JS/宿主调度 → 不注入 process | NodeCompat 没有注册 process 模块或安装 global process；Niva.api.process 的同名原生能力不能计作 Node 入口。 Native：无；可在 JS runtime 实现微任务队列。 JS：实现并验证 nextTick 与 Promise/queueMicrotask 的队列顺序；不能仅把函数名指向 queueMicrotask。 | [registration.js:6](../packages/node-compat/src/runtime/registration.js#L6)；[registration.js:55](../packages/node-compat/src/runtime/registration.js#L55) |
| `process.on` | 高 | 缺失 | 部分具备 | JS语义 / 中 | WS+宿主能力 → 不注入 process | NodeCompat 没有注册 process 模块或安装 global process；Niva.api.process 的同名原生能力不能计作 Node 入口。 Native：Niva 有 JS event emitter 与窗口事件，但没有 Node process lifecycle event source。 JS：限制并映射可提供的事件；不能把一般 Niva.on 事件冒充 Node process events。 | [registration.js:6](../packages/node-compat/src/runtime/registration.js#L6)；[process.rs:12](../crates/niva/src/app/api/process.rs#L12)；[initialize_script.js:10](../crates/niva/assets/initialize_script.js#L10) |
| `process.platform` | 高 | 缺失 | 具备 | JS封装 / 低 | 启动注入 → JS读取（main） → 不注入 process | NodeCompat 没有注册 process 模块或安装 global process；Niva.api.process 的同名原生能力不能计作 Node 入口。 Native：应用启动时收集现有进程/运行时信息并定向序列化到 main；不为每次读取调用 XHR。 JS：以同步属性暴露启动快照；env/argv 的页面内修改与宿主原始参数分开。 | [registration.js:6](../packages/node-compat/src/runtime/registration.js#L6)；[os.rs:13](../crates/niva/src/app/api/os.rs#L13)；[os.rs:21](../crates/niva/src/app/api/os.rs#L21) |
| `process.stderr.write` | 高 | 缺失 | 部分具备 | 扩展Native / 高 | WS+宿主能力 → 不注入 process | NodeCompat 没有注册 process 模块或安装 global process；Niva.api.process 的同名原生能力不能计作 Node 入口。 Native：--stdio 模式有 NDJSON stdin/stdout 和 host.send；它是消息协议而非任意字节 Node stdio streams。 JS：将受限 host message 包装成可用流或新增 raw stdio channel，并保持现有 NDJSON framing。 | [registration.js:6](../packages/node-compat/src/runtime/registration.js#L6)；[stdio.rs:18](../crates/niva/src/app/stdio.rs#L18)；[host.rs:12](../crates/niva/src/app/api/host.rs#L12) |
| `process.stdout.write` | 高 | 缺失 | 部分具备 | 扩展Native / 高 | WS+宿主能力 → 不注入 process | NodeCompat 没有注册 process 模块或安装 global process；Niva.api.process 的同名原生能力不能计作 Node 入口。 Native：--stdio 模式有 NDJSON stdin/stdout 和 host.send；它是消息协议而非任意字节 Node stdio streams。 JS：将受限 host message 包装成可用流或新增 raw stdio channel，并保持现有 NDJSON framing。 | [registration.js:6](../packages/node-compat/src/runtime/registration.js#L6)；[stdio.rs:18](../crates/niva/src/app/stdio.rs#L18)；[host.rs:12](../crates/niva/src/app/api/host.rs#L12) |
| `process.version` | 高 | 缺失 | 部分具备 | JS封装 / 中 | 启动注入 → JS读取（main） → 不注入 process | 没有 Node process 入口；Niva 本身不是 Node 运行时，不能把 Niva 应用版本伪装成 Node/依赖库版本。 Native：收集真实 Niva/WebView 运行时元数据；不凭空生成 Node/V8 版本。 JS：启动时暴露真实版本字段并说明与 Node 运行时的差异。 | [registration.js:6](../packages/node-compat/src/runtime/registration.js#L6)；[process.rs:12](../crates/niva/src/app/api/process.rs#L12)；[process.rs:138](../crates/niva/src/app/api/process.rs#L138) |
| `process.argv0` | 中 | 缺失 | 部分具备 | JS封装 / 低 | 启动注入 → JS读取（main） → 不注入 process | NodeCompat 没有注册 process 模块或安装 global process；Niva.api.process 的同名原生能力不能计作 Node 入口。 Native：应用启动时收集现有进程/运行时信息并定向序列化到 main；不为每次读取调用 XHR。 JS：以同步属性暴露启动快照；env/argv 的页面内修改与宿主原始参数分开。 | [registration.js:6](../packages/node-compat/src/runtime/registration.js#L6)；[process.rs:12](../crates/niva/src/app/api/process.rs#L12)；[process.rs:45](../crates/niva/src/app/api/process.rs#L45) |
| `process.chdir` | 中 | 缺失 | 具备 | JS封装 / 中 | XHR → 不注入 process | NodeCompat 没有注册 process 模块或安装 global process；Niva.api.process 的同名原生能力不能计作 Node 入口。 Native：Native setCurrentDir 调用 std::env::set_current_dir。 JS：通过同步 XHR 执行目录变更；后续 cwd 查询直接读 Native 当前值。 | [registration.js:6](../packages/node-compat/src/runtime/registration.js#L6)；[process.rs:12](../crates/niva/src/app/api/process.rs#L12)；[process.rs:77](../crates/niva/src/app/api/process.rs#L77) |
| `process.execPath` | 中 | 缺失 | 具备 | JS封装 / 低 | 启动注入 → JS读取（main） → 不注入 process | NodeCompat 没有注册 process 模块或安装 global process；Niva.api.process 的同名原生能力不能计作 Node 入口。 Native：应用启动时收集现有进程/运行时信息并定向序列化到 main；不为每次读取调用 XHR。 JS：以同步属性暴露启动快照；env/argv 的页面内修改与宿主原始参数分开。 | [registration.js:6](../packages/node-compat/src/runtime/registration.js#L6)；[process.rs:12](../crates/niva/src/app/api/process.rs#L12)；[process.rs:45](../crates/niva/src/app/api/process.rs#L45) |
| `process.pid` | 中 | 缺失 | 具备 | JS封装 / 低 | 启动注入 → JS读取（main） → 不注入 process | NodeCompat 没有注册 process 模块或安装 global process；Niva.api.process 的同名原生能力不能计作 Node 入口。 Native：应用启动时收集现有进程/运行时信息并定向序列化到 main；不为每次读取调用 XHR。 JS：以同步属性暴露启动快照；env/argv 的页面内修改与宿主原始参数分开。 | [registration.js:6](../packages/node-compat/src/runtime/registration.js#L6)；[process.rs:12](../crates/niva/src/app/api/process.rs#L12)；[process.rs:25](../crates/niva/src/app/api/process.rs#L25) |
| `process.stdin` | 中 | 缺失 | 部分具备 | 扩展Native / 高 | WS+宿主能力 → 不注入 process | NodeCompat 没有注册 process 模块或安装 global process；Niva.api.process 的同名原生能力不能计作 Node 入口。 Native：--stdio 模式有 NDJSON stdin/stdout 和 host.send；它是消息协议而非任意字节 Node stdio streams。 JS：将受限 host message 包装成可用流或新增 raw stdio channel，并保持现有 NDJSON framing。 | [registration.js:6](../packages/node-compat/src/runtime/registration.js#L6)；[stdio.rs:18](../crates/niva/src/app/stdio.rs#L18)；[host.rs:12](../crates/niva/src/app/api/host.rs#L12) |
| `process.versions` | 中 | 缺失 | 缺失 | JS封装 / 中 | 启动注入 → JS读取（main） → 不注入 process | 没有 Node process 入口；Niva 本身不是 Node 运行时，不能把 Niva 应用版本伪装成 Node/依赖库版本。 Native：收集真实 Niva/WebView 运行时元数据；不凭空生成 Node/V8 版本。 JS：启动时暴露真实版本字段并说明与 Node 运行时的差异。 | [registration.js:6](../packages/node-compat/src/runtime/registration.js#L6)；[process.rs:12](../crates/niva/src/app/api/process.rs#L12) |

<a id="module-http"></a>

### http

Node 参考：[官方 http 文档](https://nodejs.org/api/http.html)。

| API | 频率 | 当前状态 | Native 能力 | 工作层 / 难度 | 本地 → 跨域目标 | 差距与依赖依据 | 源码 |
|---|---|---|---|---|---|---|---|
| `http.Server.listen` | 高 | 缺失 | 不需要 | JS语义 / 中 | JS HTTP/1.1 → Native TCP → 不提供原生socket；浏览器fetch另计 | 没有对应用开放的 Node Server；Niva 内部 HTTP 服务不能替代。 目标：协议层不单独新增 Native HTTP/HTTPS；复用 net 的 TCP 客户端/监听以及 tls 的客户端/服务端握手与字节流。 JS：用纯 JS HTTP/1.1 parser 加报文序列化，封装 ClientRequest/IncomingMessage/Server/Agent、连接复用、超时及背压；HTTPS 复用同一协议实现，仅底层流换成 TLS。 | [registration.js:19](../packages/node-compat/src/runtime/registration.js#L19)；[mod.rs:22](../crates/niva/src/app/http_server/mod.rs#L22)；[mod.rs:242](../crates/niva/src/app/http_server/mod.rs#L242)；[分层依据](node-layering-plan.md) |
| `http.createServer` | 高 | 缺失 | 不需要 | JS语义 / 中 | JS HTTP/1.1 → Native TCP → 不提供原生socket；浏览器fetch另计 | 模块只导出 request/get/post；没有服务端 Server、listen 或入站请求处理入口。 目标：协议层不单独新增 Native HTTP/HTTPS；复用 net 的 TCP 客户端/监听以及 tls 的客户端/服务端握手与字节流。 JS：用纯 JS HTTP/1.1 parser 加报文序列化，封装 ClientRequest/IncomingMessage/Server/Agent、连接复用、超时及背压；HTTPS 复用同一协议实现，仅底层流换成 TLS。 | [http.js:401](../packages/node-compat/src/runtime/http.js#L401)；[mod.rs:22](../crates/niva/src/app/http_server/mod.rs#L22)；[mod.rs:242](../crates/niva/src/app/http_server/mod.rs#L242)；[分层依据](node-layering-plan.md) |
| `http.get` | 高 | 部分兼容 | 不需要 | JS语义 / 中 | JS HTTP/1.1 → Native TCP → 不提供原生socket；浏览器fetch另计 | GET 便利方法会调用 request().end() 并返回 ClientRequest；沿用缓冲 response/body 和受限 HTTP 选项语义。 目标：协议层不单独新增 Native HTTP/HTTPS；复用 net 的 TCP 客户端/监听以及 tls 的客户端/服务端握手与字节流。 JS：用纯 JS HTTP/1.1 parser 加报文序列化，封装 ClientRequest/IncomingMessage/Server/Agent、连接复用、超时及背压；HTTPS 复用同一协议实现，仅底层流换成 TLS。 | [http.js:375](../packages/node-compat/src/runtime/http.js#L375)；[http.rs:10](../crates/niva/src/app/api/http.rs#L10)；[http.rs:25](../crates/niva/src/app/api/http.rs#L25)；[http.rs:39](../crates/niva/src/app/api/http.rs#L39)；[分层依据](node-layering-plan.md) |
| `http.request` | 高 | 部分兼容 | 不需要 | JS语义 / 中 | JS HTTP/1.1 → Native TCP → 不提供原生socket；浏览器fetch另计 | 客户端请求入口可用；请求 body 收集后一次提交、响应 body 完整缓冲后才发 data，且 agent/signal/timeout/lookup 等选项被拒绝。 目标：协议层不单独新增 Native HTTP/HTTPS；复用 net 的 TCP 客户端/监听以及 tls 的客户端/服务端握手与字节流。 JS：用纯 JS HTTP/1.1 parser 加报文序列化，封装 ClientRequest/IncomingMessage/Server/Agent、连接复用、超时及背压；HTTPS 复用同一协议实现，仅底层流换成 TLS。 | [http.js:197](../packages/node-compat/src/runtime/http.js#L197)；[http.rs:10](../crates/niva/src/app/api/http.rs#L10)；[http.rs:25](../crates/niva/src/app/api/http.rs#L25)；[http.rs:39](../crates/niva/src/app/api/http.rs#L39)；[分层依据](node-layering-plan.md) |

<a id="module-https"></a>

### https

Node 参考：[官方 https 文档](https://nodejs.org/api/https.html)。

| API | 频率 | 当前状态 | Native 能力 | 工作层 / 难度 | 本地 → 跨域目标 | 差距与依赖依据 | 源码 |
|---|---|---|---|---|---|---|---|
| `https.get` | 高 | 部分兼容 | 不需要 | JS语义 / 中 | JS HTTP/1.1 → Native TLS → 不提供原生socket；浏览器fetch另计 | GET 便利方法会调用 request().end() 并返回 ClientRequest；沿用缓冲 response/body 和受限 HTTP 选项语义。 目标：协议层不单独新增 Native HTTP/HTTPS；复用 net 的 TCP 客户端/监听以及 tls 的客户端/服务端握手与字节流。 JS：用纯 JS HTTP/1.1 parser 加报文序列化，封装 ClientRequest/IncomingMessage/Server/Agent、连接复用、超时及背压；HTTPS 复用同一协议实现，仅底层流换成 TLS。 | [http.js:406](../packages/node-compat/src/runtime/http.js#L406)；[http.rs:10](../crates/niva/src/app/api/http.rs#L10)；[http.rs:25](../crates/niva/src/app/api/http.rs#L25)；[http.rs:39](../crates/niva/src/app/api/http.rs#L39)；[分层依据](node-layering-plan.md) |
| `https.request` | 高 | 部分兼容 | 不需要 | JS语义 / 中 | JS HTTP/1.1 → Native TLS → 不提供原生socket；浏览器fetch另计 | 客户端请求入口可用；请求 body 收集后一次提交、响应 body 完整缓冲后才发 data，且 agent/signal/timeout/lookup 等选项被拒绝。 目标：协议层不单独新增 Native HTTP/HTTPS；复用 net 的 TCP 客户端/监听以及 tls 的客户端/服务端握手与字节流。 JS：用纯 JS HTTP/1.1 parser 加报文序列化，封装 ClientRequest/IncomingMessage/Server/Agent、连接复用、超时及背压；HTTPS 复用同一协议实现，仅底层流换成 TLS。 | [http.js:406](../packages/node-compat/src/runtime/http.js#L406)；[http.rs:10](../crates/niva/src/app/api/http.rs#L10)；[http.rs:25](../crates/niva/src/app/api/http.rs#L25)；[http.rs:39](../crates/niva/src/app/api/http.rs#L39)；[分层依据](node-layering-plan.md) |
| `https.createServer` | 中 | 缺失 | 不需要 | JS语义 / 中 | JS HTTP/1.1 → Native TLS → 不提供原生socket；浏览器fetch另计 | 模块只导出 request/get/post；没有服务端 Server、listen 或入站请求处理入口。 目标：协议层不单独新增 Native HTTP/HTTPS；复用 net 的 TCP 客户端/监听以及 tls 的客户端/服务端握手与字节流。 JS：用纯 JS HTTP/1.1 parser 加报文序列化，封装 ClientRequest/IncomingMessage/Server/Agent、连接复用、超时及背压；HTTPS 复用同一协议实现，仅底层流换成 TLS。 | [http.js:401](../packages/node-compat/src/runtime/http.js#L401)；[mod.rs:22](../crates/niva/src/app/http_server/mod.rs#L22)；[Cargo.toml:21](../crates/niva/Cargo.toml#L21)；[分层依据](node-layering-plan.md) |

<a id="module-util"></a>

### util

Node 参考：[官方 util 文档](https://nodejs.org/api/util.html)。

| API | 频率 | 当前状态 | Native 能力 | 工作层 / 难度 | 本地 → 跨域目标 | 差距与依赖依据 | 源码 |
|---|---|---|---|---|---|---|---|
| `util.callbackify` | 高 | 基础支持 | 不需要 | JS语义 / 已具备 | JS → 纯JS目标 | 接受 Promise 函数，保留 receiver，并异步调用成功/失败 callback。 | [util.js:142](../packages/node-compat/src/runtime/util.js#L142) |
| `util.deprecate` | 高 | 部分兼容 | 不需要 | JS语义 / 低 | JS → 纯JS目标 | 包装函数会保留 this/返回值并只警告一次；只识别 deprecate.noDeprecation，未实现 Node 的 process 级弃用开关。 Native：格式化、回调/Promise 适配和比较均可由 JS 实现。 JS：按 Node 的 warning 通道、code 和 once 行为发出警告。 | [util.js:249](../packages/node-compat/src/runtime/util.js#L249) |
| `util.format` | 高 | 基础支持 | 不需要 | JS语义 / 已具备 | JS → 纯JS目标 | 常用占位符、额外参数和循环 JSON 值有实现；覆盖常见格式化调用。 | [util.js:84](../packages/node-compat/src/runtime/util.js#L84) |
| `util.inspect` | 高 | 部分兼容 | 不需要 | JS语义 / 中 | JS → 纯JS目标 | 深度和颜色可用；options 只读取 depth/colors，其他常用 inspect 选项不生效。 Native：格式化、回调/Promise 适配和比较均可由 JS 实现。 JS：补深度、colors、循环引用、自定义 inspect 和 Node 格式细节。 | [util.js:13](../packages/node-compat/src/runtime/util.js#L13) |
| `util.isDeepStrictEqual` | 高 | 部分兼容 | 不需要 | JS语义 / 中 | JS → 纯JS目标 | 有循环引用、原型、日期、正则、URL、Buffer、TypedArray、Map/Set 比较；内建对象比较为手写子集，非完整 Node 深比较语义。 Native：格式化、回调/Promise 适配和比较均可由 JS 实现。 JS：覆盖 Node 原型、属性描述、错误、集合、typed array 与 cross-realm 比较细节。 | [util.js:179](../packages/node-compat/src/runtime/util.js#L179) |
| `util.promisify` | 高 | 基础支持 | 不需要 | JS语义 / 已具备 | JS → 纯JS目标 | 支持标准 error-first callback、receiver 保留及 promisify.custom。 | [util.js:118](../packages/node-compat/src/runtime/util.js#L118) |

<a id="module-os"></a>

### os

Node 参考：[官方 os 文档](https://nodejs.org/api/os.html)。

| API | 频率 | 当前状态 | Native 能力 | 工作层 / 难度 | 本地 → 跨域目标 | 差距与依赖依据 | 源码 |
|---|---|---|---|---|---|---|---|
| `os.EOL` | 高 | 基础支持 | 具备 | JS封装 / 已具备 | 启动注入 → JS同步读取 → 不默认注入；IPC有限查询 | 按目标平台 path separator 同步提供 LF/CRLF，符合常用 Node 行尾常量语义。 Native：复用现有平台/目录数据，一次性放入启动载荷。 JS：从已注入的只读快照同步返回值，无需每次 XHR。 | [os.js:36](../packages/node-compat/src/runtime/os.js#L36)；[os.rs:13](../crates/niva/src/app/api/os.rs#L13)；[os.rs:71](../crates/niva/src/app/api/os.rs#L71) |
| `os.arch` | 高 | 部分兼容 | 具备 | JS封装 / 低 | 启动注入 → JS同步读取 → 不默认注入；IPC有限查询 | 返回 Node 架构字符串，但函数为 Promise；Node os.arch() 同步。 Native：复用现有平台/目录数据，一次性放入启动载荷。 JS：从已注入的只读快照同步返回值，无需每次 XHR。 | [os.js:47](../packages/node-compat/src/runtime/os.js#L47)；[os.rs:13](../crates/niva/src/app/api/os.rs#L13)；[os.rs:21](../crates/niva/src/app/api/os.rs#L21) |
| `os.homedir` | 高 | 部分兼容 | 具备 | JS封装 / 低 | 启动注入 → JS同步读取 → 不默认注入；IPC有限查询 | 路径可用，但实现返回 Promise；Node os.homedir() 同步。 Native：复用现有平台/目录数据，一次性放入启动载荷。 JS：从已注入的只读快照同步返回值，无需每次 XHR。 | [os.js:48](../packages/node-compat/src/runtime/os.js#L48)；[os.rs:13](../crates/niva/src/app/api/os.rs#L13)；[os.rs:34](../crates/niva/src/app/api/os.rs#L34) |
| `os.platform` | 高 | 部分兼容 | 具备 | JS封装 / 低 | 启动注入 → JS同步读取 → 不默认注入；IPC有限查询 | 返回 Node 字符串，但函数为 Promise；Node os.platform() 同步。 Native：复用现有平台/目录数据，一次性放入启动载荷。 JS：从已注入的只读快照同步返回值，无需每次 XHR。 | [os.js:46](../packages/node-compat/src/runtime/os.js#L46)；[os.rs:13](../crates/niva/src/app/api/os.rs#L13)；[os.rs:21](../crates/niva/src/app/api/os.rs#L21) |
| `os.tmpdir` | 高 | 部分兼容 | 部分具备 | 扩展Native / 中 | 启动注入 → JS同步读取 → 不默认注入；IPC有限查询 | 路径可用，但实现返回 Promise；Node os.tmpdir() 同步。 Native：现有 os.dirs.temp 是 Niva 应用临时目录；按目标 Node tmpdir 语义核对/补系统临时目录，启动时一次收集。 JS：从已注入的只读快照同步返回值，无需每次 XHR。 | [os.js:54](../packages/node-compat/src/runtime/os.js#L54)；[os.rs:13](../crates/niva/src/app/api/os.rs#L13)；[os.rs:34](../crates/niva/src/app/api/os.rs#L34) |
| `os.cpus` | 中 | 缺失 | 缺失 | 扩展Native / 中 | XHR实时查询 → IPC有限异步查询 | NodeCompat os 对象没有此方法；Niva.api.os.info/dirs 不能替代缺失的 Node 入口。 Native：补 CPU 当前信息与 times/频率查询，按调用返回实时数据。 JS：通过同步 XHR 查询并映射为 Node 返回结构。 | [os.js:37](../packages/node-compat/src/runtime/os.js#L37)；[os.rs:13](../crates/niva/src/app/api/os.rs#L13) |
| `os.freemem` | 中 | 缺失 | 缺失 | 扩展Native / 中 | XHR实时查询 → IPC有限异步查询 | NodeCompat os 对象没有此方法；Niva.api.os.info/dirs 不能替代缺失的 Node 入口。 Native：补当前空闲内存查询。 JS：通过同步 XHR 查询并映射为 Node 返回结构。 | [os.js:37](../packages/node-compat/src/runtime/os.js#L37)；[os.rs:13](../crates/niva/src/app/api/os.rs#L13) |
| `os.hostname` | 中 | 缺失 | 缺失 | 扩展Native / 中 | 启动注入 → JS同步读取 → 不默认注入；IPC有限查询 | os 模块没有 hostname 导出。 Native：启动时补齐并采集 hostname 对应原生字段；收集后注入只读快照。 JS：从已注入的只读快照同步返回值，无需每次 XHR。 | [os.js:37](../packages/node-compat/src/runtime/os.js#L37)；[os.rs:13](../crates/niva/src/app/api/os.rs#L13) |
| `os.networkInterfaces` | 中 | 缺失 | 缺失 | 扩展Native / 中 | XHR实时查询 → IPC有限异步查询 | NodeCompat os 对象没有此方法；Niva.api.os.info/dirs 不能替代缺失的 Node 入口。 Native：补当前网络接口查询。 JS：通过同步 XHR 查询并映射为 Node 返回结构。 | [os.js:37](../packages/node-compat/src/runtime/os.js#L37)；[os.rs:13](../crates/niva/src/app/api/os.rs#L13) |
| `os.release` | 中 | 缺失 | 部分具备 | 扩展Native / 中 | 启动注入 → JS同步读取 → 不默认注入；IPC有限查询 | NodeCompat os 对象没有此方法；Niva.api.os.info/dirs 不能替代缺失的 Node 入口。 Native：启动时补齐并采集 release 对应原生字段；收集后注入只读快照。 JS：从已注入的只读快照同步返回值，无需每次 XHR。 | [os.js:37](../packages/node-compat/src/runtime/os.js#L37)；[os.rs:13](../crates/niva/src/app/api/os.rs#L13)；[os.rs:21](../crates/niva/src/app/api/os.rs#L21) |
| `os.totalmem` | 中 | 缺失 | 缺失 | 扩展Native / 中 | 启动注入 → JS同步读取 → 不默认注入；IPC有限查询 | NodeCompat os 对象没有此方法；Niva.api.os.info/dirs 不能替代缺失的 Node 入口。 Native：启动时补齐并采集 totalmem 对应原生字段；收集后注入只读快照。 JS：从已注入的只读快照同步返回值，无需每次 XHR。 | [os.js:37](../packages/node-compat/src/runtime/os.js#L37)；[os.rs:13](../crates/niva/src/app/api/os.rs#L13) |
| `os.type` | 中 | 缺失 | 部分具备 | JS封装 / 低 | 启动注入 → JS同步读取 → 不默认注入；IPC有限查询 | NodeCompat os 对象没有此方法；Niva.api.os.info/dirs 不能替代缺失的 Node 入口。 Native：复用已有 os.info OS 标识，启动时映射为所需系统类型名称。 JS：从已注入的只读快照同步返回值，无需每次 XHR。 | [os.js:37](../packages/node-compat/src/runtime/os.js#L37)；[os.rs:13](../crates/niva/src/app/api/os.rs#L13)；[os.rs:21](../crates/niva/src/app/api/os.rs#L21) |
| `os.uptime` | 中 | 缺失 | 缺失 | 扩展Native / 中 | XHR实时查询 → IPC有限异步查询 | NodeCompat os 对象没有此方法；Niva.api.os.info/dirs 不能替代缺失的 Node 入口。 Native：补系统 uptime 查询，按调用返回当前值。 JS：通过同步 XHR 查询并映射为 Node 返回结构。 | [os.js:37](../packages/node-compat/src/runtime/os.js#L37)；[os.rs:13](../crates/niva/src/app/api/os.rs#L13) |
| `os.userInfo` | 中 | 缺失 | 部分具备 | 扩展Native / 中 | 启动注入 → JS同步读取 → 不默认注入；IPC有限查询 | NodeCompat os 对象没有此方法；Niva.api.os.info/dirs 不能替代缺失的 Node 入口。 Native：启动时补齐并采集 userInfo 对应原生字段；收集后注入只读快照。 JS：从已注入的只读快照同步返回值，无需每次 XHR。 | [os.js:37](../packages/node-compat/src/runtime/os.js#L37)；[os.rs:13](../crates/niva/src/app/api/os.rs#L13)；[os.rs:34](../crates/niva/src/app/api/os.rs#L34)；[process.rs:57](../crates/niva/src/app/api/process.rs#L57) |
| `os.version` | 中 | 缺失 | 部分具备 | 扩展Native / 中 | 启动注入 → JS同步读取 → 不默认注入；IPC有限查询 | NodeCompat os 对象没有此方法；Niva.api.os.info/dirs 不能替代缺失的 Node 入口。 Native：启动时补齐并采集 version 对应原生字段；收集后注入只读快照。 JS：从已注入的只读快照同步返回值，无需每次 XHR。 | [os.js:37](../packages/node-compat/src/runtime/os.js#L37)；[os.rs:13](../crates/niva/src/app/api/os.rs#L13)；[os.rs:21](../crates/niva/src/app/api/os.rs#L21) |

<a id="module-crypto"></a>

### crypto

Node 参考：[官方 crypto 文档](https://nodejs.org/api/crypto.html)。

| API | 频率 | 当前状态 | Native 能力 | 工作层 / 难度 | 本地 → 跨域目标 | 差距与依赖依据 | 源码 |
|---|---|---|---|---|---|---|---|
| `crypto.createHash` | 高 | 部分兼容 | 不需要 | JS封装 / 低 | JS/浏览器库 → 纯JS目标 | 有 update 链式接口；digest 返回 Promise，Node 返回同步 Buffer/string；算法仅 SHA-1/256/384/512，缺 MD5。 新增 @noble/hashes 纯 JS 方案；不再要求为此新增 Native 算法接口。 Native：选用浏览器 JS 方案时不需要新增 Native；原 Native 方案保存在清单备选字段。 JS：打包 @noble/hashes，映射 Node 签名、Buffer、callback/同步结果及错误。 详见浏览器第三方方案附表。 依赖：小型依赖候选；仍需封装 Node 的算法名、Buffer、callback 和错误；不据此宣称完整 node:crypto 或严格恒定时间。 上游库体积不替代 Niva 构建实测；不引入 Node 二进制。 | [crypto.js:52](../packages/node-compat/src/runtime/crypto.js#L52)；[JS方案](node-browser-js-options.md) |
| `crypto.createHmac` | 高 | 缺失 | 不需要 | JS封装 / 低 | JS/浏览器库 → 纯JS目标 | 未导出 HMAC 接口。 新增 @noble/hashes 纯 JS 方案；不再要求为此新增 Native 算法接口。 Native：选用浏览器 JS 方案时不需要新增 Native；原 Native 方案保存在清单备选字段。 JS：打包 @noble/hashes，映射 Node 签名、Buffer、callback/同步结果及错误。 详见浏览器第三方方案附表。 依赖：小型依赖候选；仍需封装 Node 的算法名、Buffer、callback 和错误；不据此宣称完整 node:crypto 或严格恒定时间。 上游库体积不替代 Niva 构建实测；不引入 Node 二进制。 | [crypto.js:87](../packages/node-compat/src/runtime/crypto.js#L87)；[JS方案](node-browser-js-options.md) |
| `crypto.randomBytes` | 高 | 部分兼容 | 不需要 | JS封装 / 低 | JS/浏览器库 → 纯JS目标 | 返回 Promise&lt;Buffer&gt;，Node 的同步返回和 callback 重载未覆盖。 Native：选用浏览器 JS 方案时不需要新增 Native；原 Native 方案保存在清单备选字段。 JS：打包 浏览器 Web Crypto（无第三方运行时），映射 Node 签名、Buffer、callback/同步结果及错误。 详见浏览器第三方方案附表。 | [crypto.js:26](../packages/node-compat/src/runtime/crypto.js#L26)；[JS方案](node-browser-js-options.md) |
| `crypto.randomUUID` | 高 | 部分兼容 | 不需要 | JS封装 / 低 | JS/浏览器库 → 纯JS目标 | 返回 Promise&lt;string&gt;，Node 返回同步 string。 Native：选用浏览器 JS 方案时不需要新增 Native；原 Native 方案保存在清单备选字段。 JS：打包 浏览器 Web Crypto（无第三方运行时），映射 Node 签名、Buffer、callback/同步结果及错误。 详见浏览器第三方方案附表。 | [crypto.js:40](../packages/node-compat/src/runtime/crypto.js#L40)；[JS方案](node-browser-js-options.md) |
| `crypto.pbkdf2` | 中 | 缺失 | 不需要 | JS封装 / 低 | JS/浏览器库 → 纯JS目标 | 未导出 callback 形式的密钥派生接口。 新增 @noble/hashes 纯 JS 方案；不再要求为此新增 Native 算法接口。 Native：选用浏览器 JS 方案时不需要新增 Native；原 Native 方案保存在清单备选字段。 JS：打包 @noble/hashes，映射 Node 签名、Buffer、callback/同步结果及错误。 详见浏览器第三方方案附表。 依赖：小型依赖候选；仍需封装 Node 的算法名、Buffer、callback 和错误；不据此宣称完整 node:crypto 或严格恒定时间。 上游库体积不替代 Niva 构建实测；不引入 Node 二进制。 | [crypto.js:87](../packages/node-compat/src/runtime/crypto.js#L87)；[JS方案](node-browser-js-options.md) |
| `crypto.pbkdf2Sync` | 中 | 缺失 | 不需要 | JS封装 / 低 | JS/浏览器库 → 纯JS目标 | 未导出同步 PBKDF2。 新增 @noble/hashes 纯 JS 方案；不再要求为此新增 Native 算法接口。 Native：选用浏览器 JS 方案时不需要新增 Native；原 Native 方案保存在清单备选字段。 JS：打包 @noble/hashes，映射 Node 签名、Buffer、callback/同步结果及错误。 详见浏览器第三方方案附表。 依赖：小型依赖候选；仍需封装 Node 的算法名、Buffer、callback 和错误；不据此宣称完整 node:crypto 或严格恒定时间。 上游库体积不替代 Niva 构建实测；不引入 Node 二进制。 | [crypto.js:87](../packages/node-compat/src/runtime/crypto.js#L87)；[JS方案](node-browser-js-options.md) |
| `crypto.scrypt` | 中 | 缺失 | 不需要 | JS封装 / 低 | JS/浏览器库 → 纯JS目标 | 未导出 scrypt。 新增 @noble/hashes 纯 JS 方案；不再要求为此新增 Native 算法接口。 Native：选用浏览器 JS 方案时不需要新增 Native；原 Native 方案保存在清单备选字段。 JS：打包 @noble/hashes，映射 Node 签名、Buffer、callback/同步结果及错误。 详见浏览器第三方方案附表。 依赖：小型依赖候选；仍需封装 Node 的算法名、Buffer、callback 和错误；不据此宣称完整 node:crypto 或严格恒定时间。 上游库体积不替代 Niva 构建实测；不引入 Node 二进制。 | [crypto.js:87](../packages/node-compat/src/runtime/crypto.js#L87)；[JS方案](node-browser-js-options.md) |
| `crypto.timingSafeEqual` | 中 | 缺失 | 缺失 | 扩展Native / 中 | XHR → 不纳入IPC | 未导出恒定时间比较；普通 JS 字节比较不能视为该安全契约。 补齐按原生算法实现或成熟库加同步 XHR 适配估算，同步传输不加级。 Native：需补明确的恒定时间字节比较实现；普通 JS 循环不保证该契约。 JS：处理长度、Buffer/TypedArray 和同步布尔返回。 | [crypto.js:87](../packages/node-compat/src/runtime/crypto.js#L87)；[Cargo.toml:8](../crates/niva/Cargo.toml#L8) |

<a id="module-child_process"></a>

### child_process

Node 参考：[官方 child_process 文档](https://nodejs.org/api/child_process.html)。

| API | 频率 | 当前状态 | Native 能力 | 工作层 / 难度 | 本地 → 跨域目标 | 差距与依赖依据 | 源码 |
|---|---|---|---|---|---|---|---|
| `child_process.exec` | 高 | 部分兼容 | 具备 | JS封装 / 中 | WS+进程后端 → 不纳入IPC | shell 命令和 callback 可用；timeout/maxBuffer、AbortSignal 等常用选项未覆盖。额外 Promise 字段本身不算不兼容。 Native：Native 可执行 program+args、cwd/env/detached，双向 piped stdio、status、cancel/reap。 JS：把 exec/shell/execFile options 映射到既有进程流并包装 Node ChildProcess/callback。 | [child_process.js:300](../packages/node-compat/src/runtime/child_process.js#L300)；[process.rs:107](../crates/niva/src/app/api/process.rs#L107)；[process.rs:150](../crates/niva/src/app/api/process.rs#L150)；[initialize_script.js:562](../crates/niva/assets/initialize_script.js#L562) |
| `child_process.execFile` | 高 | 缺失 | 具备 | JS封装 / 中 | WS+进程后端 → 不纳入IPC | child_process 模块仅暴露 spawn/exec；没有此 Node 兼容入口。 Native：Native 可执行 program+args、cwd/env/detached，双向 piped stdio、status、cancel/reap。 JS：把 exec/shell/execFile options 映射到既有进程流并包装 Node ChildProcess/callback。 | [child_process.js:355](../packages/node-compat/src/runtime/child_process.js#L355)；[process.rs:107](../crates/niva/src/app/api/process.rs#L107)；[process.rs:150](../crates/niva/src/app/api/process.rs#L150)；[initialize_script.js:562](../crates/niva/assets/initialize_script.js#L562) |
| `child_process.execSync` | 高 | 缺失 | 部分具备 | 扩展Native / 中 | XHR → 不纳入IPC | 当前未导出；按既定同步 XHR 方案复用原生进程执行，映射退出状态、stdout/stderr、错误及常用选项。 Native：Native 命令执行存在，但当前 API 是异步全双工 WS stream，不是单次 stdout/stderr/status 返回。 JS：用选定 syncXHR 暴露有界聚合结果；映射 timeout/maxBuffer/stdio/error。 | [child_process.js:355](../packages/node-compat/src/runtime/child_process.js#L355)；[process.rs:107](../crates/niva/src/app/api/process.rs#L107)；[process.rs:150](../crates/niva/src/app/api/process.rs#L150) |
| `child_process.spawn` | 高 | 部分兼容 | 部分具备 | 扩展Native / 中 | WS+进程后端 → 不纳入IPC | 可返回带事件及 stdin/stdout/stderr 的子对象；仅支持 piped stdio，限制 signal/timeout/encoding，输出流是缓冲适配对象。 Native：program/args/cwd/env、双向 piped stdio 与取消已具备；Node pid、更多 stdio 模式和信号尚未暴露。 JS：复用 execStream 包装 ChildProcess、实时输出、选项及事件。 | [child_process.js:151](../packages/node-compat/src/runtime/child_process.js#L151)；[process.rs:107](../crates/niva/src/app/api/process.rs#L107)；[process.rs:150](../crates/niva/src/app/api/process.rs#L150)；[initialize_script.js:562](../crates/niva/assets/initialize_script.js#L562) |
| `ChildProcess.kill` | 中 | 缺失 | 部分具备 | 扩展Native / 中 | WS+进程后端 → 不纳入IPC | 方法存在但恒返回 false，没有连接 native kill；按实质行为计为缺失。 Native：已有 call cancel 触发 Child::kill 并回收，但没有 Node signal 接口；默认 SIGTERM 不能直接等同当前强制 kill。 JS：通过调用/进程标识封装 kill(signal)、返回状态与事件；复用已有取消/回收设施。 | [child_process.js:198](../packages/node-compat/src/runtime/child_process.js#L198)；[process.rs:150](../crates/niva/src/app/api/process.rs#L150)；[process.rs:235](../crates/niva/src/app/api/process.rs#L235)；[mod.rs:162](../crates/niva/src/app/api_manager/mod.rs#L162) |
| `child_process.execFileSync` | 中 | 缺失 | 部分具备 | 扩展Native / 中 | XHR → 不纳入IPC | 当前未导出；按既定同步 XHR 方案复用原生进程执行，映射退出状态、stdout/stderr、错误及常用选项。 Native：Native 命令执行存在，但当前 API 是异步全双工 WS stream，不是单次 stdout/stderr/status 返回。 JS：用选定 syncXHR 暴露有界聚合结果；映射 timeout/maxBuffer/stdio/error。 | [child_process.js:355](../packages/node-compat/src/runtime/child_process.js#L355)；[process.rs:107](../crates/niva/src/app/api/process.rs#L107)；[process.rs:150](../crates/niva/src/app/api/process.rs#L150) |
| `child_process.spawnSync` | 中 | 缺失 | 部分具备 | 扩展Native / 中 | XHR → 不纳入IPC | 当前未导出；按既定同步 XHR 方案复用原生进程执行，映射退出状态、stdout/stderr、错误及常用选项。 Native：Native 命令执行存在，但当前 API 是异步全双工 WS stream，不是单次 stdout/stderr/status 返回。 JS：用选定 syncXHR 暴露有界聚合结果；映射 timeout/maxBuffer/stdio/error。 | [child_process.js:355](../packages/node-compat/src/runtime/child_process.js#L355)；[process.rs:107](../crates/niva/src/app/api/process.rs#L107)；[process.rs:150](../crates/niva/src/app/api/process.rs#L150) |

<a id="module-stream"></a>

### stream

Node 参考：[官方 stream 文档](https://nodejs.org/api/stream.html)。

| API | 频率 | 当前状态 | Native 能力 | 工作层 / 难度 | 本地 → 跨域目标 | 差距与依赖依据 | 源码 |
|---|---|---|---|---|---|---|---|
| `stream.Readable` | 高 | 缺失 | 不需要 | JS封装 / 低 | JS/浏览器库 → 纯JS目标 | NodeCompat stream 模块没有该类/函数导出；当前实现只提供 pipeline。 Native：选用浏览器 JS 方案时不需要新增 Native；原 Native 方案保存在清单备选字段。 JS：打包 readable-stream（browser 入口），映射 Node 签名、Buffer、callback/同步结果及错误。 详见浏览器第三方方案附表。 依赖：待选型测量；打包 browser 映射及纯 JS 依赖；流对象无需 Node，真实文件/进程/socket 数据源仍接 Niva Native。 上游库体积不替代 Niva 构建实测；不引入 Node 二进制。 | [stream.js:74](../packages/node-compat/src/runtime/stream.js#L74)；[JS方案](node-browser-js-options.md) |
| `stream.Transform` | 高 | 缺失 | 不需要 | JS封装 / 低 | JS/浏览器库 → 纯JS目标 | NodeCompat stream 模块没有该类/函数导出；当前实现只提供 pipeline。 Native：选用浏览器 JS 方案时不需要新增 Native；原 Native 方案保存在清单备选字段。 JS：打包 readable-stream（browser 入口），映射 Node 签名、Buffer、callback/同步结果及错误。 详见浏览器第三方方案附表。 依赖：待选型测量；打包 browser 映射及纯 JS 依赖；流对象无需 Node，真实文件/进程/socket 数据源仍接 Niva Native。 上游库体积不替代 Niva 构建实测；不引入 Node 二进制。 | [stream.js:74](../packages/node-compat/src/runtime/stream.js#L74)；[JS方案](node-browser-js-options.md) |
| `stream.Writable` | 高 | 缺失 | 不需要 | JS封装 / 低 | JS/浏览器库 → 纯JS目标 | NodeCompat stream 模块没有该类/函数导出；当前实现只提供 pipeline。 Native：选用浏览器 JS 方案时不需要新增 Native；原 Native 方案保存在清单备选字段。 JS：打包 readable-stream（browser 入口），映射 Node 签名、Buffer、callback/同步结果及错误。 详见浏览器第三方方案附表。 依赖：待选型测量；打包 browser 映射及纯 JS 依赖；流对象无需 Node，真实文件/进程/socket 数据源仍接 Niva Native。 上游库体积不替代 Niva 构建实测；不引入 Node 二进制。 | [stream.js:74](../packages/node-compat/src/runtime/stream.js#L74)；[JS方案](node-browser-js-options.md) |
| `stream.pipeline` | 高 | 部分兼容 | 不需要 | JS封装 / 低 | JS/浏览器库 → 纯JS目标 | 可连接已有 on/pipe 适配器；缺通用 Node 流、异步迭代器、AbortSignal 和完整背压/销毁语义。callback 形式返回 destination 本身不作为差异。 Native：选用浏览器 JS 方案时不需要新增 Native；原 Native 方案保存在清单备选字段。 JS：打包 readable-stream（browser 入口），映射 Node 签名、Buffer、callback/同步结果及错误。 详见浏览器第三方方案附表。 依赖：待选型测量；打包 browser 映射及纯 JS 依赖；流对象无需 Node，真实文件/进程/socket 数据源仍接 Niva Native。 上游库体积不替代 Niva 构建实测；不引入 Node 二进制。 | [stream.js:6](../packages/node-compat/src/runtime/stream.js#L6)；[JS方案](node-browser-js-options.md) |
| `stream/promises.pipeline` | 高 | 部分兼容 | 不需要 | JS封装 / 低 | JS/浏览器库 → 纯JS目标 | 导出同一个 pipeline Promise 函数；通用流、迭代器和 AbortSignal 能力仍缺失，且受缓冲 adapter 限制。 Native：选用浏览器 JS 方案时不需要新增 Native；原 Native 方案保存在清单备选字段。 JS：打包 readable-stream（browser 入口），映射 Node 签名、Buffer、callback/同步结果及错误。 详见浏览器第三方方案附表。 依赖：待选型测量；打包 browser 映射及纯 JS 依赖；流对象无需 Node，真实文件/进程/socket 数据源仍接 Niva Native。 上游库体积不替代 Niva 构建实测；不引入 Node 二进制。 | [stream-promises.js:6](../packages/node-compat/src/stream-promises.js#L6)；[JS方案](node-browser-js-options.md) |
| `stream.PassThrough` | 中 | 缺失 | 不需要 | JS封装 / 低 | JS/浏览器库 → 纯JS目标 | NodeCompat stream 模块没有该类/函数导出；当前实现只提供 pipeline。 Native：选用浏览器 JS 方案时不需要新增 Native；原 Native 方案保存在清单备选字段。 JS：打包 readable-stream（browser 入口），映射 Node 签名、Buffer、callback/同步结果及错误。 详见浏览器第三方方案附表。 依赖：待选型测量；打包 browser 映射及纯 JS 依赖；流对象无需 Node，真实文件/进程/socket 数据源仍接 Niva Native。 上游库体积不替代 Niva 构建实测；不引入 Node 二进制。 | [stream.js:74](../packages/node-compat/src/runtime/stream.js#L74)；[JS方案](node-browser-js-options.md) |
| `stream.finished` | 中 | 缺失 | 不需要 | JS封装 / 低 | JS/浏览器库 → 纯JS目标 | NodeCompat stream 模块没有该类/函数导出；当前实现只提供 pipeline。 Native：选用浏览器 JS 方案时不需要新增 Native；原 Native 方案保存在清单备选字段。 JS：打包 readable-stream（browser 入口），映射 Node 签名、Buffer、callback/同步结果及错误。 详见浏览器第三方方案附表。 依赖：待选型测量；打包 browser 映射及纯 JS 依赖；流对象无需 Node，真实文件/进程/socket 数据源仍接 Niva Native。 上游库体积不替代 Niva 构建实测；不引入 Node 二进制。 | [stream.js:74](../packages/node-compat/src/runtime/stream.js#L74)；[JS方案](node-browser-js-options.md) |

<a id="module-url"></a>

### url

Node 参考：[官方 url 文档](https://nodejs.org/api/url.html)。

| API | 频率 | 当前状态 | Native 能力 | 工作层 / 难度 | 本地 → 跨域目标 | 差距与依赖依据 | 源码 |
|---|---|---|---|---|---|---|---|
| `url.URL` | 高 | 基础支持 | 不需要 | JS语义 / 已具备 | JS → 纯JS目标 | 直接导出运行环境的 WHATWG URL 构造器。 | [url.js:55](../packages/node-compat/src/runtime/url.js#L55) |
| `url.URLSearchParams` | 高 | 基础支持 | 不需要 | JS语义 / 已具备 | JS → 纯JS目标 | 直接导出运行环境的 URLSearchParams 构造器。 | [url.js:55](../packages/node-compat/src/runtime/url.js#L55) |
| `url.fileURLToPath` | 高 | 基础支持 | 不需要 | JS语义 / 已具备 | JS → 纯JS目标 | 实现 file scheme、百分号解码、平台分隔符/主机校验及路径转换。 | [url.js:8](../packages/node-compat/src/runtime/url.js#L8) |
| `url.pathToFileURL` | 高 | 基础支持 | 不需要 | JS语义 / 已具备 | JS + 动态cwd XHR → 纯JS目标 | 先按平台解析绝对路径，再编码为 file URL；POSIX 和 Windows 盘符/UNC 均有用例。 目标接线：涉及相对路径解析时内部 XHR 读取当前 cwd，再执行 JS 路径运算。 | [url.js:31](../packages/node-compat/src/runtime/url.js#L31) |

<a id="module-buffer"></a>

### buffer

Node 参考：[官方 buffer 文档](https://nodejs.org/api/buffer.html)。

| API | 频率 | 当前状态 | Native 能力 | 工作层 / 难度 | 本地 → 跨域目标 | 差距与依赖依据 | 源码 |
|---|---|---|---|---|---|---|---|
| `Buffer.alloc` | 高 | 基础支持 | 不需要 | JS封装 / 已具备 | JS/浏览器库 → 纯JS目标 | 分配、尺寸校验和可选填充值已实现。 依赖：小型依赖候选；依赖 base64-js/ieee754 等纯 JS 包；打包后不依赖 Node 运行时，需对照当前选定 Node 契约测试。 上游库体积不替代 Niva 构建实测；不引入 Node 二进制。 | [buffer.js:110](../packages/node-compat/src/runtime/buffer.js#L110)；[JS方案](node-browser-js-options.md) |
| `Buffer.byteLength` | 高 | 基础支持 | 不需要 | JS封装 / 已具备 | JS/浏览器库 → 纯JS目标 | 字符串按编码长度计算，buffer-like 输入按 byteLength 计算。 依赖：小型依赖候选；依赖 base64-js/ieee754 等纯 JS 包；打包后不依赖 Node 运行时，需对照当前选定 Node 契约测试。 上游库体积不替代 Niva 构建实测；不引入 Node 二进制。 | [buffer.js:135](../packages/node-compat/src/runtime/buffer.js#L135)；[JS方案](node-browser-js-options.md) |
| `Buffer.concat` | 高 | 基础支持 | 不需要 | JS封装 / 已具备 | JS/浏览器库 → 纯JS目标 | 按给定/汇总长度复制 buffer-like 元素并返回 Buffer。 依赖：小型依赖候选；依赖 base64-js/ieee754 等纯 JS 包；打包后不依赖 Node 运行时，需对照当前选定 Node 契约测试。 上游库体积不替代 Niva 构建实测；不引入 Node 二进制。 | [buffer.js:120](../packages/node-compat/src/runtime/buffer.js#L120)；[JS方案](node-browser-js-options.md) |
| `Buffer.from` | 高 | 部分兼容 | 不需要 | JS封装 / 低 | JS/浏览器库 → 纯JS目标 | 字符串、数组、ArrayBuffer 可用；TypedArray 输入被按底层原始字节复制，而 Node Buffer.from(TypedArray) 按元素转为字节。 Native：选用浏览器 JS 方案时不需要新增 Native；原 Native 方案保存在清单备选字段。 JS：打包 buffer（feross/buffer），映射 Node 签名、Buffer、callback/同步结果及错误。 详见浏览器第三方方案附表。 依赖：小型依赖候选；依赖 base64-js/ieee754 等纯 JS 包；打包后不依赖 Node 运行时，需对照当前选定 Node 契约测试。 上游库体积不替代 Niva 构建实测；不引入 Node 二进制。 | [buffer.js:84](../packages/node-compat/src/runtime/buffer.js#L84)；[JS方案](node-browser-js-options.md) |
| `Buffer.isBuffer` | 高 | 基础支持 | 不需要 | JS封装 / 已具备 | JS/浏览器库 → 纯JS目标 | 识别本地 Buffer 实例及 Node 风格 _isBuffer 标记。 依赖：小型依赖候选；依赖 base64-js/ieee754 等纯 JS 包；打包后不依赖 Node 运行时，需对照当前选定 Node 契约测试。 上游库体积不替代 Niva 构建实测；不引入 Node 二进制。 | [buffer.js:140](../packages/node-compat/src/runtime/buffer.js#L140)；[JS方案](node-browser-js-options.md) |
| `Buffer.prototype.toString` | 高 | 部分兼容 | 不需要 | JS封装 / 低 | JS/浏览器库 → 纯JS目标 | 常见编码和 start/end 切片可用；TextDecoder 默认会剥除开头 UTF-8 BOM，与 Buffer 解码保留 BOM 的结果不同。 Native：选用浏览器 JS 方案时不需要新增 Native；原 Native 方案保存在清单备选字段。 JS：打包 buffer（feross/buffer），映射 Node 签名、Buffer、callback/同步结果及错误。 详见浏览器第三方方案附表。 依赖：小型依赖候选；依赖 base64-js/ieee754 等纯 JS 包；打包后不依赖 Node 运行时，需对照当前选定 Node 契约测试。 上游库体积不替代 Niva 构建实测；不引入 Node 二进制。 | [buffer.js:147](../packages/node-compat/src/runtime/buffer.js#L147)；[JS方案](node-browser-js-options.md) |
| `Buffer.allocUnsafe` | 中 | 基础支持 | 不需要 | JS封装 / 已具备 | JS/浏览器库 → 纯JS目标 | 按尺寸返回 Buffer；底层 typed array 初始化为零，作为未指定内容仍可用。 依赖：小型依赖候选；依赖 base64-js/ieee754 等纯 JS 包；打包后不依赖 Node 运行时，需对照当前选定 Node 契约测试。 上游库体积不替代 Niva 构建实测；不引入 Node 二进制。 | [buffer.js:117](../packages/node-compat/src/runtime/buffer.js#L117)；[JS方案](node-browser-js-options.md) |
| `Buffer.prototype.copy` | 中 | 部分兼容 | 不需要 | JS封装 / 低 | JS/浏览器库 → 纯JS目标 | 正常范围可复制；越界 target/source 的截断或报错行为仍有差异，需逐项对齐 Node。 Native：选用浏览器 JS 方案时不需要新增 Native；原 Native 方案保存在清单备选字段。 JS：打包 buffer（feross/buffer），映射 Node 签名、Buffer、callback/同步结果及错误。 详见浏览器第三方方案附表。 依赖：小型依赖候选；依赖 base64-js/ieee754 等纯 JS 包；打包后不依赖 Node 运行时，需对照当前选定 Node 契约测试。 上游库体积不替代 Niva 构建实测；不引入 Node 二进制。 | [buffer.js:177](../packages/node-compat/src/runtime/buffer.js#L177)；[JS方案](node-browser-js-options.md) |
| `Buffer.prototype.equals` | 中 | 基础支持 | 不需要 | JS封装 / 已具备 | JS/浏览器库 → 纯JS目标 | 比较相等与不相等字节序列并返回布尔值。 依赖：小型依赖候选；依赖 base64-js/ieee754 等纯 JS 包；打包后不依赖 Node 运行时，需对照当前选定 Node 契约测试。 上游库体积不替代 Niva 构建实测；不引入 Node 二进制。 | [buffer.js:168](../packages/node-compat/src/runtime/buffer.js#L168)；[JS方案](node-browser-js-options.md) |
| `Buffer.prototype.readUInt32LE` | 中 | 基础支持 | 不需要 | JS封装 / 已具备 | JS/浏览器库 → 纯JS目标 | 用 little-endian DataView 读取 32 位无符号整数，并检查偏移范围。 依赖：小型依赖候选；依赖 base64-js/ieee754 等纯 JS 包；打包后不依赖 Node 运行时，需对照当前选定 Node 契约测试。 上游库体积不替代 Niva 构建实测；不引入 Node 二进制。 | [buffer.js:207](../packages/node-compat/src/runtime/buffer.js#L207)；[JS方案](node-browser-js-options.md) |
| `Buffer.prototype.slice` | 中 | 部分兼容 | 不需要 | JS封装 / 低 | JS/浏览器库 → 纯JS目标 | 未覆写 Uint8Array.slice；它返回拷贝，而 Node Buffer.slice 返回共享底层存储的视图。 Native：选用浏览器 JS 方案时不需要新增 Native；原 Native 方案保存在清单备选字段。 JS：打包 buffer（feross/buffer），映射 Node 签名、Buffer、callback/同步结果及错误。 详见浏览器第三方方案附表。 依赖：小型依赖候选；依赖 base64-js/ieee754 等纯 JS 包；打包后不依赖 Node 运行时，需对照当前选定 Node 契约测试。 上游库体积不替代 Niva 构建实测；不引入 Node 二进制。 | [buffer.js:100](../packages/node-compat/src/runtime/buffer.js#L100)；[JS方案](node-browser-js-options.md) |
| `Buffer.prototype.subarray` | 中 | 基础支持 | 不需要 | JS封装 / 已具备 | JS/浏览器库 → 纯JS目标 | 继承 Uint8Array.subarray，配合 NivaBuffer species 返回共享存储的 Buffer 视图。 依赖：小型依赖候选；依赖 base64-js/ieee754 等纯 JS 包；打包后不依赖 Node 运行时，需对照当前选定 Node 契约测试。 上游库体积不替代 Niva 构建实测；不引入 Node 二进制。 | [buffer.js:100](../packages/node-compat/src/runtime/buffer.js#L100)；[JS方案](node-browser-js-options.md) |
| `Buffer.prototype.write` | 中 | 部分兼容 | 不需要 | JS封装 / 低 | JS/浏览器库 → 纯JS目标 | 常见字符串重载返回已写入字节数；实现会 String(value) 强制转型，而 Node 要求 value 为字符串。 Native：选用浏览器 JS 方案时不需要新增 Native；原 Native 方案保存在清单备选字段。 JS：打包 buffer（feross/buffer），映射 Node 签名、Buffer、callback/同步结果及错误。 详见浏览器第三方方案附表。 依赖：小型依赖候选；依赖 base64-js/ieee754 等纯 JS 包；打包后不依赖 Node 运行时，需对照当前选定 Node 契约测试。 上游库体积不替代 Niva 构建实测；不引入 Node 二进制。 | [buffer.js:155](../packages/node-compat/src/runtime/buffer.js#L155)；[JS方案](node-browser-js-options.md) |
| `Buffer.prototype.writeUInt32LE` | 中 | 基础支持 | 不需要 | JS封装 / 已具备 | JS/浏览器库 → 纯JS目标 | 用 little-endian DataView 写入 32 位无符号整数，校验值/偏移并返回下一个偏移。 依赖：小型依赖候选；依赖 base64-js/ieee754 等纯 JS 包；打包后不依赖 Node 运行时，需对照当前选定 Node 契约测试。 上游库体积不替代 Niva 构建实测；不引入 Node 二进制。 | [buffer.js:217](../packages/node-compat/src/runtime/buffer.js#L217)；[JS方案](node-browser-js-options.md) |

<a id="module-querystring"></a>

### querystring

Node 参考：[官方 querystring 文档](https://nodejs.org/api/querystring.html)。

| API | 频率 | 当前状态 | Native 能力 | 工作层 / 难度 | 本地 → 跨域目标 | 差距与依赖依据 | 源码 |
|---|---|---|---|---|---|---|---|
| `querystring.parse` | 中 | 基础支持 | 不需要 | JS语义 / 已具备 | JS → 纯JS目标 | 常用键值、重复键、加号、定制分隔符、maxKeys 和 codec options 均实现。 | [querystring.js:26](../packages/node-compat/src/runtime/querystring.js#L26) |
| `querystring.stringify` | 中 | 基础支持 | 不需要 | JS语义 / 已具备 | JS → 纯JS目标 | 对象/数组值、空值、定制分隔符和 codec options 均实现。 | [querystring.js:54](../packages/node-compat/src/runtime/querystring.js#L54) |

<a id="module-net"></a>

### net

Node 参考：[官方 net 文档](https://nodejs.org/api/net.html)。

| API | 频率 | 当前状态 | Native 能力 | 工作层 / 难度 | 本地 → 跨域目标 | 差距与依赖依据 | 源码 |
|---|---|---|---|---|---|---|---|
| `net.Socket` | 中 | 缺失 | 缺失 | 新增Native / 高 | WS+原生后端 → 不纳入IPC | 未注册 node:net，此 API 没有 NodeCompat 实现；原生 socket、长连接、流与销毁语义。 Native：现有原生注册表没有 TCP/IPC socket API；bridge 的二进制帧是 RPC 流，不是可供 Node Socket 使用的原始套接字。 传输仍须分别核对固定 WS、syncXHR 和受限 crossOriginIPC：流式/长连接只可走支持该行为的通道，跨域 IPC 保持获准的 unary 子集。 JS：提供 Node Socket 对象封装、读写流及 connect/close/error 事件。 | [registration.js:56](../packages/node-compat/src/runtime/registration.js#L56)；[mod.rs:3](../crates/niva/src/app/api/mod.rs#L3)；docs/bridge.md:32；[mod.rs:19](../crates/niva/src/app/api/mod.rs#L19)；[Cargo.toml:8](../crates/niva/Cargo.toml#L8) |
| `net.connect` | 中 | 缺失 | 缺失 | 新增Native / 高 | WS+原生后端 → 不纳入IPC | 未注册 node:net，此 API 没有 NodeCompat 实现；原生 socket、长连接、流与销毁语义。 Native：现有原生注册表没有 TCP/IPC socket API；bridge 的二进制帧是 RPC 流，不是可供 Node Socket 使用的原始套接字。 传输仍须分别核对固定 WS、syncXHR 和受限 crossOriginIPC：流式/长连接只可走支持该行为的通道，跨域 IPC 保持获准的 unary 子集。 JS：为原生 socket connect 结果补 Node 参数重载、回调时机和事件对象。 | [registration.js:56](../packages/node-compat/src/runtime/registration.js#L56)；[mod.rs:3](../crates/niva/src/app/api/mod.rs#L3)；docs/bridge.md:32；[mod.rs:19](../crates/niva/src/app/api/mod.rs#L19)；[Cargo.toml:8](../crates/niva/Cargo.toml#L8) |
| `net.createServer` | 中 | 缺失 | 缺失 | 新增Native / 高 | WS+原生后端 → 不纳入IPC | 未注册 node:net，此 API 没有 NodeCompat 实现；原生 socket、长连接、流与销毁语义。 Native：现有原生注册表没有 TCP/IPC socket API；bridge 的二进制帧是 RPC 流，不是可供 Node Socket 使用的原始套接字。 传输仍须分别核对固定 WS、syncXHR 和受限 crossOriginIPC：流式/长连接只可走支持该行为的通道，跨域 IPC 保持获准的 unary 子集。 JS：实现 server bind/listen/accept 与 connection Socket 生命周期。 | [registration.js:56](../packages/node-compat/src/runtime/registration.js#L56)；[mod.rs:3](../crates/niva/src/app/api/mod.rs#L3)；docs/bridge.md:32；[mod.rs:19](../crates/niva/src/app/api/mod.rs#L19)；[Cargo.toml:8](../crates/niva/Cargo.toml#L8) |

<a id="module-tls"></a>

### tls

Node 参考：[官方 tls 文档](https://nodejs.org/api/tls.html)。

| API | 频率 | 当前状态 | Native 能力 | 工作层 / 难度 | 本地 → 跨域目标 | 差距与依赖依据 | 源码 |
|---|---|---|---|---|---|---|---|
| `tls.connect` | 中 | 缺失 | 缺失 | 新增Native / 高 | WS+原生后端 → 不纳入IPC | 未注册 node:tls，此 API 没有 NodeCompat 实现；依赖 socket/stream，并处理证书、握手与跨平台 TLS。 Native：Rust HTTP 客户端内部有 TLS/解析器，但没有暴露原始 TLS client/server socket API。 传输仍须分别核对固定 WS、syncXHR 和受限 crossOriginIPC：流式/长连接只可走支持该行为的通道，跨域 IPC 保持获准的 unary 子集。 JS：实现 TLS socket 对象、验证/SNI、握手错误和超时语义。 | [registration.js:56](../packages/node-compat/src/runtime/registration.js#L56)；[http.rs:140](../crates/niva/src/app/api/http.rs#L140)；[mod.rs:3](../crates/niva/src/app/api/mod.rs#L3)；[mod.rs:19](../crates/niva/src/app/api/mod.rs#L19)；[Cargo.toml:8](../crates/niva/Cargo.toml#L8) |
| `tls.createServer` | 中 | 缺失 | 缺失 | 新增Native / 高 | WS+原生后端 → 不纳入IPC | 未注册 node:tls，此 API 没有 NodeCompat 实现；依赖 socket/stream，并处理证书、握手与跨平台 TLS。 Native：Rust HTTP 客户端内部有 TLS/解析器，但没有暴露原始 TLS client/server socket API。 传输仍须分别核对固定 WS、syncXHR 和受限 crossOriginIPC：流式/长连接只可走支持该行为的通道，跨域 IPC 保持获准的 unary 子集。 JS：增加 TLS listener、证书/key 配置及安全连接生命周期。 | [registration.js:56](../packages/node-compat/src/runtime/registration.js#L56)；[http.rs:140](../crates/niva/src/app/api/http.rs#L140)；[mod.rs:3](../crates/niva/src/app/api/mod.rs#L3)；[mod.rs:19](../crates/niva/src/app/api/mod.rs#L19)；[Cargo.toml:8](../crates/niva/Cargo.toml#L8) |

<a id="module-dns"></a>

### dns

Node 参考：[官方 dns 文档](https://nodejs.org/api/dns.html)。

| API | 频率 | 当前状态 | Native 能力 | 工作层 / 难度 | 本地 → 跨域目标 | 差距与依赖依据 | 源码 |
|---|---|---|---|---|---|---|---|
| `dns.lookup` | 中 | 缺失 | 部分具备 | 扩展Native / 中 | WS → 系统解析 → 按有限IPC授权另评 | 未注册 node:dns，此 API 没有 NodeCompat 实现；系统 resolver 与 DNS 查询类型/错误码映射。 目标：单独使用系统 getaddrinfo/等价平台接口并映射选项；默认DNS服务器配置由内部系统配置读取提供。不能把HTTP专用GuardedResolver的地址过滤直接当Node lookup语义。 JS：包装 callback/Promise、family/all/order 与 Node 错误；遵循系统 hosts/解析规则。 | [registration.js:56](../packages/node-compat/src/runtime/registration.js#L56)；[http.rs:140](../crates/niva/src/app/api/http.rs#L140)；[http.rs:146](../crates/niva/src/app/api/http.rs#L146)；[分层依据](node-layering-plan.md) |
| `dns.resolve` | 中 | 缺失 | 不需要 | JS语义 / 中 | JS dns-packet → dgram/net → 不提供原生UDP/TCP | 未注册 node:dns，此 API 没有 NodeCompat 实现；系统 resolver 与 DNS 查询类型/错误码映射。 目标：DNS协议不另增Native解析库；UDP/TCP socket能力由dgram/net共用，当前系统DNS配置由薄原生接口读取。 JS：用dns-packet编解码，补查询事务、超时重试、来源/ID核对、TCP回退与Node记录/错误映射。 | [registration.js:56](../packages/node-compat/src/runtime/registration.js#L56)；[http.rs:140](../crates/niva/src/app/api/http.rs#L140)；[http.rs:146](../crates/niva/src/app/api/http.rs#L146)；[分层依据](node-layering-plan.md) |
| `dns.resolve4` | 中 | 缺失 | 不需要 | JS语义 / 中 | JS dns-packet → dgram/net → 不提供原生UDP/TCP | 未注册 node:dns，此 API 没有 NodeCompat 实现；系统 resolver 与 DNS 查询类型/错误码映射。 目标：DNS协议不另增Native解析库；UDP/TCP socket能力由dgram/net共用，当前系统DNS配置由薄原生接口读取。 JS：用dns-packet编解码，补查询事务、超时重试、来源/ID核对、TCP回退与Node记录/错误映射。 | [registration.js:56](../packages/node-compat/src/runtime/registration.js#L56)；[http.rs:140](../crates/niva/src/app/api/http.rs#L140)；[http.rs:146](../crates/niva/src/app/api/http.rs#L146)；[分层依据](node-layering-plan.md) |

<a id="module-dgram"></a>

### dgram

Node 参考：[官方 dgram 文档](https://nodejs.org/api/dgram.html)。

| API | 频率 | 当前状态 | Native 能力 | 工作层 / 难度 | 本地 → 跨域目标 | 差距与依赖依据 | 源码 |
|---|---|---|---|---|---|---|---|
| `dgram.createSocket` | 场景 | 缺失 | 缺失 | 新增Native / 高 | WS+原生后端 → 不纳入IPC | 未注册 node:dgram，此 API 没有 NodeCompat 实现；原生 datagram、绑定、组播与事件。 Native：没有 UDP/datagram socket API。 传输仍须分别核对固定 WS、syncXHR 和受限 crossOriginIPC：流式/长连接只可走支持该行为的通道，跨域 IPC 保持获准的 unary 子集。 JS：实现 UDP bind/send/message/close 的 datagram Native API 和 JS Socket。 | [registration.js:56](../packages/node-compat/src/runtime/registration.js#L56)；[mod.rs:3](../crates/niva/src/app/api/mod.rs#L3)；[mod.rs:19](../crates/niva/src/app/api/mod.rs#L19)；[Cargo.toml:8](../crates/niva/Cargo.toml#L8) |

<a id="module-zlib"></a>

### zlib

Node 参考：[官方 zlib 文档](https://nodejs.org/api/zlib.html)。

| API | 频率 | 当前状态 | Native 能力 | 工作层 / 难度 | 本地 → 跨域目标 | 差距与依赖依据 | 源码 |
|---|---|---|---|---|---|---|---|
| `zlib.gunzip` | 中 | 部分兼容 | 不需要 | JS封装 / 低 | JS/浏览器库 → 纯JS目标 | 已有 Promise 包装；Node callback 和解压 options 未覆盖。 Native：选用浏览器 JS 方案时不需要新增 Native；原 Native 方案保存在清单备选字段。 JS：打包 fflate/browser，映射 Node 签名、Buffer、callback/同步结果及错误。 详见浏览器第三方方案附表。 依赖：小型依赖候选；选 browser 导出；异步 Worker 路径受 WebView/CSP 约束；Node options、Buffer、callback 仍需适配。 上游库体积不替代 Niva 构建实测；不引入 Node 二进制。 | [zlib.js:41](../packages/node-compat/src/runtime/zlib.js#L41)；[JS方案](node-browser-js-options.md) |
| `zlib.gunzipSync` | 中 | 缺失 | 不需要 | JS封装 / 低 | JS/浏览器库 → 纯JS目标 | 当前未导出；沿用同步 XHR 调用原生压缩后端，补数据编码、选项及错误映射。 Native：选用浏览器 JS 方案时不需要新增 Native；原 Native 方案保存在清单备选字段。 JS：打包 fflate/browser，映射 Node 签名、Buffer、callback/同步结果及错误。 详见浏览器第三方方案附表。 依赖：小型依赖候选；选 browser 导出；异步 Worker 路径受 WebView/CSP 约束；Node options、Buffer、callback 仍需适配。 上游库体积不替代 Niva 构建实测；不引入 Node 二进制。 | [zlib.js:41](../packages/node-compat/src/runtime/zlib.js#L41)；[JS方案](node-browser-js-options.md) |
| `zlib.gzip` | 中 | 部分兼容 | 不需要 | JS封装 / 低 | JS/浏览器库 → 纯JS目标 | 已有 Promise 包装；Node callback 和压缩 options 未覆盖。 Native：选用浏览器 JS 方案时不需要新增 Native；原 Native 方案保存在清单备选字段。 JS：打包 fflate/browser，映射 Node 签名、Buffer、callback/同步结果及错误。 详见浏览器第三方方案附表。 依赖：小型依赖候选；选 browser 导出；异步 Worker 路径受 WebView/CSP 约束；Node options、Buffer、callback 仍需适配。 上游库体积不替代 Niva 构建实测；不引入 Node 二进制。 | [zlib.js:41](../packages/node-compat/src/runtime/zlib.js#L41)；[JS方案](node-browser-js-options.md) |
| `zlib.gzipSync` | 中 | 缺失 | 不需要 | JS封装 / 低 | JS/浏览器库 → 纯JS目标 | 当前未导出；沿用同步 XHR 调用原生压缩后端，补数据编码、选项及错误映射。 Native：选用浏览器 JS 方案时不需要新增 Native；原 Native 方案保存在清单备选字段。 JS：打包 fflate/browser，映射 Node 签名、Buffer、callback/同步结果及错误。 详见浏览器第三方方案附表。 依赖：小型依赖候选；选 browser 导出；异步 Worker 路径受 WebView/CSP 约束；Node options、Buffer、callback 仍需适配。 上游库体积不替代 Niva 构建实测；不引入 Node 二进制。 | [zlib.js:41](../packages/node-compat/src/runtime/zlib.js#L41)；[JS方案](node-browser-js-options.md) |

<a id="module-assert"></a>

### assert

Node 参考：[官方 assert 文档](https://nodejs.org/api/assert.html)。

| API | 频率 | 当前状态 | Native 能力 | 工作层 / 难度 | 本地 → 跨域目标 | 差距与依赖依据 | 源码 |
|---|---|---|---|---|---|---|---|
| `assert.deepStrictEqual` | 中 | 部分兼容 | 不需要 | JS语义 / 中 | JS → 纯JS目标 | 常用对象/数组深比较可用；依赖手写的 util 深比较子集，内建类型和边缘语义并非完整 Node 语义。 Native：断言与比较由 JS 实现；没有操作系统能力依赖。 JS：扩展比较器覆盖属性描述符、原型、错误 cause、Map/Set 及跨 realm 对象。 | [assert.js:42](../packages/node-compat/src/runtime/assert.js#L42) |
| `assert.ok` | 中 | 基础支持 | 不需要 | JS语义 / 已具备 | JS → 纯JS目标 | 可调用 assert 和 ok 共享 truthy 断言逻辑。 | [assert.js:30](../packages/node-compat/src/runtime/assert.js#L30) |
| `assert.rejects` | 中 | 基础支持 | 不需要 | JS语义 / 已具备 | JS → 纯JS目标 | 支持 Promise 或返回 Promise 的函数，以及异步 rejection 匹配。 | [assert.js:126](../packages/node-compat/src/runtime/assert.js#L126) |
| `assert.strictEqual` | 中 | 基础支持 | 不需要 | JS语义 / 已具备 | JS → 纯JS目标 | 用 Object.is 比较，覆盖 NaN 相等和正负零不同的 Node 常见严格比较语义。 | [assert.js:34](../packages/node-compat/src/runtime/assert.js#L34) |
| `assert.throws` | 中 | 基础支持 | 不需要 | JS语义 / 已具备 | JS → 纯JS目标 | 支持同步抛错及 Error 构造器、正则、对象或 predicate 匹配。 | [assert.js:106](../packages/node-compat/src/runtime/assert.js#L106) |

<a id="module-string_decoder"></a>

### string_decoder

Node 参考：[官方 string_decoder 文档](https://nodejs.org/api/string_decoder.html)。

| API | 频率 | 当前状态 | Native 能力 | 工作层 / 难度 | 本地 → 跨域目标 | 差距与依赖依据 | 源码 |
|---|---|---|---|---|---|---|---|
| `string_decoder.StringDecoder` | 中 | 缺失 | 不需要 | JS封装 / 低 | JS/浏览器库 → 纯JS目标 | 当前没有 NodeCompat 入口；可直接复用浏览器 string_decoder 包，Native 无新增；与 stream 共享依赖。 | [registration.js:56](../packages/node-compat/src/runtime/registration.js#L56) |

<a id="module-timers"></a>

### timers

Node 参考：[官方 timers 文档](https://nodejs.org/api/timers.html)。

| API | 频率 | 当前状态 | Native 能力 | 工作层 / 难度 | 本地 → 跨域目标 | 差距与依赖依据 | 源码 |
|---|---|---|---|---|---|---|---|
| `timers.clearInterval` | 高 | 缺失 | 不需要 | JS语义 / 低 | JS/宿主调度 → Web定时器子集 | 未注册 node:timers 模块；浏览器同名全局另列在运行环境附表，不计入模块入口覆盖。 Native：无 Niva Native API 依赖；基础调度复用目标 WebView 的浏览器 timer 原语，Node Timeout 的进程 keep-alive 效果不能据此实现。 JS：接受 wrapper/浏览器句柄并按 Node 规则清理。 | [registration.js:56](../packages/node-compat/src/runtime/registration.js#L56) |
| `timers.clearTimeout` | 高 | 缺失 | 不需要 | JS语义 / 低 | JS/宿主调度 → Web定时器子集 | 未注册 node:timers 模块；浏览器同名全局另列在运行环境附表，不计入模块入口覆盖。 Native：无 Niva Native API 依赖；基础调度复用目标 WebView 的浏览器 timer 原语，Node Timeout 的进程 keep-alive 效果不能据此实现。 JS：接受 Timeout wrapper 和浏览器 ID，幂等清理。 | [registration.js:56](../packages/node-compat/src/runtime/registration.js#L56) |
| `timers.setInterval` | 高 | 缺失 | 不需要 | JS语义 / 中 | JS/宿主调度 → Web定时器子集 | 未注册 node:timers 模块；浏览器同名全局另列在运行环境附表，不计入模块入口覆盖。 Native：无 Niva Native API 依赖；基础调度复用目标 WebView 的浏览器 timer 原语，Node Timeout 的进程 keep-alive 效果不能据此实现。 JS：返回 Node Timeout wrapper 并处理 refresh/ref/unref。 | [registration.js:56](../packages/node-compat/src/runtime/registration.js#L56) |
| `timers.setTimeout` | 高 | 缺失 | 不需要 | JS语义 / 中 | JS/宿主调度 → Web定时器子集 | 未注册 node:timers 模块；浏览器同名全局另列在运行环境附表，不计入模块入口覆盖。 Native：无 Niva Native API 依赖；基础调度复用目标 WebView 的浏览器 timer 原语，Node Timeout 的进程 keep-alive 效果不能据此实现。 JS：返回带 ref/unref/refresh/hasRef 的 Timeout wrapper。 | [registration.js:56](../packages/node-compat/src/runtime/registration.js#L56) |
| `timers.clearImmediate` | 中 | 缺失 | 不需要 | JS语义 / 中 | JS/宿主调度 → Web定时器子集 | 未注册 node:timers 模块；浏览器同名全局另列在运行环境附表，不计入模块入口覆盖。 Native：无 Niva Native API 依赖；基础调度复用目标 WebView 的浏览器 timer 原语，Node Timeout 的进程 keep-alive 效果不能据此实现。 JS：清除 JS immediate 队列中的句柄。 | [registration.js:56](../packages/node-compat/src/runtime/registration.js#L56) |
| `timers.setImmediate` | 中 | 缺失 | 不需要 | JS语义 / 中 | JS/宿主调度 → Web定时器子集 | 未注册 node:timers 模块；浏览器同名全局另列在运行环境附表，不计入模块入口覆盖。 Native：无 Niva Native API 依赖；基础调度复用目标 WebView 的浏览器 timer 原语，Node Timeout 的进程 keep-alive 效果不能据此实现。 JS：实现 immediate 队列、取消和相对 timer/microtask 的运行顺序。 | [registration.js:56](../packages/node-compat/src/runtime/registration.js#L56) |

## 5. 浏览器全局与加载前提（不混入模块/API 分母）

| 能力 | 当前状态 | 对现有 Node 代码的影响 |
|---|---|---|
| `URL` / `URLSearchParams` | `url` 模块复用 WebView 全局；已在 url 行计数 | 同一能力不重复算全局 API。 |
| `Buffer` | 选中 buffer 后，在没有全局 Buffer 时安装 | 必须完成 NodeCompat 注册；Buffer 方法已逐项计数。 |
| `process` | 当前未安装；目标为 main 可信顶层启动注入 | 子窗口/跨域不提供真实 process 对象。 |
| `setTimeout/clearTimeout/setInterval/clearInterval` | WebView 全局可用于普通定时器；无 Node timers 模块 | handle 为浏览器 ID；没有 Node Timeout 的 ref/unref/refresh 等语义。 |
| `setImmediate/clearImmediate` | NodeCompat 未提供 | 浏览器 setTimeout/queueMicrotask 不是完整的 Node 阶段调度替代。 |
| `queueMicrotask` | 依赖 WebView 原语 | 常用微任务可复用；不能代作 process.nextTick 的时序保证。 |
| `fetch` | 依赖 WebView 原语 | 受浏览器 origin/CORS/CSP 等规则约束，与 Node fetch 环境有差异；也不是 http/https 模块完整覆盖。 |
| `TextEncoder/TextDecoder` | 复用 WebView 原语 | 当前二进制适配层已使用；具体编码能力受宿主影响。 |
| `console.*`、`performance.*` | 使用浏览器现有原语 | console/perf_hooks 已移出 Node 兼容目标，不补模块封装。 |

加载本身也是兼容前提：启用 NodeCompat、选中模块、完成 `NivaNodeCompatReady`；ESM 裸名由 importmap/bundler 映射。当前 require 是注册模块查找，不提供 Node 的任意文件、node_modules、package.json 解析和完整 CommonJS 装载。模块/API 清单达标仍不等于可直接运行所有 npm 包。[加载实现](../packages/node-compat/src/runtime/registration.js)、[当前设计](node-compat-design.md)。

## 6. 用频率、Native 缺口与依赖成本选择范围

先从高频条目中选择 JS/浏览器库方案；确需系统能力或有实测性能缺口时，再区分 Native 复用与扩展；随后选择少量有明确收益的 Native 扩展。新增后端和大型依赖单独决策，避免将同一个共享后端的成本逐 API 重复累加。

| 候选工作 | 评估方式 |
|---|---|
| Native 已有的文件/进程/系统信息 | 复用 WS 或同步 XHR，以 JS 契约接线为主；Stats/rename/进程结果等具体语义差距另列。 |
| 标准流与已有 I/O 适配 | JS 流对象可复用成熟实现；Native 已有双向流时只计适配、背压、取消等差距，不再计算 WS 协议建设。 |
| Native 缺失的小型方法 | 定位所需原生字段/操作和跨平台分支，再估算中等扩展工作。 |
| Native 缺失的系统后端 | 服务端 socket、UDP 等按实际所需后端分别判断，不能只补同名 JS 方法。 |
| 大型第三方库或完整运行时 | 评为很高；先确认是否必须引入、能否复用现有依赖，并测量 release 增量。未选型只写条件，不声称已经测过体积。 |
| 跨域降级 | 只选择小型、简单、授权明确的 unary JSON 子集；没有跨域 Sync/原生流不构成本地 API 的实现缺口。 |

本表仍是范围决策清单，不承诺全部实现，也不把定性频率转换成“覆盖多少真实 Node 使用量”。选定范围后固定验收分母，并对 Native handler、JS 契约和适用平台分别留证。

## 7. 证据、验证与复算

本轮只改文档/盘点数据；本次重评补充了三通路、Native 能力、JS 封装与依赖成本，没有因此把计划接口改成已实现。核对了模块 exports、runtime registration、实现文件和已有测试记录，并以隔离 VM + stub Niva 做了契约探针：确认默认注册 15 个模块；process 模块/全局、timers 模块、fs.readFileSync、stream.Readable 缺失；os.platform 与 crypto.randomUUID 返回 Promise。另用本机 Node v24.10.0 对照了 Buffer 的 TypedArray 转换、UTF-8 BOM、slice 共享内存、copy 越界和 write 非字符串行为，均复现表中差异。探针验证 JS 入口和返回形态，不是原生平台测试；本机 Node 版本与文档参照版本分别记录，不混作同一套验收。

既有 [NodeCompat 测试矩阵](node-compat-test-matrix.md) 和 [macOS 运行记录](../examples/macos-api-smoke/RESULTS.md) 中的“185 项检查”，是已选实现及别名的测试清单；它不以本表 Node 候选为分母，不能替代本表覆盖率。Windows 已有有限记录，仍不能从 macOS 或 mock 测试推断 Windows 全量兼容。

本表的 Node 契约以本轮读取的官方 **v26.10.0** API 页面为参照（沿用旧调研的 v26 范围）。覆盖状态以本仓源码为准。旧调研中的 `readDirSync`、`mkdirAllSync`、`process.execSync` 不是相应 Node 标准 API 名：应分别比较 `fs.readdirSync`、`fs.mkdirSync({recursive:true})`、`child_process.execSync`；Node zlib 的 callback 接口也不能写成原生 Promise 签名。

机器可读的逐项清单：[node-api-inventory.json](node-api-inventory.json)。它保存模块归属、频率、难度、状态、原因和源码/测试定位，可用下面的命令复算：

```sh
python3 - <<'PYCOUNT'
import json
from collections import Counter
with open('docs/node-api-inventory.json') as f:
    data = json.load(f)
rows = data['apis']
assert len(rows) == len({r['api'] for r in rows})
print('modules:', len(data['modules']))
print('module entries:', Counter(m['entry'] for m in data['modules']))
print('APIs:', len(rows), Counter(r['status'] for r in rows))
print('high frequency:', Counter(r['status'] for r in rows if r['frequency'] == '高'))
print('Native:', Counter(r['nativeCapability'] for r in rows))
print('work layers:', Counter(r['layer'] for r in rows if r['status'] != 'supported'))
for f in ('高', '中', '场景'):
    print('difficulty', f, Counter(r['difficulty'] for r in rows if r['frequency'] == f and r['status'] != 'supported'))
print('Native:', Counter(r['nativeCapability'] for r in rows))
print('work layers:', Counter(r['layer'] for r in rows if r['status'] != 'supported'))
for f in ('高', '中', '场景'):
    print('difficulty', f, Counter(r['difficulty'] for r in rows if r['frequency'] == f and r['status'] != 'supported'))
print('Native:', Counter(r['nativeCapability'] for r in rows))
print('work layers:', Counter(r['layer'] for r in rows if r['status'] != 'supported'))
for f in ('高', '中', '场景'):
    print('difficulty', f, Counter(r['difficulty'] for r in rows if r['frequency'] == f and r['status'] != 'supported'))
print('Native:', Counter(r['nativeCapability'] for r in rows))
print('work layers:', Counter(r['layer'] for r in rows if r['status'] != 'supported'))
for f in ('高', '中', '场景'):
    print('difficulty', f, Counter(r['difficulty'] for r in rows if r['frequency'] == f and r['status'] != 'supported'))
for m in data['modules']:
    print(m['module'], m['apiCount'], m['counts'])
PYCOUNT
```
