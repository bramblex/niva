/* eslint-disable */
/* prettier-ignore */
// @ts-nocheck
export {}
declare global {
  /** Niva 运行环境提供的对象，包含了事件监听函数及其api */
  const Niva: NivaObj;
}

interface NivaObj {
  /**
   * 绑定事件监听器。
   * @param event 要监听的事件名称，可以用 `*`、`xxxx.*` 等通配符。
   * @param listener 事件被触发时要调用的函数。
   */
  addEventListener<K extends keyof NivaEventMap>(event: K, listener: NivaEventMap[K]): void;
  /**
   * 移除特定的事件监听器。
   * @param event 要移除的事件名称。
   * @param listener 要移除的监听器函数。
   */
  removeEventListener<K extends keyof NivaEventMap>(event: K, listener: Function): void;
  /**
   * 移除特定事件的所有监听器。
   * @param event 要移除所有监听器的事件名称。
   */
  removeAllEventListeners(event: string): void;
  /** 接口方法 */
  api: {
    /** 剪切板 */
    clipboard: NivaClipboard;
    /** 弹框 */
    dialog: NivaDialog;
    /** 系统额外 */
    extra: NivaExtra;
    /** 文件系统 */
    fs: NivaFs;
    /** 网络 */
    http: NivaHttp;
    /** 监视器 */
    monitor: NivaMonitor;
    /** 系统 */
    os: NivaOs;
    /** 进程 */
    process: NivaProcess;
    /** 资源 */
    resource: NivaResource;
    /** 全局快捷键 */
    shortcut: NivaShortcut;
    /** 托盘 */
    tray: NivaTray;
    /** Webview */
    webview: NivaWebview;
    /** 窗口 */
    window: NivaWindow;
    /** 窗口额外内容 */
    windowExtra: NivaWindowExtra;
  };
}

// Niva应用程序的选项接口
interface NivaOptions {
  name: string; // 应用程序的名称
  uuid: string; // 应用程序的唯一标识符
  icon?: string; // 应用程序的图标文件路径，仅支持 png，可选

  window: NivaWindowOptions; // 应用程序窗口的选项
  tray?: NivaTrayOptions; // 应用程序托盘的选项，可选
  shortcuts?: NivaShortcutsOptions; // 应用程序全局快捷键的选项，可选

  workers?: number; // 应用程序开启的工作线程数量，可选

  // Mac平台特有选项
  activationPolicy?: "regular" | "accessory" | "prohibited"; // 应用程序的激活策略，可选
  defaultMenuCreation?: boolean; // 是否使用默认菜单创建方式，可选
  activateIgnoringOtherApps?: boolean; // 是否忽略其他应用程序的激活状态而强制激活应用程序，可选

  // 为不同平台单独配置
  macos: NivaOptions;
  windows: NivaOptions;
}

type NivaSize = { width: number; height: number };
type NivaPosition = { x: number; y: number };

interface WindowRootMenu {
  label: string; // 根菜单项的名称
  enabled?: boolean; // 是否启用菜单项
  children: MenuOptions; // 子菜单项
}

type WindowMenuOptions = Array<WindowRootMenu>;

interface NivaWindowOptions {
  entry?: string; // 应用程序入口文件路径
  devtools?: boolean; // 是否启用开发者工具

  title?: string; // 窗口标题
  icon?: string; // 窗口图标
  theme?: string; // 窗口主题
  size?: NivaSize; // 窗口大小
  minSize?: NivaSize; // 窗口最小尺寸
  maxSize?: NivaSize; // 窗口最大尺寸

  position?: NivaPosition; // 窗口位置

  resizable?: boolean; // 是否可调整大小
  minimizable?: boolean; // 是否可最小化
  maximizable?: boolean; // 是否可最大化
  closable?: boolean; // 是否可关闭

  fullscreen?: boolean; // 是否全屏
  maximized?: boolean; // 是否最大化
  visible?: boolean; // 是否可见
  transparent?: boolean; // 是否透明
  decorations?: boolean; // 是否显示窗口装饰

  alwaysOnTop?: boolean; // 是否始终置顶
  alwaysOnBottom?: boolean; // 是否始终置底
  visibleOnAllWorkspaces?: boolean; // 是否在多个工作区显示

  focused?: boolean; // 是否聚焦
  contentProtection?: boolean; // 是否启用内容保护

