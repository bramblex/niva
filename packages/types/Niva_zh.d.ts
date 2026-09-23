/* eslint-disable */
/* prettier-ignore */
// @ts-nocheck
export {}
declare global {
  /** Niva 运行环境扩展对象，包含事件监听器及其api */
  const Niva: NivaObj;
}

interface NivaObj {
  /** 当前原生 Bridge wire 版本。 */
  readonly bridgeVersion: number;
  /** 从同步注册表读取 Niva 或 NodeCompat 模块；未注册的名称会抛错。 */
  require(id: string): any;
  /** 注册一个页面本地模块。 */
  registerModule(id: string, implementation: any): void;
  /** 按当前 NodeCompat 资源映射动态导入，或读取同步注册表。 */
  import(id: string): Promise<any>;
  /**
   * 绑定事件监听器。
   * @param event 要监听的事件名称，可以用 `*`、`xxxx.*` 等通配符。
   * @param listener 事件被触发时要调用的函数。
   */
  addEventListener<K extends keyof NivaEventMap>(
    event: K,
    listener: NivaEventMap[K]
  ): void;
  /**
   * 移除特定的事件监听器。
   * @param event 要移除的事件名称。
   * @param listener 要移除的监听器函数。
   */
  removeEventListener<K extends keyof NivaEventMap>(
    event: K,
    listener: Function
  ): void;
  /**
   * 移除特定事件的所有监听器。
   * @param event 要移除所有监听器的事件名称。
   */
  removeAllEventListeners(event: string): void;
  /**
   * 发起非流式 API 调用。远程页面通过 IPC 执行此调用。
   */
  call(methodName: string, args: any): Promise<any>;
  /**
   * 发起流式调用，返回 { promise, cancel, id }。仅支持本地 WebSocket 页面。
   * handlers.onEvent 接收与本次调用关联的推送，onChunk 逐帧接收
   * 二进制 Uint8Array，onBlob 接收按 END 分组的完整 Blob。
   * 二进制回调的第二个参数标识 stderr 子流。
   */
  stream(
    methodName: string,
    args: any,
    handlers?: {
      onEvent?: (name: string, data: any) => void;
      onChunk?: (chunk: Uint8Array, isStderr: boolean) => void;
      onBlob?: (blob: Blob, isStderr: boolean) => void;
    }
  ): {
    id: number;
    promise: Promise<any>;
    cancel: () => void;
  };
  /**
   * 向流式调用发送二进制分片（stdin 式输入），end 为 true 表示结束。仅支持本地 WebSocket 页面。
   */
  streamSend(id: number, data: ArrayBuffer | Uint8Array | string, end?: boolean): boolean;
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
    /** stdio 父进程消息桥，仅 --stdio 启动时可用 */
    host: NivaHost;
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

/** Niva应用程序的选项 */
interface NivaOptions {
  /** 应用名称 */
  name: string;
  /** 应用唯一标识符 */
  uuid: string;
  /** Devtools 显示并写入 macOS/Windows 包元数据的应用版本。 */
  version?: string;
  /** 应用图标文件路径，仅支持 png */
  icon?: string;
  /** 可选的包元数据；版本从顶层 version 读取。 */
  meta?: { companyName?: string; description?: string; copyright?: string };

  /** 应用程序窗口的选项 */
  window?: NivaWindowOptions;
  /** 应用程序托盘的选项 */
  tray?: NivaTrayOptions;
  /** 应用程序全局快捷键的选项 */
  shortcuts?: NivaShortcutsOptions;

  /** 显式调试启动使用的资源目录与开发入口；正常打包启动忽略 debug.entry。 */
  debug?: { resource?: string; entry?: string };
  /** Devtools 打包时读取的静态资源目录。 */
  build?: { resource?: string };

  /** API 调度器选项（全异步运行时；旧的固定线程池 workers 已移除） */
  api?: NivaApiOptions;
  /** 可选 Node 形浏览器模块；只打包选中的模块资源。 */
  nodeCompat?: boolean | {
    modules?: NivaNodeCompatModule[];
    importmap?: boolean;
  };
  /** 应用签名选项（用户自备证书；devtools 在构建成功后自动执行） */
  sign?: NivaSignOptions;

