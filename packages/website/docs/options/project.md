---
sidebar_position: 1
---

# 项目选项

在项目根目录的 `niva.json` 中配置应用。`name` 和 `uuid` 必填；未提供 `window` 时使用默认窗口选项。

| 字段 | 用途 |
| --- | --- |
| `name`、`uuid` | 应用名称与唯一标识符。 |
| `version` | Devtools 显示的应用版本，macOS/Windows 包元数据也从该顶层字段读取。 |
| `icon` | 应用图标的 PNG 路径。 |
| `meta` | 可选的公司名、描述、版权信息；不从 `meta.version` 读取版本。 |
| `window` | 主窗口配置；字段见[窗口选项](/docs/options/window)，包括外源页面的 `permissions`。 |
| `tray`、`shortcuts` | 可选托盘和全局快捷键配置，见[托盘选项](/docs/options/tray)与[快捷键选项](/docs/options/shortcut)。 |
| `api` | API 调度配置：`timeoutMs` 默认 30000 毫秒，`maxQueue` 默认 64。旧的固定线程池 `workers` 已移除。 |
| `injectCommonJs`、`injectEsm` | 独立控制CommonJS环境与ESM import map，默认均false；基础Niva API始终提供。见[Node模块环境](/docs/api/node-compat)。 |
| `debug` | 开发资源目录 `resource` 与开发入口 `entry`。远端/Vite 入口仅在显式调试启动中生效。 |
| `build` | Devtools 打包时读取的静态资源目录 `resource`。 |
| `sign` | Devtools 打包后的可选签名配置；证书密码不得写入 `niva.json`。 |
| `macos`、`windows` | 可选的平台覆盖对象；运行时按当前平台递归合并到顶层配置。 |

例如，一个使用 Vite 的项目可写为：

```json
{
  "name": "HelloNiva",
  "uuid": "a51c1728-d174-42d4-8f57-7d296c966b51",
  "version": "1.0.0",
  "window": { "entry": "index.html" },
  "debug": { "resource": "public", "entry": "http://localhost:5173" },
  "build": { "resource": "dist" },
  "api": { "timeoutMs": 30000, "maxQueue": 64 },
  "injectCommonJs": false,
  "injectEsm": false
}
```

`macos` / `windows` 覆盖在 Niva 运行时合并。GUI与CLI共用的打包核心读取顶层 `build` 和 `sign`，不要把只供打包器使用的字段仅放进平台覆盖对象。macOS 的 `activationPolicy`、`defaultMenuCreation`、`activateIgnoringOtherApps` 可作为顶层或 `macos` 覆盖字段。窗口与来源权限的实际边界见[权限说明](/docs/api/permissions)。

展示名称不参与data/cache/temp目录身份，使用完整校验后的UUID；示例UUID应为每个应用重新生成一次并稳定保存。`--resource`与`--config`是正式宿主接口，本身不启用debug.entry。
