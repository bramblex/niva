# 窗口额外 window_extra

## Niva.api.windowExtra.setEnable

```ts
/**
 * 设置窗口是否启用。仅适用于 Windows。
 * @param enabled 是否启用。
 * @param id 区别不同窗口的可选 ID。
 * @returns 如果成功则返回 Promise.resolve()，否则返回 Promise.reject()。
 */
export function setEnable(enabled: boolean, id?: number): Promise<void>;
```

## Niva.api.windowExtra.setTaskbarIcon

```ts
/**
 * 设置任务栏图标。仅适用于 Windows。
 * @param taskbar_icon 任务栏图标。
 * @param id 区别不同窗口的可选 ID。
 * @returns 如果成功则返回 Promise.resolve()，否则返回 Promise.reject()。
 */
export function setTaskbarIcon(
  taskbar_icon: string,
  id?: number
): Promise<void>;
```

## Niva.api.windowExtra.theme

```ts
/**
 * 获取窗口的主题。仅适用于 Windows。
 * @param id 区别不同窗口的可选 ID。
 * @returns 如果成功则返回一个 Promise 对象，成功时包含当前主题（"light"、"dark" 或 "system"），否则返回 Promise.reject()。
 */
export function theme(id?: number): Promise<string>;
```

## Niva.api.windowExtra.resetDeadKeys

```ts
/**
 * 重置死键。仅适用于 Windows。
 * @param id 区别不同窗口的可选 ID。
 * @returns 如果成功则返回 Promise.resolve()，否则返回 Promise.reject()。
 */
export function resetDeadKeys(id?: number): Promise<void>;
```

## Niva.api.windowExtra.beginResizeDrag

```ts
/**
 * 开始窗口缩放拖动。仅适用于 Windows。
 * @param edge 窗口拖动的边界。
 * @param button 鼠标按钮。
 * @param x 鼠标位置 X 坐标。
 * @param y 鼠标位置 Y 坐标。
 * @param id 区别不同窗口的可选 ID。
 * @returns 如果成功则返回 Promise.resolve()，否则返回 Promise.reject()。
 */
export function beginResizeDrag(
  edge: number,
  button: number,
  x: number,
  y: number,
  id?: number
): Promise<void>;
```

## Niva.api.windowExtra.setSkipTaskbar

```ts
/**
 * 设置是否将窗口从任务栏中隐藏或删除。仅适用于 Windows。
 * @param skip 是否从任务栏中隐藏或删除。
 * @param id 区别不同窗口的可选 ID。
 * @returns 如果成功则返回 Promise.resolve()，否则返回 Promise.reject()。
 */
export function setSkipTaskbar(skip: boolean, id?: number): Promise<void>;
```

## Niva.api.windowExtra.setUndecoratedShadow

```ts
/**
 * 设置窗口无装饰时是否显示阴影。仅适用于 macOS。

 * @param shadow 是否显示窗口阴影。仅适用于 Windows。
 * @param id 区别不同窗口的可选 ID。
 * @returns 如果成功则返回 Promise.resolve()，否则返回 Promise.reject()。
 */
export function setUndecoratedShadow(
  shadow: boolean,
  id?: number
): Promise<void>;
```

## Niva.api.windowExtra.simpleFullscreen

```ts
/**
 * 获取窗口的全屏状态。仅适用于 macOS。
 * @param id 区别不同窗口的可选 ID。
 * @returns 如果窗口处于全屏状态，则返回 true，否则返回 false。成功时返回包含窗口全屏状态的 Promise 对象，否则返回 Promise.reject()。
 */
export function simpleFullscreen(id?: number): Promise<boolean>;
```

## Niva.api.windowExtra.setSimpleFullscreen

```ts
/**
 * 设置窗口是否进入全屏状态。仅适用于 macOS。
 * @param fullscreen 是否全屏。
 * @param id 区别不同窗口的可选 ID。
 * @returns 全屏状态切换成功时返回 true，否则返回 false。成功时返回包含窗口全屏状态的 Promise 对象，否则返回 Promise.reject()。
 */
export function setSimpleFullscreen(
  fullscreen: boolean,
  id?: number
): Promise<boolean>;
```

## Niva.api.windowExtra.setHasShadow

```ts
/**
 * 设置窗口是否显示阴影。仅适用于 macOS。
 * @param has_shadow 是否显示阴影。
 * @param id 区分不同窗口的可选 ID。
 * @returns 如果成功则返回 Promise.resolve()，否则返回 Promise.reject()。
 */
export function setHasShadow(has_shadow: boolean, id?: number): Promise<void>;
```

## Niva.api.windowExtra.setIsDocumentEdited

```ts
/**
 * 设置文档是否已编辑。仅适用于 macOS。
 * @param edited 是否已编辑。
 * @param id 区别不同窗口的可选 ID。
 * @returns 如果成功则返回 Promise.resolve()，否则返回 Promise.reject()。
 */
export function setIsDocumentEdited(
  edited: boolean,
  id?: number
): Promise<void>;
```

## Niva.api.windowExtra.isDocumentEdited

```ts
/**
 * 获取文档是否已编辑。仅适用于 macOS。
 * @param id 区别不同窗口的可选 ID。
 * @returns 如果文档已编辑则返回 true，否则返回 false。成功时返回一个表示文档是否被编辑的 Promise 对象，否则返回 Promise.reject()。
 */
export function isDocumentEdited(id?: number): Promise<boolean>;
```

## Niva.api.windowExtra.setAllowsAutomaticWindowTabbing

```ts
/**
 * 设置窗口是否允许自动创建选项卡。仅适用于 macOS。
 * @param enabled 是否允许自动创建选项卡。
 * @param id 区别不同窗口的可选 ID。
 * @returns 如果成功则返回 Promise.resolve()，否则返回 Promise.reject()。
 */
export function setAllowsAutomaticWindowTabbing(
  enabled: boolean,
  id?: number
): Promise<void>;
```

## Niva.api.windowExtra.allowsAutomaticWindowTabbing

```ts
/**
 * 获取窗口是否允许自动创建选项卡。仅适用于 macOS。
 * @param id 区分不同窗口的可选 ID。
 * @returns 如果窗口允许自动创建选项卡，则返回 true，否则返回 false。成功时返回表示窗口是否允许自动创建选项卡的 Promise 对象，否则返回 Promise.reject()。
 */
export function allowsAutomaticWindowTabbing(id?: number): Promise<boolean>;
```

## Niva.api.windowExtra.setTabbingIdentifier

```ts
/**
 * 设置窗口的选项卡标识符。仅适用于 macOS。
 * @param identifier 选项卡标识符。
 * @param id 区分不同窗口的可选 ID。
 * @returns 如果成功则返回 Promise.resolve()，否则返回 Promise.reject()。
 */
export function setTabbingIdentifier(
  identifier: string,
  id?: number
): Promise<void>;
```

## Niva.api.windowExtra.tabbingIdentifier

```ts
/**
 * 获取窗口的选项卡标识符。仅适用于 macOS。
 * @param id 区分不同窗口的可选 ID。
 * @returns 选项卡标识符。成功时返回表示窗口选项卡标识符的 Promise 对象，否则返回 Promise.reject()。
 */
export function tabbingIdentifier(id?: number): Promise<string>;
```
