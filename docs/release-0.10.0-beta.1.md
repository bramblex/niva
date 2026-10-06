# 0.10.0-beta.1 定版计划与验收

状态：首轮候选准备和验收已执行，仍有发布阻断，未发布。起始源码：`aabdc425855c9e04ef5afbce423a5f0e8855bbfa`。

## 冻结范围

本候选冻结现有 Niva 原生 API、统一 TypeScript runtime、独立可选的 CommonJS/ESM 注入、IPC 稳定异步通道与可选 WebSocket 优化通道，以及 GUI/CLI 共用打包核心。同步 XHR 仅用于 Node 同步兼容接口。

不将完整 Node 兼容、移除 WebSocket、IPC 零拷贝、完整跨平台验收作为已有能力。IPC 二进制边界仍使用 Base64。CJS 不支持 `require(ESM)`；RWA 的相关失败保留为兼容边界，不修改上游项目掩盖失败。

## 执行顺序与责任

1. Luna Deep `acceptance_evidence`：版本字段、平台版本格式、锁文件与打包路径收口；针对性检查。
2. Luna Deep `release_gates`：发布文档和 CI 门禁收口；历史证据与当前验收分开。
3. 主线程：审阅两包差异，处理跨包决策，冻结候选文件集。
4. Luna Fast `candidate_validation`：在冻结文件集上执行构建、测试和可用平台 smoke，记录源码差异指纹、runtime 指纹、产物 SHA256、平台和体积。
5. 失败按文件所有权交回对应执行者修复；修改相关输入后重跑受影响检查。主线程复核证据并记录可发布范围及剩余阻断。

失败诊断分包：Luna Deep `release_gates` 负责 HTTPS 环境对照及 Native 契约失败诊断；Luna Deep `intel_size` 负责不改默认配置的 Intel 体积隔离实验；`acceptance_evidence` 继续负责 Windows 隔离验证。

各包不提交、推送、打标签或发布；不修改用户全局环境。若设备、凭据或网络不可用，记录具体阻断，不冒称通过。

## 验收清单

- [x] Native、runtime、types、Devtools、packager 与锁文件版本一致；平台元数据格式合法。Mac 数字版本为 `0.10.0`，Windows 固定版本为 `0,10,0,0`，Windows ProductVersion 保留完整候选号；现有与候选映射单测通过。
- [x] macOS 本机 `cargo fmt --all -- --check`、`cargo check --workspace`、`cargo clippy --workspace --all-targets`、`cargo test --workspace`。Windows 测试失败另列，不外推为跨平台通过。
- [ ] Windows MSVC target check；单独记录与 Windows 真机验证的区别。
- [x] runtime build/test（157 项）、types 检查及三种消费者配置、fresh-pack 独立安装验证、Devtools build、API 契约测试（4 项）。
- [ ] 同一候选完整 release 构建、runtime 指纹及 SHA256；各平台主程序严格小于 3,300,000 bytes。macOS 3,000,000 bytes 为参考目标。
- [ ] 可用 macOS 真实 WebView：bootstrap、stdio/退出清理、可信 IPC 与远端授权矩阵；窗口可见性/锁屏限制如实记录。
- [ ] 打包核心与本机 kit/业务应用 smoke；三平台完整 kit 需齐备目标产物，不能由单机样本代替。
- [ ] Windows 当前候选真机：来源/权限、文件 token、菜单/快捷键/owner 与打包启动。
- [ ] 远端 CI 对应候选提交的成功记录。
- [ ] 文档中的已知限制、平台范围及测试结果一致。

## 本轮证据

证据目录：`/tmp/niva-release-0.10.0-beta.1-20260928/`。本轮在基线提交加候选未提交差异上验证；该目录记录 HEAD、源码差异指纹、工具版本及各命令日志，尚不构成已提交的发布版本。历史快照仅用于选择验证入口，不计为本候选通过记录。

