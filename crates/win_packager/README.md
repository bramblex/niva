# win_packager

Windows exe 后注入打包器，一次调用代替旧链路三件套：前端 `pako` 备料 +
`icon_creator.exe` + `ResourceHacker.exe`。

## 1. 背景：现在是怎么打 Windows 包的

`packages/devtools/src/build-scripts/build-windows.ts`：

1. 前端打包（`base.ts`）：`niva.json` 首包 + `readDirAll` 遍历 + `pako.deflateRaw`
   得到 `RESOURCE_INDEXES` / `RESOURCE_DATA` 两个临时文件；
2. `GENERATING_ICON`：`resource.extract("windows/icon_creator.exe")` 并执行，
   PNG -> 多尺寸 `icon.ico`；
3. `BUILD_EXECUTABLE_FILE`：`resource.extract("windows/ResourceHacker.exe")`，
   写 `bundle_script.txt` 调 `-script`：拷 `currentExe -> targetExe`，
   写 `RCDATA` + 删旧 `ICON 1..7` + 写 `ICONGROUP 1`。

读侧：`crates/niva/src/app/resource_manager/mod.rs:117-131` +
`win_utils.rs` 从 `RT_RCDATA` 的同名资源读回。

注意：`versionInfoTemplate` 写出的 `VERSION_INFO` 文件目前没有被 script
引用（版本信息实际没进目标 exe），新 crate 要把版本信息补上，见 §5 第 4 条。

## 2. 新链路（目标）

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
  src/windows_impl.rs  # 写盘 stub（仅 Windows 编译），TODO 见文件头
  src/main.rs          # CLI（std-only 参数解析）
```

- `pack()` 语义：校验 -> 备料（读文件/打包/转图标，无副作用）->
  `std::fs::copy(template, output)` -> Windows 下 `Begin/Update/EndUpdateResourceW`。
- `template_exe == output_exe` 会被拒绝：必须 copy-then-update，
  避免写坏运行中的模板。

## 5. Windows 实现 TODO（到 Windows 机器上做）

按 `src/windows_impl.rs` 文件头的 5 条做，摘要：

1. `apply`：`BeginUpdateResourceW` / 三步 / `EndUpdateResourceW`，失败回滚。
2. `update_rcdata`：`RT_RCDATA(10)` 逐条写，UTF-16 资源名。
3. `replace_icon`：删旧 `RT_ICON(3)` -> 用 `ico` crate 解析已备好的 ico 字节、
   写新 `RT_ICON` -> 写 `RT_GROUP_ICON(14)`。
4. `apply_version`：`VERSION_INFO`（`.rc` 文本）-> `RT_VERSION(16)`。
   先调 SDK `rc.exe` 的方案跑通，再考虑纯 Rust 手拼 `VS_VERSIONINFO` 去 SDK 依赖。
5. 验证：高层模式跑一遍，与旧产物逐字节比 `RCDATA`、用资源查看器比图标/
   版本页，再走签名。

## 6. `icon_creator` 还需要吗？

新链路跑通之前：需要，旧 `build-windows.ts` 的 `GENERATING_ICON` 仍在用它，
`public/windows/icon_creator.exe` 是已提交的预编译产物，别动。

新链路跑通之后：不需要，删三处——`crates/icon_creator/`、
根 `Cargo.toml` 的 workspace 成员、`public/windows/icon_creator.exe`——
并更新 `docs/PROJECT_MANUAL.md` §2.11 和本 README 这一节。
转换逻辑以 `icon.rs` 为准（尺寸/滤波与原来一致）。

## 7. 本地验证（macOS 可做）

```bash
cargo check -p win_packager
cargo test -p win_packager
cargo run -p win_packager -- --help
```

备料单测（打包 round-trip、ICO 头）已在 mac 跑。完整写资源验证只能在
Windows 上做（`windows_impl` 不在 mac 编译，`pack()` 在 mac 会停在
“only supported on Windows” 错误，这是预期的）。

## 8. 迁移计划：win_packager 作为唯一的 Windows 打包器（已定）

现状（旧链路，仍可用）：前端 `pako` 备料 + `icon_creator.exe` +
`ResourceHacker.exe`，见 §1。

目标（新链路）：devtools 只备 `build/` 目录 + `niva.json` + PNG，
一次 `process.exec(win_packager.exe, …)` 完成全部注入，签名保持最后。

步骤：

1. Windows 上实现 `src/windows_impl.rs`（TODO 见其文件头 5 条）。
2. 高层模式跑通，与旧产物逐字节比 `RCDATA`、资源查看器比图标/版本页
   （版本信息是新增补齐，旧产物没有，见 §1 注意）。
3. 改 `build-windows.ts`：`GENERATING_ICON` 整步删除，
   `BUILD_EXECUTABLE_FILE` 改为 extract + exec `windows/win_packager.exe`。
4. 删除 `crates/icon_creator/`、根 `Cargo.toml` 成员、
   `public/windows/icon_creator.exe` + `public/windows/ResourceHacker.exe`，
   更新 `docs/PROJECT_MANUAL.md`（§2.11/§3.5/§6.2）。
5. 验收：`build_Windows.cmd` 全链路产出可用 exe，且包内不再含任何
   第三方/预编译打包二进制（除系统 `signtool` 外，签名工具不打包）。

自举说明：首版 `win_packager.exe` 在 Windows 上用 cargo 一次性打出，
放入 `public/windows/` 提交；此后打包闭环——旧版 packager 打出含新版
packager 的包，不存在循环依赖（cargo 构建 packager 本身从不需要 packager）。
