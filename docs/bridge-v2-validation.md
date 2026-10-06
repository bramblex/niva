# Bridge v2 验收记录

本文记录 IPC 控制通道与可选 WebSocket 流数据通道的实现和验证证据。它不是 0.10.0-beta.1 全平台发布验收通过声明；源码实现、单测、macOS WebView 实测、发布产物和其他平台验证分别记录。未完成项目继续保持开放。

## 当前协议边界

- 异步 API 创建使用 IPC `t: "api_call"`。Native 通过 Wry `evaluate_script` 调用页面的 `__niva_ipc_reply` 回送带 session/rid 的结果。成功创建流后，由 reply 返回 `channelOpened` 与 capability；数据 lane attach 后才能传输流帧。
- IPC lane 通过 `channelAttach` 建立数据通道；WS lane 只能使用已有 ticket 的 `attach`、流数据、ACK 与 cancel。WS 不发送 API method/args，也不承担 API dispatch。
- WS hello 的 `v` 和 binary header version 均为 2。Binary header 共 18 bytes：version、flags、64-bit id 与 64-bit seq；payload 紧随其后。
- frame/session/origin 校验与流控制语义由 Native 侧处理。WS 是选定流数据操作的优化通道；同步 XHR 仍仅供需要同步结果的兼容 API 使用。

以上是当前实现的源码检查结论，不代表每个平台都已端到端验收。[协议定义](../crates/niva/src/app/api_manager/protocol.rs) 固定 wire version/header；[runtime bootstrap](../packages/runtime/src/bootstrap.ts) 创建 IPC API 请求、返回流 ticket 并按 ticket attach 数据 lane；[Native API manager](../crates/niva/src/app/api_manager/mod.rs) 处理 attach 与 WS session；[window builder](../crates/niva/src/app/window_manager/builder.rs) 通过 Wry 脚本执行送达 IPC reply。具体 smoke 的观测边界见 [bridge-route smoke](../examples/bridge-route-smoke/README.md)。

## 当前实现与验证证据

状态按 2026-10-06 主线程对最新源码快照的实际运行结果整理；运行成功只代表对应命令、fingerprint 和目标环境。

