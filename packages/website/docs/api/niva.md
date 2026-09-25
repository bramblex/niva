---
sidebar_position: 0
---

# Niva API 总览

本页随统一runtime重构更新；实现与验收状态以仓库架构实施台账和对应版本记录为准。

页面基础API直接位于`Niva`对象上，不依赖是否注入Node环境：

```js
const title = await Niva.window.title();
const text = await Niva.fs.promises.readFile('/absolute/path/file.txt', 'utf8');
const info = Niva.os.info; // 静态启动信息
```

窗口、剪贴板、对话框、资源、快捷键、托盘、WebView等使用直接命名空间；文件、OS、process、子进程、网络等使用Node风格模块接口。`Niva.os.dirs()`提供应用UUID对应的持久数据/缓存/临时目录扩展。

低层传输在`Niva.bridge.call/callSync/stream/streamSend`；`Niva.stream`本身是Node stream模块。不存在旧`Niva.api`聚合入口。WS、同步XHR、IPC的能力和授权不同，见[Bridge](./bridge)。

`injectCommonJs`与`injectEsm`是独立开关，默认关闭。开启后内置模块接口直接引用Niva实现，例如`require('fs').readFile === Niva.fs.readFile`。ESM使用浏览器标准，详见[Node模块环境](./node-compat)。用户自己的覆盖行为由用户负责。

Native事件仍通过`Niva.addEventListener`等事件方法接收；Node `Niva.events`是另一个标准模块接口。只读静态数据与纯JS方法不经过Native传输。
