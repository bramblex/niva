# 事件

## WebView事件

| 事件 | 数据与边界 |
| --- | --- |
| `webview.loaded` | `{ url: string }`；Wry 页面加载完成时发送，仅本地 WebSocket 页面接收。 |
| `webview.newWindowRequested` | `{ url, pageUrl, decision: "denied" }`；`target=_blank` / `window.open` 默认拒绝。 |
| `webview.downloadStarted` | `{ url, pageUrl, decision: "denied" }`；下载默认拒绝。 |
| `webview.permissionDenied` | `{ kind, pageUrl, decision: "denied" }`；对 Niva 明确拒绝的 Wry 权限请求发送。 |

以上 `pageUrl` 是事件投递时读取的顶层页面地址，不代表跨源 iframe 的实际发起地址，不能当作来源授权依据。Wry 的权限回调不提供请求源 URL；macOS 当前也无法通过该回调拦截定位权限。

## 窗口事件

### window.focused

窗口焦点事件，当窗口获得或失去焦点时触发。

```ts
Niva.addEventListener(
  "window.focused",
  (eventName: string, focused: boolean) => {
    // do somethings...
  }
);
```

### window.scaleFactorChanged

窗口缩放事件，当窗口缩放比例改变时触发。

```ts
Niva.addEventListener(
  "window.scaleFactorChanged",
  (
    eventName: string,
    payload: {
      scaleFactor: number;
      newInnerSize: { width: number; height: number };
    }
  ) => {
    // do somethings...
  }
);
```

### window.themeChanged

窗口主题事件，当窗口主题改变时触发。

```ts
Niva.addEventListener(
  "window.themeChanged",
  (eventName: string, theme: "light" | "dark" | "system") => {
    // do somethings...
  }
);
```

### window.closeRequested

窗口关闭请求事件，当用户请求关闭窗口时触发。

```ts
Niva.addEventListener(
  "window.closeRequested",
  (eventName: string, payload: null) => {
    // do somethings...
  }
);
```

### window.message

窗口消息事件，当窗口接收到来自其他窗口的消息时触发。

```ts
Niva.addEventListener(
  "window.message",
  (eventName: string, payload: { from: number; message: string }) => {
    // do somethings...
  }
);
```

## 菜单事件

### menu.clicked

菜单点击事件，当菜单被点击时触发。

```ts
Niva.addEventListener("menu.clicked", (eventName: string, menuId: number) => {
  // do somethings...
});
```

## 托盘图标事件

### tray.rightClicked

托盘图标右键点击事件，当用户右键点击托盘图标时触发。

```ts
Niva.addEventListener(
  "tray.rightClicked",
  (eventName: string, trayId: number) => {
    // do somethings...
  }
);
```

### tray.leftClicked

托盘图标左键点击事件，当用户左键点击托盘图标时触发。

```ts
Niva.addEventListener(
  "tray.leftClicked",
  (eventName: string, trayId: number) => {
    // do somethings...
  }
);
```

### tray.doubleClicked

托盘图标双击事件，当用户双击托盘图标时触发。

```ts
Niva.addEventListener(
  "tray.doubleClicked",
  (eventName: string, trayId: number) => {
    // do somethings...
  }
);
```

## 全局快捷键事件

### shortcut.emit

全局快捷键事件，当全局快捷键被触发时触发。

```ts
Niva.addEventListener(
  "shortcut.emit",
  (eventName: string, shortcutId: number) => {
    // do somethings...
  }
);
```

## 文件拖拽事件

### fileDrop.hovered

文件拖拽悬停事件，当用户在窗口中拖动文件并将其悬停时触发。

```ts
Niva.addEventListener(
  "fileDrop.hovered",
  (
    eventName: string,
    payload: { paths: string[]; position: { x: number; y: number } }
  ) => {
    // do somethings...
  }
);
```

### fileDrop.dropped

文件拖拽放置事件，当用户在窗口中拖动文件并将其放置时触发。

```ts
Niva.addEventListener(
  "fileDrop.dropped",
  (
    eventName: string,
    payload: { paths: string[]; position: { x: number; y: number } }
  ) => {
    // do somethings...
  }
);
```

### fileDrop.cancelled

文件拖拽取消事件，当用户取消文件拖拽操作时触发。

```ts
Niva.addEventListener(
  "fileDrop.cancelled",
  (eventName: string, payload: null) => {
    // do somethings...
  }
);
```

宿主输入使用主窗口process.stdin的data/end/error事件；不再提供host:message或专用stdio开关，见[标准流说明](/docs/api/stdio)。