| 项目 | 当前证据 | 状态与限制 |
| --- | --- | --- |
| Runtime 与 Native 源码门禁 | Runtime fingerprint `913941add7a034925743fae092bb4e4ef27107383d7de4426a4a78ee588ccd06`；bootstrap 625,046 bytes。主线程报告 runtime build 与 167/167 tests 通过；`cargo fmt --all -- --check`、workspace check、clippy、workspace tests、Native debug build 均 exit 0。workspace tests 报告 211 passed、1 个隔离 child test passed、2 ignored。 | 仅代表此 fingerprint 和报告中的 macOS 主机运行结果，不代表 Windows 真机验收。 |
| TypeScript/Devtools/types | 主线程报告 Devtools build、types typecheck、consumer typechecks、fresh-pack checks 全部 exit 0。 | 对应当前 runtime/package 构建，不替代远端 CI。 |
| macOS bridge route | 真实 WebView 的 custom-protocol WS 与严格 CSP IPC 两 lane 均 PASS：含 CommonJS global require/trusted-local flags、`channelOpened`→attach、WS 双向数据与ACK、CSP IPC channel attach/send/ack、1 MiB FileHandle、Node 与 Niva child_process echo、同源 iframe 独立 session，以及 iframe 后 top unary。报告 `/tmp/niva-bridge-route-smoke/result.json`；debug binary SHA-256 `f02bf63898362f8053210194dc12cb19671bdb1e384e93a4befbb40338dcf4f0`。 | 真实 macOS WebView 证据，不覆盖 Windows，也不证明跨源 iframe 的独立真机拒绝矩阵。 |
| CommonJS 严格 CSP 修复 | 旧报告曾在 CSP 限制下因 `path.resolve` 同步取 cwd 导致 CommonJS bootstrap 未完成；raw `__niva_runtime_config.injectCommonJs` 为 true，但完成 marker 与 `Niva.runtimeConfig` 未安装，故 `require` 不存在。runtime 已以 lazy-CWD/完成 marker 修复，新的双 lane smoke 已要求 runtime flags 与 global `require`，并通过。 | 旧 failure 保留为历史诊断，不表述为配置未开启。 |
| Node compatibility 与辅助 smoke | Node compatibility 179 checks / 17 cases PASS，含两个 HTTP 请求 barrier 并发检查；最新资产 bootstrap 5 cases、top-level bridge 7 cases 也 PASS。 | 对应当前资产和 macOS 主机运行；不替代 WebView bridge smoke。 |
| 远端精确授权 IPC | 真实 macOS WebView 25 checks PASS，覆盖 HTTP/HTTPS、响应限制、拒绝与 lease expiry；报告 `/tmp/niva-ipc-v2-remote-final/result.json`，debug SHA-256 与上方 bridge binary 相同。 | 仅为所列精确 origin grant 用例，不代表完整远端权限矩阵。 |
| Release runtime | 当前 ARM64 完整主程序 `target/release/niva`：3,040,832 bytes；SHA-256 `09caa686b07c65dd3743f4066ce558b302d6a8a77e799e9581e83222699ea380`。 | 通过严格小于 3,300,000 bytes 硬门禁；高于 macOS 3,000,000 bytes 参考目标。只证明此 ARM64 产物，不代表其他 target 通过。 |
| macOS Native API smoke | 默认 suite 真实运行失败，主线程结果 `/tmp/niva-bridge-macos-api-final.log`，exit 1。HTTP secondary 首次加载/pageshow 成功；首次 back 命中 BFCache 后，fixture 对 persisted 页面调用 `location.reload()`，新 document 的 API/URL、forward 能力和 stdout report 通过，显式 reload 也通过。随后 forward 等待 15 秒仍未收到 `secondary-ready`/pageshow。 | 当前完整默认 suite 未通过；没有证据确认 forward timeout 的根因。更新后的 fixture 只验证 fresh-document 路径，不验证 BFCache 原 session 恢复。 |
| BFCache session 恢复 | 当前 `packages/runtime/src/bootstrap.ts:1331-1334` 在 `pagehide` 时 unregister frame session 并永久 expire session；基线提交 `9bb3d694bfd485afc52bff789b04c4e07239e363` 的 `bootstrap.ts:1161` 已有 pagehide→expire。真实 GUI diagnostic `/tmp/niva-bridge-macos-api-diagnostic.log` 记录首次 back 后 API 错误 `Niva page session is no longer active`。 | 开放 review 问题：BFCache 恢复的 document 如何重新建立可用 Native API session 尚未实现/验收。本轮 fixture fresh reload 是测试路径 workaround，不是产品修复，也不代表原 BFCache session 恢复。 |
| Trusted IPC fallback | 最新真实 macOS WebView run 27 checks PASS，报告 `/tmp/niva-ipc-v2-trusted-final/result.json`。此前两项失败由 child window 继承默认 debug entry、递归运行 fixture 导致报告污染；当前 blank entry fixture 已消除该污染。 | 仅代表所列 fixture 和 binary，不外推为其他权限或平台矩阵通过。 |
| Windows target 与真机 | Windows target check 因 `lzma-sys` 找不到 MSVC C 标准库头 `stdlib.h` 而失败；未有 Windows WebView 真机结果。 | 工具链阻断；Windows 条件编译未覆盖，Windows 真机流程未验收。macOS 编译或交叉检查不能替代真机结果。 |
| CI | 远端 CI 尚未核对；本机 `gh` 因未认证退出 4。 | 保持开放；本地结果不能替代远端工作流证据。 |

## 仍开放的验收门禁

- [ ] 修复并通过默认 macOS Native API suite；当前 forward navigation 在 secondary `pageshow` 等待 15 秒后失败，受控 dialogs/clipboard/shortcut 等也须保留未运行标记。
- [ ] 设计并验收 BFCache 恢复后重新建立 Native API session 的行为；不能用 fixture `location.reload()` 的 fresh-document 验证替代。
- [ ] 获取并记录本提交的远端 CI 结果。
- [ ] Windows target 编译在具备 MSVC C 标准库的环境重新运行；Windows WebView2 真机关键流程仍需独立验证。
- [ ] 其他 release target 的完整主程序大小/SHA 与目标平台 evidence 补齐前，不宣称跨平台体积或验收完成。

## 当前产物记录

- Runtime fingerprint：`913941add7a034925743fae092bb4e4ef27107383d7de4426a4a78ee588ccd06`。
- Bootstrap：625,046 bytes。
- Release target/程序：macOS ARM64，`target/release/niva`，3,040,832 bytes。
- Release SHA-256：`09caa686b07c65dd3743f4066ce558b302d6a8a77e799e9581e83222699ea380`。
- Bridge debug binary SHA-256：`f02bf63898362f8053210194dc12cb19671bdb1e384e93a4befbb40338dcf4f0`。
- macOS bridge-route：PASS，`/tmp/niva-bridge-route-smoke/result.json`。
- 最新资产 Node/bootstrap/top-bridge、trusted/remote IPC 与 bridge-route 均已通过；默认 macOS Native API suite 失败，BFCache session 恢复、远端 CI、Windows target/真机仍开放，详见上表。
