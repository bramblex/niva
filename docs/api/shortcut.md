
# 全局快捷键 shortcut

## Niva.api.shortcut.register
```ts
/**
 * 注册一个新的窗口快捷键。
 * @param accelerator_str 快捷键的键序列，如 "Ctrl+N" 或 "Shift+Enter"。
 * @param window_id 要注册窗口快捷键的窗口 ID，默认为当前活动窗口 ID。
 * @returns 一个 Promise，在注册成功时解析该 Promise，或在发生错误时拒绝该 Promise。成功时返回新增的快捷键 ID。
 */
export function register(accelerator_str: string, window_id?: number): Promise<number>;
```

## Niva.api.shortcut.unregister
```ts
/**
 * 注销指定的窗口快捷键。
 * @param id 要注销的快捷键 ID。
 * @param window_id 要注销窗口快捷键的窗口 ID，默认为当前活动窗口 ID。
 * @returns 一个 Promise，在注销成功时解析该 Promise，或在发生错误时拒绝该 Promise。
 */
export function unregister(id: number, window_id?: number): Promise<void>;
```

## Niva.api.shortcut.unregisterAll
```ts
/**
 * 注销指定窗口的所有快捷键。
 * @param window_id 要注销窗口快捷键的窗口 ID，默认为当前活动窗口 ID。
 * @returns 一个 Promise，在注销成功时解析该 Promise，或在发生错误时拒绝该 Promise。
 */
export function unregisterAll(window_id?: number): Promise<void>;
```

## Niva.api.shortcut.list
```ts
/**
 * 获取指定窗口的所有快捷键列表。
 * @param window_id 要获取快捷键列表的窗口 ID，默认为当前活动窗口 ID。
 * @returns 一个 Promise，在获取成功时解析该 Promise，或在发生错误时拒绝该 Promise。成功时返回快捷键列表，列表中的每个元素包含快捷键 ID 和快捷键键序列。
 */
export function list(window_id?: number): Promise<{ id: number, accelerator: string }[]>;
```