# Bridge调用与授权

低层传输统一位于`Niva.bridge`：

```js
const title = await Niva.bridge.call('window.title', []);
// 应用通常直接调用Niva.window.title()等公开接口。
```

| 传输 | 能力 |
| --- | --- |
| WS | 异步、事件、流、二进制 |
| 同步XHR | 白名单Native同步方法、CommonJS文件加载 |
| IPC | 一次性异步JSON；文本文件、文本HTTP/HTTPS、exec文本及允许的元数据/目录操作 |

纯JS或启动静态数据不需要bridge。Native同步、ArrayBuffer/Buffer、持久流不经IPC；不支持的调用明确报错。已发送的请求不会在断线后自动重放。

本地资源及明确的本机开发origin使用窗口token与精确来源验证。普通远端/跨源frame通过平台IPC真实source URL和窗口permissions授权，默认零权限；payload自报origin或sessionId不能授予权限。IPC fallback不会绕过Rust侧检查。`--resource`/`--config`仅选择输入，不授予调试能力。

WS按连接清理；IPC在有在途任务时每1秒心跳、3秒租约，失联后失败任务并清理所属资源。窗口关闭/可观测导航由Native清理；macOS单iframe无可靠销毁通知时按租约确认。JS阻塞或平台节流也可能触发失联错误，不能声称GC/unload必然即时发生。显式close/dispose为主，GC只兜底。

IPC请求JSON最多256KiB，响应最多8MiB（包含文本转义）；专用文本操作还有更小body/输出限制。Wire版本仍为1，不能将JS接口重命名理解为协议版本升级。
