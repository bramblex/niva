# 窗口扩展 windowExtra

窗口扩展方法按目标平台分组：Windows 与 macOS 会注册不同方法；不支持的平台不会注册这些方法。

```ts
interface NivaWindowExtra {
  /**
   * 设置窗口是否启用。仅适用于 Windows。
   * @param enabled 是否启用。
   * @param id 区别不同窗口的可选 ID。
   * @returns 如果成功则返回 Promise.resolve()，否则返回 Promise.reject()。
   */
  setEnable(enabled: boolean, id?: number): Promise<void>;
  /**
   * 设置任务栏图标。仅适用于 Windows。
   * @param taskbar_icon 任务栏图标。
   * @param id 区别不同窗口的可选 ID。
   * @returns 如果成功则返回 Promise.resolve()，否则返回 Promise.reject()。
   */
  setTaskbarIcon(taskbar_icon: string, id?: number): Promise<void>;
  /**
   * 获取窗口的主题。仅适用于 Windows。
   * @param id 区别不同窗口的可选 ID。
   * @returns 如果成功则返回一个 Promise 对象，成功时包含当前主题（"light"、"dark" 或 "system"），否则返回 Promise.reject()。
   */
  theme(id?: number): Promise<string>;
  /**
   * 重置死键。仅适用于 Windows。
   * @param id 区别不同窗口的可选 ID。
   * @returns 如果成功则返回 Promise.resolve()，否则返回 Promise.reject()。
   */
  resetDeadKeys(id?: number): Promise<void>;
  /**
   * 开始窗口缩放拖动。仅适用于 Windows。
   * @param edge 窗口拖动的边界。
   * @param button 鼠标按钮。
   * @param x 鼠标位置 X 坐标。
   * @param y 鼠标位置 Y 坐标。
   * @param id 区别不同窗口的可选 ID。
   * @returns 如果成功则返回 Promise.resolve()，否则返回 Promise.reject()。
   */
  beginResizeDrag(
    edge: number,
    button: number,
    x: number,
    y: number,
    id?: number
  ): Promise<void>;
  /**
   * 设置是否将窗口从任务栏中隐藏或删除。仅适用于 Windows。
   * @param skip 是否从任务栏中隐藏或删除。
   * @param id 区别不同窗口的可选 ID。
   * @returns 如果成功则返回 Promise.resolve()，否则返回 Promise.reject()。
   */
  setSkipTaskbar(skip: boolean, id?: number): Promise<void>;
  /** 设置窗口无装饰时是否显示阴影。仅适用于 Windows。 */
  setUndecoratedShadow(shadow: boolean, id?: number): Promise<void>;
  /** 设置任务栏覆盖图标；传 null 清除。仅适用于 Windows。 */
  setOverlayIcon(iconPath: string | null, id?: number): Promise<void>;
  /** 设置窗口 RTL 样式。仅适用于 Windows。 */
  setRtl(rtl: boolean, id?: number): Promise<void>;
  /** 查询无装饰窗口阴影状态。仅适用于 Windows。 */
  hasUndecoratedShadow(id?: number): Promise<boolean>;
  /**
   * 获取窗口的全屏状态。仅适用于 macOS。
   * @param id 区别不同窗口的可选 ID。
   * @returns 如果窗口处于全屏状态，则返回 true，否则返回 false。成功时返回包含窗口全屏状态的 Promise 对象，否则返回 Promise.reject()。
   */
  simpleFullscreen(id?: number): Promise<boolean>;
  /**
   * 设置窗口是否进入全屏状态。仅适用于 macOS。
   * @param fullscreen 是否全屏。
   * @param id 区别不同窗口的可选 ID。
   * @returns 全屏状态切换成功时返回 true，否则返回 false。成功时返回包含窗口全屏状态的 Promise 对象，否则返回 Promise.reject()。
   */
  setSimpleFullscreen(fullscreen: boolean, id?: number): Promise<boolean>;
  /** 查询窗口是否显示阴影。仅适用于 macOS。 */
  hasShadow(id?: number): Promise<boolean>;
  /**
   * 设置窗口是否显示阴影。仅适用于 macOS。
   * @param has_shadow 是否显示阴影。
   * @param id 区分不同窗口的可选 ID。
   * @returns 如果成功则返回 Promise.resolve()，否则返回 Promise.reject()。
   */
  setHasShadow(has_shadow: boolean, id?: number): Promise<void>;
  /**
   * 设置文档是否已编辑。仅适用于 macOS。
   * @param edited 是否已编辑。
   * @param id 区别不同窗口的可选 ID。
   * @returns 如果成功则返回 Promise.resolve()，否则返回 Promise.reject()。
   */
  setIsDocumentEdited(edited: boolean, id?: number): Promise<void>;
  /**
   * 获取文档是否已编辑。仅适用于 macOS。
   * @param id 区别不同窗口的可选 ID。
   * @returns 如果文档已编辑则返回 true，否则返回 false。成功时返回一个表示文档是否被编辑的 Promise 对象，否则返回 Promise.reject()。
   */
  isDocumentEdited(id?: number): Promise<boolean>;
  /**
   * 设置窗口是否允许自动创建选项卡。仅适用于 macOS。
   * @param enabled 是否允许自动创建选项卡。
   * @param id 区别不同窗口的可选 ID。
   * @returns 如果成功则返回 Promise.resolve()，否则返回 Promise.reject()。
   */
  setAllowsAutomaticWindowTabbing(enabled: boolean, id?: number): Promise<void>;
  /**
   * 获取窗口是否允许自动创建选项卡。仅适用于 macOS。
   * @param id 区分不同窗口的可选 ID。
   * @returns 如果窗口允许自动创建选项卡，则返回 true，否则返回 false。成功时返回表示窗口是否允许自动创建选项卡的 Promise 对象，否则返回 Promise.reject()。
   */
  allowsAutomaticWindowTabbing(id?: number): Promise<boolean>;
  /**
   * 设置窗口的选项卡标识符。仅适用于 macOS。
   * @param identifier 选项卡标识符。
   * @param id 区分不同窗口的可选 ID。
   * @returns 如果成功则返回 Promise.resolve()，否则返回 Promise.reject()。
   */
  setTabbingIdentifier(identifier: string, id?: number): Promise<void>;
  /**
   * 获取窗口的选项卡标识符。仅适用于 macOS。
   * @param id 区分不同窗口的可选 ID。
   * @returns 选项卡标识符。成功时返回表示窗口选项卡标识符的 Promise 对象，否则返回 Promise.reject()。
   */
  tabbingIdentifier(id?: number): Promise<string>;
  /** 设置 macOS 窗口交通灯按钮相对窗口左上角的偏移量。仅适用于 macOS。 */
  setTrafficLightInset(position: NivaPosition, id?: number): Promise<void>;
  /** 运行时设置 macOS 应用激活策略。 */
  setActivationPolicyAtRuntime(policy: "regular" | "accessory" | "prohibited"): Promise<void>;
  /** 设置 macOS Dock 图标是否可见。 */
  setDockVisibility(visible: boolean): Promise<void>;
  /** 设置或清除 macOS Dock 角标。 */
  setBadgeLabel(label?: string | null): Promise<void>;
}
```
