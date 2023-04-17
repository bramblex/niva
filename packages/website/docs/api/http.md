# 网络 http

## Niva.api.http.request
```ts
/**
 * 发送 HTTP(s) 请求并返回响应结果，包括响应状态码、响应头和响应体。
 * @param options 请求选项，包括方法、URL、请求头、请求体和代理设置。
 * @returns 一个 Promise，在接收响应成功后解析该 Promise，或在发生错误时拒绝该 Promise。成功时返回一个包含响应状态码、响应头和响应体的对象。
 */
export function request(options: {
    method: string;
    url: string;
    headers?: { [key: string]: string };
    body?: string;
    proxy?: string;
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