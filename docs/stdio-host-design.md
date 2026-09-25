# Niva作为子进程UI：普通process标准流

> 2026-09-25：旧`api.host`、`--stdio`和Rust内置NDJSON协议已退出目标架构。新实现进度与平台验收见[实施台账](architecture-implementation-plan.md)；旧版本的EOF退出记录不适用于此合约。

Shell、Python或其他程序可以启动Niva，把它作为一个有UI的子进程。主窗口通过`Niva.process.stdin/stdout/stderr`收发字节；开启`injectCommonJs`后也可用`require('process')`或注入的`process`。子窗口不提供宿主process标准流。

```sh
niva --resource=/absolute/path/to/app
# 如配置文件不在资源根，可另加 --config=/absolute/path/to/niva.json
```

没有专用stdio开关，也没有框架强制的JSON格式、ready消息或请求ID。是否使用NDJSON、行文本、二进制或其他协议，由应用双方约定。Niva框架自身的日志写独立文件，不把诊断混入应用协议；日志初始化/轮转失败也不回退stdout/stderr。第三方WebView的直接系统输出需要按平台实际验证，不能仅凭删除Rust println声明所有底层输出已经隔离。

## 页面示例

```js
const process = Niva.process;
process.stdin.setEncoding('utf8'); // 跨chunk保留UTF-8字符边界
let pending = '';
process.stdin.on('data', chunk => {
  pending += chunk;
  let end;
  while ((end = pending.indexOf('\n')) >= 0) {
    const line = pending.slice(0, end);
    pending = pending.slice(end + 1);
    process.stdout.write(`received: ${line}\n`);
  }
});
process.stdin.on('end', () => {
  // EOF仅结束输入；是否退出窗口由应用决定。
});
process.stdin.on('error', error => {
  // 由应用决定如何在UI或自己的错误通道报告。
});
process.stdout.write('ready\n'); // 应用自己的业务就绪消息
```

父进程应先等待应用明确的ready，再发业务请求。管道是字节流，一次write不保证对应一次data；使用定界/长度协议时要自己处理拆包。stdout/stderr遵守各自流的回压和错误，不能把写入排队等同于对方已收到。

stdin EOF不自动退出UI。窗口关闭、应用退出或Bridge会话失联会结束其流资源，旧会话不重放输入/输出。stdout BrokenPipe也是流错误，不能悄悄切换到框架日志或重启原命令。

可运行示例在[`examples/stdio-host`](../examples/stdio-host/index.html)和[`examples/stdio_host.py`](../examples/stdio_host.py)。示例采用普通行文本；`ready`和`stdin-eof`都是该示例自己写的内容，不是新的Native协议。
