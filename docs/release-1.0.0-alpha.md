# Niva 1.0.0-alpha 候选准备与整体 Review

更新时间：2026-10-06。本文记录 alpha 候选准备与源码 review，不表示已经发布或通过所有平台门禁。当前仓库版本仍为 `0.10.0-beta.1`；本轮没有改版本号、发布 tag 或生成正式发行包。版本文件、lock、packager manifest 与 tag 应在发布准备阶段统一调整为 `1.0.0-alpha`。

候选目标暂按 Windows x86_64、macOS Apple Silicon、macOS Intel 三个平台收口。Release runtime 硬门槛分别为 Windows 严格小于 3,500,000 bytes、macOS 严格小于 3,300,000 bytes。macOS 3,000,000 bytes 只是参考目标。每个平台均需记录当前候选的完整 Native 主程序大小和 SHA；交叉编译检查不能代替平台运行验收。

## 本轮源码与文档修复

- API manager 对过期 IPC session 的预算与封禁按 `(window_id, source_origin)` 隔离；空的远端授权 grant fail-closed。资源 owner 仍是创建时的 `(window_id, IPC session)`。定向回归为 API manager 30 项、permissions 4 项。源码结果仍需结合下方平台矩阵理解。
- Windows release ZIP 名称/dirty provenance helper 加强并有 Node 测试；6 项通过。
- TypeScript HTTP `requestText` headers 收窄为 `Record<string, string>`，`process.exitCode` 可写。Positive/negative consumer checks、Runtime 179 项、types consumer 与四种 fresh-pack consumer、Devtools build 均通过。
- HTTP/stream API 文档说明 `requestText` header 的字符串契约；远端权限文档明确 `fs.*`/`fs.node` 和 `process.execText`/`process.execFileText` 属高影响 grant，应仅授予完全可信来源。
- Packager 文档更准确地区分 SPDX/源码链接声明与上游目录中实际收集到的原文许可文件。当前生成器会在 crate 目录找到 `LICENSE*`、`COPYING*` 或 `NOTICE*` 时复制；kit checker 不验证每条依赖声明都有完整原文材料。本轮依赖盘点发现 58 条第三方声明尚未在对应 crate 目录找到上述文件名，材料闭包和逐项许可验收仍开放。这是工程交付清单，不构成许可合规判断。
- `docs/bridge.md` 与 NodeCompat 说明已澄清资源 owner 是创建时 `(window_id, IPC session)`；WS/IPC 只承载数据，不决定资源 owner。

## 当前源码检查与本机证据

本节检查绑定当前工作区源码快照；Native/docs 完成提交后应保留该提交和日志作为复核依据。

