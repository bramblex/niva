# Niva 开发者工具

Niva Devtools 用于创建、导入、调试和构建 Niva 项目。当前界面提供最近项目列表、`niva.json` 配置编辑、构建步骤与命令输出；macOS 打包为 `.app`，Windows 打包为 `.exe`。

从仓库根目录启动 Vite 开发服务：

```sh
npm run start --workspace=packages/devtools
```

执行类型检查与生产构建：

```sh
npm run build --workspace=packages/devtools
```

Devtools 使用 Niva 自身的原生 API，仅在普通浏览器中不能完成调试和打包操作。Windows 真机验收状态见[路线图](../../docs/roadmap.md)。
