# 窗口 window

## Niva.api.window.current

```ts
/**
 * 获取当前窗口的 ID。
 * @returns 一个 Promise，在接收成功响应后返回当前窗口 ID。
 */
export function current(): Promise<number>;
```

## Niva.api.window.open

* 其中窗口选项 `NivaWindowOptions` 详见 [窗口选项](/docs/options/window)。

```ts
/**
 * 打开一个新窗口。
 * @param options 可选的窗口选项，包括宽度、高度、坐标、标题和 URL 等。
 * @returns 一个 Promise，在接收成功响应后返回新窗口的 ID。
 */
export function open(options: NivaWindowOptions): Promise<number>;
```

## Niva.event.window.close

```ts
/**
 * 关闭窗口。
 * @param id 可选的窗口 ID，若不提供，则关闭当前窗口。若 ID 为 0，则退出程序。
 * @returns 一个 Promise，在接收成功响应后表示已关闭窗口，若 ID 为 0 则表示退出程序。
 */
export function close(id?: number): Promise<void>;
```

## Niva.api.window.list

```ts
/**
 * 获取当前应用所有窗口的 ID、标题、是否可见列表。
 * @returns 一个 Promise，在解析成功后返回一个包含所有窗口的 ID、标题、是否可见数据组成的对象列表。
 */
export function list(): Promise<
  { id: number; title: string; visible: boolean }[]
>;
```

## Niva.api.window.sendMessage

```ts
/**
 * 向指定 ID 的窗口发送 IPC 消息。
 * @param message IPC 消息内容。
 * @param id 目标窗口的 ID 号。
 * @returns 一个 Promise，在接收到发送成功回执后解析该 Promise，或在发生错误时拒绝该 Promise。
 */
export function sendMessage(message: string, id: number): Promise<void>;
```

## Niva.api.window.setMenu

```ts
/**
 * 设置窗口菜单。
 * @param options 菜单选项，如果为 undefined 或 null，则移除菜单。
 * @param id 需要设置菜单的窗口 ID，省略则默认为当前窗口。
 * @returns 一个 Promise，在设置成功后解析该 Promise，或在发生错误时拒绝该 Promise。
 */
export function setMenu(
  options?: WindowMenuOptions | null,
  id?: number
): Promise<void>;
```

## Niva.api.window.hideMenu

```ts
/**
 * 隐藏窗口菜单。
 * @param id 需要隐藏菜单的窗口 ID，省略则默认为当前窗口。
 * @returns 一个 Promise，在隐藏成功后解析该 Promise，或在发生错误时拒绝该 Promise。
 */
export function hideMenu(id?: number): Promise<void>;
```

## Niva.api.window.showMenu

```ts
/**
 * 显示窗口菜单。
 * @param id 需要显示菜单的窗口 ID，省略则默认为当前窗口。
 * @returns 一个 Promise，在显示成功后解析该 Promise，或在发生错误时拒绝该 Promise。
 */
export function showMenu(id?: number): Promise<void>;
```

## Niva.api.window.isMenuVisible

```ts
/**
 * 判断窗口菜单是否可见。
 * @param id 窗口 ID，省略则默认为当前窗口。
 * @returns 一个 Promise，在接收到判断结果后解析该 Promise，或在发生错误时拒绝该 Promise。
 */
export function isMenuVisible(id?: number): Promise<boolean>;
```

## Niva.api.window.scaleFactor

```ts
/**
 * 获取窗口缩放因子。
 * @param id 需要获取缩放因子的窗口 ID，省略则默认为当前窗口。
 * @returns 一个 Promise，在解析成功后返回窗口缩放因子。
 */
export function scaleFactor(id?: number): Promise<number>;
```

## Niva.api.window.innerPosition

```ts
/**
 * 获取窗口客户区左上角在屏幕坐标系下的坐标。
 * @param id 需要获取位置的窗口 ID，省略则默认为当前窗口。
 * @returns 一个 Promise，在解析成功后返回窗口客户区左上角在屏幕坐标系下的坐标值。
 */
export function innerPosition(id?: number): Promise<NivaPosition>;
```

## Niva.api.window.outerPosition