  // macOS extra
  parentWindow?: number; // 父窗口 ID
  movableByWindowBackground?: boolean; // 是否可点击窗口背景移动窗口
  titleBarTransparent?: boolean; // 标题栏是否透明
  titleBarHidden?: boolean; // 标题栏是否隐藏
  titleBarButtonsHidden?: boolean; // 标题栏按钮是否隐藏
  titleHidden?: boolean; // 标题是否隐藏
  fullSizeContentView?: boolean; // 是否全尺寸显示内容
  resizeIncrements?: NivaSize; // 窗口调整尺寸步长
  disallowHiDpi?: boolean; // 是否禁用高 DPI
  hasShadow?: boolean; // 是否显示阴影
  automaticWindowTabbing?: boolean; // 是否支持多个窗口进行选项卡式浏览
  tabbingIdentifier?: string; // 设置选项卡式浏览的标题

  // windows extra
  parentWindow?: number; // 父窗口 ID
  ownerWindow?: number; // 拥有者窗口 ID
  taskbarIcon?: string; // 任务栏图标
  skipTaskbar?: boolean; // 在任务栏中是否显示
  undecoratedShadow?: boolean; // 是否显示窗口阴影

  menu?: WindowMenuOptions; // 窗口菜单选项
}

// 系统原生菜单项标签枚举类型。
enum NativeLabel {
  Hide, // 显示 "Hide"
  Services, // 显示 "Services"
  HideOthers, // 显示 "Hide Others"
  ShowAll, // 显示 "Show All"
  CloseWindow, // 显示 "Close Window"
  Quit, // 显示 "Quit"
  Copy, // 显示 "Copy"
  Cut, // 显示 "Cut"
  Undo, // 显示 "Undo"
  Redo, // 显示 "Redo"
  SelectAll, // 显示 "Select All"
  Paste, // 显示 "Paste"
  EnterFullScreen, // 显示 "Enter Full Screen"
  Minimize, // 显示 "Minimize"
  Zoom, // 显示 "Zoom"
  Separator, // 表示一个分隔线
}

// 菜单项选项枚举类型。
type MenuItemOption =
  | { type: "native"; label: NativeLabel } // 本地菜单选项
  | {
      type: "item";
      id: number;
      label: string; // 显示的文本
      enabled?: boolean; // 是否启用
      selected?: boolean; // 是否选中
      icon?: string; // 图标图片，仅支持 png
      accelerator?: string; // 快捷键
    } // 自定义菜单选项
  | { type: "menu"; label: string; enabled?: boolean; children: MenuOptions }; // 子菜单选项

// 菜单选项列表。
type MenuOptions = MenuItemOption[];

// NivaTray 的选项。
interface NivaTrayOptions {
  icon: string; // 托盘菜单的图标，仅支持 png
  title?: string; // 托盘菜单的标题
  tooltip?: string; // 托盘菜单的提示信息
  menu?: MenuOptions; // 托盘菜单的菜单选项
}

// NivaTray 的更新选项。
interface NivaTrayUpdateOptions {
  icon?: string; // 更新托盘菜单的图标，仅支持 png
  title?: string; // 更新托盘菜单的标题
  tooltip?: string; // 更新托盘菜单的提示信息
  menu?: MenuOptions; // 更新托盘菜单的菜单选项
}

// 全局快捷键的选项。
interface ShortcutOption {
  accelerator: string;
  id: number;
}

// Niva 全局快捷键的选项。
type NivaShortcutsOptions = ShortcutOption[];

interface NivaEventMap {
  /** 窗口焦点事件 */
  "window.focused": (eventName: string, focused: boolean) => void;
  /** 窗口缩放事件 */
  "window.scaleFactorChanged": (eventName: string, payload: { scaleFactor: number; newInnerSize: { width: number; height: number } }) => void;
  /** 窗口主题事件 */
  "window.themeChanged": (eventName: string, theme: "light" | "dark" | "system") => void;
  /** 窗口关闭请求事件 */
  "window.closeRequested": (eventName: string, payload: null) => void;
  /** 窗口消息事件 */
  "window.message": (eventName: string, payload: { from: number; message: string }) => void;
  /** 菜单点击事件 */
  "menu.clicked": (eventName: string, menuId: number) => void;
  /** 托盘图标右键点击事件 */
  "tray.rightClicked": (eventName: string, trayId: number) => void;
  /** 托盘图标左键点击事件 */
  "tray.leftClicked": (eventName: string, trayId: number) => void;
  /** 托盘图标双击事件 */
  "tray.doubleClicked": (eventName: string, trayId: number) => void;
  /** 全局快捷键事件 */
  "shortcut.emit": (eventName: string, shortcutId: number) => void;
  /** 文件拖拽悬停事件 */
  "fileDrop.hovered": (eventName: string, payload: { paths: string[]; position: { x: number; y: number } }) => void;
  /** 文件拖拽放置事件 */
  "fileDrop.dropped": (eventName: string, payload: { paths: string[]; position: { x: number; y: number } }) => void;
  /** 文件拖拽取消事件 */
  "fileDrop.cancelled": (eventName: string, payload: null) => void;
}

