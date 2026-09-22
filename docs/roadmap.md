# Niva Roadmap / 待完善事项

> 更新：2026-09-22。本轮重构（Rust 1.98 / wry 0.57 / 全异步 API / WS 桥 / smol 服务器）完成后盘点。

## 产品定位（已定）

- 服务**轻量开发者，甚至 AI**：普通小白 vibe-coding 出小工具，用 Niva 打包成单文件"蛋壳"可执行文件分发。
- 推论：CLI/脚本化构建与机器可消费的文档（d.ts）和 AI 亲和一样重要，不只做 GUI devtools。
- **签名责任在下游**：打包产出的是新应用，由开发者自己签名；Niva 最多给 devtools 自身做签名。

## P0 —— 正确性缺口

- [x] **Windows 构建验证**：`cargo check --target x86_64-pc-windows-msvc` 已通过
  （2026-09-23，装 target 后首验证；此前从未验证）。覆盖托盘、HWND、窗口菜单挂载、
  `beginResizeDrag`、fs 盘符路径。真机运行仍待一台 Windows。
- [ ] **`extra.getActiveWindowId` 还是 stub**（macOS 返回 None）。`active-win-pos-rs` 因 bindgen 0.59 与新 SDK 不兼容被禁，需换 `x-win` 等维护中的 crate 恢复。
- [ ] **工作区及时提交**：大重构不要长期躺在工作区。

## P1 —— 架构债

- [x] **依赖治理**：详见 `docs/dependency-audit.md`（已全部落地：
  ureq 3、rfd 0.17、objc2、winapi→windows、fs_extra→std（`app/fs_ops.rs`，
  fs_extra 语义逐字移植）、build-version 删除、x-win 恢复、base64 删除、
  winres 删除、小版本 bump 全升）。
- [ ] **超时/取消不杀底层**：exec 子进程、unblock 里的阻塞段在超时后继续跑完（abandoned）。要真杀需把 Child 句柄登记进 active 表。
- [ ] **菜单线程安全靠自觉**：muda 句柄是 Rc（thread-bound），`unsafe Send/Sync` + “只在主线程碰菜单”无编译器强制。
- [x] **安全模型文档化**：`docs/bridge.md` 安全模型 + `docs/security.md`
  全面评估（含信任模型、分级发现、eval 专项）。残留行动转 P0/P1 新条目。
- [x] **线协议版本**：`WIRE_VERSION` / `Niva.bridgeVersion = 1` 已同步，
  hello 握手显式校验版本。
- [ ] **entry 信任分级**（remote 默认无桥）：见 `docs/security.md §4` —— P0，需产品决策。
- [ ] **资源路径穿越修复**：见 `docs/security.md §4` —— P0，搭 http-auth-plan 便车。
- [ ] **Host/Origin 校验、`http` SSRF blocklist、CSP 默认模板**：见 `docs/security.md §4` —— P1。

## P2 —— 发布与工程

- [ ] **无 CI**：`cargo check/clippy/test`、`tsc`、`vite build` 全靠人肉。至少加 GitHub Actions。
- [ ] **打包脚本过期**：`build_MacOS.sh` 是 CRA 时代假设，按 vite 产物对一遍。
- [x] **签名工具（一站式，证书用户自备）**：devtools 构建成功后按 `sign` 配置自动签名。macOS（codesign deep/runtime + verify，可选 notarytool 公证 + stapler，钥匙串 profile 零秘密）；Windows（signtool + PFX，密码走 env）。秘密永不进 niva.json。
- [ ] **devtools 自身签名**：用上面这套工具给 devtools 打包签名（需证书）。
- [ ] Node.js 支持、系统通知 Notification、miniblink 按原 README TODO 排期。
- [ ] **文档站过期**：`packages/website` 缺 stream 新 API 文档。
- [ ] **测试覆盖**：6 个单测 + /tmp 手工 e2e；CI 落了之后把 e2e 脚本化进仓库。

## P3 —— 体验优化

- [ ] **Node-like 兼容层（独立仓库）**：本仓只守 bridge 合约（见 `docs/bridge.md`）；
  另起一仓做 `require('fs'/'path'/'os'/'child_process')`、`readFile`、
  `os.platform` 等 Node 形状封装，供 AI/轻量开发者直接 vibe-coding。
  频率依据与落地顺序见 `docs/node-api-frequency.md`。

- [ ] **devtools 构建进度条是假进度**：现在有 execStream + stdin，可做真进度 + 可交互终端（流式架构最直观的 payoff）。
- [ ] **clippy 剩余 warning**（多为历史遗留）。
- [ ] **JS 封装的 base64 编解码全量进内存**：大文件场景可接受，注明即可。

## 已验证基线（本轮）

- release 二进制 2.2–2.3MB（< 3MB 目标）。
- e2e：unary/流式/timeout/cancel/iframe/auth 全过；devtools 静态 + dev-server 双路径存活。