```ts
/**
 * 获取窗口左上角在屏幕坐标系下的坐标。
 * @param id 需要获取位置的窗口 ID，省略则默认为当前窗口。
 * @returns 一个 Promise，在解析成功后返回窗口左上角在屏幕坐标系下的坐标值。
 */
export function outerPosition(id?: number): Promise<NivaPosition>;
```

## Niva.api.window.setOuterPosition

```ts
/**
 * 设置窗口左上角在屏幕坐标系下的坐标。
 * @param position 窗口左上角在屏幕坐标系下的坐标值。
 * @param id 需要设置位置的窗口 ID，省略则默认为当前窗口。
 * @returns 一个 Promise，在设置成功后解析该 Promise，或在发生错误时拒绝该 Promise。
 */
export function setOuterPosition(
  position: NivaPosition,
  id?: number
): Promise<void>;
```

## Niva.api.window.innerSize

```ts
/**
 * 获取窗口客户区大小。
 * @param id 需要获取大小的窗口 ID，省略则默认为当前窗口。
 * @returns 一个 Promise，在解析成功后返回窗口客户区大小。
 */
export function innerSize(id?: number): Promise<NivaSize>;
```

## Niva.api.window.setInnerSize

```ts
/**
 * 设置窗口客户区大小。
 * @param size 要设置的窗口客户区大小。
 * @param id 需要设置大小的窗口 ID，省略则默认为当前窗口。
 * @returns 一个 Promise，在设置成功后解析该 Promise，或在发生错误时拒绝该 Promise。
 */
export function setInnerSize(size: NivaSize, id?: number): Promise<void>;
```

## Niva.api.window.outerSize

```ts
/**
 * 获取窗口大小，包括边框和菜单等非客户区部分。
 * @param id 需要获取大小的窗口 ID，省略则默认为当前窗口。
 * @returns 一个 Promise，在解析成功后返回窗口大小，包括边框和菜单等非客户区部分。
 */
export function outerSize(id?: number): Promise<NivaSize>;
```

## Niva.api.window.setMinInnerSize

```ts
/**
 * 设置窗口客户区最小大小。
 * @param size 窗口客户区最小大小。
 * @param id 需要设置大小的窗口 ID，省略则默认为当前窗口。
 * @returns 一个 Promise，在设置成功后解析该 Promise，或在发生错误时拒绝该 Promise。
 */
export function setMinInnerSize(size: NivaSize, id?: number): Promise<void>;
```

## Niva.api.window.setMaxInnerSize

```ts
/**
 * 设置窗口客户区最大大小。
 * @param size 窗口客户区最大大小。
 * @param id 需要设置大小的窗口 ID，省略则默认为当前窗口。
 * @returns 一个 Promise，在设置成功后解析该 Promise，或在发生错误时拒绝该 Promise。
 */
export function setMaxInnerSize(size: NivaSize, id?: number): Promise<void>;
```

## Niva.api.window.setTitle

```ts
/**
 * 设置窗口标题。
 * @param title 窗口标题。
 * @param id 需要设置标题的窗口 ID，省略则默认为当前窗口。
 * @returns 一个 Promise，在设置成功后解析该 Promise，或在发生错误时拒绝该 Promise。
 */
export function setTitle(title: string, id?: number): Promise<void>;
```

## Niva.api.window.title

```ts
/**
 * 获取窗口标题。
 * @param id 需要获取标题的窗口 ID，省略则默认为当前窗口。
 * @returns 一个 Promise，在解析成功后返回窗口标题。
 */
export function title(id?: number): Promise<string>;
```

## Niva.api.window.isVisible

```ts
/**
 * 判断窗口是否可见。
 * @param id 需要判断的窗口 ID，省略则默认为当前窗口。
 * @returns 一个 Promise，在接收到判断结果后解析该 Promise，或在发生错误时拒绝该 Promise。
 */
export function isVisible(id?: number): Promise<boolean>;
```

## Niva.api.window.setVisible

```ts
/**
 * 设置窗口是否可见。
 * @param visible 是否可见。
 * @param id 需要设置的窗口 ID，省略则默认为当前窗口。
 * @returns 一个 Promise，在设置成功后解析该 Promise，或在发生错误时拒绝该 Promise。
 */
export function setVisible(visible: boolean, id?: number): Promise<void>;
```

## Niva.api.window.isFocused

