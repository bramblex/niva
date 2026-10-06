# Niva 仓库门禁

适用于本仓库的代码、文档和发布工作。以当前源码和可复现的检查结果为准；`docs/` 中标为“方案（未实现）”的内容不能当作已交付能力。不要用 macOS 编译或 Windows target check 代替对应平台的真机验证。

## 0.10.0-beta.1 候选范围

本版冻结现有 Niva 原生 API、统一 TypeScript runtime、可选的 CommonJS/ESM Node 风格模块子集、以 IPC 为稳定异步通道并可选使用 WebSocket 优化流式操作，以及 GUI/CLI 共用打包核心。同步 XHR 仅用于需要同步返回的兼容 API。

本版不代表完整 Node.js 兼容、不代表移除 WebSocket、不代表 IPC 零拷贝，也不代表 v1.0 发布门禁已通过。候选验收状态以 `docs/release-0.10.0-beta.1.md` 为准；逐平台未验证项必须保留开放。

当前 bridge 已实现 API 控制面/纯数据面分离；当前源码合约与验证状态见 [`docs/bridge.md`](docs/bridge.md) 和 [`docs/bridge-v2-validation.md`](docs/bridge-v2-validation.md)。同步XHR保留为Node同步兼容入口；异步API以IPC `t:"api_call"`创建/控制，Native结果和流 `channelOpened`/capability 统一经Wry `evaluate_script`回调 `__niva_ipc_reply({sessionId,rid,sourceOrigin,response})`返回。资源owner固定为创建时IPC session，数据transport独立；WS v2仅attach/data/ack/cancel。顶层与同源iframe共用Wry路径；跨源iframe即使有grant也fail-closed。旧runtime指纹和旧平台报告不能替代当前源码证据；Windows target check 不等于真机验收。现有WKWebView reply handler与WebView2专用IPC/reply路径已移除，统一使用Wry IPC与`evaluate_script`。

## 改动验收

- Rust 代码改动完成后，运行 `cargo fmt --all -- --check`、`cargo check --workspace`、`cargo clippy --workspace --all-targets`、`cargo test --workspace`。影响 Windows 条件编译代码时，另运行 `cargo check --target x86_64-pc-windows-msvc`；无法运行的检查要写明原因与未覆盖范围。
- Devtools 或 TypeScript 改动完成后，运行 `npm run build --workspace=packages/devtools`，其中包含 `tsc --noEmit` 和 Vite 构建。`.husky/pre-commit` 会在类型检查失败时阻断提交，但不能替代直接构建证据；统一runtime还需运行 `npm run build --workspace=packages/runtime` 与types检查。仓库已有 `.github/workflows/ci.yml`，远端运行结果仍需另行核对。
- 改动 bridge、原生 API 或打包格式时，同步检查 Rust 实现、`packages/runtime/src/bootstrap.ts`、`packages/runtime/src/contracts.ts` 与生成的 `packages/types/dist/`、`docs/bridge.md` 与相关资源打包/读取代码；协议版本、帧格式、错误码和公开类型必须一致。对受影响的调用路径补有意义的测试或实际调用验证。
- 涉及窗口、托盘、菜单、快捷键、对话框或平台原生行为的改动，除编译和单测外，还要在受影响的平台做真机操作验证。无法取得平台设备时，不把该平台标为已验收。
- 改动依赖、资源打包或发布产物时，复测对应平台的完整 release runtime 主程序体积：macOS 严格小于 3,300,000 bytes，Windows 严格小于 3,500,000 bytes；macOS 3,000,000 bytes 仅为参考目标。体积需包含全部已确定目标功能的 Native 代码、内嵌 JS 与加载/索引开销，不能只量裸 Native 或库样本；记录产物、平台和实测大小。签名所需证书和密码由下游提供，秘密不得进入 `niva.json` 或仓库。

## 安全边界

- Bridge 与本地 HTTP/WS 服务的鉴权必须在 Rust 侧执行；不能只依赖前端检查、`Origin`、`Referer` 或 UA。启动 token 只在内存生成和保存，不从外部输入或日志泄露；未经授权的 API、资源及 `__niva_fs` 访问必须拒绝。
- 本地资源路径不得逃出资源根目录，包含编码后的路径穿越。远端页面和跨域 iframe 不能默认取得完整原生 bridge 权限。修改这些路径时，以 `docs/security.md`、`docs/permission-design.md` 和 `docs/http-auth-plan.md` 的威胁与验收用例核查。相关机制已有部分实现，但端到端威胁矩阵与平台验收未全部完成；不能据源码单独宣称安全验收通过。
- 不在有原生 bridge 权限的页面中执行从网络获取的不可信脚本。若确需远端内容，先明确来源和授权边界。

## v1.0 发布阻断项

以下项目来自 `docs/roadmap.md` 的“v1.0 门禁”。完成前不得将版本标为 v1.0 或宣称通过发布验收：

1. 完成并验收 origin 权限方案（本地包全权、远端默认零权），或将 v1.0 范围明确限定为仅支持本地资源，并写明远端 entry 风险。
2. 确保资源路径穿越被拒绝，并以目标平台证据验证 `docs/http-auth-plan.md` 的最小鉴权，至少证明 `__niva_fs` 未授权访问被拒绝。
3. 关闭 `docs/window-tray-menu-plan.md` 的三个 P0：快捷键初始化错误可控返回、Windows owner 字段正确、窗口菜单操作回到主线程。
4. 建立实际运行 `cargo check`、Clippy、Rust 测试、TypeScript 检查和 Vite 构建的 CI；本地执行记录不能替代 CI 门禁。
5. 在 Windows 真机运行关键流程。`cargo check --target x86_64-pc-windows-msvc` 只证明交叉编译检查通过。

每项关闭时留下对应提交、检查结果或真机记录；仅修改勾选状态或引用旧报告不算关闭。
