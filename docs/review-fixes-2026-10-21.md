# 2026-10-21 整体 review 修复与 Windows 验证

基线 `f96d613cbc135f48ac80817889e3ed8494c9ba90` 加上前一 Windows 发布准备会话的未提交修改。本轮由主线程执行，未使用 subagent，未提交、推送或发布。保留前会话修改及历史失败记录。

## 改动

- Wry WebView 和 Muda 菜单存入主线程对象表；后台 `Arc<NivaWindow>` 只携带实例 handle，不能取出线程绑定对象。关闭时在 owner 线程释放 UI。Tao Window 继续使用库自身线程契约。
- 普通 IPC 沿用注册超时和 `api.timeoutMs`；对话框使用异步 rfd、无操作 deadline，会话取消仍保留。前端普通 RPC 有配置期限加 10 秒的回包容差。
- HTTP 同时最多 128 个连接，请求头 5 秒截止；IPC 入站在建立任务前限制 256 KiB 和全应用 128 个 pending 请求。
- 新 `niva-config` 共享核心配置预检，Devtools 通过可信 Native API 使用同一实现；校验 UUID、开关、API 容量/期限、窗口常用属性、尺寸及权限。平台有效配置用于打包元数据。预检不取代操作系统相关的窗口创建检查。
- 资源格式 v2：每文件独立 64 KiB Deflate 块，Range 只解压相交块；运行端、统一打包器及低级 Windows packager 共用源码。kit schema 改为 2，旧格式必须重打，不提供兼容分支。
- 修正遗留 Windows 严格 CSP CommonJS 启动的同步 cwd 依赖，以及外部同源 iframe 在 about:blank 注入时的延迟初始化。WS smoke 按实际帧 id/seq 关联 ACK，不把后安装监听器的顺序冒充 wire 顺序；采集报告前等待当前事件分发结束。
- CI 增加 Windows 真实 bridge、生命周期和 stdio 步骤及证据上传。发布 helper 遇缺少 registry 许可原文时生成缺口清单并失败，停止静默省略。

## 本轮结果

原始日志及结构化摘要位于 `.release-evidence/review-fixes/`。

| 检查 | 结果 |
| --- | --- |
| cargo fmt --all -- --check | 通过 |
| cargo check --workspace | 通过 |
| cargo clippy --workspace --all-targets | 通过，既有 warnings 保留 |
| cargo test --workspace | 273 passed / 2 ignored；不重复计入嵌套子进程测试 |
| cargo check --target x86_64-pc-windows-msvc | 通过 |
| Runtime build + JS suite | 184/184 通过 |
| Types consumers | browser / CommonJS / Node 三类通过 |
| Devtools build / tests | 通过 / 7 项通过；chunk 大小警告保留 |
| Windows release helper / bridge runner | 7 项 / 5 项通过 |
| Windows release WS + 严格 CSP IPC | 两条 lane 通过，含同源 iframe 和二进制流 |
| Windows embedded / external 资源布局 | 两种各 7 项通过；含 33 MiB 资源 Range 与 CJS/ESM |
| Windows 既有 GUI/API smoke | 68 项通过；另覆盖 CSP、资源 token、owner、菜单状态及流 |
| stdio / 子进程退出清理 | 通过 |
| 配置 500 ms IPC deadline / 关闭执行任务中的子窗口 | 真机通过 |
| 对话框停留 40 秒后确认 | 未验收，见下述记录 |

对话框最初两轮 runner 假设确认按钮 ID=1，实际测试弹窗唯一按钮 ID=2，发送的关闭操作无效。修正为枚举自己启动进程、精确标题弹窗中的唯一按钮后，2 秒诊断轮成功返回；该诊断不证明 35 秒边界。最后长等待轮弹窗提前返回，未满足计时断言，因此保持失败/未验收，不推断提前关闭原因，也不继续打扰用户重测。对应生命周期超时与取消的 Rust 回归通过，但不替代这项交互验收。

## 产物与开放项

Windows 完整 release runtime：**3,499,008 bytes**，严格小于 3,500,000，余量 992 bytes。SHA-256 `cb775daf9ae8925cd4ed3dd87e994adae7c493cabd0190dbdefc9917b355f089`。Runtime fingerprint `6e42a939d4b3bf7ea3d51146c742bdcc2e5e08a4642a762b0d16a72bd80fcbb3`。

本轮没有 macOS 构建、体积及真机证据，相关项目继续开放。当前未提交源码没有远端 CI 运行记录；基线 combined-status 查询返回空 statuses，不能认定 CI 通过。

沿用前会话清单重新检查 Windows 依赖闭包，20 个 registry package 未找到许可原文，发布 helper 已实际拒绝该不完整 kit，清单见 `.release-evidence/review-fixes/license-kit/license-audit.json`。这说明分发材料缺口，不对许可授权状态作法律判断。资源 smoke kit 仅是隔离测试材料，不能作为完整发行 kit。

旧候选 ZIP 属于前会话快照，不是本轮新资源格式的可发布包。本轮未创建新的正式发布 ZIP，也没有放宽体积、许可证、平台或 CI 门禁。
