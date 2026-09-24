# 浏览器纯 JS 第三方实现方案

> 2026-09-24 · 补充到 [Node API 盘点](node-api-coverage.md)。这里列候选实现路径；未安装依赖、未修改运行时代码，也未把候选方案计为已实现。

## 评估口径

JS/浏览器原语/纯 JS 库是默认方案。只有系统能力不可由 JS 提供、选定契约在浏览器无法满足，或目标 WebView 实测性能不达标，才转 Native；未做基准不预设 JS 算法需要下沉。child_process.fork 已移出目标。

新增网络协议候选：`dns-packet` 5.6.1和`http-parser-js` 0.5.10可在JS层编解码协议，并复用已规划的Native net/dgram/tls；两者都不是完整Node模块，查询/连接生命周期与对象封装仍需实现。[功能、尺寸与Rust DNS比较](node-layering-plan.md)。

能用现成的浏览器纯 JS 库覆盖的范围，统一按**低难度：库集成 + Node 风格接口封装**估算。可以在构建时使用 npm/bundler；交付页面不依赖 Node 进程、Node 内置模块或原生 addon。依赖其他纯 JS polyfill 也符合这一口径，但必须正确打包 browser 入口。

这条路径与既定 WS、同步 XHR、有限跨域 IPC 并存：能够在 JS 中计算，就不需要过桥；它也不会自动赋予浏览器访问本机文件、启动进程或建立原始 socket 的能力。原来的 Native 路线作为备选保存在 JSON 的 `nativeRouteAlternative`，新增路径位于 `browserJsAlternative`。

## crypto：本表 8 项中的 7 项可采用浏览器方案

| Node API | 浏览器实现方案 | 封装工作 | 难度 |
|---|---|---|---|
| `createHash` | `@noble/hashes` 的 SHA、legacy MD5/SHA-1 等模块 | 算法名映射；`create/update/digest`；Buffer/hex/base64 | 低 |
| `createHmac` | `@noble/hashes/hmac.js` | key/input 编码及 update/digest | 低 |
| `pbkdf2` | `pbkdf2Async` | Promise 转 Node callback；参数、Buffer、错误映射 | 低 |
| `pbkdf2Sync` | 同库同步 `pbkdf2` | 直接同步返回，无需 XHR | 低 |
| `scrypt` | `scryptAsync` | Node 参数、资源选项、callback 和 Buffer | 低 |
| `randomBytes` | 浏览器 `crypto.getRandomValues`，也可用 noble 的随机辅助函数 | 同步/回调签名、Buffer；较长请求需分块 | 低 |
| `randomUUID` | 浏览器 `crypto.randomUUID`，或用安全随机字节组装 UUID | 同步字符串及选项映射 | 低 |
| `timingSafeEqual` | 暂不将普通 JS 比较作为严格等价替代 | JIT/GC 使恒定时间保证不能只靠循环写法证明 | 保留原评估 |

