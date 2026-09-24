# Node 内置 API 使用频率调研与 Niva 落地顺序

> **2026-09-24 逐项盘点**：模块与 API 的当前覆盖、频率档位及补齐难度见 [Node.js 模块与常用 API 覆盖盘点](node-api-coverage.md)，可复算数据见 [node-api-inventory.json](node-api-inventory.json)。本文件保留早期调研与候选方案；下文模块名次是定性判断，没有可复算的频率样本，不能作为真实调用占比。§2 和 §6 的简写/历史签名也不替代 Node 官方契约；覆盖统计以新表逐项状态为准。

> **难度口径**：沿用双向 WS、本地同步 XHR、跨域有限 IPC 三条路径。先判断 Niva Native 是否具备所需能力，再估算 JS API 封装；仅封装为低、扩展现有 Native 或复杂 JS 语义为中、新增整类 Native 后端为高、引入大型第三方库或完整运行时为很高。同步机制本身按低难度，库大小未实测时只列选型条件或风险。

> **纯 JS 补充**：现成浏览器库能够覆盖且无需 Node runtime 时，统一按低难度；例如 crypto 摘要/HMAC/KDF 可选 `@noble/hashes`，zlib 可选 `fflate`。候选库及限制见 [浏览器纯 JS 方案](node-browser-js-options.md)。

> **当前范围与注入决策**：目标已收敛到 22 个模块族、179 项 API；旧调研中出现的 perf_hooks、console、http2、tty、v8、vm、sqlite、test、cluster、worker_threads、readline 不再属于当前待开发目标。console 使用浏览器原生；process/os 固定信息启动注入，真实 process 仅在 main 窗口注入/注册。动态系统信息每次用同步 XHR 查询，不采用缓存或推算；实际操作按接口契约处理。child_process.fork 也已移出目标；默认 JS，仅必要系统能力/不可满足的接口行为或实测性能缺口才 Native。本文后续排名和历史方案仅作调研留档，以最新逐项盘点为当前分母。

> 日期：2026-09-22。目的：按真实使用频率排 Node builtins，指导 Niva 按“高频先行、零新增依赖优先”补齐。
> 方法：教程共识（Flavio Copes / GeeksforGeeks / W3Schools）+ Socket 供应链遥测 + npm 生态论文方法三角验证。
> 局限：**不存在权威的 builtin 级百分比排名**；SO / State of JS 只到运行时层面。以下名次 1–4 可信度高，5–12 中（教程间互换），函数级精确占比公开数据无（如需精确数，做 BigQuery `github_repos.contents` 全量 `require('node:x')` 计数或 npm Top-1000 静态扫描）。
> Node 签名以 `nodejs.org/api` v26 为准；Niva 现状以本仓实测为准（见 `docs/PROJECT_MANUAL.md §2.6`）。

> 实施状态更新（2026-09-23；同步方案说明更新于 2026-09-24）：`packages/node-compat` 已提供 15 个浏览器适配模块，Rust `nodeCompat` 配置、服务器 HTML importmap/classic 注入与 CORS 兼容资源路由、Devtools 按模块选择资源均已实现。这里的频率排名和 API 清单仍是调研/目标范围，不代表每个列出的 Node 签名都已覆盖。同步调用沿用用户确认的既定同步 XHR 方案，具体 API 接入和覆盖范围按逐项清单选择；见 §6 和 [`node-compat-design.md`](node-compat-design.md)。

## 1. 模块频率排名

| # | 模块 | 典型用途 | 可信度 |
|---|---|---|---|
| 1 | `fs` / `fs/promises` | 脚手架/CLI/构建必用 | 高 |
| 2 | `path` | 与 `fs` 捆绑，跨平台拼接 | 高 |
| 3 | `events` | 自定义事件，多数模块的基类 | 高 |
| 4 | `process`（global） | `env/argv/cwd/exit`，每个 CLI 都用 | 高 |
| 5 | `http` / `https`（渐被 `fetch` 分流） | 建 server、发请求 | 高 |
| 6 | `util` | `format/inspect/promisify` | 高 |
| 7 | `os` | 平台判断、CI 脚本 | 高 |
| 8 | `crypto` | 哈希、随机、加密 | 高 |
| 9 | `child_process` | `spawn/exec`，安全报告最高频风险 API | 高 |
| 10 | `stream` | 大文件 pipe，多为间接依赖 | 中高 |
| 11 | `url` + `URLSearchParams`（`querystring` 已 legacy） | URL 解析 | 中高 |
| 12 | `buffer`（global） | 二进制基础，多为间接使用 | 中 |
| 13 | `net` / `tls` / `dns` / `dgram` | 被 `http` 遮蔽，直接用少 | 中 |
| 14 | `zlib` | 压缩，多为间接（server gzip） | 中 |
| 15 | `readline` | CLI 交互 | 中低 |
| 16 | `worker_threads` / `cluster` | 并行，远少于上述 | 中低 |
| 17 | `assert` / `node:test`（上升中）/ `sqlite`（Node 24+ RC） | 测试、内嵌 DB | 低（趋势） |

