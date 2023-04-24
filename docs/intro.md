---
sidebar_position: 1
---

# 简介

## 什么是 Niva？

Niva 是一个跨平台（支持 Windows 和 MacOS）的桌面应用开发框架，使用前端技术开发，可用于构建轻量级的桌面应用。它基于系统 Webview 而非 Chromium，体积仅为 3MB ~ 4MB，不需要 Node.js 环境，直接提供系统 API 进行操作。Niva 兼容 React / Vue 等主流框架，对于 Vue 和 React 项目可以直接一键导入。

## Niva 开发者工具

Niva 开发者工具是一个图形化界面开发者工具，提供图形化界面进行对 Niva 项目进行构建或者调试的开发工具。

## Niva 与 Tauri 和 Electron 的异同

Niva 与 Tauri 和 Electron 定位不同。它是 Tauri 和 Electron 的轻量级替代方案，比它们更加简单易用但同时牺牲了部分生态和能力。

下表列出了 Niva、Tauri 和 Electron 的主要区别：

|          | Niva                       | Tauri                  | Electron            |
| -------- | -------------------------- | ---------------------- | ------------------- |
| 体积     | 3MB                        | 6MB+                   | 85MB+               |
| 支持系统 | Windows10+/MacOS           | Windows10+/MacOS/Linux | Windows/MacOS/Linux |
| APP 后端 | 无，但可用隐藏 window 代替 | Rust                   | Node.js             |
| Webview  | System                     | System                 | Chromium            |
| 生态     | 前端                       | Rust + 前端            | Node.js + 前端      |
| 上手难度 | 简单                       | 极难                   | 困难                |

Niva 使用了 Tauri 的跨端窗口管理库 tao 和跨端 Webview 库 wry，所以在窗口和 Webview 上面 Niva 和 Tauri 具备相同的能力，以及相同数量级的体积。不同之处在于，Niva 直接为 Webview 提供通用的 API，无需编写 Rust 代码，对前端开发者更加友好。而使用 Electron 则需要 Node.js 和 Chromium 的依赖，体积也更大。

与 Electron 相比，Niva 不依赖 Node.js 和 Chromium，因此能够实现更小的体积，对前端开发者更加友好。Niva 提供了图形化的开发者工具和简单易用的配置，能够快速将前端项目迁移至 Niva 等桌面应用。

总之，Niva 专注于提供更小的体积和更加便捷的开发体验，适合需要快速构建轻量级桌面应用的前端开发者使用。
