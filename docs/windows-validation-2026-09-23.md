# Windows 真机验证记录（2026-09-23）

基线为 `main` 的 `3bfbb74`，加上本工作树中的 Windows 修复。设备为 Windows x86_64
`10.0.26200`，WebView2 Runtime `153.0.4234.48`，Rust `1.98.1`，Node `24.18.0`。
本记录只证明下列本机操作，不代表 v1.0 全面验收或已签名发布。

## 发现并修复

1. 干净克隆直接运行 NodeCompat 测试时，classic 构建因 `dist/` 不存在而失败。
   构建脚本现自行建目录。
2. NodeCompat 在 Windows 上把根目录相对路径解析成无盘符路径，导致
   `pathToFileURL('/tmp/...')` 与 Node 行为不一致。现从当前工作目录继承盘符，
   并修正跨平台测试的路径、shell 参数及 UNC 预期。
3. 首次 Windows Devtools 自举构建生成的 exe 与原始 `niva.exe` 哈希相同，
   启动报 `Failed to find resource`。直接执行 `win_packager` 定位到删除模板中
   不存在的 `RT_ICON` ID；Win32 先返回 `0x80070057`，后续删除还返回
   `0x8007054F`。现先检查模板实际存在的图标 ID，只删除存在的资源；
   资源更新失败时移除未完成的输出 exe。新增 Windows 单测覆盖缺失和已有图标，
   以及失败输出清理。
4. CSP 测试夹具揭示打包器在资源目录也含 `niva.json` 时会用该副本覆盖
   显式 `--config`。打包器现跳过资源目录中的同名文件，新增单测确认包内配置
   来自显式参数。

## 已通过的检查

| 范围 | 本机结果 |
| --- | --- |
| `npm test --workspace=packages/node-compat` | 31/31 通过 |
| Devtools 脚本测试 | 8/8 通过 |
| Devtools `tsc --noEmit`、Vite；网站 typecheck、build | 通过 |
| `cargo fmt`、`cargo check --workspace`、Clippy | 通过，Clippy 保留既有 warning |
| `cargo test --workspace` | Niva 58/58，`win_packager` 12/12 通过 |
| `build_Windows.cmd` | 完整通过；ZIP 完整性通过 |
| `examples/stdio_host.py target/release/niva.exe` | 收到 `ready`、`page:ready`、`Hello, parent!`，坏帧后继续工作，关闭 stdin 后正常退出 |
| [Windows smoke 示例](../examples/windows-smoke/README.md) | 打包页和原生 UI 项目均通过 |

最初默认 panic 策略下，裸 `target/release/niva.exe` 为 **3,877,376 字节**。
共享的 `[profile.release]` 现设置 `panic=abort`，Windows 重建后裸
`niva.exe` 为 **2,450,432 字节**；已低于 3,000,000 字节目标。对应的
`dist/NivaDevtools.exe` 为 **3,169,792 字节**，`FileVersion`/`ProductVersion`
均为 `0.9.9`；`NivaDevtools_v0_9_10-19-g3bfbb74_Windows.zip` 为
**1,868,744 字节**，完整性检查通过。exe 在 WebView2 中显示 Devtools 首页。
ZIP、打包后的 exe 与裸 Niva 二进制是三个不同口径。产物未签名。
`panic=abort` 使 macOS 与 Windows release 在意外 Rust panic 时直接终止进程；
Rust 检查和测试仍沿用 unwind。macOS `ipc_macos.rs` 中的 `catch_unwind`
因此不再能捕获 release panic，macOS 真机需重新验收。本机 Windows stdio、
打包页和严格 CSP smoke 已用 abort 版本复测通过；尚未覆盖刻意触发 panic 的行为。
在 Windows 上尝试 `cargo check --release --target aarch64-apple-darwin -p niva`
时，`objc2-exception-helper` 的构建脚本因缺少 macOS 目标所需的 `cc` 而停止；
这不构成 Mac 构建通过或失败的结论，仍须在 Mac 主机执行 `build_MacOS.sh`。
ZIP 文件名只包含 `git describe` 的提交号；本轮 Windows 修复尚在工作树中，
因此这个名称不能当作纯 `3bfbb74` 源码构建或正式发布的证明。

Windows smoke 从打包页实际得到 `http://niva.app` origin；主页面和同源 iframe
均调用 `window.current` 成功；不透明 `data:` iframe 的相同调用被 Rust 拒绝，
错误码为 `-4`。打包静态文件及带窗口 token 的文件 URL 可读取；
精确 Origin 返回 ACAO，错误 Origin 不返回 ACAO，无效 token 返回 403，
打包模式普通 loopback 静态路径返回 404。NodeCompat 的 `path`、`fs/promises`
和 `assert/strict` 在真实 WebView2 页面运行成功。`window.setMenu`、
`hideMenu`、`showMenu` 状态正常，点击原生菜单收到 ID 7；
`Ctrl+Alt+Shift+F12` 注册后触发预期快捷键事件。带 `ownerWindow: 0` 的子窗口
通过 Win32 `GetWindow(GW_OWNER)` 确认 owner 为主窗口。另一打包页在
`script-src 'self'`、`connect-src 'self'` 等严格 CSP 下完成原生 bridge、
NodeCompat 和文件 URL 读取；作者内联脚本被 CSP 阻止。

## 尚未覆盖

远端页面和跨源 iframe 的完整 IPC 授权矩阵、CSP 模板的更多拒绝用例、窗口间 token
隔离、托盘/对话框、菜单 accelerator、完整 NodeCompat API、签名、CI 远端运行，
以及跨系统打包均未由本轮 Windows smoke 证明。`extra.getActiveWindowId`、
Devtools 构建终端的实时 stdout/stderr/stdin 交互也仍需专项测试。
WebView2 在两次正常退出时向 stderr 输出 `Failed to unregister class
Chrome_WidgetWin_0. Error = 1412`；退出码为 0，后续可排查该清理告警。
