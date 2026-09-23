# Niva Roadmap / 待完善事项

> 更新：2026-09-23。固定本地页面 origin 已接入源码；当前实现与平台验收边界见下文。
> v1.0 门禁见下一节；源码交付不等于目标平台真机验收。

## v1.0 门禁（盖章前必须清零）

- [ ] **origin 权限**：打包本地资源已通过 Wry 异步自定义协议加载；macOS 页面 origin
  为 `niva://app`，Windows WebView2 origin 为 `http://niva.app`。WS token 按窗口绑定，
  握手校验页面 `Origin`、服务 `Host` 与 token；显式 debug Vite 入口单独受精确
  loopback origin 规则约束。macOS 有限 smoke 已完成，Windows 仍只有 target check，
  跨平台真机与完整威胁用例未验收，门禁保持开放（`docs/permission-design.md`、
  `docs/wry-custom-protocol-plan.md`）。
- [ ] **资源路径穿越 + HTTP 路径边界验收**：`FileSystemResource` 已加入解码后
  canonical 根目录约束，`/__niva_fs/<window-token>/` 已有按窗口 token 校验。打包
  普通静态资源改由 `niva://app/` 提供，打包模式普通 HTTP 静态路由关闭；WS 与
  `__niva_fs` 仍使用动态 loopback 服务。显式 debug HTTP 静态/NodeCompat 路由没有
  窗口 token 鉴权；从新打包 origin 直接 fetch `__niva_fs` 属跨源请求，文件服务仅对
  token 所属窗口的精确 `trusted_ws_origin` 返回 CORS allow header。macOS smoke 已验证
  精确 origin 可读、缺失 token 返回 403、非匹配 Origin 无 ACAO；Windows 尚未验收。
  旧 session-cookie 草案已由现行 scoped file-token 方案取代，见
  `docs/http-auth-plan.md`。此项门禁保持开放，待验证实现与验收证据；不表示调试
  模式中的普通 HTTP 静态资源均已鉴权。
- [ ] **window-tray-menu 三个 P0**：源码已移除 hotkey 启动 panic、修正 Windows owner 字段，菜单三件套已回主线程；仍需 macOS/Windows 实际菜单操作和 Windows owner 行为验收（`docs/window-tray-menu-plan.md` §1–3）。
- [ ] **CI**：`.github/workflows/ci.yml` 已加入双平台检查；待推送后实际运行并处理结果。
- [ ] **Windows 真机跑一遍**（现仅 target check）。

以下能力不作为 v1.0 发布门禁：`win_packager` 迁移已落地，但 Windows PE 写入和生成程序仍需真机验收；stdio bridge 已有实现，macOS 宿主往返已验，Windows 管道仍待验证；NodeCompat 已作为 workspace package 存在，完整兼容性验收和独立仓拆分不纳入本版；wry 能力补齐（`docs/api-coverage.md` §3）与 devtools 自身签名（需证书）。MiniBlink 按用户决定暂缓，本轮不实施也不纳入验收。

## 产品定位（已定）

- 服务**轻量开发者，甚至 AI**：普通小白 vibe-coding 出小工具，用 Niva 打包成单文件"蛋壳"可执行文件分发。
- 推论：CLI/脚本化构建与机器可消费的文档（d.ts）和 AI 亲和一样重要，不只做 GUI devtools。
- **签名责任在下游**：打包产出的是新应用，由开发者自己签名；Niva 最多给 devtools 自身做签名。

## P0 —— 正确性缺口
- [x] **Windows 构建验证**：`cargo check --target x86_64-pc-windows-msvc` 已通过
  （2026-09-23，装 target 后首验证；此前从未验证）。覆盖托盘、HWND、窗口菜单挂载、
  `beginResizeDrag`、fs 盘符路径。真机运行仍待一台 Windows。
- [ ] **`extra.getActiveWindowId` 真机验收**：源码已改用 `x-win`，仍需在 macOS 和 Windows 实际切换窗口并验证返回值。

## P1 —— 架构债

- [x] **依赖治理**：详见 `docs/dependency-audit.md`（已全部落地：
  ureq 3、rfd 0.17、objc2、winapi→windows、fs_extra→std（`app/fs_ops.rs`，
  fs_extra 语义逐字移植）、build-version 删除、x-win 恢复、base64 删除、
  winres 删除、小版本 bump 全升）。
- [ ] **其他阻塞操作的取消**：`process.execStream` 普通子进程已实现取消后终止和回收；`unblock` 中其他阻塞段在超时后仍可能继续执行，需按各 API 的可取消性逐项处理。
- [ ] **菜单线程安全靠自觉**：muda 句柄是 Rc（thread-bound），`unsafe Send/Sync` + “只在主线程碰菜单”无编译器强制。
- [x] **安全模型文档化**：`docs/bridge.md` 安全模型 + `docs/security.md`
  全面评估（含信任模型、分级发现、eval 专项）。残留行动转 P0/P1 新条目。
