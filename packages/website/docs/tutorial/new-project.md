---
sidebar_position: 1
---

# 创建新项目

开始之前，需要先到[下载页面](https://github.com/bramblex/niva/releases)下载最新版本的 Niva 开发者工具。

## 通过 Niva 开发者工具创建项目

Niva 可以通过 Niva 开发者工具创建一个新项目：

[图片]

创建好新项目以后，我们可以看到项目基本描述如下：

[图片]

## 项目基本结构

我们可以通过 Niva 开发者工具打开我们的项目目录：

[图片]

新创建的 Niva 项目项目结构如下：

```
hello-niva
	- niva.json // Niva 项目的配置文件
	- index.html // Niva 项目入口文件 html 文件
	- index.js // index.html 引入 js 文件
	- logo.png // Niva 项目默认 logo
```

其中 `niva.json` 是 Niva 项目的配置文件，如果需要对项目进行配置，可以参考 [配置文档](/docs/options/project)。 `index.html` 是 Niva 项目的入口文件，Niva 的主窗口将会将 index.html 作为入口。`index.js` 是 `index.html` 文件引入的 index.js。，这时候我们就可以以这个项目作为模板进行开发我们的基础开发工作。

## 打开调试窗口

Niva 项目已经创建成功，下一步是通过 Niva 开发者工具上点击调试，打开 hello-niva 项目的调试窗口：

[图片]

在这个窗口中右键，可以打开调试窗口。

[图片]

## 构建应用

Niva 项目构建可以通过 Niva 开发者工具的构建按钮进行构建：

[图片]

构建好后可以直接点击可执行文件打开构建好的可执行文件，打开你的应用。