未进榜：`vm`、`v8`、`tty`、`string_decoder`、`http2`（远少于 http1）、`punycode/domain`（deprecated）。

## 2. 高频 API 签名（含参数）

只列每模块最常用的子集，即 Niva 应该先做的部分。

### 2.1 `path`（全量建议做，纯字符串逻辑）

```ts
join(...paths: string[]): string
resolve(...paths: string[]): string
basename(path: string, suffix?: string): string
dirname(path: string): string
extname(path: string): string
normalize(path: string): string
relative(from: string, to: string): string
parse(path: string): { root, dir, base, name, ext }
format(pathObject: { dir?, root?, base?, name?, ext? }): string
isAbsolute(path: string): boolean
sep: string; delimiter: string  // ';' win / ':' posix
posix: typeof path; win32: typeof path
matchesGlob(path: string, pattern: string): boolean // v22.5+
```

### 2.2 `fs` / `fs/promises`（现代标准是 promises 形）

```ts
readFile(path: string, options?: { encoding: 'utf8' | null }): Promise<string | Buffer>
writeFile(file: string, data: string | Buffer, options?: { encoding?: string, mode?: number }): Promise<void>
appendFile(path: string, data: string | Buffer, options?: { encoding?: string }): Promise<void>
mkdir(path: string, options?: { recursive?: boolean, mode?: number }): Promise<void>
readdir(path: string, options?: { withFileTypes?: boolean }): Promise<string[]>
stat(path: string, options?: { bigint?: boolean }): Promise<Stats>
access(path: string, mode?: number): Promise<void>
rename(oldPath: string, newPath: string): Promise<void>
rm(path: string, options?: { recursive?: boolean, force?: boolean }): Promise<void>
cp(src: string, dest: string, options?: { recursive?: boolean }): Promise<void>
copyFile(src: string, dest: string, mode?: number): Promise<void>
existsSync(path: string): boolean
watch(filename: string, options?: { recursive?: boolean }, listener?: (event: string, filename: string) => void): FSWatcher
```

### 2.3 `events`

```ts
on(event: string, listener: (...args: any[]) => void): this
once(event: string, listener: (...args: any[]) => void): this
off(event: string, listener: (...args: any[]) => void): this
emit(event: string, ...args: any[]): boolean
removeAllListeners(event?: string): this
```

### 2.4 `process`（global）

```ts
env: Record<string, string | undefined>
argv: string[]; argv0: string
cwd(): string; exit(code?: number): void
platform: string; arch: string; version: string
nextTick(fn: (...args: any[]) => void, ...args: any[]): void
```

### 2.5 `os`

```ts
platform(): string; arch(): string; hostname(): string
tmpdir(): string; homedir(): string; EOL: string
cpus(): { model: string, speed: number, times: { user, nice, sys, idle, irq } }[]
freemem(): number; totalmem(): number
uptime(): number; type(): string; release(): string; version(): string
userInfo(options?: { encoding?: string }): { username, uid, gid, shell, homedir }
networkInterfaces(): Record<string, { address, netmask, family, mac, internal, cidr }[]>
```

### 2.6 `util`

```ts
format(fmt: any, ...args: any[]): string
inspect(obj: any, options?: { depth?: number, colors?: boolean }): string
promisify(fn: Function): (...args: any[]) => Promise<any>
callbackify(fn: () => Promise<any>): Function
isDeepStrictEqual(a: any, b: any): boolean
deprecate(fn: Function, msg: string): Function
```

