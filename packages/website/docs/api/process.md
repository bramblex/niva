# 进程 process

## Niva.api.process.pid
```ts
/**
 * 获取当前进程的进程 ID。
 * @returns 一个 Promise，在获取进程 ID 成功时解析该 Promise，或在发生错误时拒绝该 Promise。成功时返回进程 ID。
 */
export function pid(): Promise<number>;
```

## Niva.api.process.currentDir
```ts
/**
 * 获取当前工作目录。
 * @returns 一个 Promise，在获取当前工作目录成功时解析该 Promise，或在发生错误时拒绝该 Promise。成功时返回当前工作目录的路径。
 */
export function currentDir(): Promise<string>;
```

## Niva.api.process.currentExe
```ts
/**
 * 获取当前可执行文件的路径。
 * @returns 一个 Promise，在获取当前可执行文件的路径成功时解析该 Promise，或在发生错误时拒绝该 Promise。成功时返回当前可执行文件的路径。
 */
export function currentExe(): Promise<string>;
```

## Niva.api.process.env
```ts
/**
 * 获取系统环境变量。
 * @returns 一个 Promise，在获取系统环境变量成功时解析该 Promise，或在发生错误时拒绝该 Promise。成功时返回一个表示系统环境变量的对象。
 */
export function env(): Promise<Record<string, string>>;
```

## Niva.api.process.args
```ts
/**
 * 获取命令行参数。
 * @returns 一个 Promise，在获取命令行参数成功时解析该 Promise，或在发生错误时拒绝该 Promise。成功时返回一个表示命令行参数的数组。
 */
export function args(): Promise<string[]>;
```

## Niva.api.process.setCurrentDir
```ts
/**
 * 设置当前工作目录。
 * @param path 要设置的新的工作目录路径。
 * @returns 一个 Promise，在设置当前工作目录成功时解析该 Promise，或在发生错误时拒绝该 Promise。
 */
export function setCurrentDir(path: string): Promise<void>;
```

## Niva.api.process.exit
```ts
/**
 * 退出 Niva 程序。
 * @returns 一个 Promise，在退出程序成功时解析该 Promise，或在发生错误时拒绝该 Promise。
 */
export function exit(): Promise<void>;
```

## Niva.api.process.exec
```ts
interface ExecOptions {
  env?: Record<string, string>;
  currentDir?: string;
  detached?: boolean;
}
/**
 * 在子进程中执行指定的命令。
 * @param cmd 要执行的命令。
 * @param args 命令的参数。
 * @param options 执行命令的选项。
 * @returns 一个 Promise，在执行命令成功时解析该 Promise，或在发生错误时拒绝该 Promise。成功时返回一个表示命令执行结果的对象。
 */
export function exec(
  cmd: string,
  args?: string[],
  options?: ExecOptions
): Promise<{
  status: number | null;
  stdout: string;
  stderr: string;
} | number>;
```

`exec` 是初始化脚本在本地 WebSocket 模式下提供的 JS wrapper，会收集 stdout/stderr，适合输出较小的命令。远端 IPC 没有 `process.exec` 原生注册项。需要持续输出或向 stdin
写入数据时，用[流式调用](./stream)中的 `process.execStream`。普通执行在所属 bridge 调用取消、项目配置的 `api.timeoutMs` 截止时间到达（默认 30 秒）或连接关闭后会终止并回收子进程；此 wrapper 没有单独的 `timeout` 选项。`detached: true` 明确让子进程独立运行。

`ExecOptions.currentDir` 设置子进程工作目录。

## 通过 Niva.stream 调用 `process.execStream`

`process.execStream` 是原生流式 handler，只能通过本地 WebSocket 的 `Niva.stream("process.execStream", args, handlers)` 调用。它不是 `Niva.api.process` 下的方法。普通模式的终局结果为 `{ status: number | null }`；若传入 `detached: true`，结果是数字 PID，且不会返回 stdout/stderr 流。stdin 数据通过 `Niva.streamSend` 写入：

```ts
const decoders = [new TextDecoder(), new TextDecoder()];
const child = Niva.stream(
  "process.execStream",
  ["python3", ["-c", "import sys; print(sys.stdin.readline().strip())"]],
  {
    onChunk(bytes, isStderr) {
      const decoder = decoders[isStderr ? 1 : 0];
      const text = decoder.decode(bytes, { stream: true });
      if (text) console.log(isStderr ? "stderr:" : "stdout:", text);
    },
    onBlob(_blob, isStderr) {
      // Flush a possible partial UTF-8 sequence at the end of this pipe.
      const tail = decoders[isStderr ? 1 : 0].decode();
      if (tail) console.log(isStderr ? "stderr:" : "stdout:", tail);
    },
  },
);

Niva.streamSend(child.id, "hello\n");
Niva.streamSend(child.id, new Uint8Array(0), true); // 关闭 stdin
const result = await child.promise;
if (typeof result === "number") {
  console.log("detached child PID:", result);
} else {
  console.log("exit status:", result.status);
}
```

`onChunk` 可逐帧实时处理 stdout/stderr；`onBlob` 在收到对应子流的 END 帧后收到完整 Blob。stdout 与 stderr 各自分组，因此不能从两个 pipe 的回调顺序推断 OS 层精确输出先后。取消 stream、丢失所属 bridge 连接或关闭窗口会终止并回收普通子进程。`detached: true` 子进程不会由该调用管理。

## Niva.api.process.open
```ts
/**
 * 打开指定的 URI。
 * @param uri 要打开的 URI。
 * @returns 一个 Promise，在打开 URI 成功时解析该 Promise，或在发生错误时拒绝该 Promise。
 */
export function open(uri: string): Promise<void>;
```

## Niva.api.process.version
```ts
/**
 * 获取当前 Niva 程序的版本号。
 * @returns 一个 Promise，在获取 Niva 程序的版本号成功时解析该 Promise，或在发生错误时拒绝该 Promise。成功时返回 Niva 程序的版本号。
 */
export function version(): Promise<string>;
```
