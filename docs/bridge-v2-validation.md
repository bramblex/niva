# Bridge v2 验收记录

本文记录 IPC 控制通道与可选 WebSocket 流数据通道的实现和验证证据。它不是 0.10.0-beta.1 全平台发布验收通过声明；源码实现、单测、macOS WebView 实测、发布产物和其他平台验证分别记录。未完成项目继续保持开放。

## 当前协议边界

- 异步 API 创建使用 IPC `t: "api_call"`。Native 通过 Wry `evaluate_script` 调用页面的 `__niva_ipc_reply` 回送带 session/rid 的结果。成功创建流后，由 reply 返回 `channelOpened` 与 capability；数据 lane attach 后才能传输流帧。
- IPC lane 通过 `channelAttach` 建立数据通道；WS lane 只能使用已有 ticket 的 `attach`、流数据、ACK 与 cancel。WS 不发送 API method/args，也不承担 API dispatch。
- WS hello 的 `v` 和 binary header version 均为 2。Binary header 共 18 bytes：version、flags、64-bit id 与 64-bit seq；payload 紧随其后。
- frame/session/origin 校验与流控制语义由 Native 侧处理。WS 是选定流数据操作的优化通道；同步 XHR 仍仅供需要同步结果的兼容 API 使用。
- 页面生命周期控制另走单向 IPC `session_close`：它不是 API，Native 按完整 session key 鉴权、取消旧 session 工作并保留 tombstone；不要求 reply。只有匹配的 persisted `pagehide`/`pageshow` 才在原页面 realm 内创建新的 session ID（密码学随机 nonce）。runtime 保留模块缓存、stdin 身份及监听器；旧调用取消、普通 Native 句柄失效、旧回调隔离，API 不重放。CSP/runtime configuration 的 `runtimeConfig.nonce` 保持不变；租约过期不能复活。

以上是当前实现的源码检查结论，不代表每个平台都已端到端验收。[协议定义](../crates/niva/src/app/api_manager/protocol.rs) 固定 wire version/header；[runtime bootstrap](../packages/runtime/src/bootstrap.ts) 创建 IPC API 请求、返回流 ticket 并按 ticket attach 数据 lane；[Native API manager](../crates/niva/src/app/api_manager/mod.rs) 处理 attach 与 WS session；[window builder](../crates/niva/src/app/window_manager/builder.rs) 通过 Wry 脚本执行送达 IPC reply。具体 smoke 的观测边界见 [bridge-route smoke](../examples/bridge-route-smoke/README.md)。

## 此前 HTTP/BFCache 验收快照（历史）

以下证据绑定 runtime fingerprint `0befa3f4ca6a5d151c54c3532ce226a01fe00c562d3a1abb87ba09f28c06f694`，仅记录此前 HTTP/BFCache 源码快照，不代表 2026-10-06 Windows review 后的当前源码。新快照证据见下节。

