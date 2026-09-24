# Node 模块补齐难度与新增体积预算

> 本页保留实施前预算。当前 macOS arm64 主程序（含内嵌 JS/索引）实测 **2,772,000 bytes**；功能检查、官方契约和 Windows 边界见[实施记录](node-compat-implementation.md)，不是所有 Node 契约均已验收。
> 2026-09-24 · 当前 22 模块 / 179 API 范围 · KiB = 1024 bytes。Native 非零区间为未实测的工程预算；JS 库样本另列实测。

## 1. 每模块预算

默认JS/浏览器库，Native预算只用于必要系统能力或有实测依据的性能缺口；本轮没有认定新的JS性能失败。`fork` 已放弃，不计Node宿主体积。

“整体难度”取该模块当前目标中最难的剩余部分；具体 API 仍保留自己的难度。以下 Native 增量面向当前 macOS arm64 release 配置；Windows 未测，只能参考量级。JS 一栏为**完成浏览器 bundle/minify 后、raw DEFLATE 压缩资源的新增预算**，不是 npm 安装目录或运行内存。

| 模块 | 整体补齐难度 | Native 代码预计增加（KiB） | JS 压缩资源预计增加（KiB） | 最大工作/预算条件 |
|---|---|---:|---:|---|
| `fs` | 高 | 24–180 | 2–8 | watch、FileHandle 和 Native 文件语义补齐；常用读写已有基础 |
| `path` | 中 | 0 | 1–5 | matchesGlob 与路径边界 |
| `events` | 中 | 0 | 1–4 | 异步迭代器、captureRejections 等 JS 语义 |
| `process` | 高 | 0–20 | 2–6 | 静态注入简单，完整 stdio/生命周期成本较高；仅 main |
| `http` | 中 | 0（TCP计入net） | 8–20 | HTTP/1.1请求/响应parser及客户端/服务器对象全部JS |
| `https` | 中 | 0（TLS计入tls） | 2–6 | 共享http协议，只将TCP流换为Native TLS流 |
| `util` | 中 | 0 | 1–6 | 复杂对象比较/inspect 语义 |
| `os` | 中 | 8–80 | 1–3 | 静态注入；动态 CPU、内存、网络信息需采集 |
| `crypto` | 中 | 0–8 | 9–14 | 7 项 JS/WebCrypto 低；严格 timingSafeEqual 单独处理 |
| `child_process` | 中 | 0–32 | 2–8 | 普通进程/Sync/信号适配；fork已放弃，不带Node宿主 |
| `stream` | 低 | 0 | 35–42 | 现成浏览器 JS 库；实际 I/O 后端计入 fs/net/http 等模块 |
| `url` | 已具备（本表） | 0 | 0 | 本表 4 项已基础支持 |
| `buffer` | 低 | 0 | 9–12；共用 stream 时 0–2 | 现成浏览器 JS 库替换/适配 |
| `querystring` | 已具备（本表） | 0 | 0 | 本表 2 项已基础支持 |
| `net` | 高 | 16–120 | 3–10 | 新增应用 TCP/socket 后端与生命周期 |
| `tls` | 高 | 8–72 | 2–6 | 复用 native-tls，补原始 TLS socket/server |
| `dns` | 中 | 0–24 | 10–14 | 系统lookup/配置薄封装；JS dns-packet复用dgram/net |
| `dgram` | 高 | 8–48 | 2–5 | 新增 UDP API 与事件生命周期，复用 smol |
| `zlib` | 低 | 0 | 6–9 | 现成 fflate/browser，Native 默认不增加 |
| `assert` | 中 | 0 | 0–2 | 剩余主要为 deepStrictEqual 与 util 共享语义 |
| `string_decoder` | 低 | 0 | 10–12；共用 stream 时 0–1 | 复用 readable-stream 配套的纯 JS string_decoder |
| `timers` | 中 | 0 | 1–4 | 封装浏览器调度；不引入 Node 运行时 |

这些是基于已读源码、现有依赖和具体方案的预算区间，不是已测得的补齐后增量；Native 尚未实现，不能当作上界保证。watch/OS 信息收集等若改用另一套大型库，需要重估。JS 预算按加入候选库及封装留量估算，未提前扣除删除旧实现可能节省的体积。`url/querystring` 的 0 仅对应本表已支持范围，不代表未来扩展永远零成本。

**DNS 的原未定大库项已改为 JS 路线：**

- 采用 dns-packet 5.6.1 codec，复用现有规划中的 dgram/net。0–24 KiB 为系统 lookup 与 DNS 配置薄封装的预算，10–14 KiB 为共享后的 JS codec 加查询/结果包装预算；均仍需最终集成验证。

