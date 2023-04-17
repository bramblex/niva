---
sidebar_position: 1
---
# Niva

## Niva.addEventListener
```ts
/**
 * 绑定事件监听器。
 * @param event 要监听的事件名称，可以用 `*`、`xxxx.*` 等通配符。
 * @param listener 事件被触发时要调用的函数。
 */
function addEventListener(event: string, listener: Function): void;
```

## Niva.removeEventListener
```ts
/**
 * 移除特定的事件监听器。
 * @param event 要移除的事件名称。
 * @param listener 要移除的监听器函数。
 */
function removeEventListener(event: string, listener: Function): void;
```

## Niva.removeAllEventListeners
```ts
/**
 * 移除特定事件的所有监听器。
 * @param event 要移除所有监听器的事件名称。
 */
function removeAllEventListeners(event: string): void;
```

## Niva.call
```ts
/**
 * 调用远程 IPC 方法，并返回一个 Promise，用于异步获取该方法返回值或错误。
 * @param method 要调用的远程 IPC 方法名。
 * @param args 要传递给远程 IPC 方法的参数列表。
 * @returns 一个 Promise，在该远程 IPC 方法调用完成后解析该 Promise，或在发生错误时拒绝该 Promise。成功时返回调用方法的返回值，失败时返回一个包含错误信息的对象。
 */
function call(method: string, args: any[]): Promise<any>;
```

## Niva.api[namespace][method]
```ts
/**
 * Niva.api 是一个语法糖，等同于调用  Niva.call，具体用法请参考具体的 API 用法。
 */
function Niva.api[namespace][method](...args: unknown[]): Promise<unknown>
```
