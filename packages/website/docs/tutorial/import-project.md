---
sidebar_position: 4
---

# 导入其他前端项目

开始之前，需要先到[下载页面](https://github.com/bramblex/niva/releases)下载最新版本的 Niva 开发者工具。

如果你有一个现成的前端项目：

## 导入一个没有编译构建系统的前端项目

找到包含 `index.html` 的项目目录，再用 Niva 开发者工具的「选择项目」打开。如果尚无 `niva.json`，工具会先询问是否创建配置文件。

后面的步骤可以参考 [创建新项目](/docs/tutorial/new-project)。

## 导入一个有编译构建系统的前端项目

注：我们默认对 React 和 Vue 的项目做了支持，如果需要导入一个 React 或者 Vue 项目可以看 [从 React 项目中导入](/docs/tutorial/import-project-from-react) 和 [从 Vue 项目中导入](/docs/tutorial/import-project-from-vue)。


对于其他项目，先在 Niva 开发者工具中打开项目根目录，并确认创建 `niva.json`。项目名优先读取 `package.json` 的 `name`，否则使用目录名；`uuid` 由工具生成。

```json
{
	"name": "my-app",
	"uuid": "replace-with-generated-uuid"
}
```

### 调试配置

如果项目有开发服务，可以通过 `niva.json` 中的 `debug` 字段配置调试信息：

```json
{
	"debug": {
		"resource": "public",
		"entry": "http://localhost:5173"
	}
}
```

把示例目录和端口改成项目实际值。启动开发服务后，在 Niva 开发者工具点击「调试」。`debug.entry` 只在显式调试启动时使用；正常打包启动不会加载开发服务器。

### 构建配置

如果要能够正常用 Niva 构建，需要配置 `niva.json` 里面的 `build` 字段以配置构建信息：

```json
{
	"build": {
		"resource": "dist"
	}
}
```

把 `dist` 改为实际构建输出目录，且该目录应包含入口 HTML。先运行前端项目自己的构建命令，再在 Niva 开发者工具点击「构建」。Windows 输出 `.exe`，macOS 输出 `.app` 应用包。
