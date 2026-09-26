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

异步方法使用该realm锁定的WS或IPC Channel；同步方法仅在可信页面通过同步XHR调用。`stat`跟随链接，`lstat`观察链接本身；具体支持的参数与签名以同版本runtime/types为准。

可信本地页面锁定IPC时，异步文件与流API仍由同一Native handler提供，二进制Channel帧在IPC边界使用Base64；具体可用操作以该版本API注册和权限为准。远端页面仍仅能调用grant明确允许的unary API，不开放流、二进制或Native同步方法。

文本/目录响应有大小限额；原生已开始的文件修改不保证可回滚。传输session建立失败或失联时明确报错，不重放已执行操作，也不静默改变返回类型。详见[Bridge](./bridge)。
