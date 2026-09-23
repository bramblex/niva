# Node 兼容层（NodeCompat）设计与当前边界

> 状态：核心浏览器适配、资源选择与服务器注入已实现；仍属有明确限制的 Node 风格子集，不是完整 Node 运行时。同步 API 未实现；一元同步与全量同步的产品范围仍待用户决定。

## 1. 目标与范围

NodeCompat 是独立的可选浏览器包 `packages/node-compat`，为 Niva 页面提供 15 个 Node 风格模块：`path`、`os`、`fs`、`child_process`、`events`、`util`、`querystring`、`buffer`、`url`、`crypto`、`zlib`、`http`、`https`、`assert`、`stream`。实现通过现有 Niva bridge 调用本地能力，库本身不引入新的 Rust 运行时。

它不提供完整 Node/npm 运行时，也不承诺所有 Node 签名、同步文件 API、`child.kill()` 这类逐进程终止或实时子进程输出。已附加的普通子进程会在所属 bridge 调用取消或超时时被终止并回收；`detached` 子进程可脱离调用继续运行。精确的支持签名和限制见 [`packages/node-compat/README.md`](../packages/node-compat/README.md)。

## 2. 配置与资源选择

`niva.json` 可用 `nodeCompat: true` 启用默认模块集，也可显式选模块并控制静态 importmap：

```json
"nodeCompat": {
  "modules": ["fs", "path", "os", "child_process"],
  "importmap": true
}
```

省略 `modules` 时选择全部 15 个模块；省略 `importmap` 时默认为 `true`。未知模块会被拒绝。Devtools 按所选模块及其 ESM 依赖闭包打包对应资源，并选择 classic 入口；Rust 服务端只允许读取所选模块所需的兼容资源。关闭时不要求也不打包 NodeCompat 资源。

## 3. 加载路径

- classic 入口 `__niva_compat/node-compat.js` 为页面注册已选模块及其 `node:` 名称；选中 `fs`、`assert`、`stream` 时也注册对应的 `fs/promises`、`assert/strict`、`stream/promises`。单文件页面通过 `require()` 使用这些名称；ESM 子路径使用各自的专用 wrapper。
- 页面静态裸名导入依赖服务端在 HTML 响应中注入 importmap。注入仅在 NodeCompat 开启、选项允许 importmap 且请求是文档导航时进行；fetch/XHR 获取 HTML 不会被改写。已有 importmap 会与 NodeCompat 映射合并，已有键优先，并通过标记避免重复注入。
- `importmap: false` 关闭静态裸名映射注入，但所选 ESM 资源仍会打包，可由应用提供自己的映射或使用打包工具配置别名。
- 浏览器在模块解析前需要 importmap。模块已被 bundler 改写为静态 URL 的页面不依赖运行时 DOM 注入；运行时注册不能修复已经解析失败的静态裸名导入。

## 4. HTTP 资源与文件凭证

服务器的 `__niva_compat/` 资源路由仅服务允许的资源，并返回 JavaScript MIME、`Access-Control-Allow-Origin: *`、`nosniff` 和 `no-store`。该静态适配器资源路由不等同于 bridge 或文件 API 的授权。

`__niva_fs/` 使用窗口作用域的 file token，并在 Rust 服务端校验 token 对应的窗口；当前实现不是旧设计中描述的 session-cookie。不要把早期方案里的 cookie 豁免、静态资源 session-cookie 保护等描述当成现状。完整 HTTP/bridge 权限边界仍以 [`http-auth-plan.md`](http-auth-plan.md)、[`permission-design.md`](permission-design.md) 和 [`security.md`](security.md) 的实际状态为准。

## 5. 浏览器与平台限制

- 页面 HTML 自带 CSP meta 时，运行时会把 Niva 注入的 import map/classic 脚本移到策略之后，并只给这两条脚本加每次导航随机 nonce；作者其他内联脚本不因此获准。模块资源仍受页面策略约束。宿主额外提供的 CSP response header 无法由 Niva 改写，仍可能阻止兼容层加载。
- Web Crypto、`CompressionStream` / `DecompressionStream` 等能力取决于 WebView 实现；crypto 与 zlib 在能力缺失时会拒绝或不可用。
- `fs` 和 `child_process` 的二进制流需要本地 Niva WebSocket 页面；远端 IPC 页面不支持这些二进制流。
- macOS 已验证隔离运行与打包后的浏览器路径；Windows WebView2 已有限实测打包页的 `path`、`fs/promises` 与 `assert/strict`，包括严格 CSP 下的成功路径。完整模块/API 与 CSP 矩阵仍待验收，见 [Windows 验证记录](windows-validation-2026-09-23.md)。

## 6. 未决 API 冲突

`docs/node-api-frequency.md` 的历史方案一度提出全量过渡同步 API，另一处又提出只开放少量同步读 API。二者与当前异步 bridge 和已实现 NodeCompat 的边界冲突。当前 NodeCompat **没有实现同步 API**，本设计不选择窄版或全量版，也不据此承诺同步 API；该产品范围等待用户决定。在决定前，兼容包继续保持异步文件和进程操作，以及纯字符串性质的同步 `path` 操作。

## 7. 验证状态

NodeCompat 包含针对适配器的测试；Devtools 有资源选择/依赖闭包测试。macOS 和 Windows 均有有限打包浏览器验证；Windows 本轮还修复了根目录相对路径的盘符继承。现有结果不消除浏览器能力、CSP 和完整模块语义的限制。
