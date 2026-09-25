# process与child_process

可信主窗口提供`Niva.process`；开启CommonJS后，`require('process')`和注入的process引用该接口。它包含启动信息、当前目录操作、退出及标准流。子窗口不提供真实宿主process标准流。

```js
Niva.process.stdout.write('application output\n');
Niva.process.stdin.setEncoding('utf8');
Niva.process.stdin.on('data', text => console.log(text));
```

标准流协议由应用自己定义；EOF仅结束输入，不自动退出UI。Niva框架日志写独立文件，不通过应用stdout/stderr。见[stdio](./stdio)。

子进程功能使用`Niva.child_process`。Node spawn/exec等需要WS提供持久生命周期及流；同步操作走同步XHR，不提供同步IPC。IPC可使用一次性高层扩展：

```js
const result = await Niva.child_process.execFileText('/path/to/program', ['argument']);
console.log(result.status, result.stdout, result.stderr);
```

`execText(command, options?)`使用shell，`execFileText(file, args?, options?)`直接启动程序；返回有界文本结果，不返回ChildProcess流对象。输出总量最多64KiB，默认10秒、最多30秒。超限、超时、所属会话失联时终止任务，不重放。调用前应按普通程序执行语义构造参数，不能把不可信字符串拼进shell命令。