## 2. 公共成本与共享依赖

| 公共工作 | Native 预算（KiB） | 计费方式 |
|---|---:|---|
| 同步 XHR 的公共请求分发/鉴权/序列化 | 8–40 | 仅计一次，不在每个 Sync API 或模块重复加入 |
| 静态信息收集与定向启动注入 | 2–12 | 仅计一次；process 只 main；动态信息仍 XHR 查询 |

公共层仍是既定方案，复杂度不因本次大小预算上调。这里给尚未生成的机器码留量，并不是重新设计传输。JS 通用加载/注册改动另预留约 1–4 KiB 压缩资源，只计一次。

不能直接将每行上限相加作为最终包大小：stream 已包含 Buffer/StringDecoder 依赖，http/https、net/tls 共用协议基础，assert 与 util 也共享实现。若同时采用标准流方案，Buffer/StringDecoder 应优先按去重后的差额计费。

## 3. 浏览器库打包实测

测量条件：esbuild 0.28.2，browser ESM、ES2022、minify，gzip/raw DEFLATE 均 level 6；没有 sourcemap。压缩测量使用本机 Node 的 node:zlib；不同 zlib/pako/flate2 实现即使同为 level 6 也可能产生不同字节数，因此这里只是受控模型，最终资源必须由实际打包器复测。测量使用临时项目，未改仓库依赖。版本：`@noble/hashes 2.4.0`、`buffer 6.0.3`、`readable-stream 4.7.0`、`string_decoder 1.3.0`、`fflate 0.8.3`。

| 样本 | minified JS bytes | raw DEFLATE bytes | gzip bytes |
|---|---:|---:|---:|
| crypto 所选摘要/HMAC/KDF | 19,920 | 8,344 | 8,362 |
| 独立 Buffer | 28,404 | 8,750 | 8,768 |
| 标准流（含其 JS 依赖） | 118,698 | 35,849 | 35,867 |
| 独立 StringDecoder（含依赖） | 32,967 | 9,838 | 9,856 |
| gzip/gunzip 同步与异步 | 11,483 | 5,583 | 5,601 |
| stream + StringDecoder 导出 | 118,796 | 35,898 | 35,916 |
| 所选库共同打包 | 150,726 | 49,921 | 49,939 |
| 现有 NodeCompat 全量 ESM（对照） | 73,017 | 22,627 | 22,645 |

这些是**候选库样本的总大小**，不直接等于最终净增。stream 已含 Buffer；组合 bundle 再显式导出 Buffer 只多 70 bytes，raw DEFLATE 后多 19 bytes。stream 显式增加 StringDecoder 导出的 raw DEFLATE 差额为 49 bytes；这是同版本同 bundle 的具体结果，不是任意构建通用常数。

检查确认 browser 入口生效、无外部 Node 内置模块导入、无 .node addon 输入；构建工具本身使用 Node/esbuild，不属于交付包运行时。这里只验证构建和字节数，未运行 Niva WebView 功能验收。上游方案与入口依据见 [浏览器纯 JS 方案](node-browser-js-options.md)。

## 4. 当前产物与资源追加模型

当前本地 `target/release/niva` 为 macOS arm64，**2,288,336 bytes**（约 2.18 MiB）；本轮读取并校验 SHA-256，没有重新编译，不能把它称为本次新增功能的构建结果。

当前 NodeCompat 资源闭包模型为 34 个文件：原始 277,765 bytes，raw DEFLATE 后 58,036 bytes；另有约 2,190 bytes 的索引模型。该模型不含用户页面、niva.json、图标和其他应用资源。

保留这批现有资源，在末尾追加一个候选 minified bundle 后重新 raw DEFLATE，测得如下**受控模型的资源增量**：

| 追加样本 | 压缩资源增量 bytes | 约 KiB |
|---|---:|---:|
| crypto 所选摘要/HMAC/KDF | 8,195 | 8.0 |
| 独立 Buffer | 8,436 | 8.2 |
| 标准流（含其 JS 依赖） | 35,543 | 34.7 |
| gzip/gunzip 同步与异步 | 5,532 | 5.4 |
| 所选库共同打包 | 49,585 | 48.4 |

组合样本的追加增量约 48.4 KiB；尚未包含新增 Node 签名封装，也未删除旧实现，不是“所有 22 模块补齐只加 48.4 KiB”。

当前 Devtools 是复制 classic 脚本和所选 ESM 资源闭包，再将资源拼接并 raw DEFLATE；**不会自动按本次实验设置 minify 所有输入**。库落地时需要准备正确的 browser bundle，不能直接复制整个 node_modules 后套用此预算。

