
# 系统额外 extra

## Niva.api.extra.hideApplication
```ts
/**
 * 隐藏应用程序，仅适用于 macOS。
 * @returns 一个 Promise，在应用程序成功隐藏时解析该 Promise，如果发生错误则拒绝该 Promise。
 */
export function hideApplication(): Promise<void>;
```

## Niva.api.extra.showApplication
```ts
/**
 * 显示应用程序，仅适用于 macOS。
 * @returns 一个 Promise，在应用程序成功显示时解析该 Promise，如果发生错误则拒绝该 Promise。
 */
export function showApplication(): Promise<void>;
```

## Niva.api.extra.hideOtherApplication
```ts
/**
 * 隐藏其他应用程序，仅适用于 macOS。
 * @returns 一个 Promise，在其他应用程序成功隐藏时解析该 Promise，如果发生错误则拒绝该 Promise。
 */
export function hideOtherApplication(): Promise<void>;
```

## Niva.api.extra.setActivationPolicy
```ts
/**
 * 设置应用程序的激活策略，仅适用于 macOS。
 * @param policy 要设置的激活策略。
 * @returns 一个 Promise，在激活策略成功设置时解析该 Promise，如果发生错误则拒绝该 Promise。
 */
export function setActivationPolicy(policy: 'regular' | 'accessory' | 'prohibited'): Promise<void>;
```

## Niva.api.extra.getActiveWindowId
```ts
/**
 * 获取当前活动窗口的 ID，仅适用于 macOS 和 Windows。
 * 对于 macOS，将返回 `process_id_window_id` 的格式，其中 `process_id` 和 `window_id` 为整数。
 * 对于 Windows，将返回窗口句柄的字符串形式。
 * @returns 一个 Promise，在获取成功时解析该 Promise 以返回当前活动窗口的 ID，或解析 `null`（如果没有活动窗口）。
 */
export function getActiveWindowId(): Promise<string | null>;
```

## Niva.api.extra.focusByWindowId
```ts
/**
 * 将焦点设置到特定 ID 的窗口，仅适用于 macOS 和 Windows。
 * 对于 macOS，ID 应该是 `process_id_window_id` 的格式，其中 `process_id` 和 `window_id` 为整数。
 * 对于 Windows，ID 应该是窗口句柄的字符串形式。
 * @param id_string 要设置焦点窗口的 ID 字符串。
 * @returns 一个 Promise，在设置焦点窗口时解析该 Promise，如果无法设置活动窗口则解析 `true`，如果窗口 ID 无效则解析 `false`，如果发生其他错误则拒绝该 Promise。
 */
export function focusByWindowId(id_string: string): Promise<boolean>;
```