  /** 应用程序的激活策略,仅Mac */
  activationPolicy?: "regular" | "accessory" | "prohibited";
  /** 是否使用默认菜单创建方式,仅Mac */
  defaultMenuCreation?: boolean;
  /** 是否忽略其他应用程序的激活状态而强制激活应用程序,仅Mac */
  activateIgnoringOtherApps?: boolean;

  /** 专为Mac单独使用的配置 */
  macos?: Partial<NivaOptions>;
  /** 专为windows单独使用的配置 */
  windows?: Partial<NivaOptions>;
}

type NivaNodeCompatModule =
  | "path" | "os" | "fs" | "child_process" | "events" | "util"
  | "querystring" | "buffer" | "url" | "crypto" | "zlib"
  | "http" | "https" | "assert" | "stream";

/** 应用签名选项 */
type NivaSignOptions = {
  /** macOS 签名：identity 为 codesign 身份（如 "Developer ID Application: Foo (TEAMID)"） */
  macos?: {
    identity: string;
    /** 项目相对路径的 entitlements plist（可选） */
    entitlements?: string;
    notarize?: {
      /** notarytool 钥匙串 profile（推荐，零秘密） */
      profile?: string;
      /** Apple ID（与 teamId 配合，需 env NIVA_APPLE_ID_PASSWORD） */
      appleId?: string;
      teamId?: string;
    };
  };
  /** Windows 签名：pfx 为项目相对路径，密码走 env NIVA_WIN_CERT_PASSWORD */
  windows?: {
    pfx: string;
    timestamp?: string;
  };
};

/** API 调度器选项 */
type NivaApiOptions = {
  /** 单个请求超时毫秒数（默认 30000；流式长任务可设更大） */
  timeoutMs?: number;
  /** 调度队列上限，超限立即拒绝（默认 64） */
  maxQueue?: number;
};

/** 尺寸 */
type NivaSize = {
  /** 宽 */
  width: number;
  /** 高 */
  height: number;
};

/** 位置 */
type NivaPosition = {
  /** 距离坐标原点横轴距离 */
  x: number;
  /** 距离坐标原点纵轴距离 */
  y: number;
};

/** Tao 窗口缩放边方向。 */
type NivaWindowResizeDirection =
  | "east" | "north" | "northEast" | "northWest"
  | "south" | "southEast" | "southWest" | "west";

/** 任务栏进度状态。部分平台会将 indeterminate/paused/error 显示为普通进度。 */
type NivaWindowProgressState = "none" | "normal" | "indeterminate" | "paused" | "error";

/** Tao 窗口任务栏进度选项。progress 取 0 到 100。 */
type NivaWindowProgressBar = {
  state?: NivaWindowProgressState | null;
  progress?: number | null;
  /** Unity 桌面环境的 .desktop 文件名，仅 Linux 使用。 */
  desktopFilename?: string | null;
};

/** RGBA 颜色分量，取 0 到 255。 */
type NivaRGBA = [number, number, number, number];

/** 窗口根菜单 */
interface WindowRootMenu {
  /** 菜单项的名称 */
  label: string;
  /** 是否启用 */
  enabled?: boolean;
  /** 子菜单项 */
  children: MenuOptions;
}

/** 窗口根菜单集合 */
type WindowMenuOptions = Array<WindowRootMenu>;

/** 窗口选项 */
interface NivaWindowOptions {
  /** 本地入口路径或绝对远端 URL；默认 index.html。远端页面默认没有原生 API 权限。 */
  entry?: string;
  /** 是否启用开发者工具 */
  devtools?: boolean;
  /** 当前窗口的外源 IPC 授权。键为精确 origin（协议、主机、端口），值为方法名或 namespace.*；默认无权限。流式方法始终不可授权。 */
  permissions?: Record<string, string[]>;

  /** 窗口标题 */
  title?: string;
  /** 窗口图标 */
  icon?: string;
  /** 窗口主题 */
  theme?: string;
  /** 窗口大小 */
  size?: NivaSize;
  /** 窗口最小尺寸 */
  minSize?: NivaSize;
  /** 窗口最大尺寸 */
  maxSize?: NivaSize;

