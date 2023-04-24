---
sidebar_position: 1
---

# 项目选项

Niva 应用选项，可在 `niva.json` 文件中编辑。

* 其中窗口选项 `NivaWindowOptions` 详见 [窗口选项](/docs/options/window)。
* 其中托盘选项 `NivaTrayOptions` 详见 [托盘选项](/docs/options/tray)。
* 其中全局快捷键选项 `NivaShortcutsOptions` 详见 [全局快捷键选项](/docs/options/shortcut)。

```ts
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
}
```
