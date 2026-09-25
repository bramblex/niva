# 在 Niva 内执行原版 TypeScript CLI

```sh
python3 examples/typescript-cli-smoke/run.py \
  --binary target/release/niva \
  --compiler node_modules/typescript \
  --output /tmp/niva-typescript-cli-check
```

使用新的输出目录。页面用 Niva 自身 CommonJS loader 加载未修改的 `typescript/lib/tsc.js`；Python 只生成输入、启动 Niva 和检查输出，不调用 Node 编译器代跑。

正确项目需要解析多文件与标准库，生成 JavaScript、声明和 source map，退出码为 0；错误项目必须报告 TS2322、退出码 1 且不生成输出。报告记录实际 TypeScript 版本、编译器文件 hash 和 Native hash。它是 CLI 编译补测，不替代 watch/语言服务或完整 Node 兼容验收。
