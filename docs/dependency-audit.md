# 第三方依赖审计（2026-09-22，crates.io 实测数据；2026-09-23 全部落地）

> 结论：核心栈（wry/tao/muda/tray-icon/smol/tungstenite）全部最新且健康；
> 8 个需要动（含 3 个已死、2 个大版本落后）——**已全部处理完毕**，见 §6。
> JS 侧刚升级完（React 19/Vite 8/TS 5.9），
> 唯一风险是 `@bramblex/state-model` 单人维护（已有 `common/state.ts` 隔离层兜底）。

## 1. 健康（不动）

| crate | 用途 | 状态 |
|---|---|---|
| wry 0.57 / tao 0.37 / muda 0.20 / tray-icon 0.25 | 窗口/菜单/托盘 | 均为 2026-09 最新，tauri 官方维护 |
| global-hotkey 0.8 | 快捷键 | 最新，官方维护（发版慢但稳定） |
| arboard 3.6 | 剪贴板 | 最新，1Password 维护 |
| smol 2.0 / tungstenite 0.30 / async-channel 2.5 | 运行时/WS/队列 | 均为最新 |
| serde / serde_json / anyhow / url / glob / flate2 | 基础 | 均为最新 |

## 2. 小版本 bump（同 crate，低风险）

| crate | 现状 → 最新 | 说明 |
|---|---|---|
| opener 0.5 → 0.8 | 同仓库持续维护（2026-06），单调用点 | 直接升 |
| directories 4 → 6 | `BaseDirs/UserDirs` API 面稳定 | 升并跑通即可 |
| sys-locale 0.2 → 0.3 | 小版本 | 升 |
| png 0.17 → 0.18 | image-rs 维护中 | 升（顺带统一 icon_creator 的 0.24→0.25，`image` 用 png-only 特性） |
| native-tls 0.2.11 → 0.2.18 / mime_guess 2.0.4 → 2.0.5 / windows 0.62.0 → 0.62.2 / os_info 3.6 → 3.15 | 补丁/小版本 | 一起升 |
| base64 0.13 → 0.22 | wry 已在用 0.22，统一去重；`encode/decode` 改 Engine 写法，小改 | 升并去重 |

## 3. 需迁移（中工作量）

| crate | 现状 | 方案 |
|---|---|---|
| ureq 2.6 → 3.4 | 2.x 已让路 3.x（2026-09 仍在发版） | 升 3.x；用量小（`_request` 一个 helper），重写即可 |
| rfd 0.11 → 0.17 | 跨 6 个大版本；新版用 rwh 0.6 | 升，**顺带删掉 tao 的 `rwh_05` 特性**（与新 rfd 对齐），dialog 真机点一遍 |
| cocoa 0.24 + objc 0.2 | **objc 已死（2019）** | 迁 objc2 0.6（活跃，1.1 亿下载）；用量小（`focus_by_window_id` 一处），中工作量 |
| winapi 0.3 | **已死（2020）** | 迁现成的 `windows` 0.62；仅两处（`extra.rs` 前后台窗口、`win_utils.rs` 资源加载），小工作量 |
| fs_extra 1.3 | 沉寂 3 年半（能用但无维护） | 用 `std::fs` + 小 helper 替换（copy/remove 语义需保留 `CopyOptions` 行为），P2 |
| build-version 0.1 | **已死（2019，2 万下载）** | 直接删，用 `CARGO_PKG_VERSION`（版本号显示不断） |
| active-win-pos-rs（已禁用） | bindgen 冲突 | 换 `x-win` 5.8（2026-08 活跃），恢复 `getActiveWindowId` |
| winres 0.1 | 已死（2021）但构建期专用、能用 | 最低优先级；替代候选需单独调研，不阻塞 |

## 4. 暂不动

- `getrandom 0.3`：0.4 已出，但依赖树里多方用 0.3，无冲突就不动。
- `image`（图标缩放用）：按 window-tray-menu-plan 用 png-only 特性引入，与 icon_creator 统一到 0.25。

## 5. 行动顺序

1. §2 全升（半天，纯机械）→ 2. winapi + build-version + base64（小）→
3. rfd 0.17（中， dialog 真机验证）→ 4. ureq 3（中）→
5. objc2 + x-win（中，需 macOS 真机）→ 6. fs_extra 替换 + winres（P2，有空再说）。

## 6. 落地记录（2026-09-23，已提交推送）

补充（审计时遗漏的两处，落地中发现并处理）：

- `win_packager` 同样拖着 `ico 0.3`/`image` 全特性：已同步升级；
  进一步发现 `ico 0.5` 写死依赖 `png 0.17`（非可选），遂**手写 ICO 编码**
  （`win_packager::icon::encode_ico`，标准 ICONDIR + PNG payload，
  结构单测逐 entry 校验魔数/偏移/解码尺寸），删掉 `ico` crate——
  `adler` 整条链（`ico→png 0.17→miniz_oxide 0.6→adler`）彻底出树。
  `icon_creator` 瘦身为 thin wrapper（二进制实测可生成合法 7-entry ICO）。
- `cargo audit` 现状：**0 错误**，仅剩 3 条 warning（`paste` 经 x-win 的
  image/avif 链、`proc-macro-error`/`glib` 经 Linux-only gtk 栈——均为
  纯传递依赖，不可达/不可修，已接受）。

验证：macOS/Windows 双 target check、workspace clippy 零错误、
20 单测全过、ureq3 HTTPS 200、fs 9 项行为电池、WS/devtools 全回归、
release 2.45MB（< 3MB）。

全部完成，偏离审计预期的只有三处（都是往好的方向）：

- `base64`：没升级，直接**删除**——Rust 侧已无人用（流式化后 base64 只在 JS 层）。
- `winres`：没换 embed-resource——仓库里根本没有 `.rc` 文件，winres 一直空转，
  直接删除 `build.rs`。
- ureq 3 根证书：默认无根，需加 `native-tls-webpki-roots`（静态 Mozilla 包，
  +~200KB）。`platform-verifier`（系统根）会拖进 ring，+1MB 以上，直接爆 3MB，
  否决（与 Tauri http 插件同路线）。

验证：macOS check/clippy/test（15 单测）全过；**Windows target check 首次通过**
（此前从未验证，roadmap P0 一并关闭）；ureq3 HTTPS 200；fs 新实现 9 项行为电池 +
6 单测；WS/devtools 全回归；release 2.45MB（< 3MB）。
`getActiveWindowId` 经 x-win 恢复（macOS 实测待真机点击，接口已通）。
