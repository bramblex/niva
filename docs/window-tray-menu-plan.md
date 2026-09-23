# Window / Tray / Menu / Shortcut / Icon 整治状态

> 状态：方案中的源码修复已落地；平台操作验收未全部完成。以共享工作树源码为实现证据，以 `docs/roadmap.md` 中记录的平台结果为验收证据。本文件不把 target check 或源码单测视为真机验收。

## 1. P0：源码已修复，平台验收待完成

### 1.1 Windows `owner_window` 字段

- **源码：已修复。** `window_manager/builder.rs` 的 Windows 分支分别读取 `windows_extra.parent_window` 和 `windows_extra.owner_window`，并传入对应 Tao builder 方法；配置反序列化有区分两字段的测试。
- **平台验收：Windows 有限通过。** 2026-09-23 真机创建带 `ownerWindow: 0` 的子窗口，以 Win32 `GetWindow(GW_OWNER)` 确认 owner 指向主窗口；其他 owner/parent 组合未测，见 [验证记录](windows-validation-2026-09-23.md)。

### 1.2 全局快捷键管理器初始化

- **源码：已修复。** 初始化失败不再通过 `expect` 使应用启动 panic；管理器记录错误、输出诊断，快捷键能力不可用时由 API 返回错误。
- **平台验收：部分完成。** Windows 真机注册并触发 `Ctrl+Alt+Shift+F12`，收到了预期 ID；初始化失败路径、占用冲突及 macOS 专项仍待验证。

### 1.3 窗口菜单 API 的线程归属

- **源码：已修复。** `window.setMenu`、`window.hideMenu`、`window.showMenu` 均通过 `run_on_main` 在事件循环主线程执行；muda 菜单创建、替换与 attach 不再从 API driver 线程直接操作。
- **平台验收：部分完成。** Windows 真机调用 `setMenu`、`hideMenu`、`showMenu` 并点击原生菜单项，收到了预期事件；macOS 菜单及更多 Windows 菜单行为仍待验证。

以上三个 P0 的源码修复状态与剩余验收要求也记录在 `docs/roadmap.md` 的 v1.0 门禁中；门禁仍为开放状态。

## 2. P1：能力与边界

### 2.1 原生预定义菜单项

- **源码：已处理。** `event_handler.rs` 对无法解析为 Niva 数字菜单 id 的 muda 预定义项静默返回。这类系统项由操作系统处理，不应因 id 非数字刷错误日志。
- **验收：未记录专项平台操作结果。** 在 macOS/Windows 菜单操作中检查系统项和应用自定义项的日志及行为。

### 2.2 菜单快捷键和图标

- **源码：已实现通用构建路径。** `menu/mod.rs` 调用 muda 的 accelerator 解析，并使用 `IconMenuItem`；源码不再只为 macOS 编译这些构建逻辑。图标加载失败时回退普通菜单项。
- **平台限制：** Windows 菜单快捷键显示与触发行为尚需真机确认；roadmap 记录的现存风险是 tao 消息循环未处理 `TranslateAcceleratorW`，因此不能仅凭跨平台构建代码宣称 Windows 快捷键会触发。
- **验收：** macOS/Windows 各检查菜单项、快捷键实际响应及图标显示；target check 只能证明编译覆盖。

### 2.3 托盘与菜单 PNG 缩放

- **源码：已实现。** `resource_manager/image_utils.rs` 使用 Lanczos3 缩放；托盘图标最长边限制为 64 px，菜单图标最长边限制为 32 px。窗口图标不走这两个缩放限制。当前输入格式仍为 PNG。
- **源码测试：** 有 1024×512 输入缩放与 RGBA 长度检查。
- **验收：** 需要在 macOS 和 Windows 真机检查托盘、菜单图标显示效果；测试通过不能替代原生 UI 检查。`.ico`/`.icns` 输入支持不在本方案范围。

## 3. 平台验收与剩余工作

当前路线图给出的证据边界：

- **源码实现：** 第 1 节三个 P0 修复以及本文件列出的 P1 源码处理已存在于当前工作树。
- **macOS：** 有整体桥接行为手工验收记录；本文件所列菜单、托盘、快捷键专项真机操作仍须补充明确结果后，才可关闭对应平台验收。
- **Windows：** 已有 [真机 smoke 记录](windows-validation-2026-09-23.md)，覆盖 owner 关系、菜单点击及全局快捷键；托盘/图标、菜单 accelerator 和更广组合仍待验收。
- **发布状态：** roadmap 的 window-tray-menu 三个 P0 v1.0 门禁仍开放，直到 macOS/Windows 菜单操作和 Windows owner 行为验收有实际记录。

建议验收时记录设备/OS、配置、操作步骤、观察结果与对应源码/提交。不要用本计划的状态文字替代验收证据，也不要据此推断其他设计文档或 v1.0 发布门禁已全部完成。
