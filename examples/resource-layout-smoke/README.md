# macOS 打包资源布局验收

```sh
python3 examples/resource-layout-smoke/run.py \
  --binary target/release/niva \
  --packager target/release/niva-packager \
  --output /tmp/niva-resource-layout-check
```

必须使用新的输出目录。驱动只生成当前 Mac 架构的隔离测试 kit，校验完整 runtime 小于 3,300,000 bytes，并传递 Rust/JS 许可材料。它不是可发布的三平台 build kit。

同一项目分别通过统一打包器生成 embedded 和 external `.app` ZIP，解包后执行 `codesign --verify --deep --strict`，再启动其中的真实程序。页面检查 33 MiB 以上资源的 HEAD、后缀 Range、416、完整读取超限、许可资源，以及打包后的 CommonJS 相邻文件读取与 ESM 接口身份。

报告记录产物 hash/体积、启动到完成检查的时间和采样到的 Native 进程 RSS。RSS 不包括 WebContent，也不是操作系统精确峰值。大文件是可复现的周期性测试字节，不是真实视频，压缩率不能用于媒体预算，也不能据此宣称视频播放或 OPT-01 优化已验收。