### 2.7 `http` / `https`（客户端优先）

```ts
createServer((req, res) => void): Server  // Niva 暂不需要服务端
server.listen(port: number, host?: string): void
get(url: string, callback?: (res) => void): ClientRequest
request(options: { method, hostname?, port?, path?, headers? }, callback?: (res) => void): ClientRequest
```

### 2.8 `child_process`

```ts
spawn(cmd: string, args?: string[], options?: { cwd?, env?, stdio?, detached?, windowsHide? }): ChildProcess
exec(cmd: string, options?: { cwd?, env?, timeout? }, callback?: (err, stdout, stderr) => void): ChildProcess
execFile(file: string, args?: string[], options?: {}, callback?: (err, stdout, stderr) => void): ChildProcess
fork(modulePath: string, args?: string[]): ChildProcess
```

### 2.9 `url` / `querystring`

```ts
new URL(input: string, base?: string)
fileURLToPath(url: string | URL): string
pathToFileURL(p: string): URL
URLSearchParams: new (init?: string | Record<string,string>) + append/get/getAll/set/toString
querystring.parse(str: string, sep?: string, eq?: string): Record<string, string | string[]>
querystring.stringify(obj: Record<string, any>): string
```

### 2.10 `crypto` / `buffer` / `stream` / `zlib`

```ts
createHash(algo: 'sha256' | 'sha1' | 'md5').update(data: string | Buffer).digest(enc: 'hex' | 'base64'): string
createHmac(algo: string, key: string | Buffer).update(data).digest(enc?: string): string
randomBytes(n: number): Buffer
randomUUID(): string
pbkdf2Sync(password, salt, iterations, keylen, digest): Buffer
Buffer.from(str: string, enc?: 'utf8' | 'base64' | 'hex'): Buffer
Buffer.alloc(size: number, fill?: number): Buffer
Buffer.concat(list: Buffer[]): Buffer
pipeline(src: Readable, ...transforms: any[], dst: Writable, cb: (err) => void): void
zlib.gzip(data: Buffer): Promise<Buffer>; zlib.gunzip(data: Buffer): Promise<Buffer>
```

### 2.11 `readline` / `assert`（按需）

```ts
createInterface(options: { input: Readable, output?: Writable }): Interface
rl.question(query: string): Promise<string>
assert.strictEqual(a: any, b: any): void; assert.deepStrictEqual(a: any, b: any): void
assert.ok(value: any, message?: string): void; assert.throws(fn: Function): void
```

## 3. Niva 覆盖状态（NodeCompat 浏览器适配器）

`packages/node-compat` 当前包含 `path`、`os`、`fs`、`child_process`、`events`、`util`、`querystring`、`buffer`、`url`、`crypto`、`zlib`、`http`、`https`、`assert`、`stream` 共 15 个模块。适配器把 Node 风格 API 映射到浏览器原语及 Niva bridge；这是有边界的子集，不能据模块名称推断 Node 全量兼容。各模块逐项支持签名、不可用 API 和运行限制以 [`packages/node-compat/README.md`](../packages/node-compat/README.md) 为准。

覆盖边界示例：`fs` 提供异步操作，不提供 sync API；`child_process` 不支持 Node 的 `timeout` 选项、`AbortSignal` 或 `child.kill()` 逐进程终止，但取消或超时的 bridge 调用会终止并回收已附加的普通子进程，`detached` 子进程除外；`crypto` 仅提供 Web Crypto 支持的异步摘要等子集；`zlib` 依赖浏览器 compression streams。`http`/`https` 仅适配客户端请求，`stream.pipeline` 仅支持本包已有的适配流，不提供通用 Transform 或背压。频率排名中的 `process`、`readline` 等独立 Node builtin 尚未成为 NodeCompat 导出的适配器模块。

本轮已有 macOS 隔离与打包浏览器验证覆盖路径和文件操作。Windows 真正的 WebView 运行时仍未验证；target 编译检查不构成 Windows 浏览器验收。页面 CSP 也必须允许所需脚本/资源。服务端 HTML 注入仅作用于文档导航，fetch 获取的 HTML 不改写；bundler 已处理的静态导入应由打包配置解决，不能依靠事后 DOM 注入。

## 4. 第三方依赖最小化分析（早期方案，非当前实现）

