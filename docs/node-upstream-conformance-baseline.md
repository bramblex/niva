# Node 官方测试子集：首次运行记录

> 2026-09-24T08:13:42.143Z · Node v22.14.0 · darwin/arm64

本次验证的是 Node 宿主中的 Niva JS 适配器，不是 WebView 或 Native 验收。固定的 30 个上游文件全部保留，未依据结果删除用例、修改断言或缩小产品目标。

**结果：5 个文件通过、22 个失败、3 个因测试依赖未适配而阻塞；命令退出码为 1。当前不能声明这批 API 已通过兼容验收。**

测试 runner 的 16 个自检通过，覆盖被测模块映射、漏调/多调回调、异步异常、未处理 rejection、未知模块/helper、吞掉不支持异常、请求跳过、提前退出及超时。另外，在临时目录篡改一个上游文件后，runner 在执行任何用例前拒绝运行并非零退出；仓库中的原始文件没有被改动。

## 可复现入口

使用 Node 22.14.0，在仓库根目录执行：

```sh
npm run build:vendor --workspace=packages/node-compat
npm run test:upstream:harness --workspace=packages/node-compat
npm run test:upstream --workspace=packages/node-compat -- --report upstream-results.json
```

完整错误栈、上游 commit、运行环境、manifest/runner 和 Niva 源文件 SHA-256 写入本地 `packages/node-compat/upstream-results.json`（不入 Git）。独立 CI job 已配置，远端结果未验证，GitHub 分支保护未修改。

本次 manifest SHA-256：`7f60ad27203feed67c223519d881b1337eb3e5618846080360c63b057d375a71`。
本次 runner SHA-256：`d3bb4ff99ad85da6cb3408b3d1decddff72c4f7167a23d60fb6b966230684817`。

## 文件结果

| 模块 | 官方测试文件 | 结果 |
| --- | --- | --- |
| buffer | [test-buffer-alloc.js](../packages/node-compat/upstream/node-v22.14.0/test/parallel/test-buffer-alloc.js) | fail |
| buffer | [test-buffer-bytelength.js](../packages/node-compat/upstream/node-v22.14.0/test/parallel/test-buffer-bytelength.js) | fail |
| buffer | [test-buffer-compare.js](../packages/node-compat/upstream/node-v22.14.0/test/parallel/test-buffer-compare.js) | fail |
| buffer | [test-buffer-concat.js](../packages/node-compat/upstream/node-v22.14.0/test/parallel/test-buffer-concat.js) | fail |
| buffer | [test-buffer-fill.js](../packages/node-compat/upstream/node-v22.14.0/test/parallel/test-buffer-fill.js) | unsupported |
| buffer | [test-buffer-from.js](../packages/node-compat/upstream/node-v22.14.0/test/parallel/test-buffer-from.js) | fail |
| buffer | [test-buffer-indexof.js](../packages/node-compat/upstream/node-v22.14.0/test/parallel/test-buffer-indexof.js) | fail |
| buffer | [test-buffer-tojson.js](../packages/node-compat/upstream/node-v22.14.0/test/parallel/test-buffer-tojson.js) | pass |
| buffer | [test-buffer-write.js](../packages/node-compat/upstream/node-v22.14.0/test/parallel/test-buffer-write.js) | fail |
| events | [test-events-list.js](../packages/node-compat/upstream/node-v22.14.0/test/parallel/test-events-list.js) | fail |
| events | [test-events-listener-count-with-listener.js](../packages/node-compat/upstream/node-v22.14.0/test/parallel/test-events-listener-count-with-listener.js) | fail |
| events | [test-events-once.js](../packages/node-compat/upstream/node-v22.14.0/test/parallel/test-events-once.js) | unsupported |
| path | [test-path-basename.js](../packages/node-compat/upstream/node-v22.14.0/test/parallel/test-path-basename.js) | fail |
| path | [test-path-dirname.js](../packages/node-compat/upstream/node-v22.14.0/test/parallel/test-path-dirname.js) | pass |
| path | [test-path-extname.js](../packages/node-compat/upstream/node-v22.14.0/test/parallel/test-path-extname.js) | pass |
| path | [test-path-glob.js](../packages/node-compat/upstream/node-v22.14.0/test/parallel/test-path-glob.js) | fail |
| path | [test-path-isabsolute.js](../packages/node-compat/upstream/node-v22.14.0/test/parallel/test-path-isabsolute.js) | pass |
| path | [test-path-join.js](../packages/node-compat/upstream/node-v22.14.0/test/parallel/test-path-join.js) | fail |
| path | [test-path-normalize.js](../packages/node-compat/upstream/node-v22.14.0/test/parallel/test-path-normalize.js) | fail |
| path | [test-path-parse-format.js](../packages/node-compat/upstream/node-v22.14.0/test/parallel/test-path-parse-format.js) | fail |
| path | [test-path-relative.js](../packages/node-compat/upstream/node-v22.14.0/test/parallel/test-path-relative.js) | fail |
| path | [test-path-resolve.js](../packages/node-compat/upstream/node-v22.14.0/test/parallel/test-path-resolve.js) | unsupported |
| path | [test-path.js](../packages/node-compat/upstream/node-v22.14.0/test/parallel/test-path.js) | fail |
| querystring | [test-querystring-escape.js](../packages/node-compat/upstream/node-v22.14.0/test/parallel/test-querystring-escape.js) | fail |
| querystring | [test-querystring-maxKeys-non-finite.js](../packages/node-compat/upstream/node-v22.14.0/test/parallel/test-querystring-maxKeys-non-finite.js) | fail |
| querystring | [test-querystring-multichar-separator.js](../packages/node-compat/upstream/node-v22.14.0/test/parallel/test-querystring-multichar-separator.js) | pass |
| querystring | [test-querystring.js](../packages/node-compat/upstream/node-v22.14.0/test/parallel/test-querystring.js) | fail |
| string_decoder | [test-string-decoder-end.js](../packages/node-compat/upstream/node-v22.14.0/test/parallel/test-string-decoder-end.js) | fail |
| string_decoder | [test-string-decoder-fuzz.js](../packages/node-compat/upstream/node-v22.14.0/test/parallel/test-string-decoder-fuzz.js) | fail |
| string_decoder | [test-string-decoder.js](../packages/node-compat/upstream/node-v22.14.0/test/parallel/test-string-decoder.js) | fail |

## 已定位的差异与阻塞

- `events` 的默认导出是对象，而这些官方用例直接将 `require("events")` 用作构造器。runner 保留 Niva 原始导出形态，没有替它包装成构造器。
- Buffer 多个失败涉及非法参数的错误码/消息与 Node 不同；另有 `base64url` 编码缺口。StringDecoder 还有无效字节解码差异。
- path 用例发现 basename、join、normalize、parse/format、relative 和 glob 等边界行为差异。通过 dirname/isAbsolute 等文件不意味着整个 path 模块已兼容。
- querystring 的 URI 转义、非有限 maxKeys 以及综合用例存在差异。
- 三个阻塞文件分别需要 `internal/errors`、`internal/event_target` 和 `child_process` 测试依赖。它们未算通过，也未据此直接判定对应 API 全部不兼容。修复测试环境后仍需运行原断言。

修复顺序建议：先处理导出形态、path/querystring 算法和编码等真实语义差异，再补错误契约；并行补齐可忠实实现的测试辅助依赖。每轮保留原分母重跑，最后将适用用例接入真实 WebView/目标平台。现有自有测试继续保留。
