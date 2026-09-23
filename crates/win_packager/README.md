# win_packager

Windows exe 后注入打包器。高层模式在一次调用中完成资源目录打包、PNG 转
ICO、RCDATA、图标组和版本资源写入；不调用 `pako`、`icon_creator.exe` 或
`ResourceHacker.exe`。

## 1. 背景：现在是怎么打 Windows 包的

旧链路 `packages/devtools/src/build-scripts/build-windows.ts`：

1. 前端打包（`base.ts`）：`niva.json` 首包 + `readDirAll` 遍历 + `pako.deflateRaw`
   得到 `RESOURCE_INDEXES` / `RESOURCE_DATA` 两个临时文件；
2. `GENERATING_ICON`：`resource.extract("windows/icon_creator.exe")` 并执行，
   PNG -> 多尺寸 `icon.ico`；
3. `BUILD_EXECUTABLE_FILE`：`resource.extract("windows/ResourceHacker.exe")`，
   写 `bundle_script.txt` 调 `-script`：拷 `currentExe -> targetExe`，
   写 `RCDATA` + 删旧 `ICON 1..7` + 写 `ICONGROUP 1`。

读侧：`crates/niva/src/app/resource_manager/mod.rs:117-131` +
`win_utils.rs` 从 `RT_RCDATA` 的同名资源读回。

新链路会把 devtools 生成的版本信息写入目标 exe。

## 2. 新链路（实现）

```text
devtools: win_packager.exe --exe <currentExe> --save-as <target>
    --resource-dir <build输出目录> --config <niva.json>
    [--icon-png <icon.png> --delete-icon-ids 1,2,3,4,5,6,7]
    [--version-info VERSION_INFO] [--lang 1033]
  -> sign-windows.ts 签名（顺序不变：签名永远在改资源之后）
```

备料（遍历压缩）和 PNG->ICO 都在 crate 内完成（`bundle.rs` / `icon.rs`，
跨平台可测），不再需要 `icon_creator.exe` 落盘、前端 `pako` 压缩、
`bundle_script.txt`。

旧脚本 -> 新参数对照：

| 旧链路 | 新 `win_packager` 参数 |
|---|---|
| `[FILENAMES] Exe=` | `--exe` |
| `[FILENAMES] SaveAs=` | `--save-as` |
| `base.ts` 打包的 INDEXES/DATA 文件 | `--resource-dir DIR --config niva.json`（crate 内打包） |
| 已备好的零散文件（调试用） | `--rcdata <name>=<f>`（可重复，低层模式） |
| `icon_creator.exe` + `-addoverwrite <ico>, ICONGROUP,1,1033` | `--icon-png <png>`（crate 内转 ICO）或 `--icon <ico> [--group-id 1]` |
| `-delete ICON,n,…` | `--delete-icon-ids n,…` |
| （缺失：VERSION_INFO 未用） | `--version-info VERSION_INFO` |
| 固定 `1033` | `--lang 1033`（默认即 1033） |

## 3. 包格式（与前端/读侧对齐）

- 首包永远是配置文件，包内 key 固定 `"niva.json"`；
- 文件 key：相对路径，`\` 换 `/`（与前端 `replace(/\\/g,"/")` 一致）；
- `INDEXES = JSON{path:[offset,length]}`，`DATA = deflate_raw(拼接)`。
- 有意差异（不影响读取）：遍历排序后打包（前端是 readdir 原生顺序），
  产物确定可复现；INDEXES 用紧凑 JSON（前端是 2 空格 pretty）。

## 4. 本 crate 结构

```text
crates/win_packager/
  src/lib.rs           # PackRequest/pack()：校验 -> 备料 -> 拷贝模板 -> 分发 Windows 实现
  src/bundle.rs        # 资源打包（跨平台，已实现，有单测）
  src/icon.rs          # PNG->ICO 内存转换，逻辑从 icon_creator 原样搬入（跨平台，已实现，有单测）
  src/version_info.rs  # VERSIONINFO .rc -> VS_VERSIONINFO（跨平台）
  src/windows_impl.rs  # Win32 UpdateResource 实现（仅 Windows 编译）
  src/main.rs          # CLI（std-only 参数解析）
```

- `pack()` 语义：校验 -> 备料（读文件/打包/转图标，无副作用）->
  `std::fs::copy(template, output)` -> Windows 下 `Begin/Update/EndUpdateResourceW`。
- `template_exe == output_exe` 会被拒绝：必须 copy-then-update，
  避免写坏运行中的模板。

## 5. Windows 写入实现

`windows_impl.rs` 使用 `BeginUpdateResourceW` / `UpdateResourceW` /
`EndUpdateResourceW`。任一写入步骤失败都会尝试丢弃本次暂存的资源更新，并
附带失败步骤；回滚失败也会报告。RCDATA 使用 UTF-16 名称和请求语言；图标替换删除请求列出的旧
`RT_ICON` ID，再按 ICO 表写入 `RT_ICON` 与 `RT_GROUP_ICON`；版本数据写为
`RT_VERSION` ID 1。

`version_info.rs` 将 devtools 生成的 VERSIONINFO `.rc` 子集转换为
`VS_VERSIONINFO` 二进制，不依赖 Windows SDK 的 `rc.exe`。该解析器支持当前
`windows-version-info-template.ts` 使用的数字版本字段、JSON 转义字符串和
Translation 字段。跨平台单测覆盖版本结构、对齐、转义字符串和无效字段。

Windows target 编译只能覆盖 Win32 API 的类型检查。通过 PE 资源查看器确认
RCDATA、图标和版本页，并在签名后启动生成 exe，仍需 Windows 真机验收。

## 6. `icon_creator` 旧链路

新的 devtools 构建脚本不再调用 `icon_creator.exe` 或 `ResourceHacker.exe`。
`build_Windows.cmd` 从本 crate 生成 `win_packager.exe`，暂时放到
`packages/devtools/public/windows/` 供 Vite 复制，再移除这个暂存文件；旧两个
工具从该脚本构建的资源包中剔除。`icon_creator` crate 和
`public/windows/` 中的旧二进制已从仓库删除；`icon_creator` crate 暂仍保留，
但新构建链路不调用它。

图标尺寸和 Lanczos3 滤波逻辑以 `icon.rs` 为准，与旧 `icon_creator` 保持一致。

## 7. 本地验证（macOS 可做）

```bash
cargo check -p win_packager
cargo test -p win_packager
cargo run -p win_packager -- --help
cargo check -p win_packager --target x86_64-pc-windows-msvc
```

bundle、ICO 和 VERSIONINFO 编码可在 macOS 测试。Windows target check 编译
`windows_impl`，但不执行 PE 写入；`pack()` 在 macOS 会在资源写入阶段返回
“only supported on Windows”。图标、版本页、签名和生成程序启动仍需 Windows
真机检查。

## 8. devtools 构建迁移

`build-windows.ts` 通过 `resource.extract("windows/win_packager.exe")` 取得
工具，并传入模板 exe、目标路径、资源目录、`niva.json`、可选 PNG 图标和
VERSIONINFO。`ProjectModel.build()` 仍在构建成功后调用 `sign-windows.ts`，
所以签名发生在所有 PE 资源修改之后。

`build_Windows.cmd` 先以 `cargo build -p win_packager --target
x86_64-pc-windows-msvc --release` 从源码构建工具，再在 devtools Vite 构建
期间暂存二进制到 public 目录；Windows 打包器本身不依赖自身来构建。旧
`ResourceHacker.exe` 和 `icon_creator.exe` 不再被构建步骤提取或执行。
