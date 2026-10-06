# Windows 源码盲审与修复记录（2026-10-06）

本记录汇总当前工作树针对 Windows 条件代码、窗口/菜单、WebView2、进程/stdio、资源校验与打包流程的源码审阅和修复。当前用户确定的完整 release runtime 硬门禁为 macOS 严格小于 3,300,000 bytes、Windows 严格小于 3,500,000 bytes；macOS 3,000,000 bytes 仅为参考目标。本轮 Windows release 产物和体积未测。检查在 macOS 主机进行；本轮没有 Windows 设备。此前 2026-09-23 的有限真机记录继续保留在[历史验证记录](windows-validation-2026-09-23.md)，不代表本轮源码的真机验收。

当前 runtime fingerprint 为 `e966c98485f436ab9adf0b78919dddc66a1374e6542457a4431d66571c9b63ef`，生成 bootstrap 为 630,009 bytes。异步 API 创建与控制仍统一通过 IPC `t:"api_call"`；Wry IPC/`evaluate_script` 与可选 WebSocket 只承载双向流数据，WS 不处理 API dispatch。当前源码没有新增平台专用 IPC/reply 协议，也没有改变公开 frame 格式或 wire version。

## 已确认并修复

| 审阅发现 | 修复与回归证据 |
| --- | --- |
| Windows 菜单 accelerator 没有接入 Tao 默认事件循环的按键翻译路径。 | 在应用 Windows 消息钩子中，按消息窗口定位对应顶层原生窗口和其菜单 accelerator，再调用 `TranslateAcceleratorW`。处理前释放窗口/菜单锁，避免 Win32 同步回调重入造成锁死；按 `GA_ROOT` 选择窗口，使 owner 子窗口使用自己的菜单。源码检查与 Windows 条件编译通过；没有本轮 Windows 实机按键记录。 |
| 菜单 API 替换时旧菜单析构可能清除新菜单；对一个可见窗口调用 `setMenu(null)` 会误把可见性偏好改成隐藏，导致随后 `setMenu(Some)` 不 attach。 | 替换前显式解除旧 native menu，再附加新菜单，并保留调用前的显隐偏好；新增纯逻辑回归覆盖连续替换和 null→Some。Windows 原生点击/快捷键矩阵仍需真机验证。 |
| `extra.getActiveWindowId` 对空 `GetForegroundWindow` 结果可能暴露伪造的字符串 `"0"`。 | 空 HWND 现在映射 `Promise<string | null>` 中的 `null`；Rust 测试通过，`contracts.ts` 与生成的 `contracts.d.ts` 均声明该类型。 |
| Windows Wry 会向每个 frame 注入 `window.ipc`，但原生消息接收只接顶层 WebView；嵌套同源 iframe 如果选择直接 parent frame 的 ipc endpoint，请求无法到达 Native。 | Runtime 从 child realm 沿完整同源祖先链选择顶层 Native IPC host，并使用既有 reply relay/session 映射；跨源或无法访问的中间祖先 fail-closed，不回退到中间 iframe endpoint。新增 Runtime tests 覆盖嵌套路由和拒绝路径；没有本轮 Windows 或嵌套 iframe 真机专项。 |
| Windows 的死键重置和窗口 resize-drag 使用了调用 API 的线程上下文，而不是窗口主线程。 | 两项窗口操作改为 `run_on_main`；其余主线程归属由源码审查确认。没有新增 Windows 真机输入操作证据。 |
| Tao 的 parent 与 owner builder 设置彼此覆盖；WS_CHILD 子窗口不支持原生窗口菜单。 | parent 与 owner 同时设置时返回配置错误；parent 子窗口配置初始菜单或调用 `setMenu(Some)` 时受控拒绝。单独 parent、单独 owner 和 owner 菜单路径保留。配置/逻辑测试和 Windows target check 通过。 |
| Windows stdin 同步读与取消之间存在消费后丢失输入的竞态，取消也可能命中已复用的线程 ID；子进程可能在分配 Job Object 前创建后代。 | stdin 对一次已完成但被取消的 OS read 保留最多 64 KiB，并由下一活跃 reader 恰好交付一次；稳定打开 reader 线程句柄后再开始读取，取消与可能阻塞的 channel send 不持有注册表锁。子进程创建时先挂起、加入 kill-on-close Job Object 后再恢复，并在启动失败时尝试终止/回收。Linux/macOS 可运行的状态机和进程回归包含在 workspace 测试中；Windows 取消时序仍需真机验证。 |
| Windows 资源路径复核用 64 位文件索引可能把 ReFS 上不唯一或无效的 ID 当成相同文件。 | 优先使用卷序列号和完整 128 位 File ID；只有 API 明确报告不支持或扩展 ID 为零时才尝试 legacy ID；legacy ID 为 0 或 `u64::MAX` 时拒绝身份比较。纯策略回归 4 项通过、Niva Windows target check 通过。没有在 NTFS/ReFS/FAT 真机复测。 |
| PE 打包失败可能覆盖或删除已有输出；runtime 与新建父目录下的输出别名检查可能漏掉 `..`、符号链接或硬链接。 | 先在输出目录创建独占临时文件、复制和修改 PE，成功后才发布；失败只清理本次临时文件，原有输出保留。路径解析处理不存在后缀、父目录 `..`、符号链接和现有硬链接。`win_packager` 17 项 Rust 测试覆盖失败保留、成功替换和别名拒绝；Windows PE 写入未在 Windows 实机运行。 |
| `EndUpdateResourceW` 的提交调用会关闭更新句柄，提交失败后不能再用相同句柄执行所谓回滚。 | 移除对已关闭句柄的二次调用并保留提交错误；由上层 staging 事务保护既有输出。条件编译检查通过，真实 PE 更新失败路径仍需 Windows 验证。 |
| `build_Windows.cmd` 使用已移除的 debug 参数，也没有执行当前统一打包器的完整 build 流程；WebView2 临时检查脚本写入失败时可能遗留半成品。 | Windows 脚本迁移到当前 `niva-packager build` 参数与本地 release 流程；Node helper 的语法检查和 6/6 回归通过。临时脚本在写入前建立清理守卫，并新增失败注入测试。WebView2 最低版本仍保留 `98.0.1108.44`，只清除已移除 Frame2 adapter 的旧注释；没有据 SDK 版本推断可下调运行时下限。 |