interface NivaClipboard {
  /**
   * 从系统剪贴板中读取当前所复制的文本内容。
   * 如果当前没有复制的文本内容，则返回 `null`。
   * @returns 一个 Promise，在 Promise 被解析时返回文本内容，或返回 `null`。
   */
  read(): Promise<string | null>;
  /**
   * 将给定的文本内容写入系统剪贴板，替换任何之前复制的文本内容。
   * @param text 要写入剪贴板的文本。
   * @returns 一个 Promise，在文本写入剪贴板成功时解析该 Promise，如果发生错误则拒绝该 Promise。
   */
  write(text: string): Promise<void>;
}

interface NivaDialog {
  /**
   * 显示一个独立消息框。
   * @param title 消息框的标题。
   * @param content 消息框的内容，如果为空，则使用默认值。
   * @param level 消息框的级别。
   * @returns 一个 Promise，在消息框关闭时解析该 Promise，或在发生错误时拒绝该 Promise。
   */
  showMessage(title: string, content?: string, level?: "info" | "warning" | "error"): Promise<void>;
  /**
   * 在文件系统中选择一个文件，支持过滤器和起始目录。
   * @param filters 文件类型筛选器。
   * @param start_dir 文件选择对话框的起始目录。
   * @returns 一个 Promise，在选择文件时解析该 Promise 以返回文件名或文件路径，或解析 `null`（如果没有选择文件）。
   */
  pickFile(filters?: string[], start_dir?: string): Promise<string | null>;
  /**
   * 在文件系统中选择一个文件夹，支持起始目录。
   * @param start_dir 文件夹选择对话框的起始目录。
   * @returns 一个 Promise，在选择文件夹时解析该 Promise 以返回文件夹路径，或解析 `null`（如果没有选择文件夹）。
   */
  pickDir(start_dir?: string): Promise<string | null>;
  /**
   * 在文件系统中保存一个文件，支持过滤器和起始目录。
   * @param filters 文件类型筛选器。
   * @param start_dir 文件保存对话框的起始目录。
   * @returns 一个 Promise，在保存文件时解析该 Promise 以返回文件名或文件路径，或解析 `null`（如果没有保存文件）。
   */
  saveFile(filters?: string[], start_dir?: string): Promise<string | null>;
}

interface NivaExtra {
  /**
   * 隐藏应用程序，仅适用于 macOS。
   * @returns 一个 Promise，在应用程序成功隐藏时解析该 Promise，如果发生错误则拒绝该 Promise。
   */
  hideApplication(): Promise<void>;
  /**
   * 显示应用程序，仅适用于 macOS。
   * @returns 一个 Promise，在应用程序成功显示时解析该 Promise，如果发生错误则拒绝该 Promise。
   */
  showApplication(): Promise<void>;
  /**
   * 隐藏其他应用程序，仅适用于 macOS。
   * @returns 一个 Promise，在其他应用程序成功隐藏时解析该 Promise，如果发生错误则拒绝该 Promise。
   */
  hideOtherApplication(): Promise<void>;
  /**
   * 设置应用程序的激活策略，仅适用于 macOS。
   * @param policy 要设置的激活策略。
   * @returns 一个 Promise，在激活策略成功设置时解析该 Promise，如果发生错误则拒绝该 Promise。
   */
  setActivationPolicy(policy: "regular" | "accessory" | "prohibited"): Promise<void>;
  /**
   * 获取当前活动窗口的 ID，仅适用于 macOS 和 Windows。
   * 对于 macOS，将返回 `process_id_window_id` 的格式，其中 `process_id` 和 `window_id` 为整数。
   * 对于 Windows，将返回窗口句柄的字符串形式。
   * @returns 一个 Promise，在获取成功时解析该 Promise 以返回当前活动窗口的 ID，或解析 `null`（如果没有活动窗口）。
   */
  getActiveWindowId(): Promise<string | null>;
  /**
   * 将焦点设置到特定 ID 的窗口，仅适用于 macOS 和 Windows。
   * 对于 macOS，ID 应该是 `process_id_window_id` 的格式，其中 `process_id` 和 `window_id` 为整数。
   * 对于 Windows，ID 应该是窗口句柄的字符串形式。
   * @param id_string 要设置焦点窗口的 ID 字符串。
   * @returns 一个 Promise，在设置焦点窗口时解析该 Promise，如果无法设置活动窗口则解析 `true`，如果窗口 ID 无效则解析 `false`，如果发生其他错误则拒绝该 Promise。
   */
  focusByWindowId(id_string: string): Promise<boolean>;
}