| 检查 | 结果 | 记录与边界 |
| --- | --- | --- |
| Rust host gates | `cargo fmt --all -- --check`、`cargo check --workspace`、`cargo clippy --workspace --all-targets` 通过；`ulimit -n 4096; cargo test --workspace -- --test-threads=1` 通过。测试共 284 passed、2 ignored（Niva 234/2 ignored，packager 27，validation integration 2，win_packager 21）。 | 日志在 `/tmp/niva-alpha-review-fmt.log`、`-check.log`、`-clippy.log`、`-test.log`。Clippy 有 warnings。首次 workspace check 被 runtime freshness guard 阻止；重建 runtime 后复跑通过。 |
| Runtime / types / Devtools | Runtime build/typecheck、179 runtime tests、types positive/negative consumer 与四种 fresh-pack consumer、Devtools build 通过；Windows helper Node tests 6 项、Devtools tests 7 项通过。 | Runtime fingerprint `d59d33576e38e6c37a985475888bdcc69b03cb6f8aa3945b748a050c4b3262ea`；bootstrap 630,009 bytes，SHA-256 `1497fda13b3e07d24a5297bdd4e5709ced7c19c9c260916988e4993e42b870de`。刷新日志 `/tmp/niva-alpha-review-runtime-build.log`。 |
| Windows target checks | `cargo check -p niva --bin niva --target x86_64-pc-windows-msvc` 与 `cargo check -p win_packager --target x86_64-pc-windows-msvc` 通过。 | 完整 workspace target check 因本机缺 MSVC C 标准库头文件、`bzip2-sys` 找不到 `stdlib.h` 而失败；日志 `/tmp/niva-alpha-review-windows-workspace.log`。这不代表 Windows 真机验收。 |
| macOS API/WebView | 当前 debug binary 完成 macOS API smoke：39 method cases PASS，包含 BFCache cached realm 恢复时创建 fresh session、stdin 身份、reload 与 forward navigation。 | `/tmp/niva-alpha-review-macos-api.log`。dialogs、clipboard、shortcuts、`process.open` 未运行。 |
| macOS Bridge | 当前 debug binary 的 WS 与 IPC bridge-route 两 lane、overall 均 PASS。 | `/tmp/niva-alpha-review-bridge-route.log` 与 `/tmp/niva-alpha-review-bridge-route/result.json`。只证明所列 macOS fixture 路径。 |
| macOS NodeCompat | 当前 debug binary 为 179 项 documented runtime/method/alias checks、17 cases PASS。 | `/tmp/niva-alpha-review-node.log`。HTTPS wrapper 本轮仅做 protocol rejection 检查；真实 outbound TLS 另行开放。 |
| macOS ARM64 Release | 当前 raw `target/release/niva` 为 3,073,984 bytes，SHA-256 `cef104d415f773d6c9c4909ec9197a508a45a2e99288bdbfdaa6dfa071431411`；按 `build_MacOS.sh` 对副本执行 `strip -N` 后为 3,061,216 bytes，SHA-256 `e84c7ed3b6c09a9b699179bf2c594217173f50dae0ef8b2cd606f1ecfeb32058`。 | stripped runtime 低于 3,300,000 门槛，高于 3,000,000 参考目标。构建日志 `/tmp/niva-alpha-review-release.log`。 |
| macOS Intel Release | 当前 raw `target/x86_64-apple-darwin/release/niva` 为 3,643,400 bytes，SHA-256 `58f0a8f1631e28a647610f91d594b9403136e6ad3cad57f16614cee889f313f4`；同一 SDK 26.5 下对副本执行 `strip -N` 后为 3,630,624 bytes，SHA-256 `b10f67c45feb58cbe8bede21bdf7cc0ec68c2ec7255e158d00e0d6e63e769e08`。 | 当前 stripped runtime 超过 3,300,000 门槛 330,624 bytes，阻断候选。需在不放宽门槛的前提下完成瘦身并重测；本轮未改 profile/依赖或实施瘦身。构建日志 `/tmp/niva-alpha-review-release-intel.log`。 |

## 发布前仍须补齐

- **Windows 真机与完整 workspace build**：在 Windows x86_64 开发环境运行 `cargo check --workspace --target x86_64-pc-windows-msvc`，然后运行 `build_Windows.cmd`。记录 `target\release\niva.exe` 的 byte size 与 SHA，并证明严格小于 3,500,000 bytes。运行 `examples\windows-smoke\build.ps1 -Interactive`；另补全 BFCache、同源嵌套 iframe、菜单 accelerator/owner、Job 子进程取消、IPC fallback、stdio 和安全拒绝矩阵，记录设备/系统版本、命令与结果。目标检查不能代替真机操作。
- **macOS Intel 候选体积**：当前工作区已在 macOS SDK 26.5 构建并测量；raw 3,643,400 bytes、`strip -N` 后 3,630,624 bytes，均超过严格门槛。旧缓存 3,569,184 bytes 不是当前源码测量。保留 Intel 平台时，必须完成瘦身后用同一完整 Native release 流程复测并记录 SHA；不得放宽门槛或用裸 Native/JS 子样本替代。
- **CI**：当前提交的 GitHub Actions 结果未能读取；本地执行不能代替远端 CI 证据。GitHub CLI 未认证，远端状态仍为 unknown。
- **Build kit 第三方许可材料**：为缺少原文文件的第三方 crate 补充并复核所需 license/notice 原文，再运行 kit 生成与 `scripts/check-packager-kit.py`；逐项确认材料随包可检索。当前 checker 未覆盖每个 SPDX 声明对应原文文件的存在性。该项目在材料闭包前保持开放，不据此作法律结论。
- **同步发布版本字段**：发布前统一 `packages/*` 元数据、Cargo、manifest、lock（如适用）和 tag 为 `1.0.0-alpha`；本轮保持 `0.10.0-beta.1`，未生成 alpha tag 或发布包。

## 后续加固与范围边界

本机 HTTP server 仍需评估 request-header 读取 deadline 与并发连接上限；当前 accept loop 为每个连接启动任务，header reader 没有独立总 deadline。本项作为后续加固跟踪，本轮没有将其认定为已证实的外部漏洞，也不列为 alpha 已解决项目。

本轮不以完整 Node.js 兼容、MiniBlink、移除 WebSocket 或跨源 iframe 原生 bridge 为候选目标。Intel 当前 release 体积门禁未通过；Windows 真机、CI 和 kit 许可材料也未补齐前，不宣称 1.0.0-alpha 全平台验收完成。
