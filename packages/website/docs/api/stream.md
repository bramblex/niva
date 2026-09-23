---
sidebar_position: 3
---

# 流式调用与二进制数据

`Niva.call(method, args)` 用于返回一个 Promise 的 unary JSON 调用。需要处理持续二进制输出或向运行中的子进程写入 stdin 时，使用 `Niva.stream`。流式调用只适用于持有本地 WebSocket bridge 的页面；来源获授权的远端 IPC 页面也只能调用非流式 JSON API。

## 接收数据

`onChunk` 在收到每个非空二进制 WebSocket 帧时同步回调，参数为该帧的 `Uint8Array` 和 `isStderr` 标志。字节数组是副本，修改它不会改写用于组装 `onBlob` 的内容。适合边到达边处理的数据使用此回调：

```ts
const decoders = [new TextDecoder(), new TextDecoder()];
const task = Niva.stream(
  "process.execStream",
  ["python3", ["-c", "import sys; print('hello'); print('notice', file=sys.stderr)"]],
  {
    onChunk(chunk, isStderr) {
      const decoder = decoders[isStderr ? 1 : 0];
      const text = decoder.decode(chunk, { stream: true });
      if (text) console.log(isStderr ? "stderr:" : "stdout:", text);
    },
  },
);
const result = await task.promise; // { status: 0 }
```

每个二进制子流结束时，`onBlob(blob, isStderr)` 会收到按序拼好的完整 Blob。它适合完整文件或响应体；进程 pipe 的 Blob 要等对应 pipe 到达 END。`onChunk` 与 `onBlob` 可同时提供：前者逐帧回调，后者仍会在 END 后回调完整内容。stdout 与 stderr 是独立子流，跨子流回调顺序只表示帧到达顺序，不代表两个 OS pipe 产生输出的精确先后。

`onEvent(name, data)` 用于调用关联的 JSON 事件。例如 `http.requestStream` 会先发 `head` 事件，随后传响应体二进制数据：

```ts
const request = Niva.stream(
  "http.requestStream",
  [{ method: "GET", url: "https://example.com/" }],
  {
    onEvent(name, data) {
      if (name === "head") console.log(data.status, data.headers);
    },
    onChunk(bytes) {
      console.log("received response bytes:", bytes.byteLength);
    },
  },
);
const result = await request.promise; // { status: number }
```

## 发送数据

`Niva.streamSend(id, data, end?)` 向某个本地流式调用发送 `ArrayBuffer`、`Uint8Array` 或字符串；字符串按 UTF-8 编码。`end: true` 结束输入子流：

```ts
const write = Niva.stream("fs.writeStream", ["output.bin"]);
Niva.streamSend(write.id, new Uint8Array([1, 2, 3]), true);
const result = await write.promise; // { bytes: 3 }
```

进程 stdin 可在进程运行期间多次发送，再用 `end: true` 关闭 stdin：

```ts
const child = Niva.stream("process.execStream", [
  "python3",
  ["-c", "import sys; print(sys.stdin.readline().strip())"],
]);
Niva.streamSend(child.id, "hello\n");
Niva.streamSend(child.id, new Uint8Array(0), true);
await child.promise;
```

## 调用生命周期与范围

`Niva.stream` 返回 `{ id, promise, cancel }`。取消会向原生端发出 cancel；断开该 frame 的 WebSocket 或关闭所属窗口也会清理相应调用。取消后不保证 Promise 收到终局结果。普通 `process.execStream` 子进程会在取消、断连或关窗时终止并回收；`detached: true` 除外。

原生流式方法：

- `fs.readStream`、`fs.writeStream`
- `resource.readStream`
- `http.requestStream`
- `process.execStream`

`Niva.api.fs.read/write/append`、`http.get/post/request`、`process.exec` 和 `resource.read` 是初始化脚本提供的 Promise wrapper，会组装完整数据。这些 wrapper 需要本地 WebSocket 页面；远端 IPC 没有相应的 unary Rust 方法。大文件或持续处理时应直接使用 stream API。详见[Bridge 与传输方式](./bridge)、[文件系统](./fs)、[HTTP](./http)和[进程](./process)。