interface NivaFsStat {
  /** 是否是目录 */
  isDir: boolean;
  /** 是否是文件 */
  isFile: boolean;
  /** 是否是软连接 */
  isSymlink: boolean;
  /** 尺寸 */
  size: number;
  /** 修改时间 */
  modified: number;
  /** 是否有访问权限 */
  accessed: number;
  /** 创建时间 */
  created: number;
}

interface NivaFsOption {
  /** 是否覆盖 */
  overwrite?: boolean;
  /**  */
  skipExist?: boolean;
  bufferSize?: number;
  copyInside?: boolean;
  contentOnly?: boolean;
  depth?: number;
}

interface NivaFs {
  /**
   * 返回文件的元数据信息。
   * @param path 要获取元数据的文件路径。
   * @returns 一个 Promise，在获取元数据成功时解析该 Promise 以返回表示文件元数据的对象，或在发生错误时拒绝该 Promise。
   */
  stat(path: string): Promise<NivaFsStat>;
  /**
   * 检查文件或目录是否存在。
   * @param path 要检查的文件或目录路径。
   * @returns 一个 Promise，在检查文件或目录是否存在时解析该 Promise 以返回一个 boolean 值，表示文件或目录是否存在。
   */
  exists(path: string): Promise<boolean>;
  /**
   * 读取文件的内容，并将其作为字符串返回。
   * @param path 要读取的文件路径。
   * @param encode 要使用的编码格式。默认为 UTF-8。
   * @returns 一个 Promise，在读取文件成功时解析该 Promise 以返回文件的内容字符串，或在发生错误时拒绝该 Promise。
   */
  read(path: string, encode?: "utf8" | "base64"): Promise<string>;
  /**
   * 将字符串写入文件。
   * @param path 要写入的文件路径。
   * @param content 要写入文件的字符串。
   * @param encode 要使用的编码格式。默认为 UTF-8。
   * @returns 一个 Promise，在写入文件成功时解析该 Promise，或在发生错误时拒绝该 Promise。
   */
  write(path: string, content: string, encode?: "utf8" | "base64"): Promise<void>;
  /**
   * 将字符串追加到文件的末尾。
   * @param path 要追加的文件路径。
   * @param content 要追加到文件的字符串。
   * @param encode 要使用的编码格式。默认为 UTF-8。
   * @returns 一个 Promise，在追加字符串到文件成功时解析该 Promise，或在发生错误时拒绝该 Promise。
   */
  append(path: string, content: string, encode?: "utf8" | "base64"): Promise<void>;

  /**
   * 将文件或目录移动到新位置。
   * @param from 要移动的文件或目录的路径。
   * @param to 新位置的路径。
   * @param options 可选的参数对象，表示可选的复制选项。
   * @returns 一个 Promise，在移动文件或目录成功时解析该 Promise，或在发生错误时拒绝该 Promise。
   */
  move(from: string, to: string, options?: NivaFsOption): Promise<void>;

  /**
   * 将文件或目录复制到新位置。
   * @param from 要复制的文件或目录的路径。
   * @param to 新位置的路径。
   * @param options 可选的参数对象，表示可选的复制选项。
   * @returns 一个 Promise，在复制文件或目录成功时解析该 Promise，或在发生错误时拒绝该 Promise。
   */
  copy(from: string, to: string, options?: NivaFsOption): Promise<void>;

