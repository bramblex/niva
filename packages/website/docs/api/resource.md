# 资源 resource

## Niva.api.resource.exists
```ts
/**
 * 检查文件或文件夹是否存在于应用程序资源中。
 * @param path 要检查的文件或文件夹路径。
 * @returns 一个 Promise，在检查成功时解析该 Promise，或在发生错误时拒绝该 Promise。成功时返回布尔值，表示该文件或文件夹是否存在。
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