```ts
/**
 * 判断窗口是否聚焦。
 * @param id 需要判断的窗口 ID，省略则默认为当前窗口。
 * @returns 一个 Promise，在接收到判断结果后解析该 Promise，或在发生错误时拒绝该 Promise。
 */
export function isFocused(id?: number): Promise<boolean>;
```

## Niva.api.window.setFocus

```ts
/**
 * 窗口设置为聚焦状态。
 * @param id 需要设置的窗口 ID，省略则默认为当前窗口。
 * @returns 一个 Promise，在设置成功后解析该 Promise，或在发生错误时拒绝该 Promise。
 */
export function setFocus(id?: number): Promise<void>;
```

## Niva.api.window.isResizable

```ts
/**
 * 判断窗口是否可以改变大小。
 * @param id 需要判断的窗口 ID，省略则默认为当前窗口。
 * @returns 一个 Promise，在接收到判断结果后解析该 Promise，或在发生错误时拒绝该 Promise。
 */
export function isResizable(id?: number): Promise<boolean>;
```

## Niva.api.window.setResizable

```ts
/**
 * 设置窗口是否可以改变大小。
 * @param resizable 是否可以改变大小。
 * @param id 需要设置的窗口 ID，省略则默认为当前窗口。
 * @returns 一个 Promise，在设置成功后解析该 Promise，或在发生错误时拒绝该 Promise。
 */
export function setResizable(resizable: boolean, id?: number): Promise<void>;
```

## Niva.api.window.isMinimizable

```ts
/**
 * 判断窗口是否可以最小化。
 * @param id 需要判断的窗口 ID，省略则默认为当前窗口。
 * @returns 一个 Promise，在接收到判断结果后解析该 Promise，或在发生错误时拒绝该 Promise。
 */
export function isMinimizable(id?: number): Promise<boolean>;
```

## Niva.api.window.setMinimizable

```ts
/**
 * 设置窗口是否可以最小化。
 * @param minimizable 是否可以最小化。
 * @param id 需要设置的窗口 ID，省略则默认为当前窗口。
 * @returns 一个 Promise，在设置成功后解析该 Promise，或在发生错误时拒绝该 Promise。
 */
export function setMinimizable(
  minimizable: boolean,
  id?: number
): Promise<void>;
```

## Niva.api.window.isMaximizable

```ts
/**
 * 判断窗口是否可以最大化。
 * @param id 需要判断的窗口 ID，省略则默认为当前窗口。
 * @returns 一个 Promise，在接收到判断结果后解析该 Promise，或在发生错误时拒绝该 Promise。
 */
export function isMaximizable(id?: number): Promise<boolean>;
```

## Niva.api.window.setMaximizable

```ts
/**
 * 设置窗口是否可以最大化。
 * @param maximizable 是否可以最大化。
 * @param id 需要设置的窗口 ID，省略则默认为当前窗口。
 * @returns 一个 Promise，在设置成功后解析该 Promise，或在发生错误时拒绝该 Promise。
 */
export function setMaximizable(
  maximizable: boolean,
  id?: number
): Promise<void>;
```

## Niva.api.window.isClosable

```ts
/**
 * 判断窗口是否可以关闭。
 * @param id 需要判断的窗口 ID，省略则默认为当前窗口。
 * @returns 一个 Promise，在接收到判断结果后解析该 Promise，或在发生错误时拒绝该 Promise。
 */
export function isClosable(id?: number): Promise<boolean>;
```

## Niva.api.window.setClosable

```ts
/**
 * 设置窗口是否可以关闭。
 * @param closable 是否可以关闭。
 * @param id 需要设置的窗口 ID，省略则默认为当前窗口。
 * @returns 一个 Promise，在设置成功后解析该 Promise，或在发生错误时拒绝该 Promise。
 */
export function setClosable(closable: boolean, id?: number): Promise<void>;
```

## Niva.api.window.isMinimized

```ts
/**
 * 判断窗口是否最小化。
 * @param id 判断的窗口 ID，如果省略则默认为当前窗口。
 * @returns 一个 Promise，在接收到判断结果后解析该 Promise，或在发生错误时拒绝该 Promise。
 */
export function isMinimized(id?: number): Promise<boolean>;
```

## Niva.api.window.setMinimized

