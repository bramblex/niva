# Niva 仓库门禁

适用于本仓库的代码、文档和发布工作。以当前源码和可复现的检查结果为准；`docs/` 中标为“方案（未实现）”的内容不能当作已交付能力。不要用 macOS 编译或 Windows target check 代替对应平台的真机验证。

## 改动验收

- Rust 代码改动完成后，运行 `cargo fmt --all -- --check`、`cargo check --workspace`、`cargo clippy --workspace --all-targets`、`cargo test --workspace`。影响 Windows 条件编译代码时，另运行 `cargo check --target x86_64-pc-windows-msvc`；无法运行的检查要写明原因与未覆盖范围。
- Devtools 或 TypeScript 改动完成后，运行 `npm run build --workspace=packages/devtools`，其中包含 `tsc --noEmit` 和 Vite 构建。不能把 `.husky/pre-commit` 的 TS 检查视为通过证据：该 hook 当前即使类型检查失败也可能继续提交。仓库目前没有 CI workflow。
- 改动 bridge、原生 API 或打包格式时，同步检查 Rust 实现、`crates/niva/assets/initialize_script.js`、`packages/types/Niva_zh.d.ts`、`docs/bridge.md` 与相关资源打包/读取代码；协议版本、帧格式、错误码和公开类型必须一致。对受影响的调用路径补有意义的测试或实际调用验证。
- 涉及窗口、托盘、菜单、快捷键、对话框或平台原生行为的改动，除编译和单测外，还要在受影响的平台做真机操作验证。无法取得平台设备时，不把该平台标为已验收。
- 改动依赖、资源打包或发布产物时，复测对应平台的 release 二进制体积；当前目标为小于 3 MB，并记录产物、平台和实测大小。签名所需证书和密码由下游提供，秘密不得进入 `niva.json` 或仓库。

## 安全边界

- Bridge 与本地 HTTP/WS 服务的鉴权必须在 Rust 侧执行；不能只依赖前端检查、`Origin`、`Referer` 或 UA。启动 token 只在内存生成和保存，不从外部输入或日志泄露；未经授权的 API、资源及 `__niva_fs` 访问必须拒绝。
- 本地资源路径不得逃出资源根目录，包含编码后的路径穿越。远端页面和跨域 iframe 不能默认取得完整原生 bridge 权限。修改这些路径时，以 `docs/security.md`、`docs/permission-design.md` 和 `docs/http-auth-plan.md` 的威胁与验收用例核查；上述权限与 HTTP 鉴权方案目前尚未全部实现，不能声称已安全落地。
- 不在有原生 bridge 权限的页面中执行从网络获取的不可信脚本。若确需远端内容，先明确来源和授权边界。

## v1.0 发布阻断项

以下项目来自 `docs/roadmap.md` 的“v1.0 门禁”。完成前不得将版本标为 v1.0 或宣称通过发布验收：

1. 落地 origin 权限方案（本地包全权、远端默认零权），或将 v1.0 范围明确限定为仅支持本地资源，并写明远端 entry 风险。
2. 修复资源路径穿越，并实现 `docs/http-auth-plan.md` 的最小鉴权，至少确保 `__niva_fs` 未授权访问被拒绝。
3. 关闭 `docs/window-tray-menu-plan.md` 的三个 P0：快捷键初始化错误可控返回、Windows owner 字段正确、窗口菜单操作回到主线程。
4. 建立实际运行 `cargo check`、Clippy、Rust 测试、TypeScript 检查和 Vite 构建的 CI；本地执行记录不能替代 CI 门禁。
5. 在 Windows 真机运行关键流程。`cargo check --target x86_64-pc-windows-msvc` 只证明交叉编译检查通过。

每项关闭时留下对应提交、检查结果或真机记录；仅修改勾选状态或引用旧报告不算关闭。
