# Node 子集的官方测试验收规则

> 历史基线说明（2026-09-25）：本页保留统一 runtime 重构前的范围与验收记录。文中的“当前”、模块/API 数量、通过数、体积和旧配置仅适用于对应历史快照，不代表 `codex/architecture-implementation` 的验证结果。新实现进度及重新验收证据见[架构实施台账](architecture-implementation-plan.md)；原始 JSON 证据保持不变。

> 2026-09-24 决策：Niva 做 Node API 的严格子集。纳入承诺的调用契约必须通过对应的 Node 官方测试；只有相同名称、基本功能或自有 smoke 通过，不算 Node 兼容。

## 当前固定集合

固定门禁现有 **58 个未修改的 Node v22.14.0 官方测试文件、22 个模块族**：48 个纯 JS 契约和 10 个 Native/网络契约。本次 darwin/arm64 按用户确认的适用范围运行：**58 pass / 0 fail / 0 unsupported，另记录 2 处环境检查点跳过**，门禁退出码 0。失败、unsupported、未运行和无结果都保留在固定 58 项分母中，不会通过删文件或重挑测试降分母。最终逐文件结果见[统一报告](node-compat-upstream-results.json)。

扩展前的 **30/30 pass、0 fail、0 unsupported** 是 Node v22.14.0 darwin/arm64 上旧 30 文件集的历史记录，时间为 `2026-09-24T10:17:12Z`，manifest SHA-256 为 `7f60ad27203feed67c223519d881b1337eb3e5618846080360c63b057d375a71`。[旧 30 项完整报告](node-upstream-initial-selection-results.json)保留原始证据。该批次只覆盖 path、buffer、events、querystring、string_decoder，不能外推到新增的 18 个纯 JS 文件或 10 个 Native 文件。更早的[首次运行记录](node-upstream-conformance-baseline.md)保留 5/22/3 的失败/阻塞基线。

运行器有 **27 个 fail-closed 自检**，包含两处豁免的精确替换/禁用/篡改校验、20 KB 结果完整传回、子测试 pass/fail/pending/skip、未完成/多次调用/漏调、异步异常、unknown helper/internal/fixture、吞掉 unsupported、source 校验和提前退出。自检只证明运行器行为，不是 API 兼容测试结果。

### 用户确认的两处环境豁免

2026-09-24，用户明确要求跳过这两处 Node 专属环境差异。版本化[豁免清单](../packages/node-compat/upstream/environment-exclusions.json)只包含：

1. `test-util-format.js:196–199`：移除原型后仍要求显示 Foo 构造器名的那一次检查。
2. `test-assert-deep.js:1365`：两个不同原生 CryptoKey 对象按内部密钥材料比较相等的那一次辅助检查调用。

不跳过文件、模块或整个 Crypto 子测试。运行器校验豁免清单 SHA、上游文件 SHA、精确位置与原文，执行时只把这两处调用换为记录标记，其他字符/行号/断言保留。每个标记必须恰好到达一次；报告显示实际到达的两处，而非将它们伪报为断言通过。其他 `skip`、未知 helper 或新失败依然阻断。

原始上游文件仍逐字节保留。[不应用豁免的报告](node-upstream-unfiltered-results.json)为 56/58；使用 `--include-environment-specific` 可重跑。下面一般“不跳过”规则有且仅有本节两项经用户授权的例外。

## 基准与承诺单位

首批上游测试固定为 **Node v22.14.0**，与仓库 CI 的 Node 22 主版本一致。它是可复现的测试语义基线，不是“最新 Node”，也不代表 Niva 携带或运行该 Node 版本。完整上游 commit、文件来源与 SHA-256 由 `packages/node-compat/upstream/` 清单记录。升级基线必须单独审查用例差异并重跑，不能跟随 `main` 自动变化。

本轮固定的上游提交是 `5d2feb257bcee090e57900eb51720171a6aa92f3`。CI 测试宿主也固定到 22.14.0，避免宿主断言工具随主版本漂移；该版本仅用于测试，不是生产环境 Node 版本建议。

