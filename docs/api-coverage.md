# Window / Webview API 覆盖审计（2026-09-23，以锁定版本实测为准）

> 基线：`tao 0.37.0` / `wry 0.57.0`（均为 crates.io 最新，无欠账）。
> 方法：registry 源码 `pub fn` 全枚举，对 `window.*` / `windowExtra.*` /
> `webview.*` 与 `NivaWindowOptions` 逐项打勾。结论：tao 侧约 95%，wry 侧薄。

## 1. tao Window（跨平台运行时）：已暴露 ✓ / 缺口 ✗

✓ 全套几何/状态/光标/菜单/注意力：`window.*` 现有 50+ 方法基本打满。

| 缺口 | 说明 | 优先级 |
|---|---|---|
| `set_window_icon`（运行时换图标） | 只有建窗 `options.icon`，换图标要重开窗 | 1.x P1 |
| `set_theme`（运行时切亮暗） | 只有建窗主题 + `theme()` 读 | 1.x P1 |
| `drag_resize_window` | 只有 `dragWindow`（移动）；Win 有 `beginResizeDrag`，跨平台无 | 1.x P1 |
| `set_progress_bar`（任务栏进度） | 三端都有语义（mac/Linux 为 app 级） | 1.x P2 |
| `request_redraw` | 小 | 1.x P2 |
| `set_ime_position`（CJK 输入法候选窗位置） | 中文场景迟早要 | 1.x P1 |
| `set_background_color` / `set_focusable` | 小众 | 1.x P2 |
| `set_min/max_inner_size` 传 null 清约束 | 现在只能设不能清 | 顺手 |
| `window.Decorated` 大写 D | 命名事故，改名 + 别名兼容 | 顺手（v1.0 前） |

## 2. 平台扩展

macOS（`windowExtra.*` 已有 9 个）：缺 `set_badge_label`（dock 角标，P1）、
`set_traffic_light_inset`（建窗+运行时都没有，P1）、
`set_activation_policy_at_runtime` / `set_dock_visibility`（P2）。

Windows（已有 7 个）：缺 `set_overlay_icon`（托盘叠加图标，P1）、
`set_rtl`（P2）；`has_undecorated_shadow` 读缺（顺手）。

**Bug（P0，`window-tray-menu-plan.md` §1 已立项）**：
`builder.rs:155` `with_owner_window` 取的是 `parent_window` 字段，
`owner_window` 配了也永远不生效且无报错。

## 3. wry WebView：薄，1.x 按序补

现状 `webview.*` 只有 5 个（devtools×3 + baseUrl×2）。

| 缺口 | 说明 | 优先级 |
|---|---|---|
| `evaluate_script` | Rust→JS 注入（无 WS 时的主线程推送） | 1.x P1 |
| `load_url` / `load_html` / `reload` / `url()` | 建窗后换页/刷新/读当前 URL，现在 entry 定死 | 1.x P1 |
| `print` | 系统打印 | 1.x P2 |
| `go_back/go_forward/can_go_back/can_go_forward` | 浏览器类应用 | 1.x P2 |
| cookie 四件套 + `clear_all_browsing_data` | 登录态/登出场景 | 1.x P1 |
| `with_document_title_changed_handler` → 标题跟随 | 小改大体验 | 1.x P1 |
| `with_on_page_load_handler` → `webview.loaded` 事件 | 小 | 1.x P1 |
| `with_new_window_req_handler`（`target=_blank` 现在去向不明） | 先查清现状再定 | 1.x P1 |
| 下载 handler / 权限 handler（摄像头/定位） | 现在默认行为不明，需先实测 | 1.x P1 |
| `with_user_agent` / `with_proxy_config` / `with_incognito` / 手势缩放 | 按需 | 1.x P2 |

注：`with_user_agent` 三端全替换语义、无读默认 API，动了会破坏页面兼容
（`node-compat-design.md` §7.5 已决策不做来源标记），UA 如要做只能做“追加模板”。
