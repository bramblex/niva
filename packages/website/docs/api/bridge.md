# Bridge调用与授权

低层传输统一位于`Niva.bridge`：

```js
const title = await Niva.bridge.call('window.title', []);
// 应用通常直接调用Niva.window.title()等公开接口。
```

| 传输 | 能力 |
| --- | --- |
| WebSocket bridge | 异步请求、事件、流和原始二进制帧 |
| IPC bridge | 异步调用与Channel；控制/发送/ACK走平台IPC，Native→JS帧/事件经 `evaluate_script`；二进制完整18-byte帧在边界使用Base64 |
| 同步XHR | `callSync`及白名单Native同步方法、CommonJS文件加载 |

纯JS或启动静态数据不需要bridge。每个JS realm首次发起async call/stream时最多等待500ms建立并认证WS；成功后锁定WS，否则锁定IPC。锁定IPC后晚到的WS不升级。WS断开会清理该session的pending calls和资源，当前调用失败，后续调用锁定IPC。已执行请求不跨传输重放。XHR只用于同步API；`/__niva_fs`是独立文件资源接口，不承载异步bridge。

本地资源及明确的本机开发origin使用窗口token与精确来源验证。普通远端/跨源frame通过平台IPC真实source URL和窗口permissions授权，默认零权限；payload自报origin或sessionId不能授予权限。本地IPC Channel同样由Rust校验来源、凭据、窗口/session/frame与调用权限；远端页面仍只可通过精确origin grant调用unary API，不开放流或二进制。`--resource`/`--config`仅选择输入，不授予调试能力。

WS按session清理；IPC session在有在途任务时每1秒心跳、3秒租约，失联后失败任务并清理所属资源。双bridge真实WebView端到端验收仍待完成。窗口关闭/可观测导航由Native清理；macOS单iframe无可靠销毁通知时按租约确认。JS阻塞或平台节流也可能触发失联错误，不能声称GC/unload必然即时发生。显式close/dispose为主，GC只兜底。

IPC请求JSON最多256KiB，响应最多8MiB（包含文本转义）；专用文本操作还有更小body/输出限制。IPC二进制Channel帧payload最多16 KiB，使用严格序号、有界队列、ACK/背压与取消；Base64会产生额外编码开销，不代表零拷贝或性能更快。Wire版本仍为1，不能将JS接口重命名理解为协议版本升级。