- [x] **线协议版本**：`WIRE_VERSION` / `Niva.bridgeVersion = 1` 已同步，
  hello 握手显式校验版本。
- [ ] **entry 信任分级**：源码已区分打包本地固定协议 origin、显式 debug loopback
  entry 与远端 IPC grant，见 `docs/permission-design.md`；v1.0 门禁仍待 Windows 真机
  与完整来源/iframe 用例验收。
- [x] **资源目录路径穿越约束**：`FileSystemResource` 解码路径并校验 canonical
  根目录，含符号链接逃逸拒绝；见 `docs/security.md`。此源码能力不代表整个
  HTTP 静态服务都已鉴权，v1.0 对应 HTTP 边界验收仍开放。
- [ ] **wry 能力验收及 tao 缺口**（1.x）：`evaluate_script`/`load_url`/`reload`/cookie/标题跟随等基础源码已补齐，macOS 已验证部分实际调用；Windows 真机与 `docs/api-coverage.md` §3 的未测方法仍待验收，tao 侧缺口见同文件 §1–2。
- [x] **WS Host/Origin 校验**：握手要求 Niva 服务 `Host`、窗口 token 与该窗口被授权的精确页面 `Origin`。打包页为 macOS `niva://app` / Windows `http://niva.app`；显式 debug 可授权精确 loopback Vite 源。macOS smoke 覆盖固定协议 origin 下的 WS 调用，Windows 真机仍待验收。
- [x] **CSP 默认模板**：新建简单项目带可编辑的严格 CSP meta；默认关闭 NodeCompat。协议页和显式 `debug-resource` 文档导航会补本次 loopback WS/HTTP 到 `connect-src`，并给 Niva 文件 URL 常用的图片、媒体、字体、样式 source 加精确 HTTP origin。启用 NodeCompat 且页面自带 CSP meta 时，Niva 注入块移到 CSP meta 后，仅两条 Niva 注入脚本获每次导航随机 nonce；没有 CSP meta 的旧页面不增加强制策略。macOS WKWebView 已验证 debug-resource 与打包 `niva://app` 正负例；Windows WebView2 真机仍未验收。宿主额外的 CSP response header 无法由本功能修改，完整边界见 `docs/security.md`。

## P2 —— 发布与工程

- [ ] **CI 落地验收**：Actions 工作流已写入仓库；仍需在 GitHub 实际运行并留下成功记录。
- [x] **Mac 双架构打包脚本**：本轮文档/版本字段核对后的候选源码已重跑 `build_MacOS.sh`，生成 arm64 和 x86_64 两份 zip；裸二进制分别为 2,835,264 与 3,175,264 字节，均低于 3,300,000 字节上限，3,000,000 字节仍是参考目标。归档完整性、Mach-O 架构和两份 `Info.plist` 的 `0.9.9.0` 版本已核对。产物标记含 `-dirty`，不等于最终提交或已签名发布包；同轮首跑曾卡在自举构建并人工终止，随后手动构建和完整脚本重跑通过，稳定性仍需进一步验收。
- [x] **签名工具（一站式，证书用户自备）**：devtools 构建成功后按 `sign` 配置自动签名。macOS（codesign deep/runtime + verify，可选 notarytool 公证 + stapler，钥匙串 profile 零秘密）；Windows（signtool + PFX，密码走 env）。秘密永不进 niva.json。
- [ ] **devtools 自身签名**：用上面这套工具给 devtools 打包签名（需证书）。
- [ ] **Stdio host bridge 验收**：`--stdio` NDJSON 管道、main-only `host.send` 与 stdout 日志隔离已有实现；macOS Python 宿主往返、坏帧恢复、EOF/BrokenPipe 退出已实测。Windows pipe 继承与退出仍需真机验证，见 `docs/stdio-host-design.md`。
- [ ] Node.js 支持、系统通知 Notification 按原 README TODO 排期。MiniBlink 按用户决定暂缓，不进入本轮实现与验收。
- [x] **文档站源码更新**：已升级到 Docusaurus 3.10.2，首页保留原产品文案，采用 Devtools logo/配色、实际 Devtools 示例项目窗口截图及重做的四张介绍图；API 页按当前 Rust 注册名、初始化脚本与 d.ts 核对，新增 Bridge、权限、流式、stdio 和 NodeCompat 入口。本地 `npm run typecheck` 与 `npm run build` 通过；线上发布与浏览器矩阵不由此项证明。
- [ ] **测试覆盖**：已有 Rust 单测与本机手工 WebView 验证；CI 落地后将关键 e2e 脚本化进仓库。
- [ ] **首页旧营销文案核对**：保留用户指定的原文案；目前没有同口径证据支持 `Electron 的 1/10`，本轮 x86_64 Niva 裸二进制为 3,175,264 字节（高于 3,000,000 字节参考值）。Windows 包含单个 exe，macOS 交付物是 `.app` 目录；跨主机导出仍是未实现的方案（见 `docs/cross-platform-packager-plan.md`）。这些证据不足以支持把相应说法当作当前所有平台的共同事实。