  /**
   * 删除文件或目录。
   * @param path 要删除的文件或目录的路径。
   * @returns 一个 Promise，在删除文件或目录时解析该 Promise，或在发生错误时拒绝该 Promise。
   */
  remove(path: string): Promise<void>;
  /**
   * 创建一个新目录。
   * @param path 要创建的目录路径。
   * @returns 一个 Promise，在创建目录成功时解析该 Promise，或在发生错误时拒绝该 Promise。
   */
  createDir(path: string): Promise<void>;
  /**
   * 创建指定的目录及其所有父目录。
   * @param path 要创建的目录路径。
   * @returns 一个 Promise，在创建目录成功时解析该 Promise，或在发生错误时拒绝该 Promise。
   */
  createDirAll(path: string): Promise<void>;
  /**
   * 读取指定目录的内容，并返回目录中的所有文件和子目录的名称。
   * @param path 要读取的目录路径。默认值为当前工作目录。
   * @returns 一个 Promise，在读取目录成功时解析该 Promise 以返回目录中的文件和子目录名称组成的字符串数组，或在发生错误时拒绝该 Promise。
   */
  readDir(path?: string): Promise<string[]>;
  /**
   * 读取指定目录（包括子目录）的内容，并返回目录中的所有文件的相对路径（相对于所提供的目录）。
   * @param path 要读取的目录路径。
   * @param excludes 一个字符串数组，包含要排除的文件路径的 glob 模式。默认为空数组。
   * @returns 一个 Promise，在读取目录中的所有文件成功时解析该 Promise 以返回所有文件的相对路径组成的字符串数组，或在发生错误时拒绝该 Promise。
   */
  readDirAll(path: string, excludes?: string[]): Promise<string[]>;
}

interface NivaHttp {
  /**
   * 发送 HTTP(s) 请求并返回响应结果，包括响应状态码、响应头和响应体。
   * @param options 请求选项，包括方法、URL、请求头、请求体和代理设置。
   * @returns 一个 Promise，在接收响应成功后解析该 Promise，或在发生错误时拒绝该 Promise。成功时返回一个包含响应状态码、响应头和响应体的对象。
   */
  request(options: { method: string; url: string; headers?: { [key: string]: string }; body?: string; proxy?: string }): Promise<{
    status: number;
    headers: { [key: string]: string };
    body: string;
  }>;
  /**
   * 发送 HTTP(s) GET 请求并返回响应结果，包括响应状态码、响应头和响应体。
   * @param url 请求的 URL。
   * @param headers 如果有，指定请求头。
   * @returns 一个 Promise，在接收响应成功后解析该 Promise，或在发生错误时拒绝该 Promise。成功时返回一个包含响应状态码、响应头和响应体的对象。
   */
  get(
    url: string,
    headers?: { [key: string]: string }
  ): Promise<{
    status: number;
    headers: { [key: string]: string };
    body: string;
  }>;
  /**
   * 发送 HTTP(s) POST 请求并返回响应结果，包括响应状态码、响应头和响应体。
   * @param url 请求的 URL。
   * @param body 请求体。
   * @param headers 如果有，指定请求头。
   * @returns 一个 Promise，在接收响应成功后解析该 Promise，或在发生错误时拒绝该 Promise。成功时返回一个包含响应状态码、响应头和响应体的对象。
   */
  post(
    url: string,
    body: string,
    headers?: { [key: string]: string }
  ): Promise<{
    status: number;
    headers: { [key: string]: string };
    body: string;
  }>;
}

interface NivaMonitorInfo {
  /** 显示器名称 */
  name: string;
  /** 大小 */
  size: { width: number; height: number };
  /** 位置 */
  position: { x: number; y: number };
  /** 物理大小 */
  physicalSize: { width: number; height: number };
  /** 物理位置 */
  physicalPosition: { x: number; y: number };
  /** 缩放比例 */
  scaleFactor: number;
}

interface NivaMonitor {
  /**
   * 列出系统中可用的所有监视器，并返回它们的信息。包括每个监视器的名称、大小、位置、物理大小、物理位置和缩放因子。
   * @returns 一个 Promise，在获取监视器信息成功时解析该 Promise，或在发生错误时拒绝该 Promise。成功时返回一个包含所有监视器信息的数组。
   */
  list(): Promise<NivaMonitorInfo[]>;
  /**
   * 获取包含当前窗口的监视器的信息，包括该监视器的名称、大小、位置、物理大小、物理位置和缩放因子。
   * @returns 一个 Promise，在获取监视器信息成功时解析该 Promise，或在发生错误时拒绝该 Promise。成功时返回包含监视器信息的对象。
   */
  current(): Promise<NivaMonitorInfo | null>;
  /**
   * 获取系统中主监视器的信息，包括该监视器的名称、大小、位置、物理大小、物理位置和缩放因子。
   * @returns 一个 Promise，在获取主监视器信息成功时解析该 Promise，或在发生错误时拒绝该 Promise。成功时返回包含监视器信息的对象。
   */
  primary(): Promise<NivaMonitorInfo | null>;
  /**
   * 获取指定坐标点所在的监视器的信息，包括该监视器的名称、大小、位置、物理大小、物理位置和缩放因子。
   * @param x 坐标点的 X 坐标。
   * @param y 坐标点的 Y 坐标。
   * @returns 一个 Promise，在获取监视器信息成功时解析该 Promise，或在发生错误时拒绝该 Promise。成功时返回包含监视器信息的对象。
   */
  fromPoint(x: number, y: number): Promise<NivaMonitorInfo | null>;
}

