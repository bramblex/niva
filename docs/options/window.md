---
sidebar_position: 2
---

# 窗口选项

* 其中菜单选项 `MenuOptions` 详见 [菜单选项](/docs/options/menu)

```typescript
type NivaSize = { width: number; height: number };
type NivaPosition = { x: number; y: number };

interface WindowRootMenu {
  label: string;               // 根菜单项的名称
  enabled?: boolean;           // 是否启用菜单项
  children: MenuOptions;       // 子菜单项
}

type WindowMenuOptions = Array<WindowRootMenu>;

interface NivaWindowOptions {
  entry?: string;              // 应用程序入口文件路径
  devtools?: boolean;          // 是否启用开发者工具

  title?: string;              // 窗口标题
  icon?: string;               // 窗口图标
  theme?: string;              // 窗口主题
  size?: NivaSize;             // 窗口大小
  minSize?: NivaSize;          // 窗口最小尺寸
  maxSize?: NivaSize;          // 窗口最大尺寸

  position?: NivaPosition;     // 窗口位置

  resizable?: boolean;         // 是否可调整大小
  minimizable?: boolean;       // 是否可最小化
  maximizable?: boolean;       // 是否可最大化
  closable?: boolean;          // 是否可关闭

  fullscreen?: boolean;        // 是否全屏
  maximized?: boolean;         // 是否最大化
  visible?: boolean;           // 是否可见
  transparent?: boolean;       // 是否透明
  decorations?: boolean;       // 是否显示窗口装饰

  alwaysOnTop?: boolean;       // 是否始终置顶
  alwaysOnBottom?: boolean;    // 是否始终置底
  visibleOnAllWorkspaces?: boolean; // 是否在多个工作区显示

  focused?: boolean;           // 是否聚焦
  contentProtection?: boolean; // 是否启用内容保护

  // macOS extra
  parentWindow?: number;           // 父窗口 ID
  movableByWindowBackground?: boolean; // 是否可点击窗口背景移动窗口
  titleBarTransparent?: boolean;   // 标题栏是否透明
  titleBarHidden?: boolean;        // 标题栏是否隐藏
  titleBarButtonsHidden?: boolean; // 标题栏按钮是否隐藏
  titleHidden?: boolean;           // 标题是否隐藏
  fullSizeContentView?: boolean;   // 是否全尺寸显示内容
  resizeIncrements?: NivaSize;     // 窗口调整尺寸步长
  disallowHiDpi?: boolean;         // 是否禁用高 DPI
  hasShadow?: boolean;             // 是否显示阴影
  automaticWindowTabbing?: boolean; // 是否支持多个窗口进行选项卡式浏览
  tabbingIdentifier?: string;      // 设置选项卡式浏览的标题

  // windows extra
  parentWindow?: number;           // 父窗口 ID
  ownerWindow?: number;            // 拥有者窗口 ID
  taskbarIcon?: string;            // 任务栏图标
  skipTaskbar?: boolean;           // 在任务栏中是否显示
  undecoratedShadow?: boolean;     // 是否显示窗口阴影

  menu?: WindowMenuOptions;        // 窗口菜单选项
}
```
