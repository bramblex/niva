# Niva 跨系统打包器

> 实现记录，2026-09-24。用户应用打包方式见 [使用说明](packager-usage.md)。MiniBlink 继续暂缓。本文的源码实现、编译检查与真机验收分别记录。

## 结构

独立的 `crates/niva_packager` 提供宿主工具 `niva-packager`；不链接进 `crates/niva`。用户使用预编译工具包，不在自己的项目中编译 Rust。

```text
Devtools 多目标构建 / CLI
  → 固定版本工具包 manifest.json
  → 运行时 SHA256 + PE/Mach-O 架构校验
  → 同一份项目资源 + 选定 NodeCompat 模块
  → Windows PE 组装 / macOS app 组装与 ad hoc 签名
  → 资源读回或签名封装、归档
  → 每目标 staging 完成后发布到输出目录
```

每个目标独立成功或失败，已有文件拒绝覆盖。输出通过同文件系统硬链接原子发布；目标文件系统不支持硬链接时返回错误，不降级为不完整输出。资源文件按路径排序，显式 `niva.json` 放在数据首部。资源根内符号链接、路径越界、特殊文件和开启 NodeCompat 时用户资源占用 `__niva_compat` 会被拒绝。

## 平台实现

| 目标 | 后端 | 输出 | 签名状态 |
| --- | --- | --- | --- |
| Windows x86_64 / WebView2 | `editpe` 编辑 PE 资源树 | `.exe` | unsigned |
| macOS Apple Silicon | `plist` + `icns` + `apple-codesign` | 含 `.app` 的 ZIP | ad-hoc |
| macOS Intel | 同上，使用 Intel 运行时 | 含 `.app` 的 ZIP | ad-hoc |

Windows 后端改用 `editpe = 0.2.3` 的纯 Rust 资源树 API，支持名称和语言节点，不再需要原方案中的 LIEF/C++ 桥接。两项 named RCDATA 使用原有 `RESOURCE_INDEXES`、`RESOURCE_DATA` 和语言 1033；ICO 与版本 payload 复用 `win_packager` 库。重写后读回 RCDATA、版本和图标；清除失效 Authenticode certificate directory，并重新计算 PE checksum。原有其他资源（例如 manifest）保留。

macOS 后端使用 `icns = 0.5.0` 生成多尺寸图标，以 `apple-codesign = 0.29.0` 的 `BundleSigner` 收集嵌套 bundle 并写到独立签名目录。整个 app 完成后签名，签名后不改内容。ZIP 在所有宿主上显式写入主程序 `0755`、普通文件 `0644`、目录 `0755` 和符号链接元数据。外部项目资源本身不接受符号链接。

应用包中的签名不代表可信开发者身份：Windows 不签名；Mac 的 ad hoc 不等于 Developer ID 或公证。系统信任和真实下载首次打开须独立验收。现有 Devtools 的本机非交互构建/正式签名路径仍由发布脚本使用；跨平台入口不会自动运行用户的 `sign.*` 配置。

## 工具包与 Devtools

`.github/workflows/packager.yml` 分别使用 Windows x64、Mac arm64、Mac Intel runner 编译运行时和宿主 packager，再汇总三个运行时生成离线工具包。`scripts/create-packager-kit.py` 生成版本/哈希清单、NodeCompat 资源、文件校验和、依赖许可原文与精确版本源码链接。工作流上传构件，不自动发布正式版本。

Devtools 中选择可信工具包目录、输出目录和目标；路径存于应用本地设置。每目标显示结果、签名状态、版本、SHA256 和产物路径。构建中防止重复调用和切页丢失进度；应用自己的 API 超时设为 10 分钟，用户项目配置不受影响。

`apple-codesign` 的 MPL-2.0 义务适用于 packager 的对应依赖，不能因主仓库使用 MIT 而省略。完整工具包携带这些许可与对应版本源码获取说明。其依赖体积与 Niva 用户运行时分开统计。

## 验收记录

