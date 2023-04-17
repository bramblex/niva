---
sidebar_position: 3
---

# 从 React 项目中导入

开始之前，需要先到[下载页面](https://github.com/bramblex/niva/releases)下载最新版本的 Niva 开发者工具。

## 创建一个 React 项目（可选）

注：如果你有一个现成 React 项目可以跳过本步骤。我们假设您已经有 React 的使用经验。

可以通过 Create React App 项目创建一个 React 项目 [https://create-react-app.dev/docs/getting-started](https://create-react-app.dev/docs/getting-started)

```bash
npx create-react-app my-react-app
```

## 导入项目

通过 Niva 开着工具导入我们的 React 项目，需要导入的是带有 `package.json` 文件的目录。

[图片]

打开 Vue 项目后会询问是否生成一个 `niva.json` 项目文件，确认后生成 `niva.json` 配置如下。

```json
{
	"name": "<项目名>",
	"uuid": "<项目uuid（随机生成，不建议改动）>",

	"debug": {
		"resource": "public",
		"entry": "http://127.0.0.1:3000/",
	},

	"build": {
		"resource": "build",
	},
}
```

如果是您是使用 Create React App 工具创创建的 React 项目，并且没有改动基础配置，那么这个配置文件也不需要修改可以直接使用。如果修改过 `public` 文件夹或者 `build` 文件夹以及项目调试端口等则需要对应修改配置。

更多配置可以参考 [配置文档](/docs/options/project)。


## 打开调试窗口

首先需要启动 React 开发服务

```bash
npm run start
```

然后再通过 Niva 开发者工具启动调试：

[图片]

在 Niva 中启动调试面板，这时候就可以跟正常开发 Vue 应用一样进行开发和调试。


## 构建应用

Niva 项目构建可以通过 Niva 开发者工具的构建按钮进行构建：

[图片]

构建好后可以直接点击可执行文件打开构建好的可执行文件，打开你的应用。