子集按 **模块 + API + 签名/选项 + 平台/运行作用域** 定义，不按模块名称计数。`fs.readFile(path, options, callback)`、`fs/promises.readFile(path, options)`、`fs.readFileSync(path, options)` 是三个契约。同步改 Promise、忽略选项、回调次数或时序变化，都不能算原契约已兼容。

既有 22 个模块族、179 项 API 清单仍是产品目标，不因第一批测试只覆盖其中一部分而缩小。清单中的“基础支持”“部分兼容”是实现盘点状态，不是官方测试验收状态。新增门禁不会自动把这些条目升级为已验收。

## 官方用例如何进入子集

1. 先依据 API 契约选择相关官方测试及依赖 fixture，再运行。不得只留下碰巧通过的测试。
2. 优先保存完整上游测试文件，保留原断言、预期值、版权和许可。运行入口、模块解析、测试辅助设施可以适配，产品语义不能适配。
3. 一个文件混合了范围内外的能力时，初期将整文件标为待适配或未覆盖，不能删除失败断言后称整文件通过。后续若按独立用例拆分，必须保留上游定位、拆分理由及全部范围内断言，单独报告为派生测试。
4. `require('path')` 和 `require('node:path')` 等被测入口必须解析到 Niva 实现；不能回退到宿主 Node。断言工具属于测试基础设施，可以使用独立可靠的实现，但这不构成 Niva `assert` 的验收。
5. `test/common` 仅补测试实际需要且语义明确的辅助函数。未知模块、未知 helper、未知 fixture、原生内部绑定、要求跳过、超时和未捕获异步错误都必须显式报告，不能用空函数或自动通过掩盖。
6. `mustCall`、`mustNotCall` 等必须检查实际调用次数。每个测试隔离运行，设置超时；不能仅因同步脚本执行完毕就认定异步测试成功。

## 门禁与状态

| 状态 | 含义 | 能否声明该契约已兼容 |
| --- | --- | --- |
| pass | 本次环境下选定官方文件的原断言全部通过 | 仅是相应用例证据；还需完整范围映射和目标平台验证 |
| fail | 断言、加载或异步执行失败 | 否；纳入门禁的失败必须阻断 |
| blocked / unsupported | runner 尚不能忠实运行这个用例，或所需能力未提供 | 否；不得计为通过 |
| not covered | 对应官方用例尚未接入 | 否；有实现也不能自动视为兼容 |
| out of scope | 契约在运行前已明确排除，附理由 | 不在承诺中；不得混入通过率 |

运行器会拒绝非 v22.14.0 的测试宿主。机器报告必须保留固定用例分母、各文件结果、失败原因和实际运行平台。选定集合中任何失败、阻塞、跳过、丢失结果或源文件校验失败，门禁均应非零退出。不得以降低分母、允许失败、提高容错阈值来维持绿色。

一个 API 的几个用例通过不等于整个模块已验收。宣布某个契约兼容之前，还要审查该契约的所有相关上游用例及未覆盖原因，包含非法参数、错误类型/错误码、返回形态、Buffer/编码、回调/事件顺序和资源生命周期等适用维度。

## 执行边界

**纯 JS 48 项**由固定 Node v22.14.0 host 加载未修改的 CommonJS 上游源文件；`require()` 将被测 builtin 路由到 Niva JS adapters，断言、test/common 中严格白名单的 helper 和 test realm 属于测试设施。它们证明 Node-host 下的 JS 适配器契约，不证明 WebView 或 Native 路径。

每个原始用例都在独立 Node 子进程中运行，host flags 为 `--expose-internals --no-warnings --unhandled-rejections=strict`。这些是测试运行器参数，不是生产运行时能力。

**Native 10 项**仍由固定 Node host 加载原始测试、执行原始断言；不是把 Node 的 CommonJS 测试直接塞进浏览器执行。Runner 将 Niva API 代理接到一个独立真实 Niva WebView relay：`Niva.callSync`、`Niva.call`、`Niva.stream` 和 `streamSend` 经 relay 调用当前 release/debug binary 的 Native handlers。产品 fs、socket、TLS、DNS、进程等行为不使用 Node host fs/net/mock 代替。Relay 使用一次性 profile/resource 和 loopback token，测试间隔离。

