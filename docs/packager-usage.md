# 跨平台打包

`niva-packager` 是独立构建工具，使用预编译的运行时，在 macOS 或 Windows 上打包 Windows x64、macOS Apple Silicon 与 macOS Intel 应用。用户项目不需要 Rust 编译器。Windows 构建通过仓库 `.cargo/config.toml` 静态链接 CRT，避免工具或应用额外依赖 VC Runtime DLL。MiniBlink 暂不支持。

## Devtools

1. 下载与当前电脑匹配的 `niva-build-kit` 并解压。工具包内含宿主打包器、三种运行时、版本/体积清单及第三方许可。Niva JavaScript runtime 随预编译运行时内嵌，不需要应用再附一份 runtime 目录。
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
  --resource-layout embedded \
  --target windows-x86_64 \
  --target macos-aarch64 \
  --target macos-x86_64
```

Windows 使用 `niva-packager.exe`。目标参数可以重复传入。`--resource-layout` 默认为 `embedded`：Windows 输出单 `.exe`，macOS 输出包含 `.app` 的 ZIP；`external` 为大媒体/大资源保留按需读取，Windows 输出含运行时和 `resources/` 的绿色 ZIP，macOS 仍输出 `.app` ZIP，业务资源位于 `Contents/Resources/app/`。外置资源在外层 ZIP 中不压缩，单次 WebView 资源响应最多 32 MiB；更大的媒体应通过 HTTP Range 分段读取。无 Range 的整文件请求超过上限会明确返回 413，不会返回截断内容。

正式签名作为同一打包核心内的可选阶段。无 `sign` 配置时 Windows 保持 unsigned、macOS 保留 ad-hoc。`sign.macos.identity` 和可选 `entitlements` 由 macOS 宿主执行身份签名；配置 `notarize.profile` 时还会提交公证并 staple/validate。Apple ID 方式只从 `NIVA_APPLE_ID_PASSWORD` 环境变量取密码。`sign.windows.pfx` 和可选 `timestamp` 由 Windows 宿主调用 `signtool` 签名并验签；PFX 密码只从 `NIVA_WIN_CERT_PASSWORD` 环境变量取。凭据不写入日志或配置；缺少目标签名平台时明确失败，不降级为 unsigned 成功。

两种布局使用同一配置和资源校验流程。外置标记是 Windows EXE 的 RCDATA `RESOURCE_MODE=external`，macOS 为 `Contents/Resources/RESOURCE_MODE` 文件；缺少标记表示 embedded，标记错误或对应目录缺失会启动失败，不回退到另一布局。基础 Niva API 始终可用；`injectCommonJs` 与 `injectEsm` 控制额外 Node 风格入口，默认关闭。运行时 JavaScript 资产由选定 runtime 提供，资源目录不得包含保留前缀 `__niva_runtime/`。Packager 原样保留这些注入设置，不要求应用资源目录另附 JS runtime。

Packager 要求 build kit 含 `LICENSE`、`THIRD_PARTY.txt` 和 `licenses/`。这些材料随业务资源放在 `META-INF/niva/`，embedded EXE 可经应用资源 URL 读取；外置 ZIP 则能直接在 `resources/` 或 `Contents/Resources/app/` 找到。项目资源不得占用 `META-INF/niva/`。

标准输出最后一行为 JSON：`results` 数组包含每个目标的 `target`、`resourceLayout`、`status`、成功时的 `path`、`sha256`、`signature`、`runtimeVersion`，失败时的 `error`。任一目标失败返回退出码 1，其他已成功目标保留。输入预检失败返回空 `results` 与顶层 `error`。

Windows 目标使用 WebView2，目标机需要可用的 WebView2 Runtime。Windows embedded 产物为 `.exe`，external 绿色包为 ZIP；macOS 两种布局均为包含完整 `.app` 的 ZIP。ZIP 显式保存执行权限。资源目录中的符号链接、越界路径、不适合跨系统使用的应用名称、签名资产和 packager 保留目录都会被校验。

`ad-hoc` 只保障代码签名完整性，不代表 Apple 信任的开发者身份，也不等于公证。正式签名需应用作者自己的证书及目标平台流程，结果字段只报告已执行且校验成功的阶段。

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

每个运行时版本必须与工具包和 packager 版本相同。Kit 生成与校验脚本逐运行时检查体积严格小于 3,300,000 bytes，并记录实际大小和 SHA256。打包时同时校验 SHA256 与 PE/Mach-O 架构，不能把宿主的 `currentExe()` 当作另一个平台的模板。

## 构建工具包

维护者使用 `.github/workflows/packager.yml`：三台原生 runner 分别构建运行时和 packager，再汇总运行时，生成三种宿主工具包。Runtime 资产在构建 Niva 运行时时已嵌入，不需要额外 stage 或 extract。工作流仅上传 Actions 构件，不自动发布正式版本。

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

脚本生成固定哈希清单、逐运行时体积报告、工具包校验和、第三方许可原文和对应版本源码下载链接。`apple-codesign` 采用 MPL-2.0；打包工具依赖只进入 packager，不链接到用户应用运行时。Runtime JavaScript 资源属于 Niva 运行时，运行时大小、packager 大小、应用完整包大小分别统计。

## 验收范围

实现与真机验收分开记录，参见 `cross-platform-packager-plan.md`。目标机启动、bridge、文件访问和窗口行为需要真实目标系统验证，编译和资源读回不代替这些测试。
