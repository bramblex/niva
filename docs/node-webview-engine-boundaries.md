# 官方用例中的引擎边界

固定 Node v22.14.0 原文件逐字节保留。用户已明确要求跳过下列两处专属环境差异；运行器按带校验的精确位置记录环境豁免，其他断言继续执行。没有删除原文件、跳过整个文件或缩小 58 项文件分母。

## 1. 移除原型后恢复构造器名

`test-util-format.js:197` 要求 `Object.setPrototypeOf(new Foo(), null)` 输出 `[Foo: null prototype] {}`。它与 `Object.create(null)` 的公开属性、原型和类型标签相同，但 Node 输出不同。Node inspect 调用了私有的 `internalBinding('util').getConstructorName`。标准 JS 没有等价接口；通过猜测类名、修改全局 Object/Proxy 或针对测试写死结果不能作为兼容实现。

来源：[Node v22.14.0 inspect](https://github.com/nodejs/node/blob/v22.14.0/lib/internal/util/inspect.js)。普通 null-prototype 对象标签、正常构造器和可观察的格式化仍可修复。

## 2. 同步比较原生 CryptoKey 的密钥内容

`test-assert-deep.js:1295–1365` 要求同步区分两个独立 CryptoKey 的密钥内容。相同公开算法、用途及 extractable 字段的 key，可能拥有相同或不同的密钥字节。Node 私下读取 `internal/crypto/util.kKeyObject`；浏览器 `exportKey()` 返回 Promise，而且不能导出 nonextractable key。

来源：[Node v22.14.0 deep comparator](https://github.com/nodejs/node/blob/v22.14.0/lib/internal/util/comparisons.js)、[Web Crypto exportKey](https://www.w3.org/TR/webcrypto/#SubtleCrypto-method-exportKey)。

Niva 自身的 SecretKeyObject 可以依靠私有存储比较内容；原生 CryptoKey 的公开元数据也可比较。两者不能代替浏览器原生非导出密钥的同步内部比较。

## 可复现证据

[诊断脚本](../packages/node-compat/scripts/probe-engine-boundaries.mjs)与[固定 Node 22.14.0 实测结果](node-webview-engine-boundary-evidence.json)。这是公开可观察性与 Node oracle 的对照，不冒称已运行浏览器测试。脚本不读取 Node 的私有 Symbol，也不改变全局原型或 Crypto 接口。

```sh
node packages/node-compat/scripts/probe-engine-boundaries.mjs
```

这两项属于原文件中的混合断言，不是整个 util/assert 模块都不可实现。公共 API 的行为差异继续修复，无豁免诊断仍单独保存；默认门禁明确记录 2 处豁免，不将其伪报为断言已通过。

## 修复后的范围核对

`util-format` 的[诊断续跑记录](node-util-format-diagnostic.json)执行 189 个断言，唯一失败为移除原型后的 Foo 标签。它只用于查找该失败之后的其他问题，不代替原始官方运行，也不改变分母。`assert-deep` 的原始运行中，其余 39 个子测试通过；原生 CryptoKey 内容比较仍失败。

调用点源码表达式不属于同样的不可观察边界：已使用同源源码读取、通用 Acorn 解析和跨引擎栈解析实现，并通过打包 WebView 的真实检查。没有把源码表达式写死为测试预期。

## 已采用的口径

[豁免清单](../packages/node-compat/upstream/environment-exclusions.json)固定为两个检查位置；[最新报告](node-compat-upstream-results.json)为 58 个文件适用断言通过、2 处环境跳过。[原始诊断报告](node-upstream-unfiltered-results.json)为 56/58。
