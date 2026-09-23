# 资源 resource

## Niva.api.resource.exists
```ts
/**
 * 检查路径是否对应一个已打包的应用程序资源文件。
 * @param path 要检查的应用程序资源文件路径。
 * @returns 一个 Promise，在检查成功时解析该 Promise，或在发生错误时拒绝该 Promise。成功时返回布尔值，表示该资源文件是否存在。
 */
export function exists(path: string): Promise<boolean>;
```

## Niva.api.resource.read
```ts
/**
 * 读取虚拟文件系统中的文件。
 * @param path 要读取的文件路径。
 * @param encode 要使用的字符编码，目前支持 "utf8" 和 "base64" 两种编码方式。
 * @returns 一个 Promise，在读取文件成功时解析该 Promise，或在发生错误时拒绝该 Promise。成功时返回读取的文件内容。
 */
export function read(path: string, encode?: "utf8" | "base64"): Promise<string>;
```

## Niva.api.resource.extract
```ts
/**
 * 将虚拟文件系统中的文件提取到本地文件系统上。
 * @param from 要提取的虚拟文件系统中的文件路径。
 * @param to 提取文件的本地文件系统路径。
 * @returns 一个 Promise，在提取文件成功时解析该 Promise，或在发生错误时拒绝该 Promise。
 */
export function extract(from: string, to: string): Promise<void>;
```

## 流式读取

`resource.readStream` 是原生二进制流式 handler，只能通过 `Niva.stream` 读取打包资源；没有 `Niva.api.resource.readStream` Promise proxy：

```ts
const read = Niva.stream("resource.readStream", ["assets/data.bin"], {
  onChunk(bytes) {
    console.log("read bytes:", bytes.byteLength);
  },
});
await read.promise;
```

`resource.exists` 只检查打包索引中的文件；目录本身不会单独写入索引。`Niva.api.resource.read` 是初始化脚本建立在 `readStream` 上的 Promise wrapper，会收集整个响应；它仅在本地 WebSocket 页面可用。远端 IPC 页面不能调用 `resource.readStream` 或该 wrapper。另见[流式调用](./stream)。
