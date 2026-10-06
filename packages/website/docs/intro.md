---
sidebar_position: 1
---

# 简介

## 什么是 Niva？

Niva 面向 Windows 与 macOS 桌面应用开发，使用前端技术和系统 WebView，不把浏览器内核或 Node.js 作为应用运行时打包；Windows 的 WebView2 本身基于 Chromium。Niva 直接提供系统 API，Devtools 可导入 React、Vue 等前端项目。各平台产物体积和真机验收范围见[候选记录](https://github.com/bramblex/niva/blob/main/docs/release-0.10.0-beta.1.md)。

## Niva 开发者工具

Niva 开发者工具是一个图形化界面开发者工具，提供图形化界面进行对 Niva 项目进行构建或者调试的开发工具。

## Niva 与 Tauri 和 Electron 的异同

Niva 与 Tauri、Electron 的运行时和扩展方式不同。下面只列架构差异；不同版本、平台的包体积和学习成本不在这里作无同口径依据的比较。

下表列出了 Niva、Tauri 和 Electron 的主要区别：

| 项目 | Niva | Tauri | Electron |
| --- | --- | --- | --- |
| 页面运行时 | 系统 WebView（Windows 为 WebView2） | 系统 WebView | 随应用打包 Chromium |
| 应用逻辑 | 内置 Niva 原生 API；可选 stdio 宿主 | Rust 扩展与前端 | Node.js 与前端 |
| Niva 构建目标形式 | Windows `.exe`、macOS `.app` | — | — |

Niva 与 Tauri 都使用 tao、wry 等底层库，但公开 API、平台适配与验收范围并不相同，不能由底层库相同推定功能完全一致。Niva 直接向 WebView 提供原生 API，常见应用无需自行编写 Rust 扩展；未覆盖与待验收的能力见仓库 `docs/api-coverage.md`。Electron 随应用携带 Node.js 和 Chromium；两者的整体产物大小需要按相同平台、应用和功能另行比较。

Niva 使用系统 WebView，不把 Node.js 或 Chromium 内核作为应用运行时打包；Windows WebView2 的 Chromium 内核由系统运行时提供。实际交付体积取决于平台、应用资源与构建选项，不能只凭运行时架构作跨产品的体积比例比较。Niva 提供图形化开发者工具和项目配置，可将前端项目迁移为桌面应用。

Niva 面向希望用前端技术构建桌面应用的开发者，提供系统 WebView 集成和图形化开发工具。
