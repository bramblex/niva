# Wry 自定义协议本地页面：实现与验收

> 状态：已接入 Niva 源码。打包资源中的相对本地 `entry` 通过 Wry `with_asynchronous_custom_protocol("niva")` 从 `niva://app/` 加载；macOS 页面 origin 为 `niva://app`，Windows WebView2 页面 origin 为 `http://niva.app`，源码的非 Windows 分支也选择 `niva://app`。Linux 尚无构建或运行验收。loopback HTTP/WS 仍使用动态端口，但打包模式不再通过普通 HTTP 路由提供页面静态资源。2026-09-23 有一组 macOS 临时 app smoke 结果；Windows 已有打包页 WebView2 有限真机 smoke，完整发布包和威胁矩阵仍待验收，见 `docs/windows-validation-2026-09-23.md`。

## 功能与边界

将打包的本地页面和静态资源改由固定的 `niva://app/` 入口加载，使页面 origin 不随 Niva 的本地服务端口变化，保持同一 `WebContext` 用户数据目录中的 localStorage 等站点数据。**必须使用 Wry 的 `with_asynchronous_custom_protocol`**：协议回调只分派请求，资源读取、解压和 HTML 改写等工作在后台执行，完成后通过 responder 回包。协议响应仍是完整的字节缓冲区，不承担双向流式传输。

本地 HTTP/WS 服务继续监听动态 loopback 端口。原生 API 的文本和二进制流继续走现有 WS；`__niva_fs` 继续使用现有按窗口 token 鉴权的 HTTP 路由。远端页面和跨源 iframe 继续走按窗口、按源授权的 unary IPC。显式开发入口（如 Vite）仍按现有精确开发 origin 规则处理，不因本地页面迁移获得打包页权限。

## Wry 独立实验记录

2026-09-23 在 macOS 上以 Wry `0.57.0`、Tao `0.37.0` 做过独立最小实验，使用持久 `WebContext` 和 `niva://app/index.html`：

| 检查 | 观察结果 |
| --- | --- |
| 主页面、同源 iframe | 二者 `location.origin` 均为 `niva://app`；iframe 可读取同源 localStorage。 |
| 重启、动态端口 | WS/HTTP 端口每次变化；保留窗口数秒让存储落盘后，localStorage 计数跨启动连续递增。实验曾在收到结果后立即退出，下一次读到旧计数，因此不能以强制立即退出作为持久化验收。 |
| WS | 从协议页面连接 `ws://127.0.0.1:<port>`，文本及二进制回显成功；服务端收到 `Origin: niva://app`。 |
| 浏览器 `fetch` HTTP POST | 对带 JSON 内容类型的跨源请求先发 OPTIONS，再发 POST；在 CSP 放行目标地址且服务端返回匹配的 CORS 头后得到 200。实验自己的 HTTP 服务支持 POST；**当前 Niva HTTP 服务只接受 GET，不能据此宣称 Niva 已支持浏览器直连 POST**。 |
| 原生交互 | Wry IPC 和 `evaluate_script` 往返成功。 |
| 同步协议回调 | 故意 `sleep(2s)` 时，macOS 协议回调与界面事件循环在同一线程；约 2 秒内没有事件循环 tick，随后恢复。因此不能用同步回调做可能耗时的资源处理。 |

