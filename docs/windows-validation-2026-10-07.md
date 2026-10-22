# Windows 验证记录（2026-10-07）

本记录覆盖 `f96d613cbc135f48ac80817889e3ed8494c9ba90` 基线及本次用于跨平台验证的窄修复。UTM guest 是 Windows 11 ARM64；Rust 使用 `x86_64-pc-windows-msvc` host/toolchain，因此 x64 Windows 产物在 ARM64 guest 上经 Windows x64 仿真运行。该证据不能替代 x64 硬件上的原生验证。Windows 完整 runtime 主程序仍须严格小于 `3,500,000` bytes。

## 验证环境

- Windows 11 Pro `10.0.26300`，guest OS 架构 `ARM64`；WebView2 `154.0.4258.62`。
- 桌面用户的 session 1 有 `explorer.exe`；QGA 命令以 `NT AUTHORITY\SYSTEM` 在 session 0 执行。以下尚未完成的桌面交互必须在已登录用户的 session 1 执行，不能从 SYSTEM/session 0 直接启动 GUI 来验收。
- Rust `1.98.1-x86_64-pc-windows-msvc`，报告的 host 为 `x86_64-pc-windows-msvc`；VS 2022 Build Tools `17.14.41`，MSVC tools `14.44.35207`，`cl` `19.44.35229`，Windows SDK `10.0.26100.0`。Build Tools 安装返回 `3010`（要求重启）；安装后未主动重启，本次工具链和 JS 构建已在当前会话中可用。
- Node `v22.23.3` ARM64，npm `10.9.9`；Python `3.14.8`，Git `2.56.0.windows.2`。JavaScript 检查是在 ARM64 Node 上运行；未运行 Node `22.14.0` 的精确版本兼容矩阵。
- guest checkout 位于 `C:\niva-validation\repo`，基于上述 `f96d613` commit，并同步本记录所述验证修复。

## 已完成的 Windows 检查

| 检查 | 结果 | 证据和边界 |
| --- | --- | --- |
| `npm ci` | exit 0 | 在 Windows guest 完成依赖安装。 |
| `npm run build --workspace=packages/runtime` | exit 0 | `typesChecked=true`，29 个 ESM facade、182 个 output artifacts，bootstrap `630,009` bytes；guest input fingerprint `374f361ebaec48e35d5c8fac29f746157a7396c4fde1e06d9ae5f811e5c0d67c`。该值是 runtime build manifest，不是最终 `niva.exe` 大小。 |
| `npm test --workspace=packages/runtime` | 179/179 pass | 包含下表所述的 Windows 路径预期修正；测试运行在 ARM64 Node。 |
| `npm run typecheck --workspace=packages/types` | exit 0 | types package 独立声明检查通过。 |
| `npm run typecheck:consumers --workspace=packages/types` | exit 0 | 包含 browser、CommonJS 与 Node 消费者类型检查。 |
| `npm run test:fresh-pack --workspace=packages/types` | exit 0 | freshly packed package 的 `browser-default`、`browser-with-node-types`、`commonjs`、`node` 四种消费者均通过。 |
| `npm run build --workspace=packages/devtools` | exit 0 | Windows guest 的 Devtools TypeScript/Vite 构建通过。 |
| `python -B -m unittest discover -s examples/bridge-route-smoke -p test_run.py -v` | 5 tests pass | 仅验证 Python runner 本身；不代表 Windows Niva app 的 bridge route 已运行。 |
| `cargo fmt --all -- --check` | exit 0 | Rust 格式门禁通过。 |

runtime build、runtime tests 和 types fresh-pack 的 Windows 日志副本记录在验证机的 `/tmp/niva-runtime-build-final.log`、`/tmp/niva-runtime-test-final.log`、`/tmp/niva-types-fresh-pack-final.log`。Devtools、runner unit tests 与 fmt 的结果由 Windows guest executor 报告。

本次验证包含以下跨平台修复；它们修正 Windows 构建调用和测试 oracle，没有改变 Native API 或 bridge 协议：

