# Bridge调用与授权

> 当前合约说明。普通异步调用经稳定IPC/evaluate_script桥；重载与流式数据可选用WebSocket优化桥。实现状态以当前源码和对应平台端到端验证为准，本文不表示所有目标行为已完成。

低层传输统一位于`Niva.bridge`：

```js
const title = await Niva.bridge.call('window.title', []);
// 应用通常直接调用Niva.window.title()等公开接口。
```

| API类别/桥接路径 | 用途 |
| --- | --- |
| 异步API：稳定IPC/evaluate_script | 普通异步调用Rust Native API；保证不依赖WS可用 |
| 异步API：可选WebSocket优化桥 | 大量文件/网络数据、二进制数据、流式 `child_process` stdio；不可用时由同语义IPC通道承载 |
| Node兼容同步API：同步XHR | 只供确需同步语义的兼容方法；每个方法首次调用时应发出warning |

纯JS或启动静态数据不需要bridge。传输属于低层实现，调用方不选择WS或IPC。稳定异步桥是IPC/evaluate_script，WebSocket只负责可选的性能优化。桥接切换不得重复已提交操作；无法确定操作是否已提交时应明确失败。进程cwd等状态变更应提供异步API，不能走同步XHR。`/__niva_fs`是独立文件资源接口，不承载异步bridge。

本地资源及明确的本机开发origin使用窗口token与精确来源验证。普通远端/跨源frame通过平台IPC真实source URL和窗口permissions授权，默认零权限；payload自报origin或sessionId不能授予权限。本地IPC Channel同样由Rust校验来源、凭据、窗口/session/frame与调用权限；远端页面仍只可通过精确origin grant调用unary API，不开放流或二进制。`--resource`/`--config`仅选择输入，不授予调试能力。

WS按socket连接清理；IPC session在有在途任务时每1秒心跳、3秒租约，失联后失败任务并清理所属资源。路由已在当前runtime/Rust源码接线；macOS基础WS bridge与WS受限IPC WebView smoke已通过，完整资源生命周期和Windows真机仍待验收。窗口关闭/可观测导航由Native清理；macOS单iframe无可靠销毁通知时按租约确认。JS阻塞或平台节流也可能触发失联错误，不能声称GC/unload必然即时发生。显式close/dispose为主，GC只兜底。

IPC请求JSON最多256KiB，响应最多8MiB（包含文本转义）；专用文本操作还有更小body/输出限制。IPC二进制Channel对完整18-byte wire frame做Base64，payload最多16 KiB，使用严格序号、有界队列、ACK/背压与取消；Base64会产生额外编码开销，不代表零拷贝或性能更快。Wire版本仍为1，不能将JS接口重命名理解为协议版本升级。
