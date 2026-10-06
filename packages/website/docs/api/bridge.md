# Bridge调用与授权

> 当前源码实现 API 控制面/纯数据面分离与 WS Wire v2；真实 macOS custom-protocol 和严格 CSP 双 lane smoke 已通过。Windows target check 因 MSVC 头文件缺失未完成，Windows 真机与 CI 仍未验证。准确范围、产物指纹及测试记录见[Bridge v2 验收记录](../../../../docs/bridge-v2-validation.md)和[Bridge 合约](../../../../docs/bridge.md)。

低层传输统一位于`Niva.bridge`：

```js
const title = await Niva.bridge.call('window.title', []);
// 应用通常直接调用Niva.window.title()等公开接口。
```

| API类别/桥接路径 | 用途 |
| --- | --- |
| 异步API控制面：IPC `t:"api_call"` | 统一创建和控制Native API调用；method/args与dispatch不走WS |
| Native→JS结果/凭证：Wry `evaluate_script` | 调用 `__niva_ipc_reply({sessionId,rid,sourceOrigin,response})` 返回小结果及 `channelOpened`/capability |
| 纯双向数据面 | 先创建并拿凭证，再attach；WS v2只处理attach/data/ack/cancel，hello和binary header版本为2；稳定路径为Wry IPC/eval |
| Node兼容同步API：同步XHR | 只供确需同步语义的兼容方法；每个方法首次调用时应发出warning |

纯JS或启动静态数据不需要bridge。Native资源owner固定归属创建它的IPC session，数据transport独立选择；attach或切换transport不改变owner。WS建连失败时可用稳定数据路径；不承诺已开始流断线无损迁移，切换数据通道不得重放API副作用，提交状态不确定时应明确失败。进程cwd等状态变更应提供异步API，不能走同步XHR。`/__niva_fs`是独立文件资源接口，不承载异步bridge。

本地顶层文档及同源iframe共用Wry IPC/eval；父页按 JS session 路由同源 child reply，Native 校验来源 URL/top-origin、token、session、call 与 ticket，不依赖 Wry 未提供的真实 Native frame ID。普通远端顶层页面仅可按精确origin grant调用unary API，不开放流或二进制。跨源iframe即使配置grant也必须fail-closed：Wry eval只定向主文档，不能安全定向跨源child，也不得让parent观察child秘密。payload自报origin或sessionId不能授予权限。`--resource`/`--config`仅选择输入，不授予调试能力。

当前实现保留session租约和资源清理；真实 macOS smoke 覆盖了同源 child session reply 和 iframe 后 parent unary。窗口关闭/可观测导航由Native清理；macOS单iframe无可靠销毁通知时按租约确认。JS阻塞或平台节流也可能触发失联错误，不能声称GC/unload必然即时发生。显式close/dispose为主，GC只兜底。Windows与其他未列平台仍须独立验证。

IPC请求JSON最多256KiB，响应最多8MiB（包含文本转义）；专用文本操作还有更小body/输出限制。IPC二进制Channel对完整18-byte wire frame做Base64，payload最多16 KiB，使用严格序号、有界队列、ACK/背压与取消；Base64会产生额外编码开销，不代表零拷贝或性能更快。当前 WS hello 与 binary header wire version 均为2；历史 Wire v1 只见[Bridge 合约的历史快照](../../../../docs/bridge.md)。