  /** 窗口位置 */
  position?: NivaPosition;

  /** 是否可调整大小 */
  resizable?: boolean;
  /** 是否可最小化 */
  minimizable?: boolean;
  /** 是否可最大化 */
  maximizable?: boolean;
  /** f */
  closable?: boolean;

  /** 是否全屏 */
  fullscreen?: boolean;
  /** 是否最大化 */
  maximized?: boolean;
  /** 是否可见 */
  visible?: boolean;
  /** 是否透明 */
  transparent?: boolean;
  /** 是否显示窗口装饰 */
  decorations?: boolean;

  /** 是否始终置顶 */
  alwaysOnTop?: boolean;
  /** 是否始终置底 */
  alwaysOnBottom?: boolean;
  /** 是否在多个工作区显示 */
  visibleOnAllWorkspaces?: boolean;

  /** 是否聚焦 */
  focused?: boolean;
  /** 是否启用内容保护 */
  contentProtection?: boolean;

  // macOS extra
  /** 父窗口 ID，仅Mac */
  parentWindow?: number;
  /** 标题栏交通灯按钮位置，仅Mac */
  trafficLightInset?: NivaPosition;
  /** 是否可点击窗口背景移动窗口，仅Mac */
  movableByWindowBackground?: boolean;
  /** 标题栏是否透明，仅Mac */
  titleBarTransparent?: boolean;
  /** 标题栏是否隐藏，仅Mac */
  titleBarHidden?: boolean;
  /** 标题栏按钮是否隐藏，仅Mac*/
  titleBarButtonsHidden?: boolean;
  /** 标题是否隐藏，仅Mac */
  titleHidden?: boolean;
  /** 是否全尺寸显示内容，仅Mac */
  fullSizeContentView?: boolean;
  /** 窗口调整尺寸步长，仅Mac */
  resizeIncrements?: NivaSize;
  /** 是否禁用高 DPI，仅Mac */
  disallowHiDpi?: boolean;
  /** 是否显示阴影，仅Mac */
  hasShadow?: boolean;
  /** 是否支持多个窗口进行选项卡式浏览，仅Mac */
  automaticWindowTabbing?: boolean;
  /** 设置选项卡式浏览的标题，仅Mac */
  tabbingIdentifier?: string;

  // windows extra
  /** 父窗口 ID */
  /** 拥有者窗口 ID */
  ownerWindow?: number;
  /** 任务栏图标 */
  taskbarIcon?: string;
  /** 在任务栏中是否显示 */
  skipTaskbar?: boolean;
  /** 是否显示窗口阴影 */
  undecoratedShadow?: boolean;