每个 Native 结果都必须同时记录固定 Node 版本、平台、relay binary 路径与 hash、原始 test 路径、测试辅助设施，以及 host 和 Native 两侧的失败信息。Node `assert` oracle 只是断言实现，除 assert 本身被测的文件外，不能据此声称 Niva `assert` 已兼容。

现有 `examples/node-compat-macos-smoke/` 是另一套自有集成测试，不等于运行了 Node 官方测试。Niva 特有的 origin/权限、帧隔离、模块加载、CSP、打包与资源清理仍需自有测试，上游套件不会替代它们。

## 接入顺序

固定 manifest 有 58 个原始文件：纯 JS 48 项加 Native relay 10 项。其模块映射和准确场景见[用例索引](node-test-case-index.md)，原始路径与 hash 见版本化 manifest。它只验收选中的契约，不表示 22 个模块或 179 个 API 全部兼容。

仓库根目录使用 Node **v22.14.0** 运行。先验证宿主版本，并在需要时构建 vendor 和做 25 个运行器自检：

```sh
node --version # 必须输出 v22.14.0
npm run build:vendor --workspace=packages/node-compat
npm run test:upstream:harness --workspace=packages/node-compat
npm run test:upstream --workspace=packages/node-compat -- --suite js --report upstream-js-results.json
env NIVA_UPSTREAM_BINARY="$PWD/target/release/niva" npm run test:upstream --workspace=packages/node-compat -- --suite native --report upstream-native-results.json
env NIVA_UPSTREAM_BINARY="$PWD/target/release/niva" npm run test:upstream --workspace=packages/node-compat -- --suite all --report upstream-all-results.json
# 诊断：恢复两处引擎检查，预期两个原始文件失败
env NIVA_UPSTREAM_BINARY="$PWD/target/release/niva" npm run test:upstream --workspace=packages/node-compat -- --include-environment-specific --report upstream-unfiltered-results.json
```

`--suite js` 运行纯 JS 48 项；`--suite native` 运行 Native 10 项，必须通过命令作用域变量 `NIVA_UPSTREAM_BINARY` 指定要验收的 Niva binary。该变量只传给这次测试命令和子进程，不持久化到用户环境。默认 `--suite all` 使用同一固定 58 项分母，任一失败都必须非零退出。各命令的报告写入 `packages/node-compat/` 下指定的文件；文档汇总报告链接为 [`node-compat-upstream-results.json`](node-compat-upstream-results.json)。

CI 的纯 JS upstream job 在 macOS 与 Windows host 执行 `--suite js`；macOS release binary 执行 Native relay suite。**Windows 没有 Native upstream relay/真机验收**，Windows target compile 或纯 JS host pass 不能替代。CI workflow 已配置不表示远端结果通过，也不自动配置 GitHub 分支保护规则。

## 已知选样限制

- `test-tls-connect-no-host.js` 的上游 CA-as-leaf 证书已通过真实 Native 测试；新路径保留当前时间、名称、用途、信任和握手验证，没有修改该 273 年 fixture 或关闭校验。
- `test-http-client-get-url.js` 所需的旧式 URL API 已由纯浏览器库实现，原文件完整通过。
- `test-https-options-boolean-check.js` 只检查 `https.createServer` 输入验证，不监听、不握手；`test-dns-resolvens-typeerror.js` 只检查 resolveNs 参数错误，不发 DNS 报文。它们不替代 HTTPS Native 握手和 DNS Native 查询覆盖。
- `test-child-process-execfile.js` 用 `process.execPath` 启动子进程。relay 测试需由显式 test-only fixture 提供 Node v22.14.0 子进程路径；不能静默改写 Niva 产品的 `process.execPath` 并把该字段算作通过。

选择中遇到内部 Cares patch、自签名证书且需要关闭校验、依赖未列入目标的 API 等情况时，原因记录在选择/结果报告里。修复必须在不改上游文件和断言的前提下完成。