| 项目 | 当前证据 | 状态与限制 |
| --- | --- | --- |
| Runtime 与 Native 源码门禁 | 当前 runtime fingerprint `0befa3f4ca6a5d151c54c3532ce226a01fe00c562d3a1abb87ba09f28c06f694`；bootstrap 629,975 bytes。Runtime 177/177 与 build/typecheck 通过。最终 Native 源码的 `cargo fmt --all -- --check`、workspace check、clippy、workspace tests、debug/release build 均 exit 0；Rust log 分别为 `/tmp/niva-http-final2-fmt.log`、`-check.log`、`-clippy.log`、`-test.log`、`-debug.log`、`-release.log`。Workspace tests：niva 219 passed/2 ignored，niva_packager 22 passed、validation integration 2 passed，win_packager 10 passed（共 253 passed/2 ignored；不重复计算集成子进程）。 | 仅代表此 fingerprint 与 macOS 主机环境；保留现有 Rust/Clippy warnings，不代表 Windows 真机验收。 |
| TypeScript/Devtools/types | Devtools build、types typecheck、consumer checks 与 fresh-pack 独立安装消费者均 exit 0。 | Devtools log `/tmp/niva-callback-final-devtools.log`；types log `/tmp/niva-session-repair-types.log`；fresh-pack 命令为 `npm_config_cache=/tmp/niva-session-types-cache npm run test:fresh-pack --workspace=packages/types`，log `/tmp/niva-session-types-fresh-pack.log`，四种 packed consumer PASS。不替代远端 CI。 |
| Dependency audits | npm dependency `source-map-js` 已从 1.2.1 更新到 1.2.2；full 与 production-only audit 均为 0 findings。 | Reports `/tmp/niva-dependency-audit-fixed.json` and `/tmp/niva-dependency-audit-prod-fixed.json`. `cargo audit` exits 1 for unpatched RUSTSEC-2023-0071 in `rsa 0.9.10`, pulled by `apple-codesign` → packager; the advisory has no patched release. Current `crates/niva/src/app/macos.rs` uses `SigningSettings::default()` and system `codesign` for formal signing, with no RSA private key handling in that path. This code review does not clear the dependency advisory; retain the audit warning and limit. Report `/tmp/niva-cargo-audit.json`. |
| macOS bridge route | 最终 debug binary 的 custom-protocol WS 与严格 CSP IPC 两 lane PASS；报告 `/tmp/niva-http-final-bridge-route/result.json`，debug SHA-256 `0c3775e26df0a06d7af44111d275bf14a1d91f64075ff7800845fddd2f2dd9fc`。 | 当前 macOS/debug 证据；不覆盖 Windows 或跨源 iframe 的独立真机拒绝矩阵。 |
| macOS bridge route (historical) | 历史快照 `39bda818` 的两 lane 证据见 `/tmp/niva-bridge-route-smoke/result.json`；旧 debug SHA-256 `f02bf63898362f8053210194dc12cb19671bdb1e384e93a4befbb40338dcf4f0`。 | 仅作历史记录，不能替代最终 debug 报告。 |
| Runtime bootstrap smoke | 当前 callback-final debug binary 的 5 个配置组合均 PASS，覆盖 Node globals/CJS/ESM 开关、严格 CSP 与作者 import map override；日志 `/tmp/niva-callback-bootstrap.log`。 | `python3 -B examples/runtime-bootstrap-smoke/run.py --binary target/debug/niva --output /tmp/niva-callback-bootstrap` exit 0；配置 smoke 不等于 macOS 主 suite 或全平台验收。 |
| Top-level bridge GUI smoke | 当前真实 macOS WebView 7 个指定方法均 PASS；日志 `/tmp/niva-callback-top.log`。覆盖 real-window message listener add/remove/removeAll、direct unary、sync compatibility OS/UI rejection、stream 与 streamSend END。 | 仅为所列 top-level API 调用，不扩展为完整 Native API matrix。 |
| CommonJS 严格 CSP 修复 | 旧报告曾在 CSP 限制下因 `path.resolve` 同步取 cwd 导致 CommonJS bootstrap 未完成；raw `__niva_runtime_config.injectCommonJs` 为 true，但完成 marker 与 `Niva.runtimeConfig` 未安装，故 `require` 不存在。runtime 已以 lazy-CWD/完成 marker 修复，新的双 lane smoke 已要求 runtime flags 与 global `require`，并通过。 | 旧 failure 保留为历史诊断，不表述为配置未开启。 |
| Node compatibility 与辅助 smoke | HTTP 修复后连续四轮 Node compatibility 179 checks/17 cases PASS；最后一轮 `/tmp/niva-http-final-node-4.log` 绑定下方最终 debug SHA。前三轮使用 `restore?` Clippy 集成调整前的同语义 debug binary；旧 `EINVAL` 失败为修复前历史诊断。 | bootstrap 5 profiles 与 top-level bridge 7 方法使用同 runtime fingerprint 的 debug 构建，分别见 `/tmp/niva-callback-bootstrap.log`、`/tmp/niva-callback-top.log`；Native HTTP 内部修复不改变其调用路径，不声称这两项绑定最终 debug SHA。 |
| 远端精确授权 IPC (callback-final) | 最终 debug binary 的真实 macOS WebView 25/25 checks PASS，报告 `/tmp/niva-http-final-ipc-remote/result.json`；debug binary SHA-256 `0c3775e26df0a06d7af44111d275bf14a1d91f64075ff7800845fddd2f2dd9fc`。覆盖列明的 HTTP/HTTPS unary、token/exact-origin/remote-permission 与 lease checks。 | 仅代表所列精确授权调用，不构成完整远端权限矩阵。 |
| 远端精确授权 IPC (historical) | 历史快照 `39bda818` 的 25-check 结果 `/tmp/niva-ipc-v2-remote-final/result.json`。 | 历史记录，不替代 callback-final 报告。 |
| Release runtime | 最终 HTTP-fix candidate 的 macOS ARM64 `target/release/niva`：3,073,968 bytes；SHA-256 `050f4ef24f48505a48668b9e395c34f26d68a5e742ade8cabda399a9b9a96b02`。 | 通过严格小于 3,300,000 bytes 硬门禁；高于 macOS 3,000,000 bytes 参考目标。其他 target 尚未验证。 |
| macOS Native API smoke | 最终 debug binary 的默认真实 macOS WebView suite 39 method cases PASS，报告 `/tmp/niva-http-final-macos-api.log`。覆盖首次 BFCache 返回、reload、forward 与最终 cached-back 流程。 | 所列用例通过；dialogs、clipboard、shortcut 与 `process.open` 明确未运行，不能外推为完整 Native API/platform matrix。 |
| BFCache session 恢复 | 两次 persisted 恢复均生成新 session ID，同时保留 CSP nonce、realm、CJS/ESM/module、stdin、stdout、stderr identity；恢复后新 stdin probe 命令收到，API 可用。fixture 也检查旧 write 错误隔离。 | 真实 GUI 证据只覆盖当前 macOS fixture 和 debug binary；租约过期不可复活、Windows 真机仍未验收。runtime pinned `readable-stream@4.7.0` stale pending callback cleanup 有严格 terminal/no-active-write/no-queue/no-end/no-manual-destroy 与旧调用 settled 前置条件，canonical stream 保持不变，旧 write/API 不重放。 |
| Trusted IPC fallback (callback-final) | 最终 debug binary 的真实 macOS WebView 27/27 checks PASS，报告 `/tmp/niva-http-final-ipc-trusted/result.json`；debug binary SHA-256 `0c3775e26df0a06d7af44111d275bf14a1d91f64075ff7800845fddd2f2dd9fc`。覆盖文本 HTTP/HTTPS/POST、限制与 lease cleanup。 | 此套件验证受信 IPC unary 路径；Node HTTP stream compatibility 另由独立真实 WebView suite 验证。 |
| Trusted IPC fallback (historical) | 旧快照 `39bda818` 的 27-check 报告 `/tmp/niva-ipc-v2-trusted-final/result.json`。 | 历史快照记录，不替代 callback-final 报告。 |
| Windows target 与真机 | Windows target check 因 `lzma-sys` 找不到 MSVC C 标准库头 `stdlib.h` 而失败；未有 Windows WebView 真机结果。 | 工具链阻断；Windows 条件编译未覆盖，Windows 真机流程未验收。macOS 编译或交叉检查不能替代真机结果。 |
| CI | 远端 CI 尚未核对；本机 `gh` 因未认证退出 4。 | 保持开放；本地结果不能替代远端工作流证据。 |

