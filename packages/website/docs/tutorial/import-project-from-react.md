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

通过 Niva 开发者工具的「选择项目」打开带有 `package.json` 的 React 项目根目录。如果目录中没有 `niva.json`，确认创建后，检测到 `react-scripts` 时会生成以下配置：

```json
{
	"name": "<项目名>",
	"uuid": "<项目uuid（随机生成，不建议改动）>",

	"debug": {
		"resource": "public",
		"entry": "http://localhost:3000"
	},

	"build": {
		"resource": "build"
	}
}
```

如果使用默认的 Create React App 配置，通常不需要再修改。若改过 `public`、`build` 目录或开发服务端口，请同步修改 `debug`、`build` 字段。

更多配置可以参考 [选项文档](/docs/options/project)。


## 打开调试窗口

首先需要启动 React 开发服务

```bash
npm run start
```

然后在 Niva 开发者工具的项目信息页点击「调试」。调试窗口会加载开发服务，并启用 WebView 开发者工具。


## 构建应用

先运行 `npm run build` 生成 `build` 目录，再在 Niva 开发者工具点击「构建」。Windows 输出 `.exe`，macOS 输出 `.app` 应用包；请在对应系统上打开产物验证。