- `packages/runtime/scripts/build-runtime.mjs` 通过 `process.execPath` 运行 TypeScript JS CLI，避免 Windows 下把仅存在的 `tsc.cmd` 当作可直接启动的 `tsc` 文件。
- `packages/runtime/test/bridge.test.js` 使用 Node 的 host `path` 构造 Windows 文件系统路径；`completed-js-core.test.js` 使用带盘符的绝对 cwd 与 Node `path.resolve` oracle，并保留 root-relative 路径断言；URL 回归测试显式选择 POSIX 模式检查反斜杠转义。
- `packages/types/scripts/test-fresh-pack.mjs` 通过 `process.execPath` 启动 npm JS CLI，优先使用 `npm_execpath`，并保留直接 Node 调用的标准安装布局 fallback；spawn 失败会报告状态与 stderr/stdout。

## 仍待验证

Rust gate 的 `cargo check --workspace` 当前在 ARM64 guest 上执行 x64 仿真的首次编译，尚无退出结果。先前只输出 PowerShell usage 的无效 helper 调用不计作 gate 结果。不得把长时间编译本身判断为失败或死锁。

以下 Rust 和 release 检查尚未取得最终结果：

```powershell
cargo check --workspace
cargo clippy --workspace --all-targets
cargo test --workspace
cargo check --target x86_64-pc-windows-msvc
& .\build_Windows.cmd
```

完成 release 后，记录 `target\release\niva.exe` 的 PE 架构、字节数和 SHA-256，以及 release helper 的包报告。只有实际完整 runtime PE 严格小于 `3,500,000` bytes 才满足 Windows 体积门槛。以上 x64 产物在这台 ARM64 guest 上运行仍属于 x64 仿真结果。

## Windows smoke、桌面与原生行为

实际 Niva app 的 Windows smoke 尚未在 guest 运行。Python runner 的 5 个单元测试不覆盖 app、WebView2 或原生 UI。完成 release 后，在登录桌面的用户 session 1 运行：

```powershell
& .\examples\windows-smoke\build.ps1
python -B .\examples\bridge-route-smoke\run.py `
  --binary .\target\release\niva.exe `
  --trusted-debug `
  --output "$env:TEMP\niva-bridge-route-smoke"
python -B .\examples\stdio-host\verify.py `
  --binary .\target\release\niva.exe `
  --output .\dist\windows-stdio
python -B .\examples\stdio-host\verify_exit.py `
  --binary .\target\release\niva.exe `
  --output .\dist\windows-stdio-exit
```

若命令只能由 QGA 发起，使用已登录用户的 interactive token 创建一次性计划任务（`New-ScheduledTaskPrincipal -UserId $user -LogonType Interactive`），让脚本在 session 1 启动；它不保存密码。先确认 `(Get-CimInstance Win32_ComputerSystem).UserName` 非空，并在 UTM 桌面观察窗口。待 `WAITING_FOR_UI` 后，实际点 Smoke 菜单中的 Ping，再按 `Ctrl+Alt+Shift+F12`。从 QGA 读取任务日志和 `Get-ScheduledTaskInfo` 结果。菜单、快捷键或 WebView2 行为不得仅以 session 0 的启动结果判通过。

可从 SYSTEM/QGA 注册并启动该任务：

```powershell
$repo = (Resolve-Path .).Path
$user = (Get-CimInstance Win32_ComputerSystem).UserName
if (-not $user) { throw 'Sign in through the UTM console first.' }
$runner = Join-Path $repo 'dist\run-windows-interactive-smoke.ps1'
New-Item -ItemType Directory -Path (Split-Path -Parent $runner) -Force | Out-Null
@'
$ErrorActionPreference = 'Stop'
$repo = Split-Path -Parent $PSScriptRoot
Set-Location -LiteralPath $repo
& (Join-Path $repo 'examples\windows-smoke\build.ps1') -Interactive *> (Join-Path $repo 'dist\windows-interactive-smoke.log')
exit $LASTEXITCODE
'@ | Set-Content -LiteralPath $runner -Encoding UTF8
$taskName = 'Niva-Windows-Interactive-Smoke'
$action = New-ScheduledTaskAction `
  -Execute "$env:SystemRoot\System32\WindowsPowerShell\v1.0\powershell.exe" `
  -Argument "-NoProfile -ExecutionPolicy Bypass -File `"$runner`"" `
  -WorkingDirectory $repo
