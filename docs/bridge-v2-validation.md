# Bridge v2 验收记录

本文记录 IPC 控制通道与可选 WebSocket 流数据通道的实现和验证证据。它不是 0.10.0-beta.1 全平台发布验收通过声明；源码实现、单测、macOS WebView 实测、发布产物和其他平台验证分别记录。未完成项目继续保持开放。

## 当前协议边界

- 异步 API 创建使用 IPC `t: "api_call"`。Native 通过 Wry `evaluate_script` 调用页面的 `__niva_ipc_reply` 回送带 session/rid 的结果。成功创建流后，由 reply 返回 `channelOpened` 与 capability；数据 lane attach 后才能传输流帧。
- IPC lane 通过 `channelAttach` 建立数据通道；WS lane 只能使用已有 ticket 的 `attach`、流数据、ACK 与 cancel。WS 不发送 API method/args，也不承担 API dispatch。
- WS hello 的 `v` 和 binary header version 均为 2。Binary header 共 18 bytes：version、flags、64-bit id 与 64-bit seq；payload 紧随其后。
- frame/session/origin 校验与流控制语义由 Native 侧处理。WS 是选定流数据操作的优化通道；同步 XHR 仍仅供需要同步结果的兼容 API 使用。
- 页面生命周期控制另走单向 IPC `session_close`：它不是 API，Native 按完整 session key 鉴权、取消旧 session 工作并保留 tombstone；不要求 reply。只有匹配的 persisted `pagehide`/`pageshow` 才在原页面 realm 内创建新的 session ID（密码学随机 nonce）。runtime 保留模块缓存、stdin 身份及监听器；旧调用取消、普通 Native 句柄失效、旧回调隔离，API 不重放。CSP/runtime configuration 的 `runtimeConfig.nonce` 保持不变；租约过期不能复活。

以上是当前实现的源码检查结论，不代表每个平台都已端到端验收。[协议定义](../crates/niva/src/app/api_manager/protocol.rs) 固定 wire version/header；[runtime bootstrap](../packages/runtime/src/bootstrap.ts) 创建 IPC API 请求、返回流 ticket 并按 ticket attach 数据 lane；[Native API manager](../crates/niva/src/app/api_manager/mod.rs) 处理 attach 与 WS session；[window builder](../crates/niva/src/app/window_manager/builder.rs) 通过 Wry 脚本执行送达 IPC reply。具体 smoke 的观测边界见 [bridge-route smoke](../examples/bridge-route-smoke/README.md)。

## 当前实现与验证证据

状态按 2026-10-06 主线程对最新源码快照的实际运行结果整理；运行成功只代表对应命令、fingerprint 和目标环境。

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

## 仍开放的验收门禁

- [x] 默认 macOS Native API suite 的 BFCache、reload 与 forward navigation 用例通过（39 method cases）；dialogs/clipboard/shortcut 与 `process.open` 保留未运行标记。
- [x] 本轮 BFCache 真 GUI、Rust workspace gates 与 stdin reader race 回归通过；跨平台和完整 Native API matrix 仍开放。
- [ ] 获取并记录本提交的远端 CI 结果。
- [ ] Windows target 编译在具备 MSVC C 标准库的环境重新运行；Windows WebView2 真机关键流程仍需独立验证。
- [ ] 其他 release target 的完整主程序大小/SHA 与目标平台 evidence 补齐前，不宣称跨平台体积或验收完成。

## 当前产物记录

- Runtime fingerprint：本轮 `0befa3f4ca6a5d151c54c3532ce226a01fe00c562d3a1abb87ba09f28c06f694`。
- Bootstrap：629,975 bytes。
- Release target/程序：macOS ARM64，`target/release/niva`，3,073,968 bytes。
- Release SHA-256：`050f4ef24f48505a48668b9e395c34f26d68a5e742ade8cabda399a9b9a96b02`。
- Bridge debug binary SHA-256：`0c3775e26df0a06d7af44111d275bf14a1d91f64075ff7800845fddd2f2dd9fc`（最终 debug binary）。
- macOS bridge-route：历史快照 `39bda818` PASS，`/tmp/niva-bridge-route-smoke/result.json`。
- 本轮 runtime 177/177、Devtools build、types typecheck/consumer/fresh-pack、Rust gates、BFCache GUI suite、Node compatibility 179 checks/17 cases、bridge-route WS/IPC 双 lane、top-level bridge、trusted IPC 27/27、remote IPC 25/25、bootstrap configuration smoke 与 macOS ARM64 release size 均通过。远端 CI、Windows target/真机和其他 release target 仍开放，详见上表。
