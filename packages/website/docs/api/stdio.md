---
sidebar_position: 4
---

# stdio 宿主桥

使用 `--stdio` 启动 Niva 时，父进程可通过 stdin/stdout 与应用主窗口交换 NDJSON 消息。它与页面的 WebSocket bridge 分开工作：stdio 是宿主与 Niva 进程间的协议，`Niva.api.host` 是页面到宿主的消息入口。

## 启动与页面 API

```sh
niva --stdio
```

页面通过 `Niva.api.host.send(name, data?)` 向父进程发送消息；只有主窗口（window id `0`）可以使用此 API。接收父进程消息时监听 `host:message`：

```ts
Niva.addEventListener("host:message", (_eventName, message) => {
  if (message.name === "hello") {
    void Niva.api.host.send("hello:result", { text: "Hello from Niva" });
  }
});

// 页面业务监听器就绪后再通知宿主，避免首条请求早于页面监听器。
await Niva.api.host.send("page:ready");
```

`host:message` 的 payload 形状是 `{ name: string; data?: unknown }`。`host.send` Promise 成功表示消息已进入 Niva 的有界输出队列，并不表示父进程已确认或处理。输出队列很小；连续发送时应 `await` 前一条 `host.send`，并处理队列已满时的 reject。需要请求/响应关联时，由业务双方在 `data` 里自行携带 request ID；stdio v1 本身不提供 RPC 或相关性字段。

## NDJSON 帧

每帧是一行 UTF-8 JSON，使用 `\n` 分隔，读取时兼容 `\r\n`：

```jsonl
{"t":"ready","v":1}
{"t":"msg","name":"hello","data":{"name":"Ada"}}
```

帧方向如下：

| 方向 | 帧 | 含义 |
| --- | --- | --- |
| Niva → 父进程 | `{"t":"ready","v":1}` | 主窗口 WebSocket hello 成功后发送一次，表示 bridge 已可用。 |
| 父进程 → Niva | `{"t":"msg","name":"...","data":...}` | 投递为主窗口的 `host:message` 事件。 |
| Niva → 父进程 | `{"t":"msg","name":"...","data":...}` | 由主窗口调用 `Niva.api.host.send()` 发送。 |

`ready` 不保证页面业务监听器已经安装。父进程通常应先等待 `ready`，再等待应用发来的 `page:ready` 业务消息，随后才发送第一条业务请求。样例宿主位于仓库 `examples/stdio_host.py`。

## 限制

- 只有主窗口可以收发宿主消息；子窗口调用 `host.send` 会拒绝。
- 输入输出一行最大 64 MiB。超长、非 UTF-8、非法 JSON 或未知帧会写入 stderr 并丢弃；不会作为应用消息转发。
- 二进制需由应用自行编码为 JSON 可承载的数据，例如 base64；stdio v1 不提供二进制分片。
- stdout 保留给 `ready` 和 `msg` NDJSON 帧，Niva 运行日志写到 stderr。stdin EOF 或 stdout 写失败会请求应用退出。
- 父进程是该管道的信任主体；stdio 协议没有额外认证。

stdio 消息协议与[流式 API](./stream)是不同通道。`process.execStream` 是 Niva 启动其他进程时使用的子进程流，与 `--stdio` 宿主桥相互独立。
