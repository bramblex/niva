# Windows MiniBlink 按需兼容方案（方案，未实现）

> 调研日期：2026-09-23。按用户决定，本方案暂缓实施。本文按“MiniBlink 是低版本 Windows 的额外兼容项”设计。普通 Windows 构建继续使用 WebView2；MiniBlink 版只加载本地页面，通过现有 WebSocket bridge 调用 Niva API，不做原生 IPC。本仓库目前没有 MiniBlink 实现，也没有 Windows 真机验收记录。**恢复本方案前须重定本地页面入口**：§“按顺序落地”中的 `http://127.0.0.1:<port>/` 是旧静态路由，而当前普通打包模式已改从应用UUID派生的Wry协议加载（`niva://app/`仅为配置别名）并关闭该 HTTP 静态路由；不能直接沿用下面的旧入口步骤。

## 结论

可行的最小路线是：**保留 tao 窗口和 Niva 本地 HTTP/WS 服务，另编译一个 Windows x64 MiniBlink 版 exe，旁放固定版本的 MiniBlink 运行时 DLL**。应用需要兼容已确认的低版本 Windows 时，显式选用这个版本；不做启动时自动探测、双内核切换或 WebView2 失败后的隐式回退。普通 WebView2 构建不包含 MiniBlink 运行库。3 MB 是参考目标，不代表所有当前平台产物都低于该数；Windows release 尺寸尚无实测。MiniBlink 版交付物的 exe、DLL 和总大小仍要分别记录，供部署者判断。

原始 [`README.md`](../README.md) 曾将“对 Windows 10 低版本增加 MiniBlink 支持”列为待办；当前 [`roadmap.md`](roadmap.md) 按用户决定明确暂缓，本方案没有实施排期。规划本身不能证明低版本兼容。恢复前先确定目标 Windows 10 版本、架构与补丁级别，在同一机器复现 WebView2 问题并验证 MiniBlink。Win7/XP、x86、ARM64 不自动进入支持范围。

