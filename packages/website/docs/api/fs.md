# 文件系统 fs

`Niva.fs`提供Node风格callback、promises与同步接口；开启对应环境后，CommonJS/ESM导出同一个对象。

```js
const fs = Niva.fs.promises;
await fs.mkdir('/absolute/path/example', {recursive: true});
await fs.writeFile('/absolute/path/example/file.txt', '你好', 'utf8');
const text = await fs.readFile('/absolute/path/example/file.txt', 'utf8');
const stat = await fs.stat('/absolute/path/example/file.txt');
console.log(text, stat.isFile(), stat.size);
```

异步文件方法属于主要异步API：普通小型调用走稳定IPC/evaluate_script桥；大量文件内容或二进制块优先使用可选WebSocket优化桥，WS不可用时由IPC Channel承载同一操作。IPC二进制帧在传输边界对完整18-byte wire frame使用Base64，payload最多16 KiB。同步方法仅用于Node兼容语义，并通过同步XHR调用；每个方法首次调用时发出一次warning。`stat`跟随链接，`lstat`观察链接本身；具体支持的参数与签名以同版本runtime/types为准。

可信本地页面的异步文件与流API由同一Native handler提供，WS只是数据量较大时的性能优化，不能成为文件操作可用性的前提；具体可用操作以该版本API注册和权限为准。远端页面仍仅能调用grant明确允许的unary API，不开放流、二进制或Native同步方法。

文本/目录响应有大小限额；原生已开始的文件修改不保证可回滚。不可用WS不得阻断文件操作；如稳定IPC本身发生错误则明确报错，且不重放已执行操作、不静默改变返回类型。本文是目标合约说明，当前实现与验收状态须以源码及对应平台测试为准。详见[Bridge](./bridge)。
