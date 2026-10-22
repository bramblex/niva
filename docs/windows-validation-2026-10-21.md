# Windows 发布准备（2026-10-21）

状态：验收进行中，未发布。

## 源码和环境

按用户要求丢弃原工作区旧代码后，`D:\Workspace\niva` 的 `main` 更新到 `origin/main`：`f96d613cbc135f48ac80817889e3ed8494c9ba90`。叠加 MacBook 同一基线的未提交 Windows 构建/测试修复（7 个 tracked 文件和 2 个新增文件），再在本机修复本轮发现的问题。没有修改远端工作区、推送、打标签或发布。

本机为 Windows 11 Pro 10.0.26200、Intel Core i5-12490F、x64 Rust 1.98.1、Node 24.18.0、npm 11.16.0、Python 3.14.6、WebView2 155.0.4283.62。进程与 Explorer 同处登录桌面 session 1。Node 22.14.0 精确版本矩阵另行开放。

当前仓库版本为 `0.10.0-beta.1`。`release-1.0.0-alpha.md` 是后续候选准备计划，不代表已经改版或满足发布门禁。用户于本轮明确确认：Windows 完整 release 主程序严格小于 3,500,000 bytes 即可接受，取代此前 3,300,000-byte 门槛。该约定与当前 helper/CI 一致，仅适用于 Windows。

## 当前检查

详细命令日志在仓库 `.release-evidence/`，每项记录退出码。

| 检查 | 结果 |
| --- | --- |
| npm ci | 通过 |
| Runtime build / tests | 通过，179/179；bootstrap 630,009 bytes |
| Types、consumer、fresh-pack | 通过，4 类独立消费者 |
| Devtools build | 通过；保留 Vite chunk warning |
| Bridge runner 单测 | 5/5；使用 unittest discover，仅证明 runner |
| Windows release helper 单测 | 6/6 |
| cargo fmt / check / clippy | 初始检查通过；保留 warnings |
| Windows MSVC target check | 初始检查通过 |
| cargo test workspace | 初次失败：209 passed、4 failed、2 ignored；正在修复 |
| Release、打包、真实 WebView | 待完成 |
| 远端 CI | 未确认；本机无 gh，connector PR runs 空结果不能证明 main CI |

原始失败分别为复制文件保留时间戳、TLS fixture 依赖 OpenSSL、请求取消时限、自定义协议 Windows origin 断言。失败日志保留，修复后记录新的完整门禁结果。

## 本轮改动

- 同步已有的 Windows runtime/types 构建和测试修复、bridge runner 修复。
- Windows release Cargo 构建增加 `--locked`，避免发布构建悄然改变依赖解析；对应 helper 测试更新并通过。
- 发布 helper 验证打包产物为 PE32+ x64 且使用 Windows GUI subsystem（值 2），拒绝会分配控制台的 console subsystem；含反例的 helper 6 项测试通过。
- Windows 子进程统一设置 `CREATE_NO_WINDOW | CREATE_SUSPENDED`，保留 Job Object 与管道。真实子进程探针证明 `GetConsoleWindow() == 0` 和 stdin/stdout 往返；取消回收测试通过。
- 修正文件时间戳测试的 Windows 写句柄要求、IPC origin 的平台期望；HTTP 测试改用测试专用自签名 PFX 并验证真实取消 EOF。测试证书不得用于生产或加入信任根。

## 验收边界

本地检查不能替代远端 CI。未执行的桌面交互、安全矩阵、BFCache、其他平台及第三方许可材料闭包，不因单个 smoke 或构建通过而标记完成。最终产物必须另列字节数、SHA-256、runtime fingerprint 和实测结果。


## 已取得的真机与发布材料证据（最终构建仍需重验）

- 初始自动 smoke 的严格 CSP 拒绝内联脚本、原生 API/Node 模块和文件 fetch 通过；150 KB 文件/资源读写、exec stdout/stderr 通过。
- 初始 smoke 在同源 iframe 失败。诊断确认 WebView2 的 document-start 启动时地址仍为 `about:blank`，runtime 缓存了 IPC 不可用状态；不能把顶层路径通过写成 iframe 已通过。
- 原始 stdio（含 UTF-8 分片）和退出后子进程清理通过，结果见 `.release-evidence/stdio/`、`stdio-exit/`。
- 本机按 release helper 的顶层 LICENSE/COPYING/NOTICE 文件规则审计 Windows 依赖闭包：508 个 package，20 个 registry package 缺少该规则能找到的原文。详见 `.release-evidence/windows-license-audit.json`；仅说明收集缺口，不推断许可授权状态。
- 直接 GitHub Actions API 对基线 SHA 查询返回 HTTP 403 rate limit exceeded，未能确认远端 CI；当前本地修复也尚未提交，不能取得这些修复的远端成功记录。