依据：[noble-hashes README](https://github.com/paulmillr/noble-hashes)、[包配置](https://github.com/paulmillr/noble-hashes/blob/main/package.json)、[随机源实现](https://github.com/paulmillr/noble-hashes/blob/main/src/utils.ts)。核心算法是 JS；同步计算选算法模块，不选异步 `webcrypto` 包装。安全随机数仍依赖浏览器提供的熵源，这与依赖 Node runtime 不同。库作者也明确说明了 JS 恒定时间的限制，见 [Constant-timeness](https://github.com/paulmillr/noble-hashes#constant-timeness)。

这里覆盖的是盘点中的 API 和选定算法，不能外推为整个 `node:crypto`、所有 OpenSSL 算法或 KeyObject 契约已兼容。还需补算法白名单、编码、参数、错误与回调顺序。KDF 的运行时间和内存开销仍由参数决定；“集成难度低”不表示运算成本低。

## 其他可复用的浏览器库

| 模块 | 第三方库 | 可以覆盖的部分 | 边界 |
|---|---|---|---|
| `zlib` | [fflate](https://github.com/101arrowz/fflate) | gzip/gunzip，同步与异步版本 | 选 `fflate/browser`；Node options、callback、Buffer 需包装；异步 Worker 路径需验证 WebView/CSP。 |
| `buffer` | [feross/buffer](https://github.com/feross/buffer) | 常用 Buffer 静态/实例方法，包括视图、复制、编码和数值读写 | 依赖 `base64-js`、`ieee754` 等纯 JS 包；需对本表已发现的 BOM、TypedArray、slice、copy 等差异回归。 |
| `stream` | [nodejs/readable-stream](https://github.com/nodejs/readable-stream) | Readable/Writable/Transform/PassThrough、pipeline、finished、promises.pipeline | browser 入口及其纯 JS 依赖必须打包；真实文件/进程/网络数据源仍接 Niva Native，不能因此把 `fs.createReadStream` 的 Native 差距全部消掉。 |
| `events` | [browserify/events](https://github.com/browserify/events) | 常用 EventEmitter 与基础 `once(emitter, name)` | 上游目标是较早 Node API；不据此覆盖 `events.on` async iterator、captureRejections 或新 options。本轮不统一降低这些未覆盖项。 |
| `path` | [path-browserify](https://github.com/browserify/path-browserify) | POSIX path 的常用字符串操作 | 不覆盖 Win32 或 `matchesGlob`；resolve 需要 cwd shim。本仓已有跨平台实现，不能直接整体替换。 |

入口与依赖核对：[fflate package.json](https://github.com/101arrowz/fflate/blob/master/package.json)、[buffer package.json](https://github.com/feross/buffer/blob/master/package.json)、[readable-stream browser 导出](https://github.com/nodejs/readable-stream/blob/main/lib/ours/browser.js)、[readable-stream package.json](https://github.com/nodejs/readable-stream/blob/main/package.json)、[events 实现](https://github.com/browserify/events/blob/main/events.js)、[path-browserify 包配置](https://github.com/browserify/path-browserify/blob/master/package.json)。

`readable-stream` 的 browser 版本使用 `buffer/events/process/string_decoder` 等 JS 包；其中 `process` 是浏览器 shim，不是 Node runtime，也不能拿它充当 Niva 的真实进程信息。其上游代码基线不等于本表参照的最新版 Node，因此仍按选定 API 验证。

真实 process 的目标注入范围现限定为 main 窗口；依赖库内部的 process shim 应保持模块私有，不能在子窗口顺带暴露主窗口的环境或参数。`worker_threads` 已移出 Node 兼容目标，但这不限制 fflate 等库使用浏览器 Worker；console 使用浏览器原生，不补 node:console。

补充的 `string_decoder` 1.3.0 已做浏览器打包测量，可直接复用其具名 `StringDecoder` 导出；通常与 readable-stream 共用依赖，按低难度封装评估。测量见 [模块体积预算](node-api-size-estimates.md)。

## 对盘点数据的影响

此次为 crypto 的 7 项、zlib 的 4 项、stream 的 7 项、buffer 的 14 项，加上 StringDecoder 的 1 项，共 **33 项**补了浏览器方案。已有基础支持的条目保持原状态；其余 **24 项**采用这条候选路线按低估算。当前“基础支持/部分兼容/缺失”的数量不变，只有方案和成本评估更新。

`events` 和 `path` 在本页列为有边界的补充候选，没有据此声称新增 API 全覆盖，也没有把 `path.matchesGlob`、`events.on` 等仍需自行实现的项目自动降级。

验证范围为上游 README、源码和包入口/依赖检查。本轮没有进行 Niva WebView 集成运行或新增产物体积测试。落地时固定版本并将所需代码打入应用资源，避免在具有 Native 权限的页面运行临时下载的 CDN 脚本。JS 资源也计入发布产物体积；上游 minified/gzip 数字不替代 Niva release 实测。