interface NivaOs {
  /**
   * 获取系统信息，包括操作系统类型，体系结构和版本信息。
   * @returns 一个 Promise，在获取系统信息成功时解析该 Promise，或在发生错误时拒绝该 Promise。成功时返回一个包含操作系统信息的对象。
   */
  info(): Promise<{
    /** 操作系统类型 */
    os: string;
    /** 体系结构 */
    arch: string;
    /** 版本信息 */
    version: string;
  }>;
  /**
   * 获取用户主目录以及与之相关的各种标准目录的路径。
   * @returns 一个 Promise，在获取目录信息成功时解析该 Promise，或在发生错误时拒绝该 Promise。成功时返回一个包含目录信息的对象。
   */
  dirs(): Promise<{
    temp: string;
    data: string;
    home?: string;
    audio?: string;
    desktop?: string;
    document?: string;
    download?: string;
    font?: string;
    picture?: string;
    public?: string;
    template?: string;
    video?: string;
  }>;
  /**
   * 获取系统路径分隔符。
   * @returns 一个 Promise，在获取系统路径分隔符成功时解析该 Promise，或在发生错误时拒绝该 Promise。成功时返回一个字符串，表示系统路径分隔符。
   */
  sep(): Promise<string>;
  /**
   * 获取系统换行符。
   * @returns 一个 Promise，在获取系统换行符成功时解析该 Promise，或在发生错误时拒绝该 Promise。成功时返回一个字符串，表示系统换行符。
   */
  eol(): Promise<string>;
  /**
   * 获取系统区域设置，包括语言代码、国家/地区和编码方案。
   * @returns 一个 Promise，在获取区域设置信息成功时解析该 Promise，或在发生错误时拒绝该 Promise。成功时返回一个字符串，表示系统区域设置。
   */
  locale(): Promise<string>;
}

interface ExecOptions {
  env?: Record<string, string>;
  current_dir?: string;
  detached?: boolean;
}

interface NivaProcess {
  /**
   * 获取当前进程的进程 ID。
   * @returns 一个 Promise，在获取进程 ID 成功时解析该 Promise，或在发生错误时拒绝该 Promise。成功时返回进程 ID。
   */
  pid(): Promise<number>;
  /**
   * 获取当前工作目录。
   * @returns 一个 Promise，在获取当前工作目录成功时解析该 Promise，或在发生错误时拒绝该 Promise。成功时返回当前工作目录的路径。
   */
  currentDir(): Promise<string>;
  /**
   * 获取当前可执行文件的路径。
   * @returns 一个 Promise，在获取当前可执行文件的路径成功时解析该 Promise，或在发生错误时拒绝该 Promise。成功时返回当前可执行文件的路径。
   */
  currentExe(): Promise<string>;
  /**
   * 获取系统环境变量。
   * @returns 一个 Promise，在获取系统环境变量成功时解析该 Promise，或在发生错误时拒绝该 Promise。成功时返回一个表示系统环境变量的对象。
   */
  env(): Promise<Record<string, string>>;
  /**
   * 获取命令行参数。
   * @returns 一个 Promise，在获取命令行参数成功时解析该 Promise，或在发生错误时拒绝该 Promise。成功时返回一个表示命令行参数的数组。
   */
  args(): Promise<string[]>;
  /**
   * 设置当前工作目录。
   * @param path 要设置的新的工作目录路径。
   * @returns 一个 Promise，在设置当前工作目录成功时解析该 Promise，或在发生错误时拒绝该 Promise。
   */
  setCurrentDir(path: string): Promise<void>;
  /**
   * 退出 Niva 程序。
   * @returns 一个 Promise，在退出程序成功时解析该 Promise，或在发生错误时拒绝该 Promise。
   */
  exit(): Promise<void>;
  /**
   * 在子进程中执行指定的命令。
   * @param cmd 要执行的命令。
   * @param args 命令的参数。
   * @param options 执行命令的选项。
   * @returns 一个 Promise，在执行命令成功时解析该 Promise，或在发生错误时拒绝该 Promise。成功时返回一个表示命令执行结果的对象。
   */
  exec(
    cmd: string,
    args?: string[],
    options?: ExecOptions
  ): Promise<{
    status: number | null;
    stdout: string;
    stderr: string;
  }>;
  /**
   * 打开指定的 URI。
   * @param uri 要打开的 URI。
   * @returns 一个 Promise，在打开 URI 成功时解析该 Promise，或在发生错误时拒绝该 Promise。
   */
  open(uri: string): Promise<void>;
  /**
   * 获取当前 Niva 程序的版本号。
   * @returns 一个 Promise，在获取 Niva 程序的版本号成功时解析该 Promise，或在发生错误时拒绝该 Promise。成功时返回 Niva 程序的版本号。
   */
  version(): Promise<string>;
}