## 2026-10-06 Windows review 后的当前快照

本节记录当前源码快照的实际检查结果；成功只覆盖列出的命令、fingerprint 与运行环境。源码审阅发现的问题与修复清单见[Windows 盲审报告](windows-blind-review-2026-10-06.md)。本轮保持协议边界：异步 API 创建/控制始终走 IPC `t:"api_call"`；Wry IPC/`evaluate_script` 与可选 WebSocket 是两条纯数据路径，WebSocket 只承担数据 attach/data/ack/cancel。runtime/types 没有公开 API 或 wire-contract 变更。

| 项目 | 当前证据 | 状态与限制 |
| --- | --- | --- |
| Runtime 与 frame routing | Runtime fingerprint `e966c98485f436ab9adf0b78919dddc66a1374e6542457a4431d66571c9b63ef`；bootstrap 630,009 bytes、182 output artifacts。Runtime build/typecheck/test 通过，179 个测试通过。新增嵌套同源 iframe 用例验证请求上路由到顶层 Native IPC host；跨源中间祖先与缺失顶层 host fail-closed。 | Runtime 单测证明 realm/路由逻辑；本轮真实 macOS fixture 覆盖同源 iframe parent relay，但没有嵌套 iframe 真机专项，也没有 Windows WebView 真机验证。 |
| TypeScript、types 与 Devtools | Runtime build；types typecheck、三个 consumer configs、fresh-pack 四种消费者；Devtools `tsc --noEmit`/Vite build 均通过。 | Devtools bundle 有既有体积提示：minified JS chunk 907.68 kB，大于 500 kB 提示阈值；构建成功。 |
| Rust workspace gates | `cargo fmt --all -- --check`、`cargo check --workspace`、`cargo clippy --workspace --all-targets`、`cargo test --workspace` 均 exit 0；测试 niva 234 passed/2 ignored、niva_packager 27、packager validation integration 2、win_packager 17，共 280 passed/2 ignored。第一次完整测试有 1 项 UDP 端口释放检查报 `AddrInUse`；单独复跑该用例通过，随后 workspace 全量复跑通过。 | 测试在 macOS 主机运行；2 个忽略测试要求可用的受信任出站 HTTPS 证书。Clippy 有 warnings。 |
| Windows 条件编译 | `cargo check -p niva --bin niva --target x86_64-pc-windows-msvc` 与 `cargo check -p win_packager --target x86_64-pc-windows-msvc` 均通过。 | `cargo check --workspace --target x86_64-pc-windows-msvc` 被本机缺失 MSVC C 标准库/头文件阻断：`lzma-sys`/`bzip2-sys` 缺 `stdlib.h`，`ring` 缺 `assert.h`。这不否定上面两个 Rust 包的条件编译检查，但完整 workspace Windows check 未通过。以上都不等于 Windows 真机验证。 |
| Windows helper 与资源身份策略 | Windows build helper `node --check` 及 Node tests 6/6 通过；资源文件身份策略纯 Rust tests 4 项通过。 | helper 测试和资源 identity policy tests 不替代真实 PE 写入或 NTFS/ReFS/FAT 文件系统验证。 |
| macOS WebView 回归 | Node compatibility：179 checks/17 cases；Native API：39 method cases；bridge-route WS/IPC 两 lane 均通过，包含 1 MiB 双向传输、Node child/FileHandle 与 parent iframe 路径。三套 GUI smoke 使用 debug binary SHA-256 `26315117ea2db5843fbe59676f44a7fba640909b529db6905a59c0df1563050d`。日志 `/tmp/niva-windows-review-node.log`、`/tmp/niva-windows-review-macos-api.log`、`/tmp/niva-windows-review-bridge-route.log`；bridge-route 结果在 `/tmp/niva-windows-review-bridge-route/result.json`。 | 只覆盖 macOS WebView；Native API 默认 suite 未运行 dialogs、clipboard、shortcuts。不能用 macOS 验证代替 Windows 菜单、WebView2、stdio 或打包真机验收。 |
| macOS release runtime | ARM64 `target/release/niva` 为 3,073,952 bytes，SHA-256 `1570fdc6680f3713f0b6a4407f680bd242c5b841e75f41688d46019c0d63dfc1`；runtime fingerprint 为上列当前值。 | 按 macOS 严格小于 3,300,000 bytes 门禁通过；超过 3,000,000 bytes 参考目标。Windows release 主程序因 Windows MSVC C toolchain 不可用而未测体积。 |
| Windows 真机与远端 CI | 本轮没有 Windows 设备；远端 CI 结果未能读取（GitHub API 返回 403）。 | Windows WebView2、原生菜单 accelerator、stdin/子进程取消、PE 打包和目标机文件系统仍开放；target checks、macOS smoke 与旧 2026-09-23 有限 Windows 记录均不能替代当前真机复测。 |

