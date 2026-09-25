# Wry 自定义协议本地页面：实现与验收

> 状态：打包资源的相对本地 `entry` 使用应用UUID派生的 Wry scheme：macOS/Linux 为 `niva-<uuid>://app/`，Windows WebView2 映射为 `http://niva-<uuid>.app/`；`niva://app/`只作为配置入口别名。固定origin的2026-09-23 macOS/Windows smoke 是历史快照，UUID变体的跨平台真机验证仍待完成。loopback HTTP/WS 继续使用动态端口，打包模式不通过普通 HTTP 路由提供页面静态资源。Windows既有有限真机 smoke 见 `docs/windows-validation-2026-09-23.md`。

## 功能与边界

将打包的本地页面和静态资源通过 Wry `with_asynchronous_custom_protocol` 加载，origin 由应用 UUID 派生。同一应用的origin不随服务端口、窗口或重启改变，不同应用取得不同origin，利用Web origin边界隔离localStorage等站点数据，不依赖特定macOS版本的WebsiteDataStore标识API。此机制不是API授权，也不迁移旧固定origin或loopback origin下的数据。协议回调只分派请求，资源读取、解压和HTML改写在后台执行，完成后通过responder回包；协议响应仍是完整字节缓冲区，不承担双向流式传输。

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

这是 Wry 独立实验的功能与阻塞机制记录，**不是 Niva 集成验收、吞吐量或页面加载性能基准**。Wry 文档说明 Windows 默认将自定义协议URL映射为 `http://<scheme>.app/...`；不能把 macOS origin硬编码为跨平台唯一字符串。[Wry 自定义协议与异步处理文档](https://docs.rs/wry/0.57.0/wry/struct.WebViewBuilder.html#method.with_asynchronous_custom_protocol)

## 当前源码实现

1. **静态页面入口**：打包资源中的相对本地 `entry` 解析到当前应用的 `niva-<uuid>://app/`。输入 `niva://app/` 会作为配置别名改写到当前UUID scheme。协议处理器只读取当前 `ResourceManager` 中的资源，不代理任意 URL。根路径和以 `/` 结尾的目录路径解析到 `index.html`，按路径选择 MIME；编码后的 `.`/`..`、反斜线及非法 authority 会被拒绝。`__niva_*` 内部路由在协议内统一拒绝，只有 NodeCompat 启用且通过模块 allowlist 的 `__niva_compat/*` 资源可以读取；协议不承载 `__niva_fs` 或 `__niva_ws`。
2. **异步处理**：协议回调只将请求投递到 4 个固定工作线程及容量为 32 的有界队列。队列满返回 503；每个已接收请求有 15 秒超时，超时返回 504；单个响应最多 32 MiB，超限返回 413。方法、URI、资源不存在及 HTML 改写失败均返回明确状态；one-shot gate 确保 Niva 侧最多调用一次 responder。Wry responder 没有取消通知或 delivery result，因此导航/关窗后的平台响应是否被接收无法由当前代码确认，仍待目标 WebView 验收。异步处理避免在 Wry 回调中同步读取资源，但慢资源可能耗尽工作线程，现有 smoke 未验证压力或性能。
3. **平台 origin 与窗口身份**：scheme为`niva-`加配置UUID（移除连字符并转小写）。macOS/Linux origin为`niva-<uuid>://app`；Windows WebView2显示`http://niva-<uuid>.app`。Linux共享WebContext对同一app scheme只注册一次，并让后续窗口复用app级handler；Linux仍无构建或运行验收。以本地包启动的窗口会安装origin检查；顶层文档只有在当前origin精确匹配该窗口可信origin、且路径不在`__niva_fs`下时才注入该窗口自己的内存token、窗口ID和动态WS地址，同源iframe可依同源规则从父frame取得凭据，各frame建立独立连接。窗口token仍按窗口绑定，不能只凭origin识别窗口。
4. **WS 鉴权**：握手仍校验 loopback 服务 `Host`、`/__niva_ws` 路径、窗口 token、窗口绑定、页面 `Origin` 和 hello 窗口 ID。打包页允许当前app的精确 `Origin`（macOS/Linux `niva-<uuid>://app`，Windows `http://niva-<uuid>.app`）；显式开发启动只对当前规则允许的本机 Vite origin 开放。浏览器 WS 不使用 HTTP CORS 预检，`Origin` 校验不能代替 token。
5. **CSP 与 HTTP**：协议页面 HTML 文档导航会对 HTML 内的 enforcing CSP meta 保留现有 source list，并把当前 `ws://127.0.0.1:<port>` 与 `http://127.0.0.1:<port>` 加到 `connect-src`；文件图片、媒体、字体与样式的精确 HTTP 源也会补入对应 directive。新建简单项目带可编辑的基础 CSP；NodeCompat 启用时，其注入块移到 CSP meta 之后，只有 Niva 注入的脚本获得每次导航随机 nonce，作者内联脚本仍受策略约束。WS 用于常规原生 API；HTTP 源用于页面直接读取 `webview.baseFileSystemUrl` 的 `__niva_fs`。文件路由先校验 token，再仅在请求 `Origin` 与该 token 所属窗口的 `trusted_ws_origin` 精确相同时返回 `Access-Control-Allow-Origin`。非匹配 origin 即使带有效 token 也不返回 CORS allow header；无效 token 返回 403。2026-09-23 macOS smoke 验证了精确 origin 的浏览器 fetch，以及严格 CSP 下的打包页 NodeCompat 和未授权内联 import map 拒绝。Windows 已有限实测文件 URL 与 CORS 正负例；严格 CSP 已有限实测正例与作者内联脚本拒绝；远端 IPC 和完整 CSP 矩阵仍待专项测试。CSP 改写无法读取或修改 WebView 宿主、代理等另外添加的 CSP response header；若额外 header 限制 loopback 源，页面仍可能无法连接。`Niva.api.http.post` 是原生 API，经 WS 调用，不等于浏览器向 Niva 服务直接发 POST。协议页与 loopback 间的其他跨源 HTTP `fetch` 仍需单独设计 CSP、CORS 和鉴权。
6. **调试与持久数据**：显式 `--debug-resource` / debug 启动继续使用 loopback 普通 HTTP；显式远端 entry 保持远端来源。打包模式普通 HTTP 静态路由只在调试参数开启时服务，loopback WS 和按 token 验证的 `__niva_fs` 继续存在。每app独立scheme提供稳定origin，让不同UUID的应用存储分开；同一UUID的多个窗口会使用相同origin，按浏览器同源规则预期共享localStorage，权限仍绑定各自窗口token；该多窗口数据读取还需补实测。macOS 26.6.2固定bundle身份的实包 A/A/B smoke 已验证同UUID跨进程重启保留localStorage、换UUID读不到该值。旧固定origin和`http://127.0.0.1:<port>`的数据不会自动迁移。构建脚本目标为macOS 11.0，但当前未在macOS 11设备验收，不能声称所有macOS版本已验收。

### UUID origin 与本地存储集成 smoke（2026-09-26）

macOS 26.6.2，将debug Niva置于固定 `CFBundleIdentifier=com.niva.perapporiginsmoke` 的临时 `.app`，通过LaunchServices连续启动三个独立进程：UUID A写入`localStorage`，UUID A重启后读取成功，UUID B读取为空。结果分别是 `niva-07a4ee28635848b48893ebd6a3caf404://app`/`persisted`、同origin/`persisted`、`niva-8f5c076050e444a38f4aa021b30b4a3e://app`/`null`。完整结果为`/tmp/niva-per-app-origin-smoke/final-result.json`，debug executable SHA-256 `aa08cd96dfb6d128df89f5bf7f7137a784824ee7620370ba108c6eefac65a477`，三次stderr均空。裸`target/debug/niva`不带固定bundle identity时的跨进程持久化失败不等同实际打包`.app`行为。此测试证明当前macOS打包身份下的A/A/B行为，不证明macOS 11运行时行为。

同一夹具替换为最终release二进制后再次通过A/A/B：同UUID首次写入及跨进程重启读取均为`persisted`，不同UUID为`null`。最终release SHA-256 `e0ba2d28a1d9c9d8977a7a72833908def98d66606bb72d3e3f3ac672edfafc0b`，结果`/tmp/niva-per-app-origin-smoke/release-result-final.json`。同进程第二窗口可创建并出现在`window.list`，但子页面没有marker，因此多窗口页面加载及读取localStorage仍未验收。

## Niva macOS 集成 smoke（2026-09-23，固定 origin 历史快照）

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

- **Windows 真机**：WebView2固定 `http://niva.app` origin的同源iframe原生调用、文件URL和部分NodeCompat已有历史有限实测；当前UUID变体 `http://niva-<uuid>.app` 尚未在Windows真机验收，跨源iframe、负向WS握手、严格CSP、动态端口与导航时序也待专项验收。见 [Windows 验证记录](windows-validation-2026-09-23.md)。
- **macOS 深度验收**：当前macOS26的实包A/A/B localStorage smoke已通过；仍需在最低目标macOS 11设备复验，并验证同进程第二窗口实际加载与localStorage读取、旧origin数据迁移边界、窗口间token隔离、跨源iframe拒绝及协议页WS二进制流。第二原生窗口已创建但未观察到子页面marker，不能算通过。`__niva_fs` 的有凭据 fetch 已在固定origin历史 smoke 中覆盖。
- **异步压力与性能**：注入慢资源并同时操作窗口；测队列上限、超时、响应上限和 responder 失效；与 loopback HTTP 入口在相同资源条件下比较。完成前不宣称吞吐量或页面加载性能已验收，也不承诺慢资源永不影响其他请求。
- **导航/关窗时序**：Wry async responder 没有取消状态或 delivery result；确认 WebView 丢弃请求后延迟响应的实际行为，不能把一次 responder 调用等同于页面收到响应。
- **CSP header 边界**：若部署环境额外添加 CSP response header，需在实际平台确认该 header 不会阻止 WS；当前实现只改 HTML 中的 meta policy。
- 已运行并通过 `cargo fmt --all -- --check`、`cargo check --workspace`、`cargo clippy --workspace --all-targets`、`cargo test --workspace` 和 Windows target check；Clippy/编译仍有非阻断 warning。格式检查中的早期 `window.rs` 差异已修正。
