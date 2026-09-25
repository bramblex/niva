# 让命令行程序拥有 Niva 窗口

应用在主窗口通过 Node 风格 `process.stdin/stdout/stderr` 交换原始字节。没有专门的宿主协议，也不需要 `--stdio`。这个示例自己的页面按行解释输入；换成 JSON、二进制或其他协议由应用决定。

在仓库根目录运行：

```sh
printf '你好，Niva\n' | ./target/release/niva --resource=examples/stdio-host
```

页面启用 `injectCommonJs`，使用 `require('process')`。关闭管道会产生 stdin 的 `end`，窗口继续存在；关闭主窗口或调用 `process.exit()` 才结束应用。Niva 框架日志进入应用 UUID 对应数据目录的 `niva.log`，不会占用宿主通信流。

可重复的验收驱动：

```sh
python3 examples/stdio-host/verify.py --binary target/release/niva --output /tmp/niva-stdio-check
python3 examples/stdio-host/verify_exit.py --binary target/release/niva --output /tmp/niva-exit-check
```

前者检查应用首条输出、UTF-8 拆块、EOF 与日志隔离；后者启动真实子进程并验证退出后不能留下延迟写文件的孤儿进程。报告绑定被测二进制 SHA256。