```ts
/**
 * 最小化或恢复窗口。
 * @param minimized 是否最小化。
 * @param id 操作的窗口 ID，如果省略则默认为当前窗口。
 * @returns 一个 Promise，在操作成功后解析该 Promise，或在发生错误时拒绝该 Promise。
 */
export function setMinimized(minimized: boolean, id?: number): Promise<void>;
```

## Niva.api.window.isMaximized

```ts
/**
 * 判断窗口是否最大化。
 * @param id 判断的窗口 ID，如果省略则默认为当前窗口。
 * @returns 一个 Promise，在接收到判断结果后解析该 Promise，或在发生错误时拒绝该 Promise。
 */
export function isMaximized(id?: number): Promise<boolean>;
```

## Niva.api.window.setMaximized

```ts
/**
 * 最大化或居中窗口。
 * @param maximized 是否最大化。
 * @param id 操作的窗口 ID，如果省略则默认为当前窗口。
 * @returns 一个 Promise，在操作成功后解析该 Promise，或在发生错误时拒绝该 Promise。
 */
export function setMaximized(maximized: boolean, id?: number): Promise<void>;
```

## Niva.api.window.isDecorated

```ts
/**
 * 判断窗口是否使用装饰。
 * @param id 判断的窗口 ID，如果省略则默认为当前窗口。
 * @returns 一个 Promise，在接收到判断结果后解析该 Promise，或在发生错误时拒绝该 Promise。
 */
export function isDecorated(id?: number): Promise<boolean>;
```

## Niva.api.window.setDecorated

```ts
/**
 * 开启或关闭窗口装饰。
 * @param decorated 是否使用窗口装饰。
 * @param id 操作的窗口 ID，如果省略则默认为当前窗口。
 * @returns 一个 Promise，在操作成功后解析该 Promise，或在发生错误时拒绝该 Promise。
 */
export function setDecorated(decorated: boolean, id?: number): Promise<void>;
```

## Niva.api.window.isFullscreen

```ts
/**
 * 判断窗口是否全屏。
 * @param id 判断的窗口 ID，如果省略则默认为当前窗口。
 * @returns 一个 Promise，在接收到判断结果后解析该 Promise，或在发生错误时拒绝该 Promise。
 */
export function isFullscreen(id?: number): Promise<boolean>;
```

## Niva.api.window.setFullscreen

```ts
/**
 * 全屏或退出全屏窗口。
 * @param isFullscreen 是否全屏。
 * @param monitorName 需要全屏到的显示器名称，或者“null”或“undefined”表示全屏到当前显示器。如果省略，会在所有可用的显示器中搜索最接近窗口的一个，并尽可能在其上对齐。
 * @param id 操作的窗口 ID，如果省略则默认为当前窗口。
 * @returns 一个 Promise，在操作成功后解析该 Promise，或在发生错误时拒绝该 Promise。
 */
export function setFullscreen(
  isFullscreen: boolean,
  monitorName?: string | null,
  id?: number
): Promise<void>;
```

## Niva.api.window.setAlwaysOnTop

```ts
/**
 * 设置窗口总在顶部。
 * @param alwaysOnTop 是否总在顶部。
 * @param id 需要设置的窗口 ID，省略则默认为当前窗口。
 * @returns 一个 Promise，在设置成功后解析该 Promise，或在发生错误时拒绝该 Promise。
 */
export function setAlwaysOnTop(
  alwaysOnTop: boolean,
  id?: number
): Promise<void>;
```

## Niva.api.window.setAlwaysOnBottom

```ts
/**
 * 设置窗口总在底部。
 * @param alwaysOnBottom 是否总在底部。
 * @param id 需要设置的窗口 ID，省略则默认为当前窗口。
 * @returns 一个 Promise，在设置成功后解析该 Promise，或在发生错误时拒绝该 Promise。
 */
export function setAlwaysOnBottom(
  alwaysOnBottom: boolean,
  id?: number
): Promise<void>;
```

## Niva.api.window.requestUserAttention

```ts
/**
 * 请求用户对该窗口进行关注。
 * @param level 请求关注类型，这将决定关注的方式和级别。可选值为 "informational" (信息) 或 "critical" (严重)。默认为 "normal"(普通)。
 * @param id 需要请求关注的窗口 ID，省略则默认为当前窗口。
 * @returns 一个 Promise，在发送请求成功后解析该 Promise，或者在发生错误时拒绝该 Promise。
 */
export function requestUserAttention(
  level?: "normal" | "informational" | "critical",
  id?: number
): Promise<void>;
```

