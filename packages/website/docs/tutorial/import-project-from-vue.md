---
sidebar_position: 2
---

# 从 Vue 项目中导入

开始之前，需要先到[下载页面](https://github.com/bramblex/niva/releases)下载最新版本的 Niva 开发者工具。

## 创建一个 Vue 项目（可选）

注：如果你已有 Vue 项目，可以跳过本步骤。下面以 Vue 官方的 Vite 模板为例。

可以通过 Vue 的官方文档创建一个 Vue 项目 [https://cn.vuejs.org/guide/quick-start.html#creating-a-vue-application](https://cn.vuejs.org/guide/quick-start.html#creating-a-vue-application)

```bash
npm init vue@latest
```

## 导入项目

通过 Niva 开发者工具的「选择项目」打开带有 `package.json` 的 Vue 项目根目录。如果目录中没有 `niva.json`，确认创建后，检测到 Vite 时会生成以下配置：

```json
{
	"name": "<项目名>",
	"uuid": "<项目uuid（随机生成，不建议改动）>",

	"debug": {
		"resource": "public",
		"entry": "http://localhost:5173"
	},

	"build": {
		"resource": "dist"
	}
}
```

如果使用默认的 Vite 配置，通常不需要再修改。若改过 `public`、`dist` 目录或开发服务端口，请同步修改 `debug`、`build` 字段。旧 Vue CLI 项目不使用 Vite 时，Devtools 的默认开发入口为 `http://localhost:8080`。

更多配置可以参考 [选项文档](/docs/options/project)。


## 打开调试窗口

首先需要启动 Vue 开发服务

```bash
npm run dev
```

然后在 Niva 开发者工具的项目信息页点击「调试」。调试窗口会加载开发服务，并启用 WebView 开发者工具。


## 构建应用

首先需要先通过 Vue 构建出 Vue 的静态代码：

```bash
npm run build
```

再通过 Niva 开发者工具的「构建」按钮打包。Windows 输出 `.exe`，macOS 输出 `.app` 应用包；请在对应系统上打开产物验证。