## 验证结果

- `cargo fmt --all -- --check`、`cargo check --workspace`、`cargo clippy --workspace --all-targets`、`cargo test --workspace` 在 `ulimit -n 4096` 下通过。最终测试结果为 Niva 234 passed/2 ignored、niva_packager 27 passed、validation integration 2 passed、win_packager 17 passed，共 280 passed/2 ignored；2 个忽略项需要平台受信任的出站 HTTPS 证书。第一次完整测试有 1 项 UDP 端口释放检查报 `AddrInUse`；单独复跑该用例通过，随后 workspace 全量复跑通过。Clippy 保留 warning。
- `cargo check -p niva --bin niva --target x86_64-pc-windows-msvc` 和 `cargo check -p win_packager --target x86_64-pc-windows-msvc` 通过。
- `cargo check --workspace --target x86_64-pc-windows-msvc` 未通过：本机没有 MSVC C 标准库/头文件，`lzma-sys` 和 `bzip2-sys` 找不到 `stdlib.h`，`ring` 找不到 `assert.h`。这项环境限制没有被改写成 Windows target 通过。
- Runtime build/typecheck/test 通过（179 tests）；types typecheck、浏览器/CommonJS/Node consumer checks 和 fresh-pack 四种消费者通过；Devtools build 通过，保留 minified JS chunk 907.68 kB 超过 500 kB 提示阈值的 warning。
- `cargo build -p niva` 成功；其 SHA-256 为 `26315117ea2db5843fbe59676f44a7fba640909b529db6905a59c0df1563050d`。真实 macOS WebView 回归：Node compatibility 179 checks/17 cases、Native API 39 method cases、bridge-route WS/IPC 两 lane 均通过。日志为 `/tmp/niva-windows-review-node.log`、`/tmp/niva-windows-review-macos-api.log`、`/tmp/niva-windows-review-bridge-route.log`；后者结果在 `/tmp/niva-windows-review-bridge-route/result.json`。Native API 默认 suite 未运行 dialogs、clipboard、shortcuts。
- macOS ARM64 `cargo build -p niva --release` 产物 `target/release/niva` 为 3,073,952 bytes，SHA-256 `1570fdc6680f3713f0b6a4407f680bd242c5b841e75f41688d46019c0d63dfc1`。它低于 macOS 3,300,000 bytes 硬限制，但高于 3,000,000 bytes 参考目标。
- 远端 GitHub Actions/CI 本轮未核对；API 读取返回 403。

## 仍需平台验收

Windows 真机仍需覆盖 WebView2 启动与主/嵌套同源 iframe IPC、菜单 accelerator 和菜单替换、stdin 取消/重读、子进程 Job Object 清理、资源路径在实际文件系统上的身份语义、PE 资源写入及更新失败清理。Windows 主程序 release 体积和完整打包产物也未测；本轮 MSVC C 头文件缺失使 Windows release build 无法完成。macOS 交叉编译、Windows target check 和 2026-09-23 的旧设备记录都不能替代这些当前源码的操作证据。
