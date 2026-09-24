# 跨平台打包

`niva-packager` 是独立构建工具，使用预编译的运行时，在 macOS 或 Windows 上打包 Windows x64、macOS Apple Silicon 与 macOS Intel 应用。用户项目不需要 Rust 编译器。MiniBlink 暂不支持。

## Devtools

1. 下载与当前电脑匹配的 `niva-build-kit` 并解压。工具包内含宿主打包器、三种运行时、NodeCompat 资源、版本清单及第三方许可。
2. 在项目详情点击构建，选择工具包目录和输出目录，再勾选目标平台。
3. 每个目标分别显示结果、输出路径、SHA256 与签名状态。已有同名产物会被拒绝；换一个空目录后重试。

工具包必须来自可信来源；选择工具包意味着执行其中的本机程序。运行时 SHA256 校验可以发现损坏和清单不匹配，不证明工具包发布者可信。

## 命令行

```sh
./niva-packager build \
  --manifest ./manifest.json \
  --config /path/to/project/niva.json \
  --resource-dir /path/to/project/dist \
  --output-dir /path/to/output \
  --target windows-x86_64 \
  --target macos-aarch64 \
  --target macos-x86_64
```

Windows 使用 `niva-packager.exe`。目标参数可以重复传入。NodeCompat 开启时默认读取工具包的 `node-compat/`；也可传 `--node-compat-dir`，目录内应有 `node-compat.js` 和 `src/`。只打包配置选中的模块及其依赖。

标准输出最后一行为 JSON：`results` 数组包含每个目标的 `target`、`status`、成功时的 `path`、`sha256`、`signature`、`runtimeVersion`，失败时的 `error`。任一目标失败返回退出码 1，其他已成功目标保留。输入预检失败返回空 `results` 与顶层 `error`。

Windows 产物为 `.exe`，签名状态为 `unsigned`；Mac 产物为 `.zip`，其中包含完整 `.app`，签名状态为 `ad-hoc`。ZIP 显式保存执行权限，Windows 生成的 Mac 包解压后也应可执行。资源目录中的符号链接、开启 NodeCompat 时冲突的 `__niva_compat/` 前缀、越界路径与不适合跨系统使用的应用名称会被拒绝。

`ad-hoc` 只保障代码签名完整性，不代表 Apple 信任的开发者身份，也不等于公证。Windows 包不会获得 Niva 的签名身份。正式签名需应用作者自己的证书及目标平台流程；跨平台入口不会默默套用 `sign.macos` 或 `sign.windows`。

## 固定运行时清单

`manifest.json` 的路径相对清单目录；不允许绝对路径、`..` 和符号链接。

```json
{
  "schemaVersion": 1,
  "version": "0.9.9",
  "runtimes": {
    "windows-x86_64": {
      "path": "runtimes/niva-windows-x86_64.exe",
      "version": "0.9.9",
      "sha256": "填写此文件的64位SHA256"
    }
  }
}
```

每个运行时版本必须与工具包和 packager 版本相同。打包时同时校验 SHA256 与 PE/Mach-O 架构，不能把宿主的 `currentExe()` 当作另一个平台的模板。

## 构建工具包

维护者使用 `.github/workflows/packager.yml`：三台原生 runner 分别构建运行时和 packager，再汇总运行时，生成三种宿主工具包。工作流仅上传 Actions 构件，不自动发布正式版本。

手工汇总时，将三种运行时按以下名称放入同一目录：

- `niva-windows-x86_64.exe`
- `niva-macos-aarch64`
- `niva-macos-x86_64`

```sh
python scripts/create-packager-kit.py \
  --runtime-dir native-artifacts \
  --packager target/release/niva-packager \
  --output dist/niva-build-kit.zip
```

脚本生成固定哈希清单、工具包校验和、第三方许可原文和对应版本源码下载链接。`apple-codesign` 采用 MPL-2.0；其及所有其他构建依赖只进入 packager，不链接到用户应用运行时。运行时大小、packager 大小、应用完整包大小分别统计。

## 验收范围

实现与真机验收分开记录，参见 `cross-platform-packager-plan.md`。目标机启动、bridge、文件访问和窗口行为需要真实目标系统验证，编译和资源读回不代替这些测试。