## P3 —— 体验优化

- [ ] **NodeCompat 能力验收**：可选浏览器包现位于 `packages/node-compat`，已有 15 个模块、ESM/classic 资源选择与 macOS 部分真实 WebView 验证；同步 API 范围仍待产品决定，Windows 真机与完整签名/行为覆盖仍待验收。见 `docs/node-compat-design.md` 和 `docs/node-api-frequency.md`。

- [ ] **Devtools 构建终端跨平台验收**：源码已移除假百分比，显示实际步骤并接入 execStream stdout/stderr/stdin；bridge 已新增逐帧 `onChunk`。macOS 临时 app 已实测进程运行期间 onChunk 先于终局 onBlob 到达，Devtools 构建窗口也完成 stdout/stderr/stdin 往返并显示构建成功；Windows 真机终端行为仍待验证。
- [ ] **clippy 剩余 warning**（多为历史遗留）。
- [ ] **JS 封装的 base64 编解码全量进内存**：大文件场景可接受，注明即可。

## 已验证记录与边界（2026-09-23）

- 本轮候选的双架构 zip 名为 `NivaDevtools_v0_9_10-18-gf3f9036-dirty_MacOS_{aarch64,x86_64}.zip`。核对结果为 arm64 裸二进制 2,835,264 字节、x86_64 3,175,264 字节；zip 均通过 `unzip -tq`，内部 Mach-O 架构分别正确。两份 `Info.plist` 的 `CFBundleShortVersionString` 均为 `0.9.9.0`，来自项目顶层 `version: "0.9.9"`。这是带未提交改动的本机候选，不代表已签名/下载的发布包；Windows release 体积未测。此前未启用 `codegen-units=1` 的本机 arm64 `target/release/niva` 为 2,904,944 字节，不应与双架构产物混为一数。
- macOS 手工 WebView 验证：本地主 frame 与同源 iframe 分别走 WS，跨源顶层页与 iframe 走 IPC；授权/拒绝、CSP `connect-src 'none'`、文件 URL 有无凭据均得到预期结果。Windows 仅完成 target check，未做真机验证。
- 固定 origin 协议 smoke：以临时 macOS `.app` 实测主页面及同源 iframe 的 `niva://app` origin、WS 原生调用与静态 JS 资源；普通 HTTP 静态路径返回 404。页面用 `webview.baseFileSystemUrl()` 成功 fetch 文件；有效 token 配精确 `niva://app` Origin 得到 ACAO，非匹配 Origin 无 ACAO，缺失/无效 token 为 403。NodeCompat 临时资源包仅选 `path/fs/assert/stream`，实测静态 `import 'path'`、`Niva.import('fs/promises')`、`require('assert/strict')` 和未选 `child_process.js` 返回 404；完整模块/API 仍未验收。未实测协议页二进制流、存储跨重启、窗口间 token 隔离或异步压力。Windows `cargo check --target x86_64-pc-windows-msvc` 通过，WebView2 真机仍待验收。
- CSP 模板 smoke：macOS WKWebView 以 Devtools `generateNewProject()` 的实际输出分别跑过 debug-resource 与临时打包 `.app`。`niva://app` 打包页实测严格模板 CSP 下 NodeCompat 静态 `import path`、`require('assert/strict')`、`Niva.import('fs/promises')`/本机读取、WebSocket `Niva.api`、`baseFileSystemUrl()` fetch/图片和成功 marker；Debug-resource 页验证同样的默认模板和 NodeCompat opt-in。负向对照页的作者 inline importmap 收到 enforcing `securitypolicyviolation`（`script-src`），随后 bare `import('probe')` 拒绝，只有这个分支会创建 `blocked-pass`，无 `unexpected-allowed` 或 fail marker。调试 smoke marker 位于 `/tmp/niva-csp-template-smoke/{default-pass,allowed-pass,blocked-pass}`；打包协议 marker 为 `/tmp/niva-csp-template-smoke/packaged-pass`。这些是本机临时夹具证据，不代替 Windows WebView2 测试；宿主 response-header CSP 未覆盖。
- 显式 debug 启动下的本机跨端口入口可获得该窗口完整 WS bridge；macOS 本机测试页已验证流式 `process.exec`，普通启动忽略嵌入配置的 `debug.entry`。完整 Devtools Vite UI/HMR 尚待实际操作验收。
