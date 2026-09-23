---
sidebar_position: 1
---

# 创建新项目

开始之前，需要先到[下载页面](https://github.com/bramblex/niva/releases)下载最新版本的 Niva 开发者工具。

## 通过 Niva 开发者工具创建项目

在 Niva 开发者工具点击「新建项目」，输入项目名，再选择用于存放项目的父目录。工具会在该目录下创建同名文件夹；如果同名文件夹已存在，会提示错误，不会覆盖原有内容。创建完成后会打开项目信息页。

## 项目基本结构

新创建的项目结构如下：

```
hello-niva
	- niva.json // Niva 项目的配置文件
	- index.html // Niva 项目入口文件 html 文件
	- index.js // index.html 引入 js 文件
```

`niva.json` 包含项目名和 UUID，可按[选项文档](/docs/options/project)继续配置。默认主窗口从 `index.html` 加载，页面以模块脚本引入 `index.js`。模板的 HTML 已包含可编辑的 CSP meta；新增外部资源来源时需要同步调整 CSP。

## 打开调试窗口

在项目信息页点击「调试」，Niva 会启动新的项目窗口，并启用 WebView 开发者工具。调试资源从项目目录读取；如果配置了 `debug.resource`，则从该子目录读取。

## 构建应用

在项目信息页点击「构建」，选择输出位置并等待构建完成。Windows 输出 `.exe`，macOS 输出 `.app` 应用包；请在目标系统上打开产物验证。