$principal = New-ScheduledTaskPrincipal -UserId $user -LogonType Interactive -RunLevel Limited
$settings = New-ScheduledTaskSettingsSet -AllowStartIfOnBatteries -DontStopIfGoingOnBatteries `
  -ExecutionTimeLimit (New-TimeSpan -Minutes 30)
Register-ScheduledTask -TaskName $taskName -Action $action -Principal $principal -Settings $settings -Force | Out-Null
Start-ScheduledTask -TaskName $taskName
```

完成后读取 `dist\windows-interactive-smoke.log` 与 `(Get-ScheduledTaskInfo -TaskName $taskName).LastTaskResult`，再执行 `Unregister-ScheduledTask -TaskName $taskName -Confirm:$false`。

| 覆盖项 | 现有 Windows fixture/harness 设计 | 当前实测状态与缺口 |
| --- | --- | --- |
| CSP 与 IPC fallback | `windows-smoke` 检查 CSP 下的基础 bridge；`bridge-route-smoke` 的 CSP-blocked lane 检查 API 控制仍走 IPC。 | 两个 app smoke 均未在 guest 运行。runner unit tests 不构成 CSP/IPC 行为证据。 |
| WebSocket 与 IPC 双 lane、双向 1 MiB | `bridge-route-smoke` 为 WS lane 和 CSP-blocked IPC lane 分别发送/读取 1 MiB `FileHandle` 并核对 checksum，也检查 channel 和 frame/protocol 行为。 | Windows guest 尚未实际启动 runner；无 Windows 1 MiB 或双 lane 结果。 |
| 子进程、取消与 Job 清理 | Windows smoke/bridge runner 有 child-process stream 场景；`stdio-host/verify_exit.py` 检查应用退出后的子进程延迟副作用。 | 这些 Windows app 场景均未运行。退出检查不直接验证 Job Object 成员关系，也不覆盖各种取消/超时下的进程树清理。 |
| Raw stdio | `stdio-host/verify.py` 覆盖原始 stdin/stdout、拆分 UTF-8、EOF 和日志隔离。 | 尚未运行。Windows smoke 自己的 JSON-lines 控制协议不能替代 raw stdio 验收。 |
| 同源与跨源 iframe | `windows-smoke` 有一个同源 iframe 和 opaque-origin 拒绝场景；bridge route harness 检查同源 child frame 的独立 session。 | 尚未运行；没有同源嵌套 iframe 的独立断言，也没有完整跨源 HTTP iframe/显式 grant 拒绝矩阵。 |
| BFCache | 当前 Windows harness 没有覆盖 WebView2 BFCache 的 back/forward、persisted Realm 或 session 恢复。 | 未覆盖。macOS API smoke 结果不能替代 Windows 证据。 |
| Native API、menu owner、快捷键 | Windows smoke 设计包含 68 个可逆 Native API 行为检查；`windows-smoke/run.py` 可检查 child HWND 的 Win32 `GW_OWNER`；interactive smoke 通过真实点击和全局快捷键完成桌面动作。 | 均未在 Windows guest 执行；Native API、owner、菜单和快捷键保持待验证。 |

## 验收边界

当前记录证明 Windows 11 ARM64 上的 x64 Rust 工具链、Node/TypeScript 构建和部分纯 JS/runner 测试可执行；它不证明 Rust workspace 门禁、最终 Windows release 产物、3,500,000-byte 体积门槛、实际 Windows bridge/stdio、WebView2 GUI 或桌面原生交互通过。未运行项继续开放。远端 CI 与原生 x64 硬件验证也未由本记录覆盖。
