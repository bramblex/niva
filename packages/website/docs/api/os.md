# 系统信息 os

`Niva.os`是Node风格OS模块，并包含应用专用扩展：

```js
console.log(Niva.os.info); // 只读启动静态数据，不是函数或XHR
const {data, cache, temp} = await Niva.os.dirs();
console.log(Niva.os.platform(), Niva.os.arch());
```

data/cache/temp的身份由完整、校验后的应用UUID决定，改展示名称不改变目录。Node的homedir/tmpdir与应用data目录不是同一个概念。

固定平台信息取启动快照；cpus、freemem、networkInterfaces、uptime等动态查询按接口读取Native，不能拿快照或估算冒充实时结果。动态同步Native查询不走IPC。普通远端页面不会因为存在Niva.os对象就自动获得启动敏感信息；仍服从来源授权及可用性限制。