interface NivaResource {
  /**
   * 检查文件或文件夹是否存在于应用程序资源中。
   * @param path 要检查的文件或文件夹路径。
   * @returns 一个 Promise，在检查成功时解析该 Promise，或在发生错误时拒绝该 Promise。成功时返回布尔值，表示该文件或文件夹是否存在。
   */
  exists(path: string): Promise<boolean>;
  /**
   * 读取虚拟文件系统中的文件。
   * @param path 要读取的文件路径。
   * @param encode 要使用的字符编码，目前支持 "utf8" 和 "base64" 两种编码方式。
   * @returns 一个 Promise，在读取文件成功时解析该 Promise，或在发生错误时拒绝该 Promise。成功时返回读取的文件内容。
   */
  read(path: string, encode?: "utf8" | "base64"): Promise<string>;
  /**
   * 将虚拟文件系统中的文件提取到本地文件系统上。
   * @param from 要提取的虚拟文件系统中的文件路径。
   * @param to 提取文件的本地文件系统路径。
   * @returns 一个 Promise，在提取文件成功时解析该 Promise，或在发生错误时拒绝该 Promise。
   */
  extract(from: string, to: string): Promise<void>;
}

interface NivaShortcut {
  /**
   * 注册一个新的窗口快捷键。
   * @param accelerator_str 快捷键的键序列，如 "Ctrl+N" 或 "Shift+Enter"。
   * @param window_id 要注册窗口快捷键的窗口 ID，默认为当前活动窗口 ID。
   * @returns 一个 Promise，在注册成功时解析该 Promise，或在发生错误时拒绝该 Promise。成功时返回新增的快捷键 ID。
   */
  register(accelerator_str: string, window_id?: number): Promise<number>;
  /**
   * 注销指定的窗口快捷键。
   * @param id 要注销的快捷键 ID。
   * @param window_id 要注销窗口快捷键的窗口 ID，默认为当前活动窗口 ID。
   * @returns 一个 Promise，在注销成功时解析该 Promise，或在发生错误时拒绝该 Promise。
   */
  unregister(id: number, window_id?: number): Promise<void>;
  /**
   * 注销指定窗口的所有快捷键。
   * @param window_id 要注销窗口快捷键的窗口 ID，默认为当前活动窗口 ID。
   * @returns 一个 Promise，在注销成功时解析该 Promise，或在发生错误时拒绝该 Promise。
   */
  unregisterAll(window_id?: number): Promise<void>;
  /**
   * 获取指定窗口的所有快捷键列表。
   * @param window_id 要获取快捷键列表的窗口 ID，默认为当前活动窗口 ID。
   * @returns 一个 Promise，在获取成功时解析该 Promise，或在发生错误时拒绝该 Promise。成功时返回快捷键列表，列表中的每个元素包含快捷键 ID 和快捷键键序列。
   */
  list(window_id?: number): Promise<{ id: number; accelerator: string }[]>;
}

interface NivaTray {
  /**
   * 在系统托盘中创建一个新的托盘图标。
   * @param options 创建托盘图标的配置项。
   * @param window_id 要创建托盘图标的窗口 ID，默认为当前活动窗口 ID。
   * @returns 一个 Promise，在创建成功时解析该 Promise，或在发生错误时拒绝该 Promise。成功时返回新创建的托盘图标 ID。
   */
  create(options: NivaTrayOptions, window_id?: number): Promise<number>;
  /**
   * 销毁指定的托盘图标。
   * @param id 要销毁的托盘图标 ID。
   * @param window_id 要销毁托盘图标的窗口 ID，默认为当前活动窗口 ID。
   * @returns 一个 Promise，在销毁成功时解析该 Promise，或在发生错误时拒绝该 Promise。
   */
  destroy(id: number, window_id?: number): Promise<void>;
  /**
   * 销毁指定窗口的所有托盘图标。
   * @param window_id 要销毁托盘图标的窗口 ID，默认为当前活动窗口 ID。
   * @returns 一个 Promise，在销毁成功时解析该 Promise，或在发生错误时拒绝该 Promise。
   */
  destroyAll(window_id?: number): Promise<void>;
  /**
   * 获取指定窗口当前存在的所有托盘图标 ID。
   * @param window_id 要获取托盘图标 ID 的窗口 ID，默认为当前活动窗口 ID。
   * @returns 一个 Promise，在获取成功时解析该 Promise，或在发生错误时拒绝该 Promise。成功时返回托盘图标 ID 的数组。
   */
  list(window_id?: number): Promise<number[]>;
  /**
   * 更新指定托盘图标的配置项。
   * @param id 要更新的托盘图标 ID。
   * @param options 新的托盘图标配置项。
   * @param window_id 要更新托盘图标的窗口 ID，默认为当前活动窗口 ID。
   * @returns 一个 Promise，在更新成功时解析该 Promise，或在发生错误时拒绝该 Promise。
   */
  update(id: number, options: NivaTrayUpdateOptions, window_id?: number): Promise<void>;
}

