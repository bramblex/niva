# Webview webview

## Niva.api.webview.isDevToolsOpen
```ts
/**
 * 检查开发工具是否打开。
 * @returns 一个 Promise，在检查成功时解析该 Promise，或在发生错误时拒绝该 Promise。成功时返回布尔值，表示开发工具是否打开。
 */
export function isDevToolsOpen(): Promise<boolean>;
```

## Niva.api.webview.openDevTools
```ts
/**
 * 打开开发工具。
 * @returns 一个 Promise，该 Promise 始终解析。
 */
export function openDevTools(): Promise<void>;
```


## Niva.api.webview.closeDevTools
```ts
/**
 * 关闭开发工具。
 * @returns 一个 Promise，该 Promise 始终解析。
 */
export function closeDevTools(): Promise<void>;
```

## Niva.api.webview.baseUrl
```ts
/**
 * 获取应用程序的基本 URL。
 * @returns 一个 Promise，在获取成功时解析该 Promise，或在发生错误时拒绝该 Promise。成功时返回应用程序的基本 URL。
 */
export function baseUrl(): Promise<string>;
```

## Niva.api.webview.baseFileSystemUrl
```ts
/**
 * 获取应用程序文件系统的基本 URL。
 * @returns 一个 Promise，在获取成功时解析该 Promise，或在发生错误时拒绝该 Promise。成功时返回应用程序文件系统的基本 URL。
 */
export function baseFileSystemUrl(): Promise<string>;
```