- runtime fingerprint：`55b8d5335b0fc709da9b4dd23d51cc402574306b9c78db86a519e9b07931f676`；源码构建生成 182 个 runtime artifacts，bootstrap 原文 619,125 bytes。
- Rust 本机 fmt/check/clippy/test 通过；保留现有 warnings，未绕过检查。
- Windows 交叉检查：`cargo check --target x86_64-pc-windows-msvc -p niva` 通过。完整 workspace 交叉检查失败于 C 依赖 `ring`/`lzma-sys` 缺少 `assert.h`/`stdlib.h`；不能把 runtime 单包通过写成 workspace 全通过。Windows 原生隔离验证另行记录。
- Devtools 构建保留现有 chunk size warning。

### 已冻结的本机构建

同一 runtime fingerprint 下的完整主程序（不含业务资源及独立 packager）：

| 目标 | bytes | SHA256 | 体积门禁 |
|---|---:|---|---|
| macOS ARM64 | 2,994,968 | `59ed77dfe5e4008417a10c9f2a43695940e375f8823f040562821243fb5f5a44` | 通过 |
| macOS x86_64 | 3,569,184 | `7cb7f8420b07b32230f17b1078ed67edf191b30fd3db1b8864f53fa7874b1f50` | 失败，超过 3,300,000 |
| Windows x86_64 | 3,403,776 | `fc1d0811c5e6ff18874b9de229082fc136d7b32d949dde5993216426cd1a22d4` | 失败，超过 3,300,000 |

产物保存在证据目录的 `frozen-arm64/` 与 `frozen-x86_64/`，各自的 `artifact-record.txt` 另列 packager 体积与 hash。两份主程序均执行项目规定的 `strip -N`。Intel 构建通过不等于 Intel 设备运行验收。

### 本轮真实运行与契约结果

- ARM64 冻结主程序：bootstrap 5 种配置、stdio 往返、process-exit 子进程清理通过。
- embedded/external 两种 resource-layout smoke 通过，覆盖大资源 Range/416/超限拒绝、许可证、CommonJS 相邻资源和 ESM identity。
- 使用缓存中的精确 Node v22.14.0 作为测试宿主：初始 upstream harness 28/28；修复测试框架后新增 fail-closed 回归，最终 29/29。共享 runner 修改后 JS suite 重新执行 48/48 通过；保留原有两处获批准的环境排除，不额外放宽断言。
- Native suite 初始 8/10：文件测试失败发生在测试框架初始化临时目录，尚未执行上游断言。已修复真实本地 relay 的信任状态传递，仅 ready 字段严格为 `true` 才允许测试适配器使用同步文件 API；缺字段/false/非布尔值回归全部通过。生产 runtime 和 Rust 授权未修改。
- 同一冻结 ARM64 binary 上修复后 Native suite **9/10**，typed-array 写入用例通过；唯一失败 `test-process-chdir.js` 是 Node 同步调用后立即检查 cwd，而当前 Niva `process.chdir` 明确为异步 IPC。保留真实失败，不替换 host API、不改上游断言、不新增排除。新报告为 `upstream-native-results-trust-fix.json`、`upstream-js-results-trust-fix.json`；测试脚本修改不影响 runtime fingerprint。
- 可信 IPC 与远端授权 IPC 的完整运行各有一项 HTTPS 失败，其他文件、HTTP、二进制 channel、进程与 lease 检查通过。两份报告均绑定 ARM64 冻结 hash；默认 `https://example.com/` 请求返回 `os error 35`。独立对照中同机 curl 对两个受信网站均在 TLS 握手阶段超时，尚未到证书校验，当前倾向出站 TLS 环境阻断；这不是该用例通过的证据，环境恢复后仍须复跑。

### Windows 原生隔离验证

通过既有 SSH `windows` 连接，在全新 `D:\Temp\niva-release-0.10.0-beta.1-20260928\source` 中验证，未覆盖原 `D:\Workspace\niva` 工作区。使用已安装的固定 Rust `1.98.1`，不修改默认工具链或全局环境。runtime fingerprint 与本机相同。

远端原始日志已回收至本地证据目录的 `windows/remote-evidence/`，包含 workspace check/test/release、统一打包器和两项 stdio 结果；快照清单保存在 `windows/remote-source-snapshot.json`。