- macOS：JS 压缩数据位于 `.app/Contents/Resources/RESOURCE_DATA`，与 Native 可执行文件分开。
- Windows：压缩数据写进 PE 的资源段，最终 EXE 也包含 JS 资源；不能把原始 JS 字节直接加到 EXE。
- Native 编译参数为 LTO、opt-level=z、单 codegen unit、strip、panic=abort；链接与平台差异会改变结果。安装器压缩、签名、索引、PE 对齐、图标、页面资源和运行时内存均不在逐模块表中。

打包依据：[macOS](../packages/devtools/src/build-scripts/build-macos.ts#L87)、[资源闭包](../packages/devtools/src/build-scripts/node-compat-assets.mjs#L61)、[Windows](../crates/win_packager/src/bundle.rs#L65)、[Native 解压](../crates/niva/src/app/resource_manager/mod.rs#L251)。

## 5. 复现与后续验收

精确的包版本、入口源码、构建参数、压缩参数、lockfile 快照、当前文件哈希、逐模块 Native 假设和预算依据保存在 [node-api-size-evidence.json](node-api-size-evidence.json)；模块预算同时写入 [node-api-inventory.json](node-api-inventory.json) 的 `sizeEstimate`。库样本数据可按其中的 entryTexts 和命令重新构建。

真正补齐后，再对固定平台/工具链做同一配置的前后 release 构建，分别比较 Native 文件及最终资源/应用包。当前数字用于选范围，不能据此宣布 3.3 MB 发布门禁通过；尤其网络生命周期及最终链接差分尚未实测。fork 已移出目标及预算。

## 6. 全部目标 Native + JS 内嵌后的体积条件

用户接受的上限为 **3.3 MB（3,300,000 bytes）**；现有 CI 与构建脚本按严格小于该值验收。统计对象是 **22 个模块、179 项目标 API 全部完成后的 Native 实现 + NodeCompat JS/第三方库 + 内嵌加载/索引/对齐**组成的完整 release 主程序。达标后直接采用内嵌方案，默认提供 Node 风格开发入口。

### 全范围预算合算

| 组成 | 预算 bytes | 依据 |
|---|---:|---|
| 现存 Native 文件 | 2,288,336 | macOS arm64 文件实测，未重新构建 |
| 当前 NodeCompat 压缩资源及索引 | 60,226 | 58,036 + 2,190，受控资源模型 |
| 补齐目标模块的 Native 增量 | 65,536–598,016 | 64–584 KiB，TCP/UDP/TLS各计一次，HTTP协议不另建Native后端 |
| 补齐目标模块的 JS 压缩资源增量 | 90,112–179,200 | 88–175 KiB，已计HTTP parser、DNS codec和JS封装预算 |
| 公共 XHR/启动注入 Native 成本 | 10,240–53,248 | 10–52 KiB，仅计一次 |
| 公共 JS 加载/注册成本 | 1,024–4,096 | 1–4 KiB，仅计一次 |
| **已量化预算项的合计** | **2,515,474–3,183,122** | **约 2.52–3.18 MB（十进制）** |

这是预算项的算术合计，不是完整实现测量或保证上界。按新的 socket 基座 + JS 协议层，上端距 3,300,000 bytes 约 **116,878 bytes（114.1 KiB）**。相比旧方案减少了重复的 Native HTTP/HTTPS 增量预算，并增加 JS parser/codec/生命周期包装预算；没有提前抵扣删除 ureq 可能节省的空间。

因此新分层在预算上更有机会满足 3.3 MB，但仍须完成全部目标后逐平台实测。Native socket/系统DNS接线、JS HTTP严格解析与对象生命周期、最终内嵌/链接/对齐会影响结果；macOS arm64 的预算不能直接当作 Windows 或 x86_64 的测量。[分层依据和小库实测](node-layering-plan.md)。

### 约 2.40 MB 仅是局部样本

先前的 **2,398,147 bytes** = 当前 Native 2,288,336 + “现有资源追加所选库”的压缩模型 107,621 + 原索引 2,190。它没有包含全部目标 Native/API 封装，**不能用于回答完整 179 项功能是否低于上限**。此数字只保留为库资源成本参考。

最终按完整目标候选进行逐平台 release 构建并计量，满足 3.3 MB 门禁后直接内置 NodeCompat。不能通过遗漏目标功能、只量裸 Native、把必需兼容层外置排除统计来宣布达标。应用自行选择的业务页面/npm 依赖另按应用资源计算；这里不引入 Node 运行时，fork 已放弃。
