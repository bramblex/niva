# 文件系统 fs

## Niva.api.fs.stat
```ts
/**
 * 返回文件的元数据信息。
 * @param path 要获取元数据的文件路径。
 * @returns 一个 Promise，在获取元数据成功时解析该 Promise 以返回表示文件元数据的对象，或在发生错误时拒绝该 Promise。
 */
export function stat(path: string): Promise<{
    isDir: boolean;
    isFile: boolean;
    isSymlink: boolean;
    size: number;
    modified: number;
    accessed: number;
    created: number;
}>;
```

## Niva.api.fs.exists
```ts
/**
 * 检查文件或目录是否存在。
 * @param path 要检查的文件或目录路径。
 * @returns 一个 Promise，在检查文件或目录是否存在时解析该 Promise 以返回一个 boolean 值，表示文件或目录是否存在。
 */
export function exists(path: string): Promise<boolean>;
```

## Niva.api.fs.read
```ts
/**
 * 读取文件的内容，并将其作为字符串返回。
 * @param path 要读取的文件路径。
 * @param encode 要使用的编码格式。默认为 UTF-8。
 * @returns 一个 Promise，在读取文件成功时解析该 Promise 以返回文件的内容字符串，或在发生错误时拒绝该 Promise。
 */
export function read(path: string, encode?: 'utf8' | 'base64'): Promise<string>;
```

## Niva.api.fs.write
```ts
/**
 * 将字符串写入文件。
 * @param path 要写入的文件路径。
 * @param content 要写入文件的字符串。
 * @param encode 要使用的编码格式。默认为 UTF-8。
 * @returns 一个 Promise，在写入文件成功时解析该 Promise，或在发生错误时拒绝该 Promise。
 */
export function write(path: string, content: string, encode?: 'utf8' | 'base64'): Promise<void>;
```

## Niva.api.fs.append
```ts
/**
 * 将字符串追加到文件的末尾。
 * @param path 要追加的文件路径。
 * @param content 要追加到文件的字符串。
 * @param encode 要使用的编码格式。默认为 UTF-8。
 * @returns 一个 Promise，在追加字符串到文件成功时解析该 Promise，或在发生错误时拒绝该 Promise。
 */
export function append(path: string, content: string, encode?: 'utf8' | 'base64'): Promise<void>;
```
## Niva.api.fs.move
```ts

/**
 * 将文件或目录移动到新位置。
 * @param from 要移动的文件或目录的路径。
 * @param to 新位置的路径。
 * @param options 可选的参数对象，表示可选的复制选项。
 * @returns 一个 Promise，在移动文件或目录成功时解析该 Promise，或在发生错误时拒绝该 Promise。
 */
export function move(from: string, to: string, options?: {
    overwrite?: boolean;
    skipExist?: boolean;
    bufferSize?: number;
    copyInside?: boolean;
    contentOnly?: boolean;
    depth?: number;
}): Promise<void>;
```
## Niva.api.fs.copy
```ts

/**
 * 将文件或目录复制到新位置。
 * @param from 要复制的文件或目录的路径。
 * @param to 新位置的路径。
 * @param options 可选的参数对象，表示可选的复制选项。
 * @returns 一个 Promise，在复制文件或目录成功时解析该 Promise，或在发生错误时拒绝该 Promise。
 */
export function copy(from: string, to: string, options?: {
    overwrite?: boolean;
    skipExist?: boolean;
    bufferSize?: number;
    copyInside?: boolean;
    contentOnly?: boolean;
    depth?: number;
}): Promise<void>;
```
## Niva.api.fs.remove
```ts

/**
 * 删除文件或目录。
 * @param path 要删除的文件或目录的路径。
 * @returns 一个 Promise，在删除文件或目录时解析该 Promise，或在发生错误时拒绝该 Promise。
 */
export function remove(path: string): Promise<void>;
```
## Niva.api.fs.createDir
```ts
/**
 * 创建一个新目录。
 * @param path 要创建的目录路径。
 * @returns 一个 Promise，在创建目录成功时解析该 Promise，或在发生错误时拒绝该 Promise。
 */
export function createDir(path: string): Promise<void>;
```
## Niva.api.fs.createDirAll
```ts
/**
 * 创建指定的目录及其所有父目录。
 * @param path 要创建的目录路径。
 * @returns 一个 Promise，在创建目录成功时解析该 Promise，或在发生错误时拒绝该 Promise。
 */
export function createDirAll(path: string): Promise<void>;
```
## Niva.api.fs.readDir
```ts
/**
 * 读取指定目录的内容，并返回目录中的所有文件和子目录的名称。
 * @param path 要读取的目录路径。默认值为当前工作目录。
 * @returns 一个 Promise，在读取目录成功时解析该 Promise 以返回目录中的文件和子目录名称组成的字符串数组，或在发生错误时拒绝该 Promise。
 */
export function readDir(path?: string): Promise<string[]>;
```
## Niva.api.fs.readDirAll
```ts
/**
 * 读取指定目录（包括子目录）的内容，并返回目录中的所有文件的相对路径（相对于所提供的目录）。
 * @param path 要读取的目录路径。
 * @param excludes 一个字符串数组，包含要排除的文件路径的 glob 模式。默认为空数组。
 * @returns 一个 Promise，在读取目录中的所有文件成功时解析该 Promise 以返回所有文件的相对路径组成的字符串数组，或在发生错误时拒绝该 Promise。
 */
export function readDirAll(path: string, excludes?: string[]): Promise<string[]>;
```