  /** 窗口菜单选项 */
  menu?: WindowMenuOptions;
}

/** 系统原生菜单项标签；序列化为 Rust serde 的 camelCase 字符串。 */
type NativeLabel =
  | "hide" | "services" | "hideOthers" | "showAll" | "closeWindow"
  | "quit" | "copy" | "cut" | "undo" | "redo" | "selectAll"
  | "paste" | "enterFullScreen" | "minimize" | "zoom" | "separator";

/** 菜单项选项枚举类型 */
type MenuItemOption =
  /** 本地菜单选项 */
  | { type: "native"; label: NativeLabel }
  /** 自定义菜单选项 */
  | {
      type: "item";
      id: number;
      /** 显示的文本 */
      label: string;
      /** 是否启用 */
      enabled?: boolean;
      /** 是否选中 */
      selected?: boolean;
      /** 图标图片，仅支持 png */
      icon?: string;
      /** 菜单快捷键。Windows 当前仅显示组合键，tao 事件循环未接入 TranslateAcceleratorW，因此不会触发菜单项。 */
      accelerator?: string;
    }
  /** 子菜单选项 */
  | { type: "menu"; label: string; enabled?: boolean; children: MenuOptions };

/** 菜单选项列表 */
type MenuOptions = MenuItemOption[];

/** 托盘选项 */
interface NivaTrayOptions {
  /** 托盘的图标，仅支持 png */
  icon: string;
  /** 托盘的标题 */
  title?: string;
  /** 托盘的提示信息 */
  tooltip?: string;
  /** 托盘菜单的菜单选项 */
  menu?: MenuOptions;
}

/** 托盘的更新选项 */
interface NivaTrayUpdateOptions {
  /** 托盘的图标，仅支持 png */
  icon?: string;
  /** 托盘的标题 */
  title?: string;
  /** 托盘的提示信息 */
  tooltip?: string;
  /** 托盘菜单的菜单选项 */
  menu?: MenuOptions;
}

/** 全局快捷键的选项 */
interface ShortcutOption {
  accelerator: string;
  id: number;
}

/** 全局快捷键的选项集合 */
type NivaShortcutsOptions = ShortcutOption[];

/** Wry WebView 权限种类 */
type NivaWebviewPermissionKind =
  | "microphone"
  | "camera"
  | "geolocation"
  | "notifications"
  | "clipboard-read"
  | "display-capture"
  | "midi"
  | "sensors"
  | "media-key-system-access"
  | "local-fonts"
  | "window-management"
  | "pointer-lock"
  | "automatic-downloads"
  | "file-system-access"
  | "autoplay"
  | "other";

/** Niva 事件集合 */
interface NivaEventMap {
  /** WebView 页面加载完成；由 Wry 的 Finished 回调触发，仅本地 WebSocket 页面可接收。 */
  "webview.loaded": (eventName: string, payload: { url: string }) => void;
  /** target=_blank/window.open 请求；Niva 默认拒绝。url 是目标地址，pageUrl 是事件投递时当前顶层页面地址，不能代表 iframe 的实际发起地址。 */
  "webview.newWindowRequested": (
    eventName: string,
    payload: {
      url: string;
      pageUrl: string | null;
      decision: "denied";
    }
  ) => void;
  /** 下载开始请求；Niva 默认拒绝。url 是下载资源地址，pageUrl 是事件投递时当前顶层页面地址。 */
  "webview.downloadStarted": (
    eventName: string,
    payload: {
      url: string;
      pageUrl: string | null;
      decision: "denied";
    }
  ) => void;
  /** Wry 权限请求被 Niva 拒绝。Wry 回调不提供请求来源 URL；pageUrl 是事件投递时当前顶层页面地址。 */
  "webview.permissionDenied": (
    eventName: string,
    payload: {
      kind: NivaWebviewPermissionKind;
      pageUrl: string | null;
      decision: "denied";
    }
  ) => void;
  /** 父进程经 stdin 发送的消息，仅主窗口收到 */
  "host:message": (
    eventName: string,
    message: { name: string; data?: unknown }
  ) => void;
  /** 窗口焦点事件 */
  "window.focused": (eventName: string, focused: boolean) => void;
  /** 窗口缩放事件 */
  "window.scaleFactorChanged": (
    eventName: string,
    payload: {
      scaleFactor: number;
      newInnerSize: { width: number; height: number };
    }
  ) => void;
  /** 窗口主题事件 */
  "window.themeChanged": (
    eventName: string,
    theme: "light" | "dark" | "system"
  ) => void;
  /** 窗口关闭请求事件 */
  "window.closeRequested": (eventName: string, payload: null) => void;
  /** 窗口消息事件 */
  "window.message": (
    eventName: string,
    payload: { from: number; message: string }
  ) => void;
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
  "fileDrop.hovered": (
    eventName: string,
    payload: { paths: string[]; position: { x: number; y: number } }
  ) => void;
  /** 文件拖拽放置事件 */
  "fileDrop.dropped": (
    eventName: string,
    payload: { paths: string[]; position: { x: number; y: number } }
  ) => void;
  /** 文件拖拽取消事件 */
  "fileDrop.cancelled": (eventName: string, payload: null) => void;
  [k: string]: (eventName: string, payload: any) => void;
}

/** stdio 父进程消息桥 */
interface NivaHost {
  /** 向父进程 stdout 发送一条消息。仅主窗口可以调用。 */
  send(name: string, data?: unknown): Promise<void>;
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
   * @param content 消息框的内容；省略时传入空字符串。
   * @param level 消息框的级别。
   * @returns 一个 Promise，在消息框关闭时解析该 Promise，或在发生错误时拒绝该 Promise。
   */
  showMessage(
    title: string,
    content?: string,
    level?: "info" | "warning" | "error"
  ): Promise<void>;
  /**
   * 在文件系统中选择一个文件，支持过滤器和起始目录。
   * @param filters 文件类型筛选器。
   * @param startDir 文件选择对话框的起始目录。
   * @returns 一个 Promise，在选择文件时解析该 Promise 以返回文件名或文件路径，或解析 `null`（如果没有选择文件）。
   */
  pickFile(filters?: string[], startDir?: string): Promise<string | null>;
  /**
   * 在文件系统中选择多个文件；取消对话框时解析为 null。
   * @param filters 文件类型筛选器。
   * @param startDir 文件选择对话框的起始目录。
   * @returns 选择的文件路径数组，或在取消时返回 null。
   */
  pickFiles(filters?: string[], startDir?: string): Promise<string[] | null>;
  /**
   * 在文件系统中选择一个文件夹，支持起始目录。
   * @param startDir 文件夹选择对话框的起始目录。
   * @returns 一个 Promise，在选择文件夹时解析该 Promise 以返回文件夹路径，或解析 `null`（如果没有选择文件夹）。
   */
  pickDir(startDir?: string): Promise<string | null>;
  /**
   * 在文件系统中选择多个文件夹；取消对话框时解析为 null。
   * @param startDir 文件夹选择对话框的起始目录。
   * @returns 选择的文件夹路径数组，或在取消时返回 null。
   */
  pickDirs(startDir?: string): Promise<string[] | null>;
  /**
   * 在文件系统中保存一个文件，支持过滤器和起始目录。
   * @param filters 文件类型筛选器。
   * @param startDir 文件保存对话框的起始目录。
   * @returns 一个 Promise，在保存文件时解析该 Promise 以返回文件名或文件路径，或解析 `null`（如果没有保存文件）。
   */
  saveFile(filters?: string[], startDir?: string): Promise<string | null>;
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
  hideOtherApplications(): Promise<void>;
  /**
   * 设置应用程序的激活策略，仅适用于 macOS。
   * @param policy 要设置的激活策略。
   * @returns 一个 Promise，在激活策略成功设置时解析该 Promise，如果发生错误则拒绝该 Promise。
   */
  setActivationPolicy(
    policy: "regular" | "accessory" | "prohibited"
  ): Promise<void>;
  /**
   * 获取当前活动窗口的 ID，仅适用于 macOS 和 Windows。
   * 对于 macOS，将返回 `process_id_window_id` 的格式，其中 `process_id` 和 `window_id` 为整数。
   * 对于 Windows，将返回窗口句柄的字符串形式。
   * @returns macOS 返回活动窗口 ID 或无活动窗口时的 null；Windows 始终返回前台窗口句柄字符串。
   */
  getActiveWindowId(): Promise<string | null>;
  /**
   * 将焦点设置到特定 ID 的窗口，仅适用于 macOS 和 Windows。
   * 对于 macOS，ID 应该是 `process_id_window_id` 的格式，其中 `process_id` 和 `window_id` 为整数。
   * 对于 Windows，ID 应该是窗口句柄的字符串形式。
   * @param idString 要设置焦点窗口的 ID 字符串。
   * @returns macOS 返回是否成功激活；Windows 调用 SetForegroundWindow 后解析为 void（Windows API 返回的 BOOL 不会透传）。ID 格式错误或 bridge 错误会拒绝。
   */
  focusByWindowId(idString: string): Promise<boolean | void>;
}

interface NivaFsStat {
  /** 是否是目录 */
  isDir: boolean;
  /** 是否是文件 */
  isFile: boolean;
  /** 当前实现跟随符号链接，不报告路径本身是否为软链接。 */
  isSymlink: boolean;
  /** 尺寸 */
  size: number;
  /** 修改时间 */
  modified: number;
  /** 最近访问时间，Unix 毫秒时间戳。 */
  accessed: number;
  /** 创建时间 */
  created: number;
}

interface NivaFsOption {
  /** 是否覆盖 */
  overwrite?: boolean;
  /**  */
  skipExist?: boolean;
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
  write(
    path: string,
    content: string,
    encode?: "utf8" | "base64"
  ): Promise<void>;
  /**
   * 将字符串追加到文件的末尾。
   * @param path 要追加的文件路径。
   * @param content 要追加到文件的字符串。
   * @param encode 要使用的编码格式。默认为 UTF-8。
   * @returns 一个 Promise，在追加字符串到文件成功时解析该 Promise，或在发生错误时拒绝该 Promise。
   */
  append(
    path: string,
    content: string,
    encode?: "utf8" | "base64"
  ): Promise<void>;

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
   * @param options 请求选项，包括方法、URL、请求头和请求体。原生请求禁用代理。
   * @returns 一个 Promise，在接收响应成功后解析该 Promise，或在发生错误时拒绝该 Promise。成功时返回一个包含响应状态码、响应头和响应体的对象。
   */
  request(options: {
    method: string;
    url: string;
    headers?: { [key: string]: string };
    body?: string;
  }): Promise<{
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
  /** Rust serde camelCase option used by process.execStream. */
  currentDir?: string;
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
  ): Promise<number | {
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
   * 检查已打包文件是否存在于应用程序资源索引中。
   * @param path 要检查的文件路径。
   * @returns 一个 Promise，在检查成功时解析该 Promise，或在发生错误时拒绝该 Promise。
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
   * @param window_id 要注册窗口快捷键的窗口 ID，默认为发起调用的窗口 ID。
   * @returns 一个 Promise，在注册成功时解析该 Promise，或在发生错误时拒绝该 Promise。成功时返回新增的快捷键 ID。
   */
  register(accelerator_str: string, window_id?: number): Promise<number>;
  /**
   * 注销指定的窗口快捷键。
   * @param id 要注销的快捷键 ID。
   * @param window_id 要注销窗口快捷键的窗口 ID，默认为发起调用的窗口 ID。
   * @returns 一个 Promise，在注销成功时解析该 Promise，或在发生错误时拒绝该 Promise。
   */
  unregister(id: number, window_id?: number): Promise<void>;
  /**
   * 注销指定窗口的所有快捷键。
   * @param window_id 要注销窗口快捷键的窗口 ID，默认为发起调用的窗口 ID。
   * @returns 一个 Promise，在注销成功时解析该 Promise，或在发生错误时拒绝该 Promise。
   */
  unregisterAll(window_id?: number): Promise<void>;
  /**
   * 获取指定窗口的所有快捷键列表。
   * @param window_id 要获取快捷键列表的窗口 ID，默认为发起调用的窗口 ID。
   * @returns 一个 Promise，在获取成功时解析该 Promise，或在发生错误时拒绝该 Promise。成功时返回快捷键列表，列表中的每个元素包含快捷键 ID 和快捷键键序列。
   */
  list(window_id?: number): Promise<{ id: number; accelerator: string }[]>;
}

interface NivaTray {
  /**
   * 在系统托盘中创建一个新的托盘图标。
   * @param options 创建托盘图标的配置项。
   * @param window_id 要创建托盘图标的窗口 ID，默认为发起调用的窗口 ID。
   * @returns 一个 Promise，在创建成功时解析该 Promise，或在发生错误时拒绝该 Promise。成功时返回新创建的托盘图标 ID。
   */
  create(options: NivaTrayOptions, window_id?: number): Promise<number>;
  /**
   * 销毁指定的托盘图标。
   * @param id 要销毁的托盘图标 ID。
   * @param window_id 要销毁托盘图标的窗口 ID，默认为发起调用的窗口 ID。
   * @returns 一个 Promise，在销毁成功时解析该 Promise，或在发生错误时拒绝该 Promise。
   */
  destroy(id: number, window_id?: number): Promise<void>;
  /**
   * 销毁指定窗口的所有托盘图标。
   * @param window_id 要销毁托盘图标的窗口 ID，默认为发起调用的窗口 ID。
   * @returns 一个 Promise，在销毁成功时解析该 Promise，或在发生错误时拒绝该 Promise。
   */
  destroyAll(window_id?: number): Promise<void>;
  /**
   * 获取指定窗口当前存在的所有托盘图标 ID。
   * @param window_id 要获取托盘图标 ID 的窗口 ID，默认为发起调用的窗口 ID。
   * @returns 一个 Promise，在获取成功时解析该 Promise，或在发生错误时拒绝该 Promise。成功时返回托盘图标 ID 的数组。
   */
  list(window_id?: number): Promise<number[]>;
  /**
   * 更新指定托盘图标的配置项。
   * @param id 要更新的托盘图标 ID。
   * @param options 新的托盘图标配置项。
   * @param window_id 要更新托盘图标的窗口 ID，默认为发起调用的窗口 ID。
   * @returns 一个 Promise，在更新成功时解析该 Promise，或在发生错误时拒绝该 Promise。
   */
  update(
    id: number,
    options: NivaTrayUpdateOptions,
    window_id?: number
  ): Promise<void>;
}

interface NivaWebview {
  /** 在当前 WebView 主页面执行脚本。仅接受 UTF-8 字节数不超过 64 KiB 的脚本，不返回脚本结果。 */
  evaluateScript(script: string): Promise<void>;
  /** 将当前 WebView 导航到绝对 HTTP(S) URL。相对 URL 和其他 scheme 会拒绝；可用 baseUrl() 拼接本地资源地址。 */
  loadUrl(url: string): Promise<void>;
  /** 将 HTML 文本加载到当前 WebView；此接口不接收 base URL。 */
  loadHtml(html: string): Promise<void>;
  /** 重新加载当前页面。 */
  reload(): Promise<void>;
  /** 获取当前页面 URL。 */
  url(): Promise<string>;
  /** 打开系统打印流程。 */
  print(): Promise<void>;
  /** 后退到上一条历史记录。 */
  goBack(): Promise<void>;
  /** 前进到下一条历史记录。 */
  goForward(): Promise<void>;
  /** 查询是否可以后退。 */
  canGoBack(): Promise<boolean>;
  /** 查询是否可以前进。 */
  canGoForward(): Promise<boolean>;
  /** 获取当前 Wry WebContext 共享数据存储中的 Cookie，返回 Set-Cookie 格式字符串。Android 上 Wry 返回空数组。 */
  cookies(): Promise<string[]>;
  /** 获取指定 URL 可用的 Cookie，返回 Set-Cookie 格式字符串；筛选遵循各平台 Wry 实现。 */
  cookiesForUrl(url: string): Promise<string[]>;
  /** 设置共享 WebContext 中的 Cookie，参数使用 Set-Cookie 格式字符串；其他窗口可能同时看到此 Cookie。Android 上会以不支持错误拒绝。 */
  setCookie(cookie: string): Promise<void>;
  /** 从共享 WebContext 删除 Cookie，参数传入含 name、domain、path 的 Set-Cookie 格式字符串；其他窗口可能同时看到此变更。Android 上会以不支持错误拒绝。 */
  deleteCookie(cookie: string): Promise<void>;
  /** 请求清理底层 WebView 数据存储的全部浏览数据；同一存储上下文中的其他窗口也可能受影响。 */
  clearAllBrowsingData(): Promise<void>;
  /**
   * 检查开发工具是否打开。
   * @returns 一个 Promise，在检查成功时解析该 Promise，或在发生错误时拒绝该 Promise。成功时返回布尔值，表示开发工具是否打开。
   */
  isDevtoolsOpen(): Promise<boolean>;
  /**
   * 打开开发工具。
   * @returns 一个 Promise，该 Promise 始终解析。
   */
  openDevtools(): Promise<void>;
  /**
   * 关闭开发工具。
   * @returns 一个 Promise，该 Promise 始终解析。
   */
  closeDevtools(): Promise<void>;
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
   * @param taskbarIcon 任务栏图标的应用资源路径。
   * @param id 区别不同窗口的可选 ID。
   * @returns 如果成功则返回 Promise.resolve()，否则返回 Promise.reject()。
   */
  setTaskbarIcon(taskbarIcon: string, id?: number): Promise<void>;
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
   * @param hasShadow 是否显示阴影。
   * @param id 区分不同窗口的可选 ID。
   * @returns 如果成功则返回 Promise.resolve()，否则返回 Promise.reject()。
   */
  setHasShadow(hasShadow: boolean, id?: number): Promise<void>;
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