- `cargo +1.98.1 check --workspace` 通过，包含 Niva、packager、win_packager 和 icon_creator；有既有 warnings。
- `cargo +1.98.1 build --release --workspace` 通过。完整 `niva.exe` 的体积/hash 见上表；`niva-packager.exe` 为 4,936,704 bytes，SHA256 `1fcc7b14928c09da23f78d11693faa38ec70072ae274cf6366cd0b283817fe53`，`--version` 返回 `niva-packager 0.10.0-beta.1`。
- 统一 `niva-packager build` 使用候选版本及真实 runtime SHA 后，因 `runtime exceeds the strict 3300000 byte size limit` 拒绝打包。该发布阻断是实际调用结果，未改 manifest 大小或绕过检查；后续低层 fixture 的 Native smoke 即使通过，也不能替代统一打包器验收。
- `cargo +1.98.1 test --workspace` 失败：Niva 175 passed / 3 failed / 2 ignored；该失败阻止完整 workspace 测试继续，不能认为其余包测试也通过。
- `node_cp_preserves_file_timestamps_when_requested`：Win32 Access denied。
- `request_text_rejects_an_untrusted_tls_certificate`：未找到用于生成测试证书的 OpenSSL。
- `request_text_cancellation_closes_an_active_socket`：未满足取消小于 1 秒的断言。时间戳和取消失败的根因尚未定位，不能概括为环境问题。

`verify.py` 与 `verify_exit.py` 均实际运行并很快退出，未达到 ready/子进程启动阶段。两份应用日志记录 `WebView2 error: WindowsError(HRESULT(0x80070578), "无效的窗口句柄。")`。设备已安装 WebView2 Runtime，不能归因为未安装；本轮从 SSH 会话启动，尚未证明该失败在交互桌面会话同样出现。未改用计划任务等新启动路径，Windows bridge/菜单/快捷键交互 smoke 未运行。全部失败保留，不放宽断言。

### Intel 体积隔离实验

当前默认 profile 已有 `opt-level=z`、fat LTO、单 codegen unit、panic abort 与 strip。最多两项隔离配置实验均未达到门禁：Thin LTO 为 4,543,520 bytes；显式重复 Darwin `dead_strip` 为 3,565,088 bytes。链接参数证实 Rust 默认已启用 `dead_strip`，后者微小差异不能归因于冗余参数。两项都不采用，默认配置及冻结候选保持不变；后续需要单独的源码/依赖体积分析，不能继续靠更改门禁或削减功能放行。

### 当前未关闭项

1. Intel Mac 和 Windows 完整主程序超过硬体积门禁，分别超出 269,184 和 103,776 bytes；默认 profile 不变，隔离实验不能替代候选产物。
2. Native suite 仍有 `process.chdir` 一项同步 Node 契约失败；测试框架的信任状态问题已修复。当前 CI 的 Native 契约步骤会因此失败，不能标为全部通过。
3. HTTPS 正常证书请求尚无当前环境的成功结果。
4. Windows 原生 Rust 测试有三项失败；SSH 启动的 stdio smoke 在 WebView2 创建时遇到无效窗口句柄；统一打包被体积门禁阻断，完整三平台 kit 和交互矩阵未完成。
5. 远端 GitHub CI 未核实：`gh` 未登录，SSH/API 连接失败。未推送或触发发布。

## 发布条件

版本号用于候选准备，不表示发布验收已通过。对外发布前复核实际支持平台、对应产物及未关闭问题；v1.0 五项门禁继续独立保留。没有对应真机和 CI 证据的平台不得标为已验收。

## 下一阶段顺序

1. 优先定位 Windows 时间戳权限与取消时限失败，移除测试对未声明 OpenSSL 可执行程序的依赖或补齐明确的测试环境；不得直接忽略失败。
2. 针对 Windows/Intel 相同功能集合做源码和依赖体积归因，设计可维护的减量方案，再以三平台完整产物重验。现有两项 profile/link 实验已经排除，不重复尝试。
3. 明确官方 Node 同步 `process.chdir` 对照与本项目异步 API 的公开兼容边界；保留对照结果，不能在 CI 中静默跳过失败后宣称完整契约通过。
4. 在可用出站 TLS 环境复跑 HTTPS；候选源变化后重建并重新绑定指纹，补齐统一打包、交互矩阵及远端 CI。
5. 全部所声明平台门禁满足后，才进入标签、发布资产及对外发布环节。
