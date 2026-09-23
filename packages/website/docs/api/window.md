---
sidebar_position: 10
---

# 窗口 window

窗口选项见[窗口配置](../options/window)。运行时 `window.open()` 的 options 可以省略，此时使用默认窗口选项。

```ts
interface NivaWindow {
  /**
   * 获取当前窗口的 ID。
   * @returns 一个 Promise，在接收成功响应后返回当前窗口 ID。
   */
  current(): Promise<number>;
  /**
   * 打开一个新窗口。
   * @param options 可选的窗口选项，包括宽度、高度、坐标、标题和 URL 等。
   * @returns 一个 Promise，在接收成功响应后返回新窗口的 ID。
   */
  open(options?: NivaWindowOptions): Promise<number>;
  /**
   * 关闭窗口。
   * @param id 可选的窗口 ID，若不提供，则关闭当前窗口。若 ID 为 0，则退出程序。
   * @returns 一个 Promise，在接收成功响应后表示已关闭窗口，若 ID 为 0 则表示退出程序。
   */
  close(id?: number): Promise<void>;
  /**
   * 获取当前应用所有窗口的 ID、标题、是否可见列表。
   * @returns 一个 Promise，在解析成功后返回一个包含所有窗口的 ID、标题、是否可见数据组成的对象列表。
   */
  list(): Promise<{ id: number; title: string; visible: boolean }[]>;
  /**
   * 向指定 ID 的窗口发送 IPC 消息。
   * @param message IPC 消息内容。
   * @param id 目标窗口的 ID 号。
   * @returns 一个 Promise，在接收到发送成功回执后解析该 Promise，或在发生错误时拒绝该 Promise。
   */
  sendMessage(message: string, id: number): Promise<void>;
  /**
   * 设置窗口菜单。
   * @param options 菜单选项，如果为 undefined 或 null，则移除菜单。
   * @param id 需要设置菜单的窗口 ID，省略则默认为当前窗口。
   * @returns 一个 Promise，在设置成功后解析该 Promise，或在发生错误时拒绝该 Promise。
   */
  setMenu(options?: WindowMenuOptions | null, id?: number): Promise<void>;
  /**
   * 隐藏窗口菜单。
   * @param id 需要隐藏菜单的窗口 ID，省略则默认为当前窗口。
   * @returns 一个 Promise，在隐藏成功后解析该 Promise，或在发生错误时拒绝该 Promise。
   */
  hideMenu(id?: number): Promise<void>;
  /**
   * 显示窗口菜单。
   * @param id 需要显示菜单的窗口 ID，省略则默认为当前窗口。
   * @returns 一个 Promise，在显示成功后解析该 Promise，或在发生错误时拒绝该 Promise。
   */
  showMenu(id?: number): Promise<void>;
  /**
   * 判断窗口菜单是否可见。
   * @param id 窗口 ID，省略则默认为当前窗口。
   * @returns 一个 Promise，在接收到判断结果后解析该 Promise，或在发生错误时拒绝该 Promise。
   */
  isMenuVisible(id?: number): Promise<boolean>;
  /**
   * 获取窗口缩放因子。
   * @param id 需要获取缩放因子的窗口 ID，省略则默认为当前窗口。
   * @returns 一个 Promise，在解析成功后返回窗口缩放因子。
   */
  scaleFactor(id?: number): Promise<number>;
  /**
   * 获取窗口客户区左上角在屏幕坐标系下的坐标。
   * @param id 需要获取位置的窗口 ID，省略则默认为当前窗口。
   * @returns 一个 Promise，在解析成功后返回窗口客户区左上角在屏幕坐标系下的坐标值。
   */
  innerPosition(id?: number): Promise<NivaPosition>;
  /**
   * 获取窗口左上角在屏幕坐标系下的坐标。
   * @param id 需要获取位置的窗口 ID，省略则默认为当前窗口。
   * @returns 一个 Promise，在解析成功后返回窗口左上角在屏幕坐标系下的坐标值。
   */
  outerPosition(id?: number): Promise<NivaPosition>;
  /**
   * 设置窗口左上角在屏幕坐标系下的坐标。
   * @param position 窗口左上角在屏幕坐标系下的坐标值。
   * @param id 需要设置位置的窗口 ID，省略则默认为当前窗口。
   * @returns 一个 Promise，在设置成功后解析该 Promise，或在发生错误时拒绝该 Promise。
   */
  setOuterPosition(position: NivaPosition, id?: number): Promise<void>;
  /**
   * 获取窗口客户区大小。
   * @param id 需要获取大小的窗口 ID，省略则默认为当前窗口。
   * @returns 一个 Promise，在解析成功后返回窗口客户区大小。
   */
  innerSize(id?: number): Promise<NivaSize>;
  /**
   * 设置窗口客户区大小。
   * @param size 要设置的窗口客户区大小。
   * @param id 需要设置大小的窗口 ID，省略则默认为当前窗口。
   * @returns 一个 Promise，在设置成功后解析该 Promise，或在发生错误时拒绝该 Promise。
   */
  setInnerSize(size: NivaSize, id?: number): Promise<void>;
  /**
   * 获取窗口大小，包括边框和菜单等非客户区部分。
   * @param id 需要获取大小的窗口 ID，省略则默认为当前窗口。
   * @returns 一个 Promise，在解析成功后返回窗口大小，包括边框和菜单等非客户区部分。
   */
  outerSize(id?: number): Promise<NivaSize>;
  /**
   * 设置窗口客户区最小大小；传 null 清除最小尺寸约束。
   * @param size 窗口客户区最小大小，或 null 清除约束。
   * @param id 需要设置大小的窗口 ID，省略则默认为当前窗口。
   * @returns 一个 Promise，在设置成功后解析该 Promise，或在发生错误时拒绝该 Promise。
   */
  setMinInnerSize(size: NivaSize | null, id?: number): Promise<void>;
  /**
   * 设置窗口客户区最大大小；传 null 清除最大尺寸约束。
   * @param size 窗口客户区最大大小，或 null 清除约束。
   * @param id 需要设置大小的窗口 ID，省略则默认为当前窗口。
   * @returns 一个 Promise，在设置成功后解析该 Promise，或在发生错误时拒绝该 Promise。
   */
  setMaxInnerSize(size: NivaSize | null, id?: number): Promise<void>;
  /**
   * 设置运行时窗口图标，传 null 清除图标。图标路径位于应用资源中；Tao 在 Windows/Linux 支持此 API，macOS 无窗口图标语义，Niva 在 macOS/iOS/Android 返回不支持错误。
   */
  setWindowIcon(iconPath?: string | null, id?: number): Promise<void>;
  /**
   * 运行时设置窗口主题。system 或 null 使用系统主题；Linux/macOS 的主题状态由 Tao 按应用级处理；iOS/Android 不支持。
   */
  setTheme(theme?: "light" | "dark" | "system" | null, id?: number): Promise<void>;
  /**
   * 开始窗口边缘缩放拖动。Tao 在 Windows/Linux 支持；macOS/iOS/Android 返回不支持错误。
   */
  dragResizeWindow(direction: NivaWindowResizeDirection, id?: number): Promise<void>;
  /**
   * 设置任务栏进度。progress 取 0 到 100；Linux/macOS 的进度是应用级，Linux 需桌面环境提供 libunity 支持；iOS/Android 不支持。
   */
  setProgressBar(options: NivaWindowProgressBar, id?: number): Promise<void>;
  /** 请求原生窗口重绘。Android 的 Tao 0.37 实现不支持，Niva 会返回错误。 */
  requestRedraw(id?: number): Promise<void>;
  /** 设置 CJK 输入法候选窗在窗口客户区的逻辑坐标；Linux Tao 实现为空操作，Niva 会返回不支持错误。 */
  setImePosition(position: NivaPosition, id?: number): Promise<void>;
  /** 设置窗口背景 RGBA；Windows 会忽略 alpha 分量；iOS/Android 不支持。 */
  setBackgroundColor(color: NivaRGBA | null, id?: number): Promise<void>;
  /** 设置窗口是否可聚焦；macOS 已聚焦窗口设为不可聚焦后不能直接取消聚焦；iOS/Android 不支持。 */
  setFocusable(focusable: boolean, id?: number): Promise<void>;
  /**
   * 设置窗口标题。
   * @param title 窗口标题。
   * @param id 需要设置标题的窗口 ID，省略则默认为当前窗口。
   * @returns 一个 Promise，在设置成功后解析该 Promise，或在发生错误时拒绝该 Promise。
   */
  setTitle(title: string, id?: number): Promise<void>;
  /**
   * 获取窗口标题。
   * @param id 需要获取标题的窗口 ID，省略则默认为当前窗口。
   * @returns 一个 Promise，在解析成功后返回窗口标题。
   */
  title(id?: number): Promise<string>;
  /**
   * 判断窗口是否可见。
   * @param id 需要判断的窗口 ID，省略则默认为当前窗口。
   * @returns 一个 Promise，在接收到判断结果后解析该 Promise，或在发生错误时拒绝该 Promise。
   */
  isVisible(id?: number): Promise<boolean>;
  /**
   * 设置窗口是否可见。
   * @param visible 是否可见。
   * @param id 需要设置的窗口 ID，省略则默认为当前窗口。
   * @returns 一个 Promise，在设置成功后解析该 Promise，或在发生错误时拒绝该 Promise。
   */
  setVisible(visible: boolean, id?: number): Promise<void>;
  /**
   * 判断窗口是否聚焦。
   * @param id 需要判断的窗口 ID，省略则默认为当前窗口。
   * @returns 一个 Promise，在接收到判断结果后解析该 Promise，或在发生错误时拒绝该 Promise。
   */
  isFocused(id?: number): Promise<boolean>;
  /**
   * 窗口设置为聚焦状态。
   * @param id 需要设置的窗口 ID，省略则默认为当前窗口。
   * @returns 一个 Promise，在设置成功后解析该 Promise，或在发生错误时拒绝该 Promise。
   */
  setFocus(id?: number): Promise<void>;
  /**
   * 判断窗口是否可以改变大小。
   * @param id 需要判断的窗口 ID，省略则默认为当前窗口。
   * @returns 一个 Promise，在接收到判断结果后解析该 Promise，或在发生错误时拒绝该 Promise。
   */
  isResizable(id?: number): Promise<boolean>;
  /**
   * 设置窗口是否可以改变大小。
   * @param resizable 是否可以改变大小。
   * @param id 需要设置的窗口 ID，省略则默认为当前窗口。
   * @returns 一个 Promise，在设置成功后解析该 Promise，或在发生错误时拒绝该 Promise。
   */
  setResizable(resizable: boolean, id?: number): Promise<void>;
  /**
   * 判断窗口是否可以最小化。
   * @param id 需要判断的窗口 ID，省略则默认为当前窗口。
   * @returns 一个 Promise，在接收到判断结果后解析该 Promise，或在发生错误时拒绝该 Promise。
   */
  isMinimizable(id?: number): Promise<boolean>;
  /**
   * 设置窗口是否可以最小化。
   * @param minimizable 是否可以最小化。
   * @param id 需要设置的窗口 ID，省略则默认为当前窗口。
   * @returns 一个 Promise，在设置成功后解析该 Promise，或在发生错误时拒绝该 Promise。
   */
  setMinimizable(minimizable: boolean, id?: number): Promise<void>;
  /**
   * 判断窗口是否可以最大化。
   * @param id 需要判断的窗口 ID，省略则默认为当前窗口。
   * @returns 一个 Promise，在接收到判断结果后解析该 Promise，或在发生错误时拒绝该 Promise。
   */
  isMaximizable(id?: number): Promise<boolean>;
  /**
   * 设置窗口是否可以最大化。
   * @param maximizable 是否可以最大化。
   * @param id 需要设置的窗口 ID，省略则默认为当前窗口。
   * @returns 一个 Promise，在设置成功后解析该 Promise，或在发生错误时拒绝该 Promise。
   */
  setMaximizable(maximizable: boolean, id?: number): Promise<void>;
  /**
   * 判断窗口是否可以关闭。
   * @param id 需要判断的窗口 ID，省略则默认为当前窗口。
   * @returns 一个 Promise，在接收到判断结果后解析该 Promise，或在发生错误时拒绝该 Promise。
   */
  isClosable(id?: number): Promise<boolean>;
  /**
   * 判断窗口是否最小化。
   * @param id 判断的窗口 ID，如果省略则默认为当前窗口。
   * @returns 一个 Promise，在接收到判断结果后解析该 Promise，或在发生错误时拒绝该 Promise。
   */
  isMinimized(id?: number): Promise<boolean>;
  /**
   * 最小化或恢复窗口。
   * @param minimized 是否最小化。
   * @param id 操作的窗口 ID，如果省略则默认为当前窗口。
   * @returns 一个 Promise，在操作成功后解析该 Promise，或在发生错误时拒绝该 Promise。
   */
  setMinimized(minimized: boolean, id?: number): Promise<void>;
  /**
   * 判断窗口是否最大化。
   * @param id 判断的窗口 ID，如果省略则默认为当前窗口。
   * @returns 一个 Promise，在接收到判断结果后解析该 Promise，或在发生错误时拒绝该 Promise。
   */
  isMaximized(id?: number): Promise<boolean>;
  /**
   * 最大化或居中窗口。
   * @param maximized 是否最大化。
   * @param id 操作的窗口 ID，如果省略则默认为当前窗口。
   * @returns 一个 Promise，在操作成功后解析该 Promise，或在发生错误时拒绝该 Promise。
   */
  setMaximized(maximized: boolean, id?: number): Promise<void>;
  /** 设置窗口是否可以关闭。 */
  setClosable(closable: boolean, id?: number): Promise<void>;
  /**
   * 判断窗口是否使用装饰。
   * @param id 判断的窗口 ID，如果省略则默认为当前窗口。
   * @returns 一个 Promise，在接收到判断结果后解析该 Promise，或在发生错误时拒绝该 Promise。
   */
  isDecorated(id?: number): Promise<boolean>;
  /**
   * 开启或关闭窗口装饰。
   * @param decorated 是否使用窗口装饰。
   * @param id 操作的窗口 ID，如果省略则默认为当前窗口。
   * @returns 一个 Promise，在操作成功后解析该 Promise，或在发生错误时拒绝该 Promise。
   */
  setDecorated(decorated: boolean, id?: number): Promise<void>;
  /**
   * 判断窗口是否全屏。
   * @param id 判断的窗口 ID，如果省略则默认为当前窗口。
   * @returns 一个 Promise，在接收到判断结果后解析该 Promise，或在发生错误时拒绝该 Promise。
   */
  fullscreen(id?: number): Promise<boolean>;
  /**
   * 全屏或退出全屏窗口。
   * @param isFullscreen 是否全屏。
   * @param monitorName 需要全屏到的显示器名称，或者“null”或“undefined”表示全屏到当前显示器。如果省略，会在所有可用的显示器中搜索最接近窗口的一个，并尽可能在其上对齐。
   * @param id 操作的窗口 ID，如果省略则默认为当前窗口。
   * @returns 一个 Promise，在操作成功后解析该 Promise，或在发生错误时拒绝该 Promise。
   */
  setFullscreen(
    isFullscreen: boolean,
    monitorName?: string | null,
    id?: number
  ): Promise<void>;
  /**
   * 设置窗口总在顶部。
   * @param alwaysOnTop 是否总在顶部。
   * @param id 需要设置的窗口 ID，省略则默认为当前窗口。
   * @returns 一个 Promise，在设置成功后解析该 Promise，或在发生错误时拒绝该 Promise。
   */
  setAlwaysOnTop(alwaysOnTop: boolean, id?: number): Promise<void>;
  /**
   * 设置窗口总在底部。
   * @param alwaysOnBottom 是否总在底部。
   * @param id 需要设置的窗口 ID，省略则默认为当前窗口。
   * @returns 一个 Promise，在设置成功后解析该 Promise，或在发生错误时拒绝该 Promise。
   */
  setAlwaysOnBottom(alwaysOnBottom: boolean, id?: number): Promise<void>;
  /**
   * 请求用户对该窗口进行关注。
   * @param level 请求关注类型，这将决定关注的方式和级别。可选值为 "informational" (信息) 或 "critical" (严重)。默认为 "normal"(普通)。
   * @param id 需要请求关注的窗口 ID，省略则默认为当前窗口。
   * @returns 一个 Promise，在发送请求成功后解析该 Promise，或者在发生错误时拒绝该 Promise。
   */
  requestUserAttention(
    level?: "normal" | "informational" | "critical",
    id?: number
  ): Promise<void>;
  /**
   * 设置窗口的内容保护模式。
   * @param enabled 是否开启内容保护模式。
   * @param id 需要设置的窗口 ID，省略则默认为当前窗口。
   * @returns 一个 Promise，在设置成功后解析该 Promise，或在发生错误时拒绝该 Promise。
   */
  setContentProtection(enabled: boolean, id?: number): Promise<void>;
  /**
   * 设置窗口是否在所有工作区域都可见。
   * @param visible 是否在所有工作区域都可见。
   * @param id 需要设置的窗口 ID，省略则默认为当前窗口。
   * @returns 一个 Promise，在设置成功后解析该 Promise，或在发生错误时拒绝该 Promise。
   */
  setVisibleOnAllWorkspaces(visible: boolean, id?: number): Promise<void>;
  /**
   * 设置当前指针的图标。
   * @param icon 指针所使用的图标名称或 URL。
   * @param id 需要设置指针图标的窗口 ID，省略则默认为当前窗口。
   * @returns 一个 Promise，在设置成功后解析该 Promise，或在发生错误时拒绝该 Promise。
   */
  setCursorIcon(icon: string, id?: number): Promise<void>;
  /**
   * 获取鼠标光标的当前位置。
   * @param id 获取光标位置的窗口 ID，省略则默认为当前窗口。
   * @returns 一个 Promise，在接收到光标的位置信息后解析该 Promise，或在发生错误时拒绝该 Promise。
   */
  cursorPosition(id?: number): Promise<NivaPosition>;
  /**
   * 将鼠标光标移动到指定的屏幕位置。
   * @param position 鼠标光标要移动到的屏幕位置。
   * @param id 设置光标位置的窗口 ID，省略则默认为当前窗口。
   * @returns 一个 Promise，在设置成功后解析该 Promise，或在发生错误时拒绝该 Promise。
   */
  setCursorPosition(position: NivaPosition, id?: number): Promise<void>;
  /**
   * 捕获或释放鼠标光标，并允许光标离开窗口。只有在捕获时才能接收所有鼠标或触摸输入。
   * @param grab 是否捕获光标。
   * @param id 操作窗口的 ID，省略则默认为当前窗口。
   * @returns 一个 Promise，在操作成功后解析该 Promise，或在发生错误时拒绝该 Promise。
   */
  setCursorGrab(grab: boolean, id?: number): Promise<void>;
  /**
   * 设置窗口光标是否可见。
   * @param visible 是否可见。
   * @param id 区别不同窗口的可选 ID。
   * @returns 如果成功则返回 Promise.resolve()，否则返回 Promise.reject()。
   */
  setCursorVisible(visible: boolean, id?: number): Promise<void>;
  /**
   * 设置窗口可拖动。
   * @param id 区别不同窗口的可选 ID。
   * @returns 如果成功则返回 Promise.resolve()，否则返回 Promise.reject()。
   */
  dragWindow(id?: number): Promise<void>;
  /**
   * 设置窗口是否忽略鼠标事件。
   * @param ignore 是否忽略。
   * @param id 区别不同窗口的可选 ID。
   * @returns 如果成功则返回 Promise.resolve()，否则返回 Promise.reject()。
   */
  setIgnoreCursorEvents(ignore: boolean, id?: number): Promise<void>;
  /**
   * 获取当前窗口的主题。
   * @param id 区别不同窗口的可选 ID。
   * @returns 如果成功则返回一个 Promise，Promise 成功时包含当前主题（"light"、"dark" 或 "system"），否则返回 Promise.reject()。
   */
  theme(id?: number): Promise<string>;
  /**
   * 设置窗口是否拦截默认的关闭操作，`true`的话会由`window.closeRequested`拦截。
   * @param blocked 是否开启拦截
   * @param id 区别不同窗口的可选 ID。
   */
  blockCloseRequested(blocked: boolean, id?: number): Promise<void>;
}
```
