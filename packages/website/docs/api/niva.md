---
sidebar_position: 0
---

# Niva API 总览

`Niva` 是页面启动时注入的运行时对象。调用方法以 Promise 为主；本地页面使用本机 WebSocket，获授权的远端页面使用 unary JSON IPC。两种传输能力不同，详见[Bridge 说明](./bridge)。

## 基本调用

```ts
// Niva.call 接受原生方法名和位置参数数组。
const windowId = await Niva.call("window.current", []);

// Niva.api 是同一套 API 的命名空间代理。
const title = await Niva.api.window.title();

// 原生 API 返回或 handler 错误会分别 resolve/reject 该 Promise。
```

`Niva.api[namespace][method](...args)` 通常转发到 `Niva.call(namespace + "." + method, args)`。文件读写、HTTP、进程执行和资源读取等少数方法在本地 WebSocket 页面由初始化脚本封装为流式 Promise；这些 wrapper 不会令远端 IPC 自动获得流能力。

## Bridge 方法

### 事件

```ts
Niva.addEventListener(event, listener): void;
Niva.removeEventListener(event, listener): void;
Niva.removeAllEventListeners(event): void;
```

事件回调形状是 `(eventName, payload)`。受支持事件包括窗口焦点/主题/关闭、窗口间消息、菜单、托盘、快捷键、文件拖放、WebView 导航/下载/权限拒绝以及 stdio 宿主消息。完整 payload 类型见 `packages/types/Niva_zh.d.ts` 的 `NivaEventMap`；逐类事件见[事件文档](../event)。远端 IPC 本身不提供事件订阅或广播。

WebView 相关事件还包括 `webview.loaded`、`webview.newWindowRequested`、`webview.downloadStarted` 和 `webview.permissionDenied`。Niva 拒绝新窗口与下载；它只对 `camera`、`microphone`、`display-capture`、`other` 权限种类返回拒绝，`geolocation` 在 Wry 会调用 handler 的平台上也拒绝；macOS Wry 0.57 没有 geolocation 权限回调，无法由此 handler 拦截。其他权限种类返回 Wry `Default`。`webview.permissionDenied` 只观察实际被拒绝的请求，不能在事件回调里放行。这些是 WebView 能力请求，与远端 IPC 的 `permissions` 配置不是同一套授权：

```ts
Niva.addEventListener("webview.newWindowRequested", (_eventName, request) => {
  console.log(request.url, request.pageUrl, request.decision); // decision: "denied"
});

Niva.addEventListener("webview.permissionDenied", (_eventName, request) => {
  console.log(request.kind, request.pageUrl, request.decision);
});
```

`pageUrl` 是事件投递时的顶层页面 URL，不一定是 iframe 的发起地址；权限拒绝回调本身不提供请求 frame URL。`webview.loaded` 只推送给本地 WebSocket 页面。远端 IPC 不提供这些事件。

### 调用与流

```ts
Niva.call(methodName: string, args: any): Promise<any>;
Niva.stream(methodName, args, handlers?: {
  onEvent?: (name: string, data: any) => void;
  onChunk?: (chunk: Uint8Array, isStderr: boolean) => void;
  onBlob?: (blob: Blob, isStderr: boolean) => void;
}): {
  id: number;
  promise: Promise<any>;
  cancel: () => void;
};
Niva.streamSend(id, data, end?): boolean;
```

`Niva.stream` / `streamSend` 仅用于本地 WebSocket 页面。流式方法、二进制帧和 stdin 发送示例见[流式调用](./stream)。

### 模块注册

```ts
Niva.bridgeVersion: number;
Niva.registerModule(id: string, implementation: any): void;
Niva.require(id: string): any;
Niva.import(id: string): Promise<any>;
```

`Niva.require("niva:fs")` 读取内置的 Niva API 命名空间；`Niva.registerModule(id, value)` 可注册页面自己的同步模块。NodeCompat classic 入口加载后，也能按已选模块名称调用 `Niva.require("path")`。`Niva.import(id)` 会优先按 NodeCompat 资源映射动态导入，未命中时读取同步模块注册表。Niva 不会默认注册 Node.js 核心模块；宿主桥通过 `Niva.api.host` 使用。参见[NodeCompat](./node-compat)。

## 原生 API 命名空间

| 页面 | 命名空间 | 说明 |
| --- | --- | --- |
| [剪切板](./clipboard) | `clipboard` | 文本读写。 |
| [对话框](./dialog) | `dialog` | 消息框、文件与目录选择。 |
| [系统额外 API](./extra) | `extra` | 应用级窗口焦点和 macOS 应用控制。 |
| [文件系统](./fs) | `fs` | 路径文件操作与流式读写。 |
| [HTTP](./http) | `http` | HTTP(S) 请求和响应流。 |
| [监视器](./monitor) | `monitor` | 显示器枚举与位置。 |
| [操作系统](./os) | `os` | 平台、目录和区域信息。 |
| [进程](./process) | `process` | 当前进程信息、命令执行和打开 URI。 |
| [资源](./resource) | `resource` | 应用打包资源读取与提取。 |
| [快捷键](./shortcut) | `shortcut` | 全局快捷键管理。 |
| [托盘](./tray) | `tray` | 系统托盘管理。 |
| [WebView](./webview) | `webview` | 当前窗口 WebView 操作。 |
| [窗口](./window) | `window` | 窗口控制和窗口间消息。 |
| [窗口扩展](./window_extra) | `windowExtra` | 平台额外窗口能力。 |
| [宿主桥](./stdio) | `host` | `--stdio` 模式下向父进程发送消息。 |

API 签名见对应命名空间页面；区分 Rust 原生 handler 与初始化脚本中的 JS wrapper 时，参阅[Bridge 说明](./bridge#js-wrappers)。远端权限入口见[权限说明](./permissions)。