最终对外口径是：“Niva 在列明的平台和作用域中，通过固定 Node 版本的这些 API 契约测试。”不使用“全部 Node 兼容”或笼统的“模块已支持”替代可核查的范围。

## 本轮修复与运行器边界

- path / querystring 对齐路径边界、动态 cwd、参数错误、编码和非有限 maxKeys；真实 Niva main 的 process.cwd 覆盖也会被 path.resolve 读取。
- events 模块默认导出可构造的 EventEmitter；once 支持 EventTarget、AbortSignal、清理及阻止原 abort 事件传播时的取消。
- Buffer / StringDecoder 对齐错误码、范围检查、copyBytesFrom、base64url、损坏 UTF-8 分块与大字符串限制。
- 扩展清单的 58 个官方测试与所选 fixture 都按 pinned commit 校验 SHA-256；原 30 项的原文件/断言保持在 baseline manifest 中。运行器同时硬校验扩展 manifest 的 SHA-256 与 58 项固定数量，不能只改 manifest 数量缩小门禁。固定分母为 58，不移除失败断言，也不把 skip/unsupported 计为通过。
- 除 assert/util 自身被测时映射 Niva 实现外，host assert/util 用作断言及诊断工具；vm、调度、当前 cwd/env、内存容量、预期错误构造器和 EventTarget 内省 Symbol 是显式测试基础设施。被测模块均指向 Niva 适配器。util 的 stat fixture 和子进程 fixture 明确单列；Native FS/网络操作不替换为 host API。
- buffer 内部边界用例调用 Niva 的 bufferBinding.fill 共享校验/填充路径；host 子进程 helper 只运行清单中的固定 fixture，其 path/util 加载 Niva；child_process 被测 API 经真实 Native 启动外部程序。未知 helper / binding / fixture 会阻断，即使测试捕获了异常。

原 30 项阶段的修复和运行记录是历史实现证据，不会自动升级新增 18 项纯 JS 或 10 项 Native 契约的状态。两处引擎差异已按用户要求单列豁免；适用断言门禁已通过，不等同于全部 Node/V8 行为兼容。

## 原 8 项失败的处理结果

| 官方文件 | 本次结果 |
|---|---|
| test-util-format.js | 适用检查通过；仅该 Foo 标签检查按授权跳过。诊断续跑 189 个断言，188 通过；诊断不冒称原始无豁免通过 |
| test-assert.js | 通过，包括 18 个子测试；修复诊断、错误匹配及调用表达式恢复 |
| test-assert-deep.js | 适用检查通过；仅相同材料原生 CryptoKey 的比较调用按授权跳过，其他检查继续执行 |
| test-zlib-kmaxlength-rangeerror.js | 通过；产品模块注册表与 ESM 共用首次加载初始化 |
| test-child-process-execfile.js | 通过；真实 Native 执行、close/负状态、AbortSignal 及流结束语义 |
| test-http-client-get-url.js | 通过；补齐纯 JS 旧式 URL 解析 |
| test-https-options-boolean-check.js | 通过；只代表构造和参数验证。多套身份的 listen 仍明确 ENOTSUP |
| test-tls-connect-no-host.js | 通过；真实 Native、原证书原断言、主机名与信任检查保留 |

[引擎边界与可复现证据](node-webview-engine-boundaries.md)列出不能用普通 WebView JS 补齐的两处。仅调整了用户明确授权的两处环境豁免。默认适用范围门禁为 **58/58 + 2 处跳过**，退出码 **0**；无豁免原始运行仍是 **56/58**，两种结果分开保存。

本次自有 JS 回归 **109/109**、打包 WebView **45/45**、Rust **120 + 10**。fmt、workspace check、Clippy、Windows target check、TypeScript/Vite 均通过；Clippy/Windows 编译有现存 warning，Windows 真机与远端 CI 未验收。完整 macOS arm64 主程序 **2,772,208 bytes**，剩余 **527,792 bytes**，含 Native、内嵌 JS、索引及加载器。

测试 relay 每个文件使用独立进程、目录与端口。退出清理是有界异步清理，进程可能短暂重叠；未以此证明全局资源零重叠。