以下依赖选择记录的是 NodeCompat 落地前的方案比较，不是当前 Rust crate 或浏览器适配器依赖清单。当前覆盖以 §3 和 `packages/node-compat/README.md` 为准。

原则：纯 JS shim > Rust std > 已有 crate 复用 > 新增小 crate > 新增大 crate（拒绝）。

| 层级 | 模块 | 依据 |
|---|---|---|
| 纯 JS，0 Rust 改动 | `events`、`util`、`path`、`querystring`、`assert`、`URLSearchParams` | 字符串/对象逻辑；`path` 可直接搬 Node 的 posix/win32 算法 |
| Rust std 即可，0 新增 crate | `fs` 核心（`std::fs`）、`os` 补齐（`std::env` + 已有 `os_info/directories`）、`process`/`child_process`（`std::process::Command`）、`readline` 简版（`std::io::stdin`） | Niva 已有 `fs_extra/glob` 覆盖 `cp/rm` 语义 |
| 已有 crate 复用，0 新增 | `url`（`url 2.3`）、`buffer/base64`（`base64 0.13`）、`zlib`（`flate2`）、`randomBytes/randomUUID`（`getrandom 0.3`）、`http` 一元版（`ureq 2.6 + native-tls`） | 只需在现有实现上包一层 Promise 形 |
| 唯一可能新增（小、纯 Rust） | `createHash/createHmac`（`sha2 + hmac + md-5`，三小 crate） | std 做不了 SHA；替代方案：暂只暴露 `random*`，哈希引导用 Webview 自带 `SubtleCrypto.digest`，零新增 |
| 拒绝/延后 | `net/dns` 服务端、`worker_threads`、`sqlite`、`http2` | 需 `tokio` 级运行时或大依赖，与“二进制零增长”（`node-compat-design.md §2`）冲突 |

注意旧依赖四件套（`ureq 2.6 / rfd 0.11 / base64 0.13 / cocoa+objc`，见 `roadmap.md P1`）已在升级队列，新增 crate 前先查是否与它们重复。

## 5. 落地状态与后续调研

NodeCompat 的 15 个浏览器模块、Rust `nodeCompat` 配置和服务器资源/HTML 注入、Devtools 的模块资源选择已实现。上述阶段表已被当前实现取代，不再作为未开始的交付计划。剩余差距应按实际用户需求逐项决策；本调研频率不能单独决定新增 bridge 能力、原生依赖或公开 API。

## 6. Sync API：既定同步 XHR 方案与历史细节

**2026-09-24 用户确认：同步调用沿用同步 XHR，属于既定方案，同步机制按低难度评估。** Sync API 的补齐成本主要是具体原生能力、接口接线及返回/错误映射；最新评级见 [逐项盘点](node-api-coverage.md)。

以下 §6.1–§6.4 保留此前技术方案及白名单、超时、限额等细节。窄版/全量白名单的历史差异不再作为“同步方案未决”的依据；具体覆盖范围仍按模块/API 清单选择。当前盘点分支的 NodeCompat 未导出所列同步文件 API，不能仅凭方案已确定就把这些接口计为已实现。

历史提案曾讨论同步 XHR 与打包期改写两条技术路线：

### 6.1 同步 XHR 打 loopback（~20 行）

`http_server` 加 `POST /__niva_sync`（token 鉴权同 WS，JSON body `{token, method, args}`），
Rust 侧同步执行完再回包；JS 侧 `new XMLHttpRequest().open(..., false)`。
主线程可用、实现最快，但卡 UI 且 Chrome 已废弃主线程同步 XHR（有 warning，随时可砍）。
**传输只用 POST**（全量 sync 含写操作与大参数，不做 GET/POST 双轨）。

现状核实（2026-09）：WebView2（Chromium 内核）主线程同步 XHR 仍可用，仅 deprecation warning；
真正禁掉的只有页面卸载时（`beforeunload/unload/pagehide`，Chrome 80+）和设置了 `timeout!=0` / `responseType` 时（抛 `InvalidAccessError`）。
WKWebView（macOS）主线程同步 XHR 同样仍可用，但有两坑：耗时长的同步请求可能被 abort（约 10s 量级，有案例）、默认网络超时 60s。
本仓 server 目前只实现 GET（`handle_conn` 非 GET 直接 405），实现 sync 需补
Content-Length body 解析（约 30 行）。
运行时加特征探测：启动时对 ping 端点试一次同步请求，抛错则禁用 sync 垫片并报可读错误（引导用异步）。

