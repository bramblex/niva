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
  current_dir?: string;
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
}>;
```

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