- Mac 本机工作区 fmt/check/Clippy/test 与 Devtools TypeScript/Vite 构建通过。Clippy 有既有 warning。工具包实际解压后使用其中的 packager 一次生成三目标，产物哈希与 Mac 签名检查通过。
- Mac 生成的 arm64 app：Apple `codesign --verify --deep --strict` 通过；归档/解压后执行位保留；实际启动后通过 stdio 回传 OS、窗口调用、资源读取、文件读写、NodeCompat 的成功结果。
- Mac 生成的 Intel app：同样通过签名验证与实际调用；运行环境为 Apple Silicon 上的 Rosetta，不能替代 Intel 真机。
- 真实 PNG 生成的 ICNS 由 Apple `iconutil` 成功读回。
- Windows 真机运行时基线 `25ee4b3` 编译成功，2,455,040 bytes。Mac 生成的 EXE 与 Windows 自己生成的 EXE 均通过上述 stdio 应用 smoke。
- Windows 原生 packager 生成的 ARM/Intel Mac ZIP，已传回 Mac 并通过 Apple `codesign --verify --deep --strict`、解压权限和上述应用 smoke；Intel 在 Rosetta 运行。ARM ZIP SHA256 `a51345097fc369de7b60400b9a226e45692d0f79e8fda98760f88200c7a73317`，Intel ZIP SHA256 `3acbdc32dbc255e9fef5167f4282b8f448e2e63bf921658b8caf036894079057`，对应最初验收候选 `8c8d392`。
- Mac 运行时：arm64 2,288,400 bytes，Intel 2,616,136 bytes；Mac packager 初测 3,868,432 bytes，三运行时完整工具包 ZIP 约 7.65 MB。工具包与运行时分别统计。
- Devtools 新面板使用浏览器 mock 点验了目标选择、构建中锁定、混合成功/失败结果与顶层预检错误。锁屏阻止了原生窗口点击验收，因此不把该浏览器检查当作完整原生 UI 验收。
- Mac 到 Windows 的全工作区 target check 被 packager 加密依赖的 MSVC C 头文件缺失阻断；`cargo check -p niva --target x86_64-pc-windows-msvc` 通过。packager 的 Windows 构建必须在 Windows 原生工具链验证。

可复用的真实包验证脚本：`scripts/packager-smoke.py --fixture <dir>` 生成测试项目，打包后用 `--artifact <exe或zip>` 在对应宿主启动与验收。测试覆盖普通 app，不宣称覆盖所有第三方嵌套 framework、操作系统版本、Gatekeeper 下载首开或企业策略。

## 最终代码候选与离线依赖（2026-09-24）

代码候选 `228c9afc33fcfc46ed13cbb7e09685156c2e6ebc` 的 [常规 CI](https://github.com/bramblex/niva/actions/runs/35955733263) 与 [三宿主工具包工作流](https://github.com/bramblex/niva/actions/runs/35955733328) 均已通过。三个宿主各自解压工具包、校验文件哈希，并生成 Windows x64、Mac ARM 和 Mac Intel 三目标；Mac runner 另做 Apple 签名校验。

`.cargo/config.toml` 对 Windows x64 固定静态 CRT，避免此前产物对 VC Runtime DLL 的依赖。由上述 CI 生成并下载核对的文件：

| 文件 | 字节数 | SHA256 |
| --- | ---: | --- |
| Windows runtime | 2,590,208 | `0997338b707b41823e0ec5df0c700f79b1c99226959ba6ba242d557291045091` |
| Windows packager | 4,845,568 | `81581b8e0564b802f8f570a0568bd8dab6e71e8bb6d6ff35f92a5d551603f576` |

解析 PE 导入表确认两者均无 `VCRUNTIME` / `MSVCP` / `CONCRT` DLL 依赖，只导入 Windows 系统 DLL。更新本地 Mac 工具包中的 Windows runtime 后，再次完成三目标打包与输出哈希、Mac 签名校验。

剩余验收明确保留：Windows 连接变得不可用，未能取得最终静态 CRT 产物的 GUI smoke 回报；该最终链接配置已通过 Windows CI 编译、单测和完整打包链，但没有把此前动态 CRT 候选的真机启动结果挪作它的最终启动证明。原生 Devtools 点击/文件夹对话框也因 Mac 锁屏未完成；已有浏览器 mock 交互验证。PR 保持草稿，未合并 main。
