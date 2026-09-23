# 网络 http

## Niva.api.http.request
```ts
/**
 * 发送 HTTP(s) 请求并返回响应结果，包括响应状态码、响应头和响应体。
 * @param options 请求选项，包括方法、URL、请求头和请求体。原生请求禁用代理。
 * @returns 一个 Promise，在接收响应成功后解析该 Promise，或在发生错误时拒绝该 Promise。成功时返回一个包含响应状态码、响应头和响应体的对象。
 */
export function request(options: {
    method: string;
    url: string;
    headers?: { [key: string]: string };
    body?: string;
}): Promise<{
    status: number;
    headers: { [key: string]: string };
    body: string;
}>;
```

## Niva.api.http.get
```ts
/**
 * 发送 HTTP(s) GET 请求并返回响应结果，包括响应状态码、响应头和响应体。
 * @param url 请求的 URL。
 * @param headers 如果有，指定请求头。
 * @returns 一个 Promise，在接收响应成功后解析该 Promise，或在发生错误时拒绝该 Promise。成功时返回一个包含响应状态码、响应头和响应体的对象。
 */
export function get(url: string, headers?: { [key: string]: string }): Promise<{
    status: number;
    headers: { [key: string]: string };
    body: string;
}>;
```

## Niva.api.http.post
```ts
/**
 * 发送 HTTP(s) POST 请求并返回响应结果，包括响应状态码、响应头和响应体。
 * @param url 请求的 URL。
 * @param body 请求体。
 * @param headers 如果有，指定请求头。
 * @returns 一个 Promise，在接收响应成功后解析该 Promise，或在发生错误时拒绝该 Promise。成功时返回一个包含响应状态码、响应头和响应体的对象。
 */
export function post(url: string, body: string, headers?: { [key: string]: string }): Promise<{
    status: number;
    headers: { [key: string]: string };
    body: string;
}>;
```

## Niva.api.http.requestStream

`request/get/post` 会在 JS 层收集完整响应体。需要直接处理二进制响应时，使用
[`Niva.stream`](stream.md) 调用 `http.requestStream`：`head` 事件包含状态码和响应头，
`onChunk` 可逐帧处理正文；`onBlob` 可在 END 后取得完整正文，终局结果包含状态码。

```ts
const request = Niva.stream("http.requestStream", [{ method: "GET", url: "https://example.com/" }], {
  onEvent(name, data) {
    if (name === "head") console.log(data.status, data.headers);
  },
  onChunk(bytes) {
    console.log("received response bytes:", bytes.byteLength);
  },
});
await request.promise;
```

原生请求只支持 HTTP(S)，不使用显式或环境代理，并拒绝向非公开网络地址发起请求。
只有当前 Niva 实例的精确 loopback 服务地址是例外。重定向后的每一跳同样会检查。
远端 IPC 页面不能调用流式 HTTP API。
