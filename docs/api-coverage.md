# Window / Webview API 覆盖审计（2026-09-23，以锁定版本实测为准）

> 基线：`tao 0.37.0` / `wry 0.57.0`（均为 crates.io 最新，无欠账）。
> 方法：registry 源码 `pub fn` 全枚举，对 `window.*` / `windowExtra.*` /
> `webview.*` 与 `NivaWindowOptions` 逐项打勾。结论：tao 侧约 95%，wry 侧薄。

## 1. tao Window（运行时 API）

基于锁定的 Tao 0.37.0 registry 源码，补齐以下运行时能力。原生窗口操作通过 `run_on_main` 调度；表中平台限制来自 Tao 0.37.0 的公开注释或平台实现。

| Niva API | 行为 | Tao 0.37.0 平台支持与限制 |
|---|---|---|
| `window.setWindowIcon(iconPath, id?)` | 从应用资源读取图标；`null` 清除窗口图标。 | Windows/Linux 有实现；macOS 实现为空操作（macOS 没有 Tao 定义的窗口图标语义），iOS/Android 不支持；Niva 在这些平台返回不支持错误。 |
| `window.setTheme(theme, id?)` | `light` / `dark` 设置主题，`system` 或 `null` 恢复系统默认。 | Windows、Linux、macOS 有实现；Linux/macOS 是应用级主题；iOS/Android 不支持且由 Niva 返回错误。 |
| `window.dragResizeWindow(direction, id?)` | 按 `east`、`north`、`northEast`、`northWest`、`south`、`southEast`、`southWest`、`west` 开始边缘缩放。 | Windows/Linux 有实现；macOS/iOS/Android 返回 `NotSupported`。Tao 要求鼠标左键刚刚按下，经过 WS 异步调用不保证能满足该时序。 |
| `window.setProgressBar(options, id?)` | 设置 `state`、0–100 的 `progress` 和可选 `desktopFilename`。 | Windows/Linux/macOS 有实现；Linux/macOS 是应用级进度。Linux 需支持 libunity 的桌面环境；`desktopFilename` 仅用于 Linux Unity。Tao 在 Linux/macOS 将 `indeterminate` 当作普通进度，在 Linux 也将 `paused` / `error` 当作普通进度；iOS/Android 不支持且由 Niva 返回错误。 |
| `window.requestRedraw(id?)` | 请求事件循环在当前 OS 事件处理后发出 `RedrawRequested`。 | Windows/Linux/macOS 有实现；iOS 要求主线程（Niva 会调度到主线程）；Android 不支持且由 Niva 返回错误。 |
| `window.setImePosition(position, id?)` | 设置输入法候选窗在客户区左上角起算的位置，使用 Niva 逻辑坐标。 | Windows/macOS 有实现；Tao Linux 0.37.0 实现仍为空操作；Linux、iOS、Android 由 Niva 返回不支持错误。 |
| `window.setBackgroundColor(color, id?)` | 设置 RGBA 背景色；传 `null` 清除。 | Windows/Linux/macOS 有实现；Windows 忽略 alpha；iOS/Android Tao 实现为空操作且由 Niva 返回错误。 |
| `window.setFocusable(focusable, id?)` | 设置窗口是否可聚焦。 | Windows/Linux/macOS 有实现；macOS 已聚焦窗口设为不可聚焦后不能直接取消聚焦；iOS/Android 不支持且由 Niva 返回错误。 |
| `window.setMinInnerSize(null, id?)` / `window.setMaxInnerSize(null, id?)` | 清除相应尺寸约束；传 `NivaSize` 设置约束。 | Tao 的 `set_min_inner_size` / `set_max_inner_size` 接受 `Option<Size>`；iOS/Android 不支持且由 Niva 返回错误。 |
| `window.isDecorated(id?)` | 查询窗口装饰状态。 | 修复旧 `window.Decorated` 的大写命名错误；旧 ID 不再注册，统一使用 `isDecorated`。 |

这些接口在 macOS、Windows、Linux 桌面构建中可编译；Tao 的方法存在不等于该平台必定呈现相同系统效果。特别是任务栏进度、IME 候选窗、重绘和原生拖拽受桌面环境及输入时序影响。本轮没有相应平台的真机验收时，不将这些平台行为标记为已验证。

## 2. 平台扩展

macOS（`windowExtra.*` 已有 9 个）：缺 `set_badge_label`（dock 角标，P1）、
`set_traffic_light_inset`（建窗+运行时都没有，P1）、
`set_activation_policy_at_runtime` / `set_dock_visibility`（P2）。

