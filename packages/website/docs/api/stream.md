# 流与二进制

`Niva.stream`是Node stream模块；低层Native流通过`Niva.bridge.stream`：

```js
const task = Niva.bridge.stream('resource.readStream', ['large.bin'], {
  onChunk(bytes) { console.log(bytes.byteLength); }
});
await task.promise;
```

task包含id、promise和cancel。显式cancel结束Promise并通知Native清理；队列拒绝或断线明确失败，不能静默丢块。onChunk按片段接收；需要聚合全部数据时必须自行考虑内存界限。Native任务需要Drop/监督者保证future被取消后也释放资源。

双向写入使用`Niva.bridge.streamSend`。WS二进制头为18字节：version=2、flags、u64大端channel id、u64大端channel seq，然后是payload；flags为START=1/END=2/STDERR=4。IPC 数据通道使用相同的序号和背压语义，完整二进制帧在传输边界以Base64编码，单帧payload最多16 KiB。

API 创建与控制通过 IPC `t:"api_call"`。流创建并取得凭证后，运行时选择已就绪的WebSocket优化数据传输，或使用稳定IPC数据通道；WS不承载API dispatch。WS未建立时流仍可走IPC，远端 origin grant 则仅允许受限的 unary JSON API，不开放流或二进制通道。需要一次性文本结果时使用明确的文件文本、`requestText`或`execText`接口。