## 仍开放的验收门禁

- [x] 当前 macOS Native API suite 的 BFCache、reload 与 forward navigation 用例通过（39 method cases）；dialogs/clipboard/shortcuts 与 `process.open` 保留未运行标记。
- [x] 当前 runtime、TypeScript/types/Devtools、Rust workspace gates 与 macOS Node/bridge route 回归通过；跨平台和完整 Native API matrix 仍开放。
- [ ] 获取并记录本提交的远端 CI 结果。
- [ ] 在具备 MSVC C 标准库的环境完成 Windows full-workspace target check，并在 Windows 真机验证 WebView2、原生菜单、stdio/进程和打包关键路径。
- [ ] Windows 与其他 release target 的完整主程序大小/SHA 与目标平台 evidence 补齐前，不宣称跨平台体积或验收完成。

## 当前产物记录

- Runtime fingerprint：当前 Windows review 快照 `e966c98485f436ab9adf0b78919dddc66a1374e6542457a4431d66571c9b63ef`（此前 HTTP/BFCache snapshot `0befa3f4ca6a5d151c54c3532ce226a01fe00c562d3a1abb87ba09f28c06f694` 仅为历史证据）。
- Bootstrap：当前 630,009 bytes；历史 fingerprint 对应 629,975 bytes。
- Release target/程序：当前 macOS ARM64 `target/release/niva` 为 3,073,952 bytes，SHA-256 `1570fdc6680f3713f0b6a4407f680bd242c5b841e75f41688d46019c0d63dfc1`。此前产物 hash 仅代表其各自源码快照。
- Bridge debug binary SHA-256：本节本轮仅报告 smoke 日志路径，未单独记录当前 debug binary hash；不得沿用历史 `0c3775e26df0a06d7af44111d275bf14a1d91f64075ff7800845fddd2f2dd9fc` 作为当前值。
- macOS bridge-route：历史快照 `39bda818` PASS，`/tmp/niva-bridge-route-smoke/result.json`。
- 当前快照的 runtime 179/179、Devtools build、types typecheck/consumer/fresh-pack、Rust gates、BFCache GUI suite、Node compatibility 179 checks/17 cases、bridge-route WS/IPC 双 lane 与 macOS ARM64 release size 均通过。远端 CI、Windows 完整 workspace target check/真机、Windows release 体积和其他 release target 仍开放，详见上表。

