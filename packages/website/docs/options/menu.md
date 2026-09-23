---
sidebar_position: 3
---

# 窗口菜单选项

```ts
// 系统原生菜单项标签在 niva.json 中使用 camelCase 字符串。
type NativeLabel =
  | "hide" | "services" | "hideOthers" | "showAll" | "closeWindow"
  | "quit" | "copy" | "cut" | "undo" | "redo" | "selectAll"
  | "paste" | "enterFullScreen" | "minimize" | "zoom" | "separator";

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

Windows 当前可显示 `accelerator` 组合键，但窗口菜单尚未接入 `TranslateAcceleratorW`，按键不会因此触发该菜单项。
