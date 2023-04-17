# 消息框 dialog

## Niva.api.dialog.showMessage
```ts
/**
 * 显示一个独立消息框。
 * @param title 消息框的标题。
 * @param content 消息框的内容，如果为空，则使用默认值。
 * @param level 消息框的级别。
 * @returns 一个 Promise，在消息框关闭时解析该 Promise，或在发生错误时拒绝该 Promise。
 */
export function showMessage(title: string, content?: string, level?: 'info' | 'warning' | 'error'): Promise<void>;
```

## Niva.api.dialog.pickFile
```ts
/**
 * 在文件系统中选择一个文件，支持过滤器和起始目录。
 * @param filters 文件类型筛选器。
 * @param start_dir 文件选择对话框的起始目录。
 * @returns 一个 Promise，在选择文件时解析该 Promise 以返回文件名或文件路径，或解析 `null`（如果没有选择文件）。
 */
export function pickFile(filters?: string[], start_dir?: string): Promise<string | null>;
```

## Niva.api.dialog.pickFiles
```ts
/**
 * 在文件系统中选择多个文件，支持过滤器和起始目录。
 * @param filters 文件类型筛选器。
 * @param start_dir 文件选择对话框的起始目录。
 * @returns 一个 Promise，在选择文件时解析该 Promise 以返回文件名数组或文件路径数组，或解析 `null`（如果没有选择文件）。
 */
export function pickFiles(filters?: string[], start_dir?: string): Promise<string[] | null>;
```

## Niva.api.dialog.pickDir
```ts
/**
 * 在文件系统中选择一个文件夹，支持起始目录。
 * @param start_dir 文件夹选择对话框的起始目录。
 * @returns 一个 Promise，在选择文件夹时解析该 Promise 以返回文件夹路径，或解析 `null`（如果没有选择文件夹）。
 */
export function pickDir(start_dir?: string): Promise<string | null>;
```

## Niva.api.dialog.pickDirs
```ts
/**
 * 在文件系统中选择多个文件夹，支持起始目录。
 * @param start_dir 文件夹选择对话框的起始目录。
 * @returns 一个 Promise，在选择文件夹时解析该 Promise 以返回文件夹路径数组，或解析 `null`（如果没有选择文件夹）。
 */
export function pickDirs(start_dir?: string): Promise<string[] | null>;

```

## Niva.api.dialog.saveFile
```ts
/**
 * 在文件系统中保存一个文件，支持过滤器和起始目录。
 * @param filters 文件类型筛选器。
 * @param start_dir 文件保存对话框的起始目录。
 * @returns 一个 Promise，在保存文件时解析该 Promise 以返回文件名或文件路径，或解析 `null`（如果没有保存文件）。
 */
export function saveFile(filters?: string[], start_dir?: string): Promise<string | null>;
```
