# Window / Tray / Menu / Shortcut / Icon 整治方案

> 状态：方案（未实现）。范围：`window_manager`、`tray_manager`、
> `shortcut_manager`、`menu`、`image_utils` 及对应 API 层。
> 以下每条均已对代码实锤，先修正确性，再补能力。

## 1. `owner_window` 取错字段（bug，P0）

- 位置：`window_manager/builder.rs` Windows 分支，`with_owner_window`
  用的 `windows_extra.parent_window`，应为 `windows_extra.owner_window`
  （`options.rs:41` 字段存在，从未被读）。
- 后果：配了 owner 的窗口实际拿到 parent 语义；owner 语义（最小化/层叠跟随）
  永远配不出来，且无任何报错。
- 修法：改一行 + 单测（options 反序列化出 owner 字段；行为级验证需 Windows 真机，
  见 §7）。

## 2. `GlobalHotKeyManager::new().expect()` 启动即 panic（P0）

- 位置：`shortcut_manager/mod.rs:29`。无障碍权限/被占用等情况下整个应用
  启动崩溃，无日志、无降级。
- 修法：`new()` 改返回 `Result`，`NivaApp::new` 用 `?` 上抛（已有 anyhow 链路），
  启动日志打一行可读错误。快捷键是可选能力，缺它不该死机。

## 3. muda 菜单跨线程触碰（soundness，P0）

- 现状：`NivaWindow` 靠 `unsafe Send/Sync` 声明线程安全，但 muda 句柄是
  Rc-based（thread-bound）。`api/window.rs` 的 `set_menu/hide_menu/show_menu`
  跑在 smol driver 线程，直接调 `window.set_menu` → `build_menu`（文件 IO +
  创建 muda item）+ `attach_menu`（拿 `menu_handle`），全在非主线程。
  `switch_menu` 走事件循环没问题，tray/shortcut 的 API 已包 `run_on_main`，
  唯独 window 菜单三件套漏了。
- 修法：三件套包 `run_on_main`（与 tray/shortcut 一致），零行为变化。
  长期：roadmap P1 的"菜单线程安全靠自觉"项保留，终极解是把 muda 句柄收归
  主线程所有 + 跨线程只传 id/命令（本次不做）。

## 4. 原生菜单项点击刷错误日志（噪音，P1）

- 现状：`event_handler::dispatch_menu_event` 把 `event.id().0` 按 u16 解析，
  解析失败记 `Invalid menu id` 错误。但 PredefinedMenuItem（Quit/Copy/…）
  的 id 是 muda 自动生成的非数字串，点一次刷一条 error。
- 修法：解析失败直接静默忽略（return Ok），只对"能解析但查无此窗口"的
  报 warn。原生项本就由 OS 接管，我们无事可做。

## 5. 快捷键与菜单图标只在 macOS 生效（能力缺口，P1）

- 现状：`menu/mod.rs` 里 accelerator 解析与 `IconMenuItem` 全用
  `#[cfg(target_os = "macos")]` 包住，非 macOS 硬编码 `None`/普通项。
- 事实：muda 的 `Accelerator::from_str` 与 `IconMenuItem` 都是跨平台的，
  cfg 门是迁移时保守留下的，无技术原因。
- 修法：去 cfg，全平台解析 + 全平台 IconMenuItem；图标加载失败已回退普通项。
- 已知限制（文档化，不修）：Windows 下菜单快捷键需消息循环里调
  `TranslateAcceleratorW`，tao 事件循环没做——Windows 快捷键显示但不触发，
  写进 d.ts 注释与文档站，避免用户报 bug。

## 6. Tray/菜单图标超大 PNG 直接喂（显示问题，P1）

- 现状：`png_to_tray_icon` / `png_to_muda_icon` 原尺寸转 RGBA，
  用户配 1024px 图标时 Windows 托盘显示异常/模糊，菜单图标同理；
  非 png 格式直接报错（tray 场景用户手里经常是 .ico）。
- 修法：
  - 加 `image` 依赖（`default-features = false, features = ["png"]`，
    复用已有的 `png` crate，体积增量 KB 级，符合 3MB 红线）。
  - `image_utils` 加 `resize_rgba(rgba, w, h, max_dim)`（Lanczos3，经 `image`）。
  - tray 图标最长边压到 64，菜单图标压到 32；窗口/任务栏图标保持原尺寸（OS 自己缩放）。
  - 非 png 输入：错误文案写清"只支持 png"（.ico/.icns 支持另排期，不在本轮）。
- 验证：单测（1024px 进 → 64px 出 + RGBA 长度校验）；真机看托盘（macOS/Windows 各一次）。

## 7. 验证与风险

- `cargo check/clippy/test` + `tsc`（零新增 API，d.ts 只加一条 Windows 快捷键限制注释）。
- e2e：现有 WS 电池照跑；托盘/菜单真机点一遍（macOS 本机 + Windows 需真机，
  顺带把 roadmap P0 的"Windows 从未编译验证"蹭了——本轮改动含 Windows 分支，
  正好当理由）。
- 平台范围：**不支持 Linux**（已决策）。`#[cfg(not(any(target_os = "macos", target_os = "windows")))]`
  分支保持"能编译、不承诺行为"，不测、不修、不写文档。
- 体积：新增 `image(png-only)`，release 复测 3MB 红线。
