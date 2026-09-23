---
sidebar_position: 1
---

# 简介

## 什么是 Niva？

Niva 是一个跨平台（支持 Windows 和 macOS）的桌面应用开发框架，使用前端技术开发，可用于构建轻量级的桌面应用。它使用系统 WebView，不随应用打包浏览器内核或 Node.js；Windows 的 WebView2 本身基于 Chromium。Niva 直接提供系统 API，Devtools 可导入 React、Vue 等前端项目。各平台产物体积和验收范围见[路线图](https://github.com/bramblex/niva/blob/main/docs/roadmap.md)。

## Niva 开发者工具

Niva 开发者工具是一个图形化界面开发者工具，提供图形化界面进行对 Niva 项目进行构建或者调试的开发工具。

## Niva 与 Tauri 和 Electron 的异同

Niva 与 Tauri、Electron 的运行时和扩展方式不同。下面只列架构差异；不同版本、平台的包体积和学习成本不在这里作无同口径依据的比较。

下表列出了 Niva、Tauri 和 Electron 的主要区别：

| 项目 | Niva | Tauri | Electron |
| --- | --- | --- | --- |
| 页面运行时 | 系统 WebView（Windows 为 WebView2） | 系统 WebView | 随应用打包 Chromium |
| 应用逻辑 | 内置 Niva 原生 API；可选 stdio 宿主 | Rust 扩展与前端 | Node.js 与前端 |
| Niva 当前交付形式 | Windows `.exe`、macOS `.app` | — | — |

Niva 与 Tauri 都使用 tao、wry 等底层库，但公开 API、平台适配与验收范围并不相同，不能由底层库相同推定功能完全一致。Niva 直接向 WebView 提供原生 API，常见应用无需自行编写 Rust 扩展；未覆盖与待验收的能力见仓库 `docs/api-coverage.md`。Electron 则随应用携带 Node.js 和 Chromium，交付体积通常更大。

与 Electron 相比，Niva 不随应用打包 Node.js 和 Chromium，因此能够实现更小的交付体积。Niva 提供图形化开发者工具和项目配置，可将前端项目迁移为桌面应用。Windows WebView2 的 Chromium 内核由系统运行时提供，并非应用包的一部分。

总之，Niva 专注于提供更小的体积和更加便捷的开发体验，适合需要快速构建轻量级桌面应用的前端开发者使用。
