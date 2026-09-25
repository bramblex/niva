# HTTP与HTTPS

`Niva.http`、`Niva.https`实现Node风格请求/响应和服务器接口，其内置模块导出引用同一实现。流式request/get/createServer需要WS与Native TCP/TLS能力；HTTP协议处理在JS，应用服务器与Niva内部bridge服务器各自独立。

IPC提供明确的一次性文本扩展：

```js
const response = await Niva.https.requestText({url: 'https://example.com/'});
console.log(response.statusCode, response.headers, response.body);
```

requestText可经获准的IPC执行，返回JSON文本结果，不返回ClientRequest或Readable，也不冒充Node流式request/get。状态码及响应体保留；连接失败、超时、超限、非法文本编码正常拒绝。

已授权请求可访问内网、外网和localhost，HTTPS保留证书链及主机名验证。默认不自动继承环境HTTP代理。文本body最多1MiB，默认10秒、最多30秒；调用选项只可收紧。完整Agent、upgrade、连接复用等Node行为不能仅凭接口存在就视为已验收。