这是 Wry 独立实验的功能与阻塞机制记录，**不是 Niva 集成验收、吞吐量或页面加载性能基准**。Wry 文档说明 Windows 默认将自定义协议 URL 映射为 `http://niva.app/...` 形式；不能把 macOS 的 `niva://app` 字符串硬编码为跨平台唯一 origin。[Wry 自定义协议与异步处理文档](https://docs.rs/wry/0.57.0/wry/struct.WebViewBuilder.html#method.with_asynchronous_custom_protocol)

## 当前源码实现

1. **静态页面入口**：打包资源中的相对本地 `entry` 解析到 `niva://app/`。协议处理器只读取当前 `ResourceManager` 中的资源，不代理任意 URL。根路径和以 `/` 结尾的目录路径解析到 `index.html`，按路径选择 MIME；编码后的 `.`/`..`、反斜线及非法 authority 会被拒绝。`__niva_*` 内部路由在协议内统一拒绝，只有 NodeCompat 启用且通过模块 allowlist 的 `__niva_compat/*` 资源可以读取；协议不承载 `__niva_fs` 或 `__niva_ws`。
2. **异步处理**：协议回调只将请求投递到 4 个固定工作线程及容量为 32 的有界队列。队列满返回 503；每个已接收请求有 15 秒超时，超时返回 504；单个响应最多 32 MiB，超限返回 413。方法、URI、资源不存在及 HTML 改写失败均返回明确状态；one-shot gate 确保 Niva 侧最多调用一次 responder。Wry responder 没有取消通知或 delivery result，因此导航/关窗后的平台响应是否被接收无法由当前代码确认，仍待目标 WebView 验收。异步处理避免在 Wry 回调中同步读取资源，但慢资源可能耗尽工作线程，现有 smoke 未验证压力或性能。
3. **平台 origin 与窗口身份**：源码在非 Windows 分支选择 `niva://app`；Windows WebView2 对外显示 `http://niva.app`，Rust 的 Wry 请求回调还原为 `niva://app/...`。Linux 构建和运行尚未验收。以本地包启动的窗口会安装 origin 检查；顶层文档只有在当前 origin 精确匹配该窗口可信 origin、且路径不在 `__niva_fs` 下时才注入该窗口自己的内存 token、窗口 ID 和动态 WS 地址，同源 iframe 可依同源规则从父 frame 取得凭据，各 frame 建立独立连接。窗口 token 仍按窗口绑定，不能只凭 origin 识别窗口。
4. **WS 鉴权**：握手仍校验 loopback 服务 `Host`、`/__niva_ws` 路径、窗口 token、窗口绑定、页面 `Origin` 和 hello 窗口 ID。打包页允许的精确 `Origin` 为 macOS `niva://app` 或 Windows `http://niva.app`；非 Windows 源码分支也选择 `niva://app`，但 Linux 未验收。显式开发启动只对当前规则允许的本机 Vite origin 开放。浏览器 WS 不使用 HTTP CORS 预检，`Origin` 校验不能代替 token。
5. **CSP 与 HTTP**：协议页面 HTML 文档导航会对 HTML 内的 enforcing CSP meta 保留现有 source list，并把当前 `ws://127.0.0.1:<port>` 与 `http://127.0.0.1:<port>` 加到 `connect-src`；文件图片、媒体、字体与样式的精确 HTTP 源也会补入对应 directive。新建简单项目带可编辑的基础 CSP；NodeCompat 启用时，其注入块移到 CSP meta 之后，只有 Niva 注入的脚本获得每次导航随机 nonce，作者内联脚本仍受策略约束。WS 用于常规原生 API；HTTP 源用于页面直接读取 `webview.baseFileSystemUrl` 的 `__niva_fs`。文件路由先校验 token，再仅在请求 `Origin` 与该 token 所属窗口的 `trusted_ws_origin` 精确相同时返回 `Access-Control-Allow-Origin`。非匹配 origin 即使带有效 token 也不返回 CORS allow header；无效 token 返回 403。2026-09-23 macOS smoke 验证了精确 origin 的浏览器 fetch，以及严格 CSP 下的打包页 NodeCompat 和未授权内联 import map 拒绝。Windows 已有限实测文件 URL 与 CORS 正负例；严格 CSP 已有限实测正例与作者内联脚本拒绝；远端 IPC 和完整 CSP 矩阵仍待专项测试。CSP 改写无法读取或修改 WebView 宿主、代理等另外添加的 CSP response header；若额外 header 限制 loopback 源，页面仍可能无法连接。`Niva.api.http.post` 是原生 API，经 WS 调用，不等于浏览器向 Niva 服务直接发 POST。协议页与 loopback 间的其他跨源 HTTP `fetch` 仍需单独设计 CSP、CORS 和鉴权。
6. **调试与持久数据**：显式 `--debug-resource` / debug 启动继续使用 loopback 普通 HTTP；显式远端 entry 保持远端来源。打包模式普通 HTTP 静态路由只在调试参数开启时服务，loopback WS 和按 token 验证的 `__niva_fs` 继续存在。新 origin 与旧 `http://127.0.0.1:<port>` 的存储天然隔离，不会自动迁移旧 localStorage。不同窗口可共享页面 origin，权限仍绑定各自窗口 token。

## Niva macOS 集成 smoke（2026-09-23）

使用当前 arm64 release 二进制和临时资源索引构造本地 `.app`，实际由 Niva/Wry 打开；这是临时 smoke app，不是 Devtools 发布包。

| 检查 | 结果 |
| --- | --- |
| origin、主页面和 iframe | UI accessibility 显示 `PASS FRAME niva://app wid=0`；主页面与同源 iframe 都加载。 |
| WS 原生 API 与静态资源 | 主页面先加载 `/probe.js` 再调用 `window.current`；主页面和 iframe 均调用 `window.current` 与 `fs.createDir` 成功。HTML 使用 `connect-src 'self'`，页面仍连上动态 loopback WS，验证了该样例的 meta CSP 补源。 |
| HTTP 边界 | 同一运行实例请求 `/index.html` 返回 404；无凭据请求 `__niva_fs` 返回 403。 |
| `__niva_fs` CORS | 页面从 `webview.baseFileSystemUrl()` 取得 token URL，并在 `connect-src 'self'` 下通过浏览器 `fetch` 读到 `cors-read-ok`。独立 HTTP 探测确认 `Origin: niva://app` 返回 ACAO `niva://app`；有效 token 配 `https://wrong.example` 返回 HTTP 200 但无 ACAO；无效 token 返回 403 且无 ACAO。 |
| NodeCompat ESM 与 allowlist | 仅打包 `path`、`fs`、`assert`、`stream`：页面的 `import path from 'path'`、`Niva.import('fs/promises')` 文件读取、`require('assert/strict')` 断言成功；所选 `stream-promises.js` 返回 200，未选 `child_process.js` 返回 404。UI 显示 `PASS NODECOMPAT niva://app`。 |

该 smoke 没有验证二进制/流式 WS、跨源 frame、不同窗口 token 隔离、localStorage 跨重启、导航/关窗时序、NodeCompat 其他模块及完整 API 语义，也没有测并发/超时压力或页面加载性能。它是手工构造的临时 app，不是 Devtools 发布包；另一节列出的 Wry standalone 实验也不等同于 Niva 集成测试。

## 尚待验收

- **Windows 真机**：WebView2 打包页的 `http://niva.app` origin、同源 iframe 原生调用、文件 URL 和部分 NodeCompat 已有限实测；跨源 iframe、负向 WS 握手、严格 CSP、动态端口与导航时序仍待专项验收。见 [Windows 验证记录](windows-validation-2026-09-23.md)。
- **macOS 深度验收**：仍需验证窗口间 token 隔离、跨源 iframe 拒绝、跨重启存储与迁移边界及协议页 WS 二进制流。`__niva_fs` 的有凭据 fetch 已在上表有限 smoke 中覆盖。
- **异步压力与性能**：注入慢资源并同时操作窗口；测队列上限、超时、响应上限和 responder 失效；与 loopback HTTP 入口在相同资源条件下比较。完成前不宣称吞吐量或页面加载性能已验收，也不承诺慢资源永不影响其他请求。
- **导航/关窗时序**：Wry async responder 没有取消状态或 delivery result；确认 WebView 丢弃请求后延迟响应的实际行为，不能把一次 responder 调用等同于页面收到响应。
- **CSP header 边界**：若部署环境额外添加 CSP response header，需在实际平台确认该 header 不会阻止 WS；当前实现只改 HTML 中的 meta policy。
- 已运行并通过 `cargo fmt --all -- --check`、`cargo check --workspace`、`cargo clippy --workspace --all-targets`、`cargo test --workspace` 和 Windows target check；Clippy/编译仍有非阻断 warning。格式检查中的早期 `window.rs` 差异已修正。
