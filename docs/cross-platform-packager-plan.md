# Niva 跨系统打包器（方案，未实现）

> 2026-09-23。本文只定义用户在 macOS 或 Windows 的 Devtools 中，勾选目标平台后为**自己的项目**导出可分享应用的方案。不是 Niva Devtools 自身的发布脚本，也不是已完成的跨系统打包能力。编译好的各平台 Niva 运行时由发布流程预先提供；本打包器不负责交叉编译运行时。

## 结论与交付边界

可以做一个独立的 `niva-packager`：同一套源码分别编译为 macOS 和 Windows 宿主程序，在任一宿主上读取项目资源与预编译运行时，按勾选目标分别输出 Windows exe 和 macOS app。打包器是**构建工具**，不得链接进用户应用的 `niva` 运行时；其体积与运行时大小分别计算。3 MB 是运行时参考目标，用户已接受 x86_64 略超该值；当前 Mac release 上限为 3.3 MB。

| 可选目标 | 预备运行时 | 零证书交付 | 当前状态 |
| --- | --- | --- | --- |
| Windows x86_64（WebView2） | Windows x64 `niva.exe` | 未签名 exe；SmartScreen 可能要求用户继续，Smart App Control 或管理策略可能禁止 | 现有 Devtools 只在 Windows 宿主打包 |
| Windows x86_64（MiniBlink） | 专用 exe、锁定版本的运行时 DLL | 同上；DLL 另列哈希、许可与实际签名状态 | [`miniblink-windows-plan.md`](miniblink-windows-plan.md) 标为未实现，UI 暂不可选 |
| macOS Apple Silicon | `aarch64-apple-darwin` 主程序 | 完整 app 做 ad hoc 签名；接收方可能需要“仍要打开” | 现有 Devtools 只在 macOS 宿主打包 |
| macOS Intel | `x86_64-apple-darwin` 主程序 | 同上；最低 macOS 版本还由 deployment target 和依赖决定 | 现有 Devtools 只在 macOS 宿主打包 |

这些包可以导出并分享，**不承诺每台设备都能直接运行**。没有受信任证书时，不能把系统警告称为签名成功；“已损坏”、签名校验失败、缺 DLL、丢失可执行权限或读不到资源则是打包失败。正式开发者签名与公证属于后续可选步骤，不能用 Niva 的签名身份替所有用户项目签名。

## 当前代码基线

- [`ProjectModel.build()`](../packages/devtools/src/models/project.model.ts) 根据当前 `os.info()` 在 [`build-macos.ts`](../packages/devtools/src/build-scripts/build-macos.ts) 和 [`build-windows.ts`](../packages/devtools/src/build-scripts/build-windows.ts) 中二选一，没有目标平台参数。
- [`win_packager`](../crates/win_packager/src/lib.rs) 已能跨平台准备压缩资源、PNG→ICO 和版本资源；最终 PE 写入调用 [`BeginUpdateResourceW/UpdateResourceW`](../crates/win_packager/src/windows_impl.rs)，非 Windows 宿主会明确报错。
- [`icon_creator`](../crates/icon_creator/src/main.rs) 现在只是调用 [`win_packager::icon::png_to_ico_bytes`](../crates/win_packager/src/icon.rs) 的薄 CLI。新打包器应直接复用这段库代码，不再下载或启动一个独立的 `icon_creator.exe`。
- macOS 路径仍调用 `sips`/`iconutil` 生成 ICNS；只有配置了 `sign.macos` 才进入当前签名函数。跨系统打包还缺跨平台 PNG→ICNS、归档权限保存与默认 ad hoc 签名。
- 当前工作区没有名为 `niva-core` 的独立 crate；下文的“运行时”指预编译的 [`crates/niva`](../crates/niva) 产物，不把“拆出 niva-core crate”当作本方案的前置条件。

## 依赖选择：能作为库，但有一道 LIEF 接口门槛

### LIEF：Windows PE 编辑

