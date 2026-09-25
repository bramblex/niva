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

异步方法按可用通道执行；同步方法仅在可信页面通过同步XHR调用。`stat`跟随链接，`lstat`观察链接本身；具体支持的参数与签名以同版本runtime/types为准。

IPC fallback支持显式UTF-8文本读写/追加，以及stat/lstat/readdir/access/realpath/mkdir/rename/copyFile/rm/unlink/cp的一次性JSON分支。默认readFile返回Buffer，因此无encoding的readFile不能经IPC；Buffer/ArrayBuffer写入、流、文件句柄、watch和所有Native同步方法也不支持IPC。

文本/目录响应有大小限额；原生已开始的文件修改不保证可回滚。Bridge不通且无有效fallback时明确报错，不挂起等待或静默换结果类型。详见[Bridge](./bridge)。