## 2026-10-06 依赖清理后的 Runtime 快照

本轮清理中，Devtools 移除了未使用的 `pako` 与 `@types/pako` 依赖；bridge 源码、contracts 和 bootstrap 内容未变。lockfile 变化生成了新的 runtime input fingerprint，因此此处单独记录产物身份，不把上节 Windows review 的 Native、macOS GUI 或 release 证据冒充为本次重跑结果。

| 项目 | 依赖清理后证据 | 状态与限制 |
| --- | --- | --- |
| Runtime 产物 | fingerprint `f46d3e5c3c180587c8752019c847864d62815b7a598f8679d4e2df63e0d3ca1f`；bootstrap 630,009 bytes，SHA-256 `1497fda13b3e07d24a5297bdd4e5709ced7c19c9c260916988e4993e42b870de`。Runtime 179/179、types 检查与四种 fresh-pack consumer、Devtools build/7 tests 通过。 | lockfile 变化生成新 fingerprint，不表示 bridge 内容变化。 |
| Rust workspace gates | 冻结后 `cargo fmt --all -- --check`、`cargo check --workspace`、`cargo clippy --workspace --all-targets`、`ulimit -n 4096; cargo test --workspace -- --test-threads=1` 均通过；测试 niva 230 passed/2 ignored、niva_packager 27、validation integration 2、win_packager 21，共 280 passed/2 ignored。日志 `/tmp/niva-cleanup-fmt.log`、`-check.log`、`-clippy.log`、`-test.log`。 | macOS 主机结果；Clippy 有 warnings。 |
| Windows 条件编译 | 冻结后 `niva` 与 `win_packager` 定向 Windows target checks 通过；资源身份策略 4 tests、`win_packager` 21 tests。 | 完整 `cargo check --workspace --target x86_64-pc-windows-msvc` 因本机缺 MSVC C 标准库头文件失败：`bzip2-sys`/`lzma-sys` 找不到 `stdlib.h`。日志 `/tmp/niva-cleanup-windows-workspace-check.log`。Windows 真机与 release 体积仍未验收。 |
| macOS release runtime | ARM64 `target/release/niva`：3,073,952 bytes，SHA-256 `0dadbfed16f89c8c77224452dc0786d7b7ad2979d18cc5e59b202bf35b771377`。构建日志 `/tmp/niva-cleanup-release.log`。 | 通过 macOS 严格小于 3,300,000 bytes 门槛；高于 3,000,000 bytes 参考目标。 |

