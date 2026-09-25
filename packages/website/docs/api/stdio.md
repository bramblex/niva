---
sidebar_position: 4
---

# 作为子进程UI的标准流

父程序直接启动`niva --resource=/path/to/app`，通过普通stdin/stdout/stderr管道和主窗口`Niva.process`交互。没有`--stdio`开关、`api.host`或框架内置NDJSON。

```js
const process = Niva.process;
process.stdin.setEncoding('utf8');
process.stdin.on('data', chunk => process.stdout.write(chunk));
process.stdin.on('end', () => console.log('输入结束，UI仍可运行'));
process.stdout.write('ready\n'); // 应用自己定义的业务就绪消息
```

管道是字节流，一次write不保证对应一次data。应用自行选择行分隔、长度帧或其他协议；父进程应等待应用的就绪消息后再发请求。EOF与BrokenPipe按流语义处理，不自动结束桌面UI或重启任务。只有可信主窗口可使用真实process标准流，其他窗口通过正常窗口/事件能力协作。

仓库`examples/stdio-host`与`examples/stdio_host.py`提供普通行文本示例；其中ready和stdin-eof不是Native协议。Niva自己的日志与应用标准流分开。