### 6.2 打包期 asyncify（零运行时，互补）

devtools 打包时用 codemod 把异步上下文中的 `readFileSync(p)` 改写为 `await readFile(p)`。
构造器、顶层同步代码改不了，只能做辅助。

历史提案（未决）：过渡同步**全量白名单**（读＋写＋exec，见 §6.4），
每次调用 warning + 60s 硬超时 + 4MiB caps，用"疼"推用户迁异步。

### 6.4 过渡兼容方案（全量 sync + 60s 超时 + 每次 warning）

原则：先让用户跑起来，再推他改。每一次 sync 调用都 `console.warn`（spam 是故意的过渡压力，不去重）；
超时/超限/不支持三类错误文案里都写清“不要用 Sync API + 给异步替代名”。

**超时**：所有 sync 调用统一 60s deadline。服务端内部 55s 提前掐并回包（避开 WebKit 默认 60s
网络超时的竞态），JS 侧按超时错误直接抛。文案模板：
`[niva] <name> timed out after 60s. Do not use Sync APIs — use <asyncAlt> instead.`

**传输**：只用 `POST /__niva_sync`（JSON body `{token, method, args}`；
server 需补 Content-Length body 解析，约 30 行；不做 GET/POST 双轨）。

**白名单（全量，除流式）**

- 读：`fs.existsSync/statSync/readDirSync/readFileSync`（读 cap 4MiB，超了报错引导异步；
  `stat` 形状与异步版逐字段一致）。
- 写：`fs.writeFileSync/appendFileSync/mkdirSync/mkdirAllSync/rmSync/renameSync`
  （单次写 cap 4MiB；`stat` 形状与异步版逐字段一致）。
- 执行：`process.execSync`（`std::process::Command` + 轮询 `try_wait`，
  55s 到则 `kill` 并报超时；stdout/stderr 各 cap 4MiB，超了报错引导异步；
  `detached:true` 拒绝同步——语义本就异步，直接报错引导）。
- 不开：stream 系（本就是流式，无同步语义）；
  白名单外 method 一律 `code -1 "sync api not found"` + message 写清仅服务白名单。

**JS（`assets/initialize_script.js`）**

- 新增 `Niva.callSync(method, args)`（只 POST；`timeout` 保持 0、
  `responseType` 不碰）。
- `warnSync` 每次调用都 warn（不去重），文案统一：
  `[niva] fs.readFileSync is transitional and blocks the window. Use Niva.api.fs.read instead.`
- 垫片经 `overrideApi` 挂读四件套 + 写六件套 + `execSync`，参数顺序与异步版一致。
- 启动特征探测保留：ping 失败则 sync 垫片直接抛可读错误（引导异步）。

**类型（`packages/types/Niva_zh.d.ts`）**

- 白名单 sync 全加声明，逐个标 `@deprecated 过渡 API（阻塞窗口＋每次 warning＋60s 超时），新代码用异步版`。

**明确不做**：GET 传输、大文件分片。

**验证**：写侧 round-trip（write→read 对比一致）、`execSync sleep 70` 必超时报错、
`execSync` 超大输出报错、console 里每次调用都有 warning。

## 7. 来源
- `https://nodejs.org/api/documentation.html`（模块全表 + Stability）
- `https://nodejs.org/api/path.html`、`.../fs.html`、`.../os.html`（本轮核对签名的三页）
- `https://flaviocopes.com/node-core-modules/`、`https://www.geeksforgeeks.org/node-js/node-js-core-modules/`、`https://www.w3schools.com/nodejs/ref_modules.asp`（教程共识）
- `https://survey.stackoverflow.co/2025/technology`（运行时层面，无 builtin 下钻）
- `https://socket.dev/blog/asyncapi-supply-chain-attack`、`https://socket.dev/blog/node-ipc-package-compromised`（`fs/child_process/path/https` 高频佐证）
- `https://arxiv.org/pdf/1709.04638`、`https://www.software-lab.org/publications/npm_study_arXiv_1902.09217.pdf`（npm 生态实证方法先例）