Windows（已有 7 个）：缺 `set_overlay_icon`（托盘叠加图标，P1）、
`set_rtl`（P2）；`has_undecorated_shadow` 读缺（顺手）。

**Bug（P0，`window-tray-menu-plan.md` §1 已立项）**：
`builder.rs:155` `with_owner_window` 取的是 `parent_window` 字段，
`owner_window` 配了也永远不生效且无报错。

## 3. wry WebView：运行时 API 已补齐基础覆盖

`webview.*` 现在有 20 个 API 方法（原有 devtools×3、baseUrl×2，新增运行时方法 15 个）。WebView 操作通过 `run_on_main` 在事件循环线程调用 Wry。

| 已实现方法 | 行为与边界 |
|---|---|
| `evaluateScript(script)` | 在当前 WebView 主页面执行脚本，不读取返回值；UTF-8 字节数上限为 64 KiB。它是单次脚本执行 API，不承载 bridge 消息。 |
| `loadUrl(url)` / `loadHtml(html)` / `reload()` / `url()` | 导航只接受绝对 HTTP(S) URL（拒绝相对 URL、`javascript:`、`data:` 和其他 scheme）；相对应用资源可先用 `baseUrl()` 拼接。`loadHtml` 没有 base URL 参数。 |
| `print()` | 调起 Wry 的系统打印流程。 |
| `goBack()` / `goForward()` / `canGoBack()` / `canGoForward()` | 浏览历史及查询历史可用性。 |
| `cookies()` / `cookiesForUrl(url)` / `setCookie(cookie)` / `deleteCookie(cookie)` | 读操作返回 Set-Cookie 格式字符串数组；写入和删除接收 Set-Cookie 格式字符串。删除时传回含匹配 `name`、`domain`、`path` 的 cookie。Cookie 存储由 Wry WebContext 共享，改动可能影响同一应用的其他窗口；`cookiesForUrl` 的筛选遵循平台 Wry 实现。macOS Wry 0.57 对 IPv4 字面地址可能筛不出同域 Cookie，可用 `cookies()` 读取完整列表或使用 `localhost` 地址。Android 上 Wry 的 `cookies()` 返回空数组，`setCookie`、`deleteCookie` 返回不支持错误。 |
| `clearAllBrowsingData()` | 请求 Wry 清理底层 WebView 数据存储中的全部浏览数据；同一存储上下文中的其他窗口可能同时受影响。Windows 的 Wry 调用会异步请求清理，因此 Promise 解析表示请求已提交，不代表底层清理已完成。 |
| `with_document_title_changed_handler` | 文档标题变化后更新对应原生窗口标题。 |
| `with_on_page_load_handler` | Wry 报告 `Finished` 时发送 `webview.loaded`，payload 为 `{ url }`，仅本地 WebSocket 页面可接收。它代表 WebView 加载回调完成，不保证 HTTP 页面成功。 |
| `with_new_window_req_handler` | `target=_blank` / `window.open` 请求一律返回 `Deny`，不创建新原生窗口；发送 `webview.newWindowRequested`，其中 `url` 是目标地址。Wry 回调不提供发起页 URL；事件中的 `pageUrl` 是事件循环投递时读到的顶层 WebView URL，不能标识发起请求的 iframe。事件仅本地 WebSocket 页面可接收。 |
| `with_download_started_handler` | 一律返回 `false`，拒绝开始下载且不写入文件；发送 `webview.downloadStarted`，其中 `url` 是下载资源地址。`pageUrl` 是事件循环投递时读到的顶层 WebView URL，不一定是发起下载的 iframe。macOS/Windows 使用 Wry 0.57 对应平台下载回调。 |
| `with_permission_handler` | 对 `camera`、`geolocation`、`microphone`、`display-capture` 和 `other` 返回 `Deny`，并发送 `webview.permissionDenied`；其余权限返回 Wry `Default`，保留平台/浏览器默认行为（可能出现系统权限流程）。Wry 权限回调只有权限种类，没有请求来源 URL；事件的 `pageUrl` 是事件循环投递时读取的顶层页面 URL，不是来源授权依据。Wry 文档说明已持久化的站点权限可能绕过 handler。**macOS 的 Wry 0.57 未实现 geolocation 权限回调**，因此此处无法拦截或报告 macOS 定位请求；Windows 的 Wry handler 支持 geolocation。 |

尚未实现且不在本次范围：`with_user_agent`、`with_proxy_config`、`with_incognito` 和手势缩放。UA 仍遵循 `node-compat-design.md` §7.5 的决策：不做来源标记；若后续支持，只考虑追加模板。
