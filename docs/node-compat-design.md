# Node 兼容方案（NodeCompat）

> 状态：方案设计（未实现）。

## 1. 背景与目标

- Niva 已有：smol 轻运行时、自有 loopback HTTP + WS 桥（见 `docs/bridge.md`）、
  全异步 API 调度、`require("niva:*")` 内存注册表、`Niva.import` 动态 import、
  `Niva.bridgeVersion = 1`。
- 目标：在**二进制零增长**前提下，支持 Node 风格
 （`require("fs")` / `import ... from "fs"`）。
- 非目标：完整 Node 运行时、npm 生态兼容、同步 API、服务端渲染。

## 2. 总体设计：编译期为主，运行时为辅

| 需求 | 方案 | 可靠性 |
|---|---|---|
| `require("fs")` | 内存注册表 + classic 脚本自注册 | 完全可靠 |
| 动态 `import()` | `Niva.import(id)` → 完整 URL | 完全可靠 |
| 首屏静态 `import "fs"` | **服务端改写**：server 在响应字节发出前插入 importmap | 完全可靠 |
| 运行时 DOM 注入 importmap | 仅作兜底（动态 import / 后导航页） | 首屏有竞态，不可依赖 |

原因：浏览器预加载扫描器会抢在注入脚本前抓取 module；
服务端改写发生在字节到达浏览器之前，无竞态（构建期补丁方案已废弃，
见 §7——服务端改写是其超集，还覆盖 debug 目录且不碰用户文件）。

## 3. 体积策略：可选才打包

- 兼容库**不进 Rust 二进制**：静态 JS 文件，打包时塞进资源目录
  （`__niva_compat/*.js`），由现有 HTTP server 直接 serve。
- 开关关闭时：文件不存在，行为与今天完全一致。
- 粒度：`nodeCompat: true | { modules: [...], importmap: true }`。

## 4. 配置（niva.json）

```json
"nodeCompat": {
  "modules": ["fs", "path", "os", "child_process"],
  "importmap": true
}
```

## 5. 兼容库（四件套）

先随 devtools 走；拆独立仓时原样搬运，接口不变。

| 模块 | 实现方式 |
|---|---|
| `path` | 纯 JS 实现 |
| `os` | 桥接 `niva:os` |
| `fs` | 桥接 `fs.readStream/writeStream`（readFile/writeFile/appendFile Promise 形） |
| `child_process` | 桥接 `process.execStream`（exec 回调形仿真 / spawn 事件形） |

## 6. 明确不支持

- **同步 API**（已决策 B 修窄版）：只给小文件读三件套
  （`existsSync/statSync/readFileSync`）开过渡同步（同步 XHR + 60s 硬超时 +
  每次调用 warning）；`execSync`/写同步永不给（必卡 UI / 语义本就异步）。
  详见 `docs/node-api-frequency.md §6.4`。
- **`require` 任意第三方包**：只认白名单，未知 id 保持抛错。
- **sync XHR 拉模块**：不做。
- **静默改写用户 HTML**：importmap 补丁只在显式 opt-in 时做，只动入口文件、可重复执行。

## 7. 服务端注入（已决策，替代构建期补丁）

1. 按 `modules` 把 `__niva_compat/*.js` 写进资源包（照旧）。
2. 仅当**同时满足**才改写响应字节：`nodeCompat` 开启（opt-in）＋
   `Content-Type` 为 `text/html` ＋ **浏览器直接加载**
   （`is_document_navigation`，已落地：`Sec-Fetch-Mode: navigate` 优先，
   无 Fetch Metadata 的旧引擎回退看 `Accept` 是否含 `text/html`；
   fetch 拿到的 HTML 是数据，必须原字节通过）。
3. 插入点：`<head>` 开标签之后；用户自带 importmap 时**合并**（用户优先）；
   带幂等标记注释；每次改写打 server 日志。
4. CSP 注意：用户页自带 CSP 时需放行，文档注明。
5. **不做来源标记**（已决策）：wry 的 `with_user_agent` 在三端都是全替换语义
   且无读默认 UA 的 API，自拼 UA 会破坏页面兼容性；`with_headers` 只管首个请求。
6. **HTTP 鉴权已决策**（`docs/http-auth-plan.md`）：静态读走 session-cookie，
   `__niva_fs/` 在鉴权内，`/__niva_compat/*` 豁免 + CORS。本节旧文案中
   "读不需要身份 / 不留钩子" 已作废，以 http-auth-plan 为准。

## 8. 运行时改动（小）

- `require` 注册表已存在；compat 脚本自注册 Node 形 id。
- `Niva.import(id)`：补裸名到 URL 的映射。
- DOM 注入 importmap 兜底。

## 9. 版本与安全

- wire 版本已同步为 1；compat 库声明 `NIVA_BRIDGE_MIN = 1`，不匹配抛可读错误。
- token 只在内存、只绑 loopback；compat 层不接触秘密。
- 签名工具与本方案正交：compat 文件只是普通资源，随包签名。

## 10. 测试计划

- 单文件页：`require("fs")` 读写、`await Niva.import("path")`、静态 `import`、未知模块抛错。
- 打包应用双路径：勾选/不勾选（验证无 `__niva_compat`、行为零差异）。
- 子集打包：只选 `path` 时其它模块 404 且报错可读。
- 回归：现有 WS 电池全过；`tsc` + `vite build` 干净。

## 11. 实施顺序

1. compat 库四件套。2. 打包注入。3. 配置/d.ts/文案。4. 双路径验证。5. （后话）拆独立仓库。
