# 剪切板 clipboard

## Niva.api.clipboard.read
```ts
/**
 * 从系统剪贴板中读取当前所复制的文本内容。
 * 如果当前没有复制的文本内容，则返回 `null`。
 * @returns 一个 Promise，在 Promise 被解析时返回文本内容，或返回 `null`。
 */
export function read(): Promise<string | null>;
```

## Niva.api.clipboard.write

```ts
/**
 * 将给定的文本内容写入系统剪贴板，替换任何之前复制的文本内容。
 * @param text 要写入剪贴板的文本。
 * @returns 一个 Promise，在文本写入剪贴板成功时解析该 Promise，如果发生错误则拒绝该 Promise。
 */
export function write(text: string): Promise<void>;
```
