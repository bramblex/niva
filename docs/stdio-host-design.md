# Niva Stdio Host Bridge 设计（niva 当子进程 UI）

> 状态：设计稿，未实现。目标：任意程序 A 启动 `niva.exe --stdio` 把它当 UI 窗口，
> 双方经 stdin/stdout 换 JSON 消息（A 收到 `sayHello` → 回写 `sayHelloResult` → 前端拿到）。
>
> 架构约束（已定）：Niva 是主窗口（id 0）+ 子窗口的多窗口模型，主窗口在主循环、
> 主窗口退出即整个程序退出（`window.rs:115-117`、`event_handler.rs:190-191`，
> 主窗口 ≈ Electron 主线程）。因此管道与 app 级 API 只归 main 窗口，
> 子窗口走原有 WS 桥，不变。

## 1. 结论

逻辑成立。模式有三重先例：LSP（server 当子进程走 stdio）、Chrome Native
Messaging（stdin/stdout 传 JSON）、Chrome DevTools pipe。管道只做最基础的
双向透传，不做 RPC、不做相关性、不做二进制分片——三帧走天下。

## 2. 数据流（只进 main，wid 固定 0）

```text
A（任意语言，父进程）                  niva.exe --stdio（子进程）
  spawn + pipe stdin/stdout
  写 stdin {"t":"msg",...}    ────────►  只投递给主窗口（wid 0）→ 前端 host:message
  读 stdout {"t":"msg",...}   ◄────────  只接受主窗口的 host.send；子窗口调直接报错
  读 stdout {"t":"ready",...} ◄────────  主窗口 hello 成功后发一次，宿主以此确认 UI 就绪
```

前端 API（与 `Niva.call/stream` 并列，不碰 wire v2，见 `docs/bridge.md`）：

```ts
Niva.api.host.send(name: string, data?: unknown): Promise<void>; // 子窗口调 → reject
Niva.addEventListener("host:message", (msg: { name: string; data: unknown }) => void); // 只在 main 收到
```

## 3. 线格式 v1（NDJSON，只三帧）

一行一 JSON，UTF-8，`\n` 分隔（容忍 `\r`）。就三种：

```text
niva → A   {"t":"ready","v":1}                 # 主窗口就绪，一次
niva → A   {"t":"msg","name":string,"data"?:any}
A → niva   {"t":"msg","name":string,"data"?:any}
```

- 二进制放 base64 字段；大文件不走 stdio（走文件/extract）；单行上限 64MB。
- 不要 `id/result/call`：v1 无请求-响应相关性，需要往返的业务自己在 `data`
  里带 `reqId`（A 与前端约定的业务层字段，管道不理解）。
- 坏行（非 UTF-8/非 JSON）：记 stderr，不崩进程（对标 EventHandler
  “错误只打日志不崩循环”）。

## 4. Rust 实现规则（对照现代码）

1. `--stdio` 显式门控（`app/mod.rs NivaArguments` 加键）。非 stdio 模式零变化，
   双击打开的正常应用不受影响。
2. 新模块 `app/stdio.rs`，只两件事：reader 线程（`smol::unblock` 包阻塞
   `read_line`，driver 线程永不直接阻塞）→ 按行解析 → `get_window(0)` 投
   `host:message`（main 已关则丢弃）；writer 单通道串行 + 每次 `flush`
   （stdout 块缓冲，不 flush 宿主会“卡住”）。
3. `host.send` 在 Rust 侧校验 `window.id == 0`，子窗口调直接回业务错
   （`code -1 "host.* is main-window only"`），不在前端拦——后门也得守同一条。
4. **stdout 纯洁性（v1 最大坑）**：`--stdio` 下 stdout 只允许上面三帧。
   现状 `utils.rs` 的 `log!` 系宏用 `println!`，必须全转 `eprintln!` 或门控；
   `[niva] http server listening` 已是 `eprintln!`（✓）。实现前先审计全部
   `println!/print!`。
5. 生命周期复用现有主窗口退出路径，不另起炉灶：stdin EOF → 按 id 0 走
   `close_window_inner + cleanup` → `ControlFlow::Exit` 全程序退出（防孤儿窗口）；
   写 broken pipe → 同样退出。退出码 0 正常，非 0 协议致命错误。
6. Windows：release 已是 `windows_subsystem = "windows"`（无控制台），父进程
   建 pipe 继承句柄理论可用，真机验证；换行统一 `\n`。
7. 安全：默认信任 stdin 持有者（= 拉起它的父进程）；终端手输是 feature。

## 5. 与现有能力的复用/冲突

- `process.execStream` 是 niva 当父进程收发子进程流；新桥是 niva 当子进程，
  对称，reader/writer 照抄思路，但线格式用 NDJSON（跨语言好解析）。
- `runCmd`/`process.exec` 捕获 stdout 的调用方：若目标是 `--stdio` niva，
  需按 NDJSON 解析而非裸文本。`win_packager` 是普通模式，不受影响。

## 6. 落地步骤

1. `println!` 审计 + log 宏转 stderr。
2. `app/stdio.rs`（reader/writer，只认 wid 0）+ `--stdio` 参数 + EOF 走主窗退出。
3. `host` API namespace（含 `id == 0` 校验）+ `packages/types` d.ts。
4. 宿主示例（Python/Node 各 10 行 sayHello 回显，主窗）+ e2e 脚本化。
5. Windows 真机验证 pipe 继承 + EOF 退出。
