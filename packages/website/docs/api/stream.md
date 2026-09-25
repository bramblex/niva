# 流与二进制

`Niva.stream`是Node stream模块；低层Native流通过`Niva.bridge.stream`：

```js
const task = Niva.bridge.stream('resource.readStream', ['large.bin'], {
  onChunk(bytes) { console.log(bytes.byteLength); }
});
await task.promise;
```

task包含id、promise和cancel。显式cancel结束Promise并通知Native清理；队列拒绝或断线明确失败，不能静默丢块。onChunk按片段接收；需要聚合全部数据时必须自行考虑内存界限。Native任务需要Drop/监督者保证future被取消后也释放资源。

双向写入使用`Niva.bridge.streamSend`。WS二进制头为18字节：version=1、flags、u64大端id、u64大端seq，然后payload；flags为START=1/END=2/STDERR=4。文本result是单次终局，不占流序号。

IPC不支持流或二进制，也不靠Base64把它们伪装成JSON接口。需要一次性文本结果时使用明确的文件文本、requestText或execText接口。
