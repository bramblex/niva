---
sidebar_position: 4
---

# 系统托盘图标选项
* 其中菜单选项 `MenuOptions` 详见 [菜单选项](/docs/options/menu)

```typescript
// NivaTray 的选项。
interface NivaTrayOptions {
  icon: string;         // 托盘菜单的图标，仅支持 png
  title?: string;       // 托盘菜单的标题
  tooltip?: string;     // 托盘菜单的提示信息
  menu?: MenuOptions;   // 托盘菜单的菜单选项
}

// NivaTray 的更新选项。
interface NivaTrayUpdateOptions {
  icon?: string;        // 更新托盘菜单的图标，仅支持 png
  title?: string;       // 更新托盘菜单的标题
  tooltip?: string;     // 更新托盘菜单的提示信息
  menu?: MenuOptions;   // 更新托盘菜单的菜单选项
}
```