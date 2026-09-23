# Niva Stdio Host Bridge 设计（niva 当子进程 UI）

> 状态：已实现；Windows 已实测 Python 宿主往返和 EOF 退出，完整边界仍待验收。目标：任意程序 A 启动 `niva.exe --stdio` 把它当 UI 窗口，
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
Niva.addEventListener("host:message", (eventName, msg) => void); // listener 签名为 (eventName, payload)，只投递给 id 0
```

`ready` 表示主窗口的 WebSocket hello 已成功，不代表页面脚本已安装业务监听器。
需要避免页面启动竞态时，由页面先通过 `host.send` 发一条业务就绪消息，宿主收到后再发首条请求。

## 3. 线格式 v1（NDJSON，只三帧）

一行一 JSON，UTF-8，`\n` 分隔（容忍 `\r`）。就三种：

```text
niva → A   {"t":"ready","v":1}                 # 主窗口就绪，一次
niva → A   {"t":"msg","name":string,"data"?:any}
A → niva   {"t":"msg","name":string,"data"?:any}
```

- 二进制放 base64 字段；大文件不走 stdio（走文件/extract）；单行上限 64 MiB（含换行）。
- 不要 `id/result/call`：v1 无请求-响应相关性，需要往返的业务自己在 `data`
  里带 `reqId`（A 与前端约定的业务层字段，管道不理解）。
- 坏行（非 UTF-8、非 JSON、未知帧或超长行）：记 stderr 后丢弃；超长行会读到行尾再继续，进程不崩。

## 4. Rust 实现

1. `--stdio` 显式启用。reader 通过 `smol::unblock` 执行阻塞 stdin 读取，按行解析后只投递到 `get_window(0)`；主窗口不存在或已关闭时丢弃消息。
2. stdout writer 单通道串行写入，每帧后 `flush`。输出队列有界；队列已满或帧超过 64 MiB 时，`host.send` 返回业务错误。
3. `host.send` 在 Rust 侧校验 `window.id == 0`。子窗口调用返回 `code -1` 和 `host.send is main-window only`。
4. **stdout 纯洁性**：`--stdio` 下 `log!` 系宏转到 stderr，普通模式仍输出 stdout；HTTP、WebSocket 等运行日志写 stderr。stdout 只含 `ready` 和 `msg` NDJSON 帧。
5. stdin EOF 或 stdout 写失败时，事件投递回主循环，按 id 0 调用 `close_window_inner + cleanup` 后设置 `ControlFlow::Exit`。在主窗创建前到达的 EOF 会排队，主窗创建后仍走同一关闭路径。
6. Windows：release 使用 `windows_subsystem = "windows"`（无控制台）；2026-09-23 已实测 Python 父进程管道继承、坏帧恢复和 EOF 退出。输出换行统一为 `\n`。
7. 安全：默认信任 stdin 持有者（即启动 Niva 的父进程）；不为终端手输提供额外交互功能。

## 5. 与现有能力的复用/冲突

- `process.execStream` 是 niva 当父进程收发子进程流；新桥是 niva 当子进程，
  对称，reader/writer 照抄思路，但线格式用 NDJSON（跨语言好解析）。
- `runCmd`/`process.exec` 捕获 stdout 的调用方：若目标是 `--stdio` niva，
  需按 NDJSON 解析而非裸文本。`win_packager` 是普通模式，不受影响。

## 6. 示例与验证

Python 父进程和最小 UI 示例位于 `examples/stdio_host.py` 与 `examples/stdio-host/`。构建后执行：

```sh
cargo build -p niva
python examples/stdio_host.py target/debug/niva
```

示例会先等待 `ready`，再等页面发出 `page:ready`，完成 `sayHello` 往返，最后关闭 stdin 并等待主窗口走 EOF 退出路径。Windows 真机已在 2026-09-23 跑通该示例，详情见 [验证记录](windows-validation-2026-09-23.md)；BrokenPipe 和子窗口拒绝仍待专项验证。