LIEF 核心采用 [Apache-2.0](https://raw.githubusercontent.com/lief-project/LIEF/1.0.0/LICENSE)。其官方 Rust API 支持 [修改 PE 资源树并写回文件](https://lief.re/doc/latest/formats/pe/modifications/resources.html)，所以可以作为 packager 的原生库依赖，而不必下载 Python 或运行独立 GUI 工具。拟锁定的 `lief` 1.0.0 不是纯 Rust：官方 [`lief-build`](https://raw.githubusercontent.com/lief-project/LIEF/1.0.0/api/rust/crates/lief-build/src/lib.rs) 会针对宿主目标取得并链接 LIEF/lief-sys 原生库；它列有 macOS arm64/x86_64 与 Windows x86_64 MSVC，但这些宿主的真实链接与运行还需 PoC。

但不能直接断言“加上 `lief` Rust crate 就完成现有资源格式”：Niva 目前按**名称**读取 `RT_RCDATA` 的 `RESOURCE_INDEXES`、`RESOURCE_DATA`。LIEF 1.0.0 Rust 的[资源节点接口](https://raw.githubusercontent.com/lief-project/LIEF/1.0.0/api/rust/crates/lief/src/pe/resources.rs)提供 `Directory::with_id`、`Data::with_buffer`、`add_child` 和写回，却没有给节点设置名称的方法；数据叶子也没有设置语言 ID 的公开方法，无法直接重现当前的 `1033` 语言项。[C++ `ResourceNode` 接口](https://lief.re/doc/latest/doxygen/classLIEF_1_1PE_1_1ResourceNode.html)则可设置名称和 ID。方案采用一个很小的 C++/FFI 桥接层，仅负责把 Rust 已准备好的命名 RCDATA、图标组和版本字节按正确语言 ID 写入 PE；打包流程仍由一个 Rust `niva-packager` 进程掌控。桥接范围与链接方式必须先通过 PoC 定型，不把 Python 解释器或第二个打包器作为隐藏依赖。

`RT_ICON`、`RT_GROUP_ICON`、`RT_VERSION` 的 payload 继续复用现有 [`icon.rs`](../crates/win_packager/src/icon.rs) 和 [`version_info.rs`](../crates/win_packager/src/version_info.rs) 的编码逻辑；LIEF 负责 PE 资源树与文件重建，不负责理解 Niva 的资源包协议。若预备 exe 含旧 Authenticode 签名，修改 PE 会使其失效：必须在写资源后确认产物状态为**未签名**，不能留下看似有签名却验证失败的文件。

### apple-codesign：macOS ad hoc 签名

`rcodesign` 的核心是 Rust [`apple-codesign` crate](https://docs.rs/apple-codesign/latest/apple_codesign/)，可以作为 packager 依赖。公开 [`BundleSigner`](https://docs.rs/apple-codesign/latest/apple_codesign/struct.BundleSigner.html) 和 [`SigningSettings`](https://docs.rs/apple-codesign/latest/apple_codesign/struct.SigningSettings.html) API 支持直接处理 `.app`；不配置签名密钥即为 ad hoc。实现必须在 `new_from_path` 后显式调用 `collect_nested_bundles()`，再以默认 `SigningSettings` 调用 `write_signed_bundle()`。packager 对**最终完成的 bundle**签名，并写到独立 staging 目录，不在签名后再修改 `Info.plist`、资源或主程序。它也支持 Windows 宿主，但 Apple 签名兼容性仍须在 macOS 实机验证。

`apple-codesign` 的 bundle 签名并非 Apple `codesign` 的已证等价物：上游仍列有 [bundle handling 限制](https://gregoryszorc.com/docs/apple-codesign/stable/apple_codesign_quirks.html)，0.29.0 还有[嵌套 framework 校验失败的报告](https://github.com/indygreg/apple-platform-rs/issues/311)。PoC 必须使用 Niva 的真实 bundle，并由 Apple `codesign` 和目标机启动验证，不能只看 `BundleSigner` 返回成功。

该 crate 为 [MPL-2.0](https://raw.githubusercontent.com/indygreg/apple-platform-rs/apple-codesign/0.29.0/apple-codesign/Cargo.toml)。即使 Niva 保持 MIT，发布 packager 二进制时仍须提供对应的 MPL 覆盖源码及获取说明，保留所选发行版的第三方许可文件。可关闭不需要的 `notarize` 默认 feature，但不得假定库或 packager 很小；只有最终产物测量能回答体积问题。[MPL 第 3 节](https://www.mozilla.org/en-US/MPL/2.0/)

### 其他跨平台依赖与归档

- PNG→ICNS：已有纯 Rust 的 [`icns` 0.5.0](https://github.com/mdsteele/rust-icns/tree/v0.5.0) 候选库，支持 `IconFamily::add_icon_with_type()`、`write()`，采用 [MIT 许可](https://raw.githubusercontent.com/mdsteele/rust-icns/v0.5.0/Cargo.toml)。2026-09-23 已用 crates.io 索引与 v0.5.0 源码核实版本及 `pngio` feature；docs.rs 当前默认页仍显示 0.4.0，不能据此推断 0.5.0 未发布。复用当前 `image` 依赖把源 PNG 缩放为 16–1024 像素的各档 RGBA 图，再由 `icns` 写一个多尺寸 icon family，替换 `sips`/`iconutil`。仅做 PNG 编码时先试 `default-features = false, features = ["pngio"]`，避免带入默认的 JPEG 2000 支持。该库目前编码为 PNG，不覆盖所有较新的 ICNS 子类型；以 Finder/Dock 显示、透明边缘和 Apple `iconutil` 读回的样例实测作为选型门槛，而不是把“有 crate”当作已验收。
- macOS 归档：ZIP 必须保留 `.app` 目录结构、符号链接及主程序的 Unix 可执行权限。macOS 宿主以 Apple 推荐的 [`ditto -c -k --keepParent`](https://developer.apple.com/documentation/xcode/packaging-mac-software-for-distribution) 作对照；Windows 宿主的 ZIP 实现必须产生等效元数据。Windows NTFS 上的普通 Rust [`Permissions`](https://doc.rust-lang.org/stable/std/fs/struct.Permissions.html) 不提供 Unix mode 设置能力，因此不能只靠 `set_permissions` 宣称 ZIP 中有 `0755`；须从带权限/符号链接的输入归档保留或在 ZIP 条目中明确写入目标元数据，并在 Mac 解压后检查。
- 运行时清单：每个目标绑定固定版本、来源和 SHA-256，不在构建时猜测 `latest`。MiniBlink DLL 单列版本、许可和哈希。Devtools 可以按需下载**宿主版 packager**与预备运行时，放在项目外的应用本地缓存；不改全局环境。

## 单一 packager 的流水线

1. **输入与预检**：Devtools 传项目目录、`niva.json`、已生成的资源目录、选中目标和输出根目录。验证所需运行时和工具哈希、目标最低系统版本、图标源、输出空间；MiniBlink 未落地时拒绝该目标。
2. **共用备料**：沿用现有 NodeCompat 资源 staging 约定；把 `niva.json` 首包，再确定性遍历资源，生成 `RESOURCE_INDEXES` 和压缩的 `RESOURCE_DATA`。每个目标只消费同一份经验证的项目内容。
3. **逐目标组装到独立 staging 目录**：
   - Windows：复制对应预编译 exe → 用 LIEF/FFI 写入两项命名 RCDATA、图标与版本 → 重读 PE 并核对资源名、字节与图标组。MiniBlink 版另放锁定的 DLL 清单。
   - macOS：创建 `Contents/MacOS`、`Resources` 和 `Info.plist` → 复制对应架构主程序并设置可执行权限 → 写资源数据及 ICNS → 用 `apple-codesign` 给整个 app 做 ad hoc 签名 → 归档。
4. **完成后验证和原子交付**：分别校验输出格式、资源读回、签名完整性或明确未签名状态、压缩包内容和哈希；全部通过才从与目标同一文件系统的 staging 目录原子移动到目标路径。默认拒绝覆盖既有产物，只有用户显式选择覆盖才替换；一个目标失败不抹掉其他目标结果，也不发布该目标的半成品。
5. **Devtools 结果页**：每个勾选项独立显示 `完成／失败／未支持`、架构、打包器与运行时版本、签名状态、哈希、产物路径和失败步骤。禁止把“ad hoc”标成“受信任签名”。

## 实施前必须关闭的 PoC

1. **LIEF PE 资源**：在 macOS 和 Windows 各用同一份预编译 exe，写入命名 `RESOURCE_INDEXES`/`RESOURCE_DATA`、多尺寸图标组和版本；对照现有 Win32 打包结果，用 Windows `FindResourceW` 和 Niva 运行时读回，检查 PE 校验和、旧签名处理、资源语言 ID 与实际启动。若小型 C++ 桥接无法稳定满足命名资源，不把 LIEF Rust API 的演示视为完成。
2. **apple-codesign app**：在 Windows 和 macOS 都给同一份 arm64/x86_64 测试 bundle ad hoc 签名。测试无图标、有图标、嵌套代码与长路径；在 Mac 上运行 `codesign --verify --deep --strict`、检查 `Info.plist`、架构、可执行位，再经**真实下载**首次打开。`spctl` 对 ad hoc 的不受信任结论与“已损坏”分开记录；“已损坏”不得放行。
3. **交付矩阵**：本轮只验证 Windows WebView2 普通版；MiniBlink 按用户决定暂缓，不是普通跨系统打包的前置 PoC，也不在 UI 中开放。macOS 两架构分别在对应机器或明确记录的 Rosetta 条件下验证。交叉打包、`cargo check --target`、哈希或本机签名校验都不能代替目标系统的启动、文件访问、窗口与 bridge 验收。
4. **许可证与产物**：固定 LIEF 与 apple-codesign 版本，记录最终 packager 的库/第三方许可与源码获取链接；确认用户导出的 app 不携带构建工具。分别记录 `niva` 运行时、packager、MiniBlink DLL 和完整交付目录的大小。

## 2026-09-23 本机核对记录（尚未关闭前置 PoC）

- 在 macOS arm64 临时副本上，用系统 `codesign --force --deep --sign -` 给 Devtools 测试 `.app` 做 ad hoc 签名；签名前后及用 `ditto -c -k --sequesterRsrc --keepParent` 归档、解包后，`codesign --verify --deep --strict` 均通过。解包后主程序仍为 `0755`，测试符号链接仍指向 `RESOURCE_DATA`。`spctl --assess` 返回 rejected，表明签名完整性与系统信任是两项检查。测试图标是占位文件，未做真实下载、首次打开或应用启动。
- 源码/元数据核对支持三个选型前提：LIEF 1.0.0 Rust 资源节点只有名称 getter，C++ 有名称 setter；`apple-codesign` 0.29.0 的 ad hoc、`BundleSigner` 三步 API 和 MPL-2.0 元数据存在；`icns` 0.5.0 与 `pngio` feature 已发布。**没有实际链接或运行 LIEF、apple-codesign、icns**，不能把接口核对等同于库 PoC。
- 尚缺 Windows 普通版的命名 RCDATA/图标/版本读回与启动、LIEF 跨宿主链接、`apple-codesign` 在 Mac/Windows 的实际产物、真实下载的 Gatekeeper 首开、两架构目标设备运行，以及许可清单。当前没有 Windows 真机；MiniBlink 按用户决定暂缓，不阻断普通版 PoC，但也不声称该目标可选。

PoC 通过后再把此文档转换为实施任务、UI 目标清单和 release 门禁。当前不得将本方案或 MiniBlink 兼容性标为已交付。
