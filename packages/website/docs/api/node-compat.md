---
sidebar_position: 5
---

# NodeCompat

NodeCompat 是可选的浏览器端 Node 风格 API 子集。它使用 Niva 提供的异步原生 API，不是 Node.js 运行时，也不实现 Node 的完整签名或所有模块行为。

## 启用与模块选择

在项目 `niva.json` 顶层配置 `nodeCompat`。设为 `true` 选择全部受支持模块，也可以显式选择模块和是否注入静态 import map：

```json
{
  "nodeCompat": {
    "modules": ["path", "fs", "os", "child_process"],
    "importmap": true
  }
}
```

省略 `modules` 时选择全部 15 个模块：`path`、`os`、`fs`、`child_process`、`events`、`util`、`querystring`、`buffer`、`url`、`crypto`、`zlib`、`http`、`https`、`assert` 和 `stream`。未知模块会在构建配置校验中拒绝。NodeCompat 默认关闭；未启用时不要求或打包其模块资源。

## 使用方式

启用 `nodeCompat` 后，Devtools 会打包已选模块；Niva 在 HTML 文档导航响应中自动插入 classic 入口和（选用时）import map，应用通常不必手动添加脚本标签。入口就绪后，`Niva.require()` 可读取已注册模块；异步页面也可使用 `Niva.import()`：

```js
await NivaNodeCompatReady;
const path = Niva.require("path");
const fs = Niva.require("fs");
const contents = await fs.readFile(path.resolve("notes.txt"), "utf8");
console.log(contents);
```

单文件 ESM 页面可使用 `Niva.import("path")`；普通静态裸名导入（例如 `import fs from "fs"`）依赖 NodeCompat 注入的 import map。它只对 HTML 文档导航响应生效，模块必须在解析前收到 import map。已有 import map 的同名映射优先。`importmap: false` 会关闭自动注入，但仍打包所选 ESM 资源；应用可使用自己的 import map 或 bundler alias。被 JavaScript fetch/XHR 获取的 HTML 不会自动改写，如需运行 NodeCompat，应自行在 `initialize_script.js` 之后加载 `__niva_compat/node-compat.js`。

使用 bundler 的 ESM 项目也可以从包子路径导入，例如 `@niva/node-compat/path`、`@niva/node-compat/fs` 和 `@niva/node-compat/child_process`。

选中 `fs`、`assert`、`stream` 时还会注册相应 `fs/promises`、`assert/strict` 和 `stream/promises` 名称。打包只包含所选模块及其资源依赖。

## 模块能力与限制

- `path` 提供路径字符串操作；`path.resolve()` 所需的当前工作目录来自异步 bridge 初始化，在 `NivaNodeCompatReady` 完成前可能仍是初始目录。
- `fs` 和 `fs/promises` 提供异步文件 API，不提供同步文件 API。Native `Stats` 不包含 Node 专有的权限、inode 或 owner 语义。
- `child_process.spawn/exec` 由 `process.execStream` 适配；stdin 可写入。它不是完整 PTY，不支持 Node 的 `timeout` 选项、AbortSignal、任意 Node stdio 模式、附着进程 ID 或 `child.kill()` 逐进程终止；`child.cancel()` 取消 bridge stream 后会终止并回收关联的普通子进程，`detached` 进程除外。项目的 `api.timeoutMs` bridge 截止时间也会生效，默认 30 秒。
- `http` / `https` 用 Niva `http.requestStream` 传输响应；选项、请求体和事件是包提供的 Node 风格适配，不等同于 Node Agent/Socket。原生侧禁止代理与非公开网络目标（包括重定向到这些地址）；唯一 loopback 例外是当前 Niva 服务的精确端口。二进制流和 HTTP stream 需要本地 WebSocket 页面。
- `stream` 仅包含包所支持对象上的 `pipeline()` 适配，不提供 Node 的完整 Readable/Writable/Transform 类、背压或通用流构造器。
- `crypto` 和 `zlib` 使用 Web Crypto、`CompressionStream` / `DecompressionStream` 等浏览器能力，具体可用性取决于当前 WebView。
- 页面在 HTML 内自带 CSP meta 时，Niva 会将自动注入的 import map/classic 脚本放在策略之后，并只给这两条 Niva 脚本加每次导航生成的 nonce；作者的其他内联脚本不会因此获准执行。外部模块与资源仍须符合页面策略。Niva 无法改写宿主另加的 CSP response header；若该 header 阻止资源，项目仍需自行调整。
- `events`、`util`、`querystring`、`buffer`、`url` 和 `assert` 提供有限的浏览器实现；请按项目实际需要确认方法覆盖范围。

远端 IPC 页面不支持流式调用，因此依赖 Niva binary stream 的 `fs`、`child_process`、`http` 等操作不可用。查看[流式调用](./stream)与[Bridge 传输边界](./bridge)。

## 方法覆盖摘要

| 模块 | 当前适配的方法 |
| --- | --- |
| `path` | `join`、`resolve`、`basename`、`dirname`、`extname`、`normalize`、`relative`、`parse`、`format`、`isAbsolute`、`toNamespacedPath`、常用 glob 模式的 `matchesGlob`；包含 `posix` / `win32` 变体。 |
| `fs` / `fs/promises` | `readFile`、`writeFile`、`appendFile`、`mkdir`、`readdir`、`stat`、`access`、`rename`、`rm`、`cp`、`copyFile`。 |
| `os` | `platform`、`arch`、`homedir`、`tmpdir`，以及 `EOL`、`sep`、`delimiter` 常量。 |
| `child_process` | `spawn`、callback 风格的 `exec`；子进程 stdout/stderr、`stdin.write()` 与 `stdin.end()`。 |
| `http` / `https` | `request`、`get`、`post`、`ClientRequest` 与 `IncomingMessage` 的有限属性、Promise 和事件接口。 |
| `stream` | `pipeline(source, ..., destination, callback?)`，适配包内受支持的 child process、HTTP message/request 流。另有 `stream/promises`。 |
| `assert` | 可调用 `assert`、`ok`、`fail`、`equal`、`notEqual`、strict/deep equality、`throws`、`doesNotThrow`、`rejects`、`doesNotReject`、`ifError`、`match`、`doesNotMatch` 和 `AssertionError`。另有 `assert/strict`。 |
| `events` | `EventEmitter` 的监听、移除、触发、枚举方法，包括 `on`、`once`、`off`、`emit`、`listeners`、`listenerCount` 和 `eventNames`。 |
| `util` | `format`、`inspect`、`promisify`、`callbackify`、`isDeepStrictEqual`、`deprecate`。 |
| `querystring` | `parse`、`stringify`、`escape`、`unescape`。 |
| `buffer` | `Buffer.from`、`alloc`、`allocUnsafe`、`concat`、`byteLength`、编码转换、比较/查找、复制、填充、JSON 与 8/16/32 位整数读写。 |
| `url` | WebView 原生 `URL` / `URLSearchParams`，以及 `fileURLToPath`、`pathToFileURL` 的本地路径适配。 |
| `crypto` | `randomBytes`、`randomUUID`、`createHash`；支持 SHA-1 / SHA-256 / SHA-384 / SHA-512 的异步摘要。 |
| `zlib` | `gzip`、`gunzip`，依赖浏览器 `CompressionStream` / `DecompressionStream`。 |

该表只列支持面，不承诺完整 Node 签名或事件时序。有关未支持项、编码边界、平台能力和行为限制，以包 README 与实现测试为准。