MiniBlink 的 [旧 `miniblink49` 仓库](https://github.com/weolar/miniblink49)主要保存较老的 49 内核源码，发行页另有较新 SDK。现有独立的 [MiniBlink 132 源码仓库](https://github.com/weolar/miniblink132)（页面标示 Apache-2.0）以及 [2025-12-12 的 132 SDK](https://github.com/weolar/miniblink49/releases/tag/20251212)。本方案选 132，不选 49 做新应用的浏览器基线。132 源码仓库说明很少；选定 SDK 的二进制再分发、第三方许可和后续安全更新需单独核实。最新 SDK 压缩包为 **64,696,903 字节**，SHA-256 为 `237f5166701780918c27a1a340c3e1249776aee3db0673a3dffc1f9a009e6528`；这个数字是整个下载归档，**不是运行时 DLL 大小**。[发行资产](https://github.com/weolar/miniblink49/releases/expanded_assets/20251212)

## 启用与交付边界

- 增加显式 Windows 构建选择，例如 `cargo build -p niva --target x86_64-pc-windows-msvc --release --no-default-features --features miniblink`。实际 feature 名称由实施时确定；验收标准是一次构建只编入一个 Windows 浏览器后端，普通构建仍选 WebView2。MiniBlink 版不应依赖系统 WebView2 Runtime，使用 `cargo tree` 和无 WebView2 Runtime 的目标机启动验证。
- MiniBlink 版先用现有 `win_packager` 以专用 exe 为模板生成应用 exe，再将固定的运行时 DLL 及所需文件放在同一交付目录。开发工具当前从 `process.currentExe()` 复制模板，见 `packages/devtools/src/build-scripts/build-windows.ts`；第一阶段用 `win_packager` 命令行指定 MiniBlink 模板即可，不为这个额外项先改 Devtools UI。应用签名仍在修改 PE 资源之后进行。
- 运行时用 exe 所在目录下的**固定文件名、绝对路径**加载 DLL，校验锁定版本/哈希并避免从工作目录搜索。缺失、架构错误或哈希不符时明确报错。不要在构建时下载 `latest`，也不把 DLL 塞进 Niva 普通版 exe。
- MiniBlink 版只接受打包的本地 entry；远端 entry、`permissions` grants、跨源顶层导航、指向远端 URL 的 `webview.loadUrl` 要明确拒绝。`webview.loadHtml` 在 origin 与 bridge 身份未定义前也拒绝。跨源 iframe 可以按页面规则加载，但拿不到本地 token，也没有原生 IPC 通道。显式 debug/Vite 跨端口入口留待后续单独验证。

## 需要改的代码

| 位置 | 最小改动 |
| --- | --- |
| `crates/niva/Cargo.toml`、`app/mod.rs` | 增加 Windows MiniBlink 构建 feature 与 C ABI 加载；MiniBlink 版不编译/调用 `ipc_windows_frames.rs`。保持普通 WebView2 版不变。 |
| `app/window_manager/builder.rs`、`window.rs`、`mod.rs` | 给现有 WebView 调用点建小范围平台后端边界。tao 仍创建顶层窗口；MiniBlink 在 tao HWND 下创建子窗口，负责尺寸、DPI、焦点、键鼠、输入法、关闭及回调释放。 |
| `app/api/webview.rs`、`docs/api-coverage.md` | 先实现本地应用要用的加载、重载、当前 URL、脚本执行和事件；其余公开方法逐项核对，未实现的返回明确“不支持”错误，不能静默成功。 |
| `assets/initialize_script.js`、`app/http_server/mod.rs`、`docs/bridge.md` | 复用本地 WS wire v1、每窗 token 与 Rust API；MiniBlink 版不注册 `mbOnJsQuery`，不注入远端 IPC transport。验证初始化脚本在页面脚本之前执行。 |
| `build_Windows.cmd`、`win_packager` | 形成可复现的 MiniBlink 专用 exe 模板和 DLL 文件清单；第一阶段不要求修改 packager 的资源格式或 Devtools UI。 |

官方 [MiniBlink C API 文档](https://weolar.github.io/miniblink/views/doc/api-vip.html)描述了以父 HWND 创建 `CONTROL` 子窗口、页面加载和 JS 上下文回调；**具体 132 函数签名及导出以锁定 SDK 的 `mb.h`/DLL 为准**。对照 [132 源码 `mb.h`](https://github.com/weolar/miniblink132/blob/main/mbvip/core/mb.h) 和实际 DLL 导出表，不把旧版文档当作已验证的 132 行为。MiniBlink 的浏览器对象和 HWND 在 tao UI 线程创建、调用和释放；回调跨线程时只传可验证的窗口 ID/弱引用，不把裸指针交给异步任务。

## 按顺序落地

1. **供应物与机器基线**：固定 SDK SHA-256，清点 DLL/依赖/许可文件/导出表与各文件字节数。核对选中资产和源码版本关系及可再分发条件。目标 Windows 机器上记录系统版本、WebView2 失败现象；跑官方最小 MiniBlink 子窗口样例，验证显示、输入、DPI、关闭重开。
2. **Niva 本地最小切片**：把 MiniBlink 子窗口放进 tao 窗口，加载现有 `http://127.0.0.1:<port>/` 本地资源。以窗口原生 ID 和精确本地 origin 为条件，只给主 frame 注入该窗口的 WS token 和 `initialize_script.js`；同源 iframe 按现有规则继承。必须在真机证明脚本注入早于页面脚本且 WebSocket 握手带正确 `Origin`。若 SDK 做不到，停在实验阶段，不改用无鉴权桥。
3. **本地功能**：验证 `Niva.call`、一个二进制/流式调用、同源 iframe、跨源 iframe 拒绝、窗口开关与事件。再按 [`api-coverage.md`](api-coverage.md) 的 20 个 `webview.*` 方法逐项列出支持/不支持；打印、Cookie、DevTools 不能凭旧文档推定可用。首版未支持项显式报错并写入 MiniBlink 版能力表。
4. **交付**：从锁定 DLL 清单组装专用版目录；离线、无 WebView2 Runtime 的目标机启动打包应用。检查 DLL 缺失/被替换时的错误、签名、启动时间、内存和 exe/DLL/目录总大小。普通 Windows 构建继续按既有门禁测试，不因 MiniBlink 实验改变默认交付。

## 安全与验收门禁

Niva 的本地桥能力很大，不能因为跳过 IPC 就跳过鉴权。复测 [`security.md`](security.md)、[`permission-design.md`](permission-design.md) 与 [`http-auth-plan.md`](http-auth-plan.md) 的本地相关用例：WS `Origin`/`Host`/token/窗口绑定；同源 iframe 与跨源 iframe；`__niva_fs` 未授权访问；编码路径穿越；导航或关窗后旧连接失效。MiniBlink 提供关闭同源/CSP 检查的选项，Niva 不启用它。[MiniBlink API 文档](https://weolar.github.io/miniblink/views/doc/api-vip.html)

| 门禁 | 通过证据 |
| --- | --- |
| 低版本适配 | 指定 Windows 版本上 WebView2 问题复现、MiniBlink 专用版可启动并完成同一关键流程。 |
| 本地桥 | 主 frame/同源 iframe 的 WS 调用与流正常；跨源 frame、远端 entry/grant/导航和无 token 访问被拒绝。 |
| 窗口与 API | 页面显示、尺寸/DPI、键鼠/输入法、焦点、关闭重开真机通过；20 个公开 WebView 方法的支持表与错误行为经过真机核对。 |
| 构建与交付 | 对实际改动执行 `cargo fmt --all -- --check`、`cargo check --workspace`、`cargo clippy --workspace --all-targets`、`cargo test --workspace`、`cargo check --target x86_64-pc-windows-msvc`；若改 Devtools/TS，再执行 `npm run build --workspace=packages/devtools`。另记录 SDK 版本/哈希、许可、exe/DLL/目录大小及 Windows 运行结果。 |

Windows target check 只证明条件编译；本方案目前没有实现或真机结果，不能声称 MiniBlink 已解决低版本 Windows 兼容问题。供应商安全更新策略、选定 SDK 二进制许可与运行时 DLL 大小尚待核实。普通 WebView2 版与 MiniBlink 版的 Windows 产物尺寸都尚未实测，后续应分别记录 exe、DLL 和完整交付目录大小；3 MB 作为参考值单独评估。
