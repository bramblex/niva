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

Windows 源码已在原生消息钩子中将按键交给目标窗口的菜单 accelerator；该路径尚无本轮 Windows 真机操作记录，不能据 target check 宣称快捷键已在设备验收。
