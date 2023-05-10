---
sidebar_position: 4
---

# 导入其他前端项目

开始之前，需要先到[下载页面](https://github.com/bramblex/niva/releases)下载最新版本的 Niva 开发者工具。

如果你有一个现成的前端项目：

## 导入一个没有编译构建系统的前端项目

跟前面打开项目类似，只需要找到前端项目 index.html 存在的文件夹，然后通过 Niva 开发者工具打开这个目录即可。

![screenshot](@site/static/img/import-project/screenshot1.png)

后面的步骤可以参考 [创建新项目](/docs/tutorial/new-project)。

## 导入一个有编译构建系统的前端项目

注：我们默认对 React 和 Vue 的项目做了支持，如果需要导入一个 React 或者 Vue 项目可以看 [从 React 项目中导入](/docs/tutorial/import-project-from-react) 和 [从 Vue 项目中导入](/docs/tutorial/import-project-from-vue)。


对于其他项首先在通过 Niva 开发者工具打开项目的根目录，并且自动生成 `niva.json` 文件。

```json
{
	"name": "<项目名>", // 如果有 package.json 则自动从 package.json 中读取
	"uuid": "<项目 uuid>" // 自动生成，不要修改
}
```

### 调试配置

如果项目有开发服务，可以通过 `niva.json` 中的 `debug` 字段配置调试信息：

```json
{
	// ... 
	"debug": {
		"resource": "<调试环境静态文件目录>", // 静态文件目录，一般是项目中 public 目录
		"entry": "<调试环境入口>", // 开发服务的入口
	}
}
```

配置好以后，就可以启动开发服务。之后再用 Niva 开发者工具启动项目调试窗口。接下来就跟在浏览器中开发调试一样了。

### 构建配置

如果要能够正常用 Niva 构建，需要配置 `niva.json` 里面的 `build` 字段以配置构建信息：

```json
{
	"build": {
		"resource": "<构建时的静态文件目录>" // 项目编译后的目标目录，往往包含 index.html
	}
}
```

配置好以后，就可以先构建项目，得到构建后的静态项目文件。之后再用 Niva 开发者工具构建可执行文件。