## Niva.api.window.setContentProtection

```ts
/**
 * 设置窗口的内容保护模式。
 * @param enabled 是否开启内容保护模式。
 * @param id 需要设置的窗口 ID，省略则默认为当前窗口。
 * @returns 一个 Promise，在设置成功后解析该 Promise，或在发生错误时拒绝该 Promise。
 */
export function setContentProtection(
  enabled: boolean,
  id?: number
): Promise<void>;
```

## Niva.api.window.setVisibleOnAllWorkspaces

```ts
/**
 * 设置窗口是否在所有工作区域都可见。
 * @param visible 是否在所有工作区域都可见。
 * @param id 需要设置的窗口 ID，省略则默认为当前窗口。
 * @returns 一个 Promise，在设置成功后解析该 Promise，或在发生错误时拒绝该 Promise。
 */
export function setVisibleOnAllWorkspaces(
  visible: boolean,
  id?: number
): Promise<void>;
```

## Niva.api.window.setCursorIcon

```ts
/**
 * 设置当前指针的图标。
 * @param icon 指针所使用的图标名称或 URL。
 * @param id 需要设置指针图标的窗口 ID，省略则默认为当前窗口。
 * @returns 一个 Promise，在设置成功后解析该 Promise，或在发生错误时拒绝该 Promise。
 */
export function setCursorIcon(icon: string, id?: number): Promise<void>;
```

## Niva.api.window.cursorPosition

```ts
/**
 * 获取鼠标光标的当前位置。
 * @param id 获取光标位置的窗口 ID，省略则默认为当前窗口。
 * @returns 一个 Promise，在接收到光标的位置信息后解析该 Promise，或在发生错误时拒绝该 Promise。
 */
export function cursorPosition(id?: number): Promise<NivaPosition>;
```

## Niva.api.window.setCursorPosition

```ts
/**
 * 将鼠标光标移动到指定的屏幕位置。
 * @param position 鼠标光标要移动到的屏幕位置。
 * @param id 设置光标位置的窗口 ID，省略则默认为当前窗口。
 * @returns 一个 Promise，在设置成功后解析该 Promise，或在发生错误时拒绝该 Promise。
 */
export function setCursorPosition(
  position: NivaPosition,
  id?: number
): Promise<void>;
```

## Niva.api.window.setCursorGrab

```ts
/**
 * 捕获或释放鼠标光标，并允许光标离开窗口。只有在捕获时才能接收所有鼠标或触摸输入。
 * @param grab 是否捕获光标。
 * @param id 操作窗口的 ID，省略则默认为当前窗口。
 * @returns 一个 Promise，在操作成功后解析该 Promise，或在发生错误时拒绝该 Promise。
 */
export function setCursorGrab(grab: boolean, id?: number): Promise<void>;
```

## Niva.api.setCursorVisible

```ts
/**
 * 设置窗口光标是否可见。
 * @param visible 是否可见。
 * @param id 区别不同窗口的可选 ID。
 * @returns 如果成功则返回 Promise.resolve()，否则返回 Promise.reject()。
 */
export function setCursorVisible(visible: boolean, id?: number): Promise<void>;
```

## Niva.api.dragWindow

```ts
/**
 * 设置窗口可拖动。
 * @param id 区别不同窗口的可选 ID。
 * @returns 如果成功则返回 Promise.resolve()，否则返回 Promise.reject()。
 */
export function dragWindow(id?: number): Promise<void>;
```

## Niva.api.setIgnoreCursorEvents

```ts
/**
 * 设置窗口是否忽略鼠标事件。
 * @param ignore 是否忽略。
 * @param id 区别不同窗口的可选 ID。
 * @returns 如果成功则返回 Promise.resolve()，否则返回 Promise.reject()。
 */
export function setIgnoreCursorEvents(
  ignore: boolean,
  id?: number
): Promise<void>;
```

## Niva.api.theme

```ts
/**
 * 获取当前窗口的主题。
 * @param id 区别不同窗口的可选 ID。
 * @returns 如果成功则返回一个 Promise，Promise 成功时包含当前主题（"light"、"dark" 或 "system"），否则返回 Promise.reject()。
 */
export function theme(id?: number): Promise<string>;
```
