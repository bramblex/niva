---
sidebar_position: 5
---

# NodeCompat

NodeCompat 默认内嵌在 Niva 主程序中，提供面向 WebView 的 Node 风格 API。它使用纯 JS 库、浏览器能力和 Niva 的原生系统接口，不包含 Node.js 运行时。

## 配置

不设置 `nodeCompat` 时默认启用；`false` 关闭；也可选择模块与 import map：

```json
{
  "nodeCompat": {
    "modules": ["path", "fs", "os", "child_process"],
    "importmap": true
  }
}
```

模块集包含 22 个模块：`path`、`os`、`fs`、`child_process`、`events`、`util`、`querystring`、`buffer`、`url`、`crypto`、`zlib`、`http`、`https`、`assert`、`stream`、`process`、`net`、`dgram`、`tls`、`dns`、`string_decoder`、`timers`。未知模块名会被拒绝。宿主 `process` 只注册到可信主窗口的顶层页面。

主程序包含这些模块的 JS 与加载索引；Devtools 打包应用时不再复制一份 NodeCompat 资源。

## 使用

```js
await NivaNodeCompatReady;
const fs = require("fs");
const path = require("path");

const text = fs.readFileSync(path.resolve("notes.txt"), "utf8");
const bytes = await require("fs/promises").readFile("image.png");
console.log(text, bytes.length);
```

`require` / `Niva.require` 读取已注册模块，支持 `node:` 前缀。选择相关模块时还提供 `fs/promises`、`stream/promises`、`dns/promises`、`timers/promises` 和 `assert/strict`。

本地 HTML 文档可通过注入的 import map 使用 `import fs from "node:fs"`。已有 import map 的同名配置优先；`importmap: false` 关闭自动映射。`Niva.import("fs")` 使用内嵌模块 URL；bundler 项目也可以导入 `@niva/node-compat/fs` 等子路径。

## 实现分层

| 能力 | 实现位置 |
| --- | --- |
| Buffer、事件、流、路径、常用哈希/KDF、压缩、DNS 编解码、HTTP 协议 | JS 及浏览器可运行的第三方库 |
| 文件、进程、TCP/UDP socket、TLS、系统信息 | Native 系统接口 |
| `http` / `https` 客户端与应用服务器 | JS，使用 Native TCP/TLS 基座 |
| `dns.lookup` | 系统 resolver，保留 hosts 语义 |
| DNS 记录查询 | JS DNS 协议，使用 UDP/TCP |
| 静态 `os` / `process` 字段 | 启动快照 |
| 动态系统值和同步系统 API | 本地同步 XHR |

WS 提供二进制流和事件；同步 XHR 在 Rust 侧校验窗口凭证、精确 Origin 和方法范围。外部页面仅有授权后的有限 unary IPC，不能使用 socket、二进制流或同步 XHR。

## 兼容边界

覆盖常用 API 不等于兼容整个 Node 运行时。HTTP/1.x、TLS 选项、文件系统平台差异和进程行为仍有子集限制；依赖 Node 内部 binding、原生扩展或事件循环存活语义的库需要单独验证。浏览器 timer 的 `ref` / `unref` 不控制 Niva 进程生命周期。

不提供 `child_process.fork`。以 `--stdio` 启动时，stdin/stdout 保留给宿主 NDJSON 协议，不能同时当作普通 Node 标准流使用。

旧 `Niva.api.http` 已移除，请使用 [HTTP / HTTPS 模块](./http)。Niva 内部的资源、同步调用和 WS 服务继续由 Native 提供，与应用创建的 HTTP server 分开。