## 2026-10-06 1.0.0-alpha Review 快照

以下结果绑定本轮候选工作区快照，不构成发布 tag 或正式产物验收。IPC API 的 session budget/ban 按 `(window_id, source_origin)` 隔离，空远端 grant fail-closed；资源 owner 仍按创建时 `(window_id, IPC session)` 确定，WS/IPC 仅承担数据传输。

| 项目 | 当前证据 | 状态与限制 |
| --- | --- | --- |
| Runtime | input fingerprint `d59d33576e38e6c37a985475888bdcc69b03cb6f8aa3945b748a050c4b3262ea`；bootstrap 630,009 bytes，SHA-256 `1497fda13b3e07d24a5297bdd4e5709ced7c19c9c260916988e4993e42b870de`；179 tests 通过。API manager session-domain regression 30 项、permissions 4 项通过。 | Runtime/tests 为源码回归证据，不代表完整 origin/iframe 权限矩阵已在目标平台验收。 |
| macOS WebView | 当前 debug binary 的 WS/IPC bridge-route 两 lane PASS；macOS API smoke 39 method cases PASS（BFCache cached realm 恢复、新 session、stdin identity、reload/forward）；NodeCompat 179 checks/17 cases PASS。 | API smoke 未运行 dialogs、clipboard、shortcuts、`process.open`。NodeCompat HTTPS wrapper 只覆盖 protocol rejection，真实 outbound TLS 仍未验收。日志 `/tmp/niva-alpha-review-bridge-route.log`、`/tmp/niva-alpha-review-macos-api.log`、`/tmp/niva-alpha-review-node.log`。 |
| macOS ARM64 release | Raw `target/release/niva` 3,073,984 bytes，SHA-256 `cef104d415f773d6c9c4909ec9197a508a45a2e99288bdbfdaa6dfa071431411`；按 `build_MacOS.sh` 对副本执行 `strip -N` 后 3,061,216 bytes，SHA-256 `e84c7ed3b6c09a9b699179bf2c594217173f50dae0ef8b2cd606f1ecfeb32058`。 | stripped artifact 通过 `< 3,300,000` 门槛。 |
| macOS Intel release | SDK 26.5 构建的 raw `target/x86_64-apple-darwin/release/niva` 为 3,643,400 bytes，SHA `58f0a8f1631e28a647610f91d594b9403136e6ad3cad57f16614cee889f313f4`；`strip -N` 后 3,630,624 bytes，SHA `b10f67c45feb58cbe8bede21bdf7cc0ec68c2ec7255e158d00e0d6e63e769e08`。 | stripped artifact 超过 3,300,000 门槛 330,624 bytes；本轮未改 profile/依赖或实施瘦身，当前候选的 Intel 门禁未通过。日志 `/tmp/niva-alpha-review-release-intel.log`。 |
| Windows / CI | Niva 与 `win_packager` 定向 Windows target checks 通过。 | 完整 workspace target check 受本机缺失 MSVC `stdlib.h` 阻断；Windows 真机、Windows release size、当前远端 CI 未验收/读取。 |