interface NivaWebview {
  /**
   * 检查开发工具是否打开。
   * @returns 一个 Promise，在检查成功时解析该 Promise，或在发生错误时拒绝该 Promise。成功时返回布尔值，表示开发工具是否打开。
   */
  isDevToolsOpen(): Promise<boolean>;
  /**
   * 打开开发工具。
   * @returns 一个 Promise，该 Promise 始终解析。
   */
  openDevtools(): Promise<void>;
  /**
   * 关闭开发工具。
   * @returns 一个 Promise，该 Promise 始终解析。
   */
  closeDevTools(): Promise<void>;
  /**
   * 获取应用程序的基本 URL。
   * @returns 一个 Promise，在获取成功时解析该 Promise，或在发生错误时拒绝该 Promise。成功时返回应用程序的基本 URL。
   */
  baseUrl(): Promise<string>;
  /**
   * 获取应用程序文件系统的基本 URL。
   * @returns 一个 Promise，在获取成功时解析该 Promise，或在发生错误时拒绝该 Promise。成功时返回应用程序文件系统的基本 URL。
   */
  baseFileSystemUrl(): Promise<string>;
}

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
  open(options: NivaWindowOptions): Promise<number>;
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
   * 设置窗口客户区最小大小。
   * @param size 窗口客户区最小大小。
   * @param id 需要设置大小的窗口 ID，省略则默认为当前窗口。
   * @returns 一个 Promise，在设置成功后解析该 Promise，或在发生错误时拒绝该 Promise。
   */
  setMinInnerSize(size: NivaSize, id?: number): Promise<void>;
  /**
   * 设置窗口客户区最大大小。
   * @param size 窗口客户区最大大小。
   * @param id 需要设置大小的窗口 ID，省略则默认为当前窗口。
   * @returns 一个 Promise，在设置成功后解析该 Promise，或在发生错误时拒绝该 Promise。
   */
  setMaxInnerSize(size: NivaSize, id?: number): Promise<void>;
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
  isFullscreen(id?: number): Promise<boolean>;
  /**
   * 全屏或退出全屏窗口。
   * @param isFullscreen 是否全屏。
   * @param monitorName 需要全屏到的显示器名称，或者“null”或“undefined”表示全屏到当前显示器。如果省略，会在所有可用的显示器中搜索最接近窗口的一个，并尽可能在其上对齐。
   * @param id 操作的窗口 ID，如果省略则默认为当前窗口。
   * @returns 一个 Promise，在操作成功后解析该 Promise，或在发生错误时拒绝该 Promise。
   */
  setFullscreen(isFullscreen: boolean, monitorName?: string | null, id?: number): Promise<void>;
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
  requestUserAttention(level?: "normal" | "informational" | "critical", id?: number): Promise<void>;
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
}

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
  beginResizeDrag(edge: number, button: number, x: number, y: number, id?: number): Promise<void>;
  /**
   * 设置是否将窗口从任务栏中隐藏或删除。仅适用于 Windows。
   * @param skip 是否从任务栏中隐藏或删除。
   * @param id 区别不同窗口的可选 ID。
   * @returns 如果成功则返回 Promise.resolve()，否则返回 Promise.reject()。
   */
  setSkipTaskbar(skip: boolean, id?: number): Promise<void>;
  /**
 * 设置窗口无装饰时是否显示阴影。仅适用于 macOS。

 * @param shadow 是否显示窗口阴影。仅适用于 Windows。
 * @param id 区别不同窗口的可选 ID。
 * @returns 如果成功则返回 Promise.resolve()，否则返回 Promise.reject()。
 */
  setUndecoratedShadow(shadow: boolean, id?: number): Promise<void>;
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
}
