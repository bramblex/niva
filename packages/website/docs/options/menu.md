---
sidebar_position: 3
---

# 窗口菜单选项

```ts
// 系统原生菜单项标签枚举类型。
enum NativeLabel {
  Hide,              // 显示 "Hide"
  Services,          // 显示 "Services"
  HideOthers,        // 显示 "Hide Others"
  ShowAll,           // 显示 "Show All"
  CloseWindow,       // 显示 "Close Window"
  Quit,              // 显示 "Quit"
  Copy,              // 显示 "Copy"
  Cut,               // 显示 "Cut"
  Undo,              // 显示 "Undo"
  Redo,              // 显示 "Redo"
  SelectAll,         // 显示 "Select All"
  Paste,             // 显示 "Paste"
  EnterFullScreen,   // 显示 "Enter Full Screen"
  Minimize,          // 显示 "Minimize"
  Zoom,              // 显示 "Zoom"
  Separator,         // 表示一个分隔线
}

// 菜单项选项枚举类型。
type MenuItemOption =
  | { type: "native"; label: NativeLabel }  // 本地菜单选项
  | {
      type: "item";
      id: number;
      label: string;                  // 显示的文本
      enabled?: boolean;              // 是否启用
      selected?: boolean;             // 是否选中
      icon?: string;                  // 图标图片，仅支持 png
      accelerator?: string;           // 快捷键
    }                                // 自定义菜单选项
  | { type: "menu"; label: string; enabled?: boolean; children: MenuOptions }; // 子菜单选项

// 菜单选项列表。
type MenuOptions = MenuItemOption[];
```
