# 事件

## 窗口事件

### window.focused
窗口焦点事件，当窗口获得或失去焦点时触发。
```ts
Niva.addEventListener("window.focused", (focused: boolean) => {
  // do somethings...
});
```

### window.scaleFactorChanged
窗口缩放事件，当窗口缩放比例改变时触发。
```ts
Niva.addEventListener(
  "window.scaleFactorChanged",
  (payload: {
    scaleFactor: number;
    newInnerSize: { width: number; height: number };
  }) => {
    // do somethings...
  }
);
```

### window.themeChanged
窗口主题事件，当窗口主题改变时触发。
```ts
Niva.addEventListener(
  "window.themeChanged",
  (theme: "light" | "dark" | "system") => {
    // do somethings...
  }
);
```

### window.closeRequested
窗口关闭请求事件，当用户请求关闭窗口时触发。
```ts
Niva.addEventListener("window.closeRequested", (payload: null) => {
  // do somethings...
});
```

### window.message
窗口消息事件，当窗口接收到来自其他窗口的消息时触发。
```ts
Niva.addEventListener(
  "window.message",
  (payload: { from: number; message: string }) => {
    // do somethings...
  }
);
```

## 菜单事件

### menu.clicked
菜单点击事件，当菜单被点击时触发。
```ts
Niva.addEventListener("menu.clicked", (menuId: number) => {
  // do somethings...
});
```

## 托盘图标事件

### tray.rightClicked
托盘图标右键点击事件，当用户右键点击托盘图标时触发。
```ts
Niva.addEventListener("tray.rightClicked", (trayId: number) => {
  // do somethings...
});
```

### tray.leftClicked
托盘图标左键点击事件，当用户左键点击托盘图标时触发。
```ts
Niva.addEventListener("tray.leftClicked", (trayId: number) => {
  // do somethings...
});
```

### tray.doubleClicked
托盘图标双击事件，当用户双击托盘图标时触发。
```ts
Niva.addEventListener("tray.doubleClicked", (trayId: number) => {
  // do somethings...
});
```

## 全局快捷键事件

### shortcut.emit
全局快捷键事件，当全局快捷键被触发时触发。

```ts
Niva.addEventListener("shortcut.emit", (shortcutId: number) => {
  // do somethings...
});
```

## 文件拖拽事件

### fileDrop.hovered
文件拖拽悬停事件，当用户在窗口中拖动文件并将其悬停时触发。
```ts
Niva.addEventListener(
  "fileDrop.hovered",
  (payload: { paths: string[]; position: { x: number; y: number } }) => {
    // do somethings...
  }
);
```

### fileDrop.dropped
文件拖拽放置事件，当用户在窗口中拖动文件并将其放置时触发。
```ts
Niva.addEventListener(
  "fileDrop.dropped",
  (payload: { paths: string[]; position: { x: number; y: number } }) => {
    // do somethings...
  }
);
```

### fileDrop.cancelled
文件拖拽取消事件，当用户取消文件拖拽操作时触发。
```ts
Niva.addEventListener(
  "fileDrop.cancelled",
  (payload: null) => {
    // do somethings...
  }
);
```