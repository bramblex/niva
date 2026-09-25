# Cypress Real World App 对照验收

此目录保存测试驱动，不包含业务后端替代实现。上游版本、Node参考版本与范围固定在 `manifest.json`。参考Node通过不代表Niva通过；Niva必须在WebView内经自己的CommonJS loader与Native API执行后端。

## 准备与参考运行

在临时目录克隆manifest指定仓库并checkout指定提交，按原yarn.lock安装依赖。用于本地认证后端的准备阶段使用 `yarn install --frozen-lockfile --ignore-scripts --non-interactive`；它没有下载Cypress浏览器测试二进制或执行项目安装钩子。第三方OAuth不在此验收范围。

从Niva仓库执行：

```sh
node examples/realworld-acceptance/prepare.mjs /absolute/path/to/cypress-realworld-app
python3 examples/realworld-acceptance/reference.py /absolute/path/to/cypress-realworld-app/.niva-compiled --output /absolute/path/to/reference-evidence
```

准备脚本逐文件转译原始TS，保留require依赖，不bundle后端、不修改业务源码；使用项目自身的mock AWS配置启动本地认证。产物留在原checkout下，依赖沿真实node_modules路径解析。准备会复制种子数据库，所以不可在持久化检查之间重新准备。

外部Python HTTP驱动检查未授权、错误密码、注册、登录/会话、用户、账户校验与读写、联系人、模拟付款、通知、真实退出登录，以及后端重启后的磁盘持久化。所有请求只访问独立启动的loopback实例，付款为上游模拟业务。

`result.json`记录每项结果、参考引擎、固定提交与依赖锁hash；构建目录的`build-manifest.json`记录转译输入hash。失败必须保留，不能删用例或回退host Node掩盖Niva缺口。原版前端UI和第三方OAuth不在这些HTTP检查的通过声明中。

2026-09-25参考Node v24.10.0的18项检查通过。Niva执行与bridge断线资源清理尚待新runtime集成后验证，不能以此参考报告标为通过。

## Niva运行

完成新runtime与Native构建后，在同一份准备产物上执行：

```sh
python3 examples/realworld-acceptance/niva.py /absolute/path/to/cypress-realworld-app/.niva-compiled --binary /absolute/path/to/niva --output /absolute/path/to/niva-evidence
```

它启动实际WebView并由页面require原始转译后端；应用仍使用原node_modules依赖。驱动没有启动Node后端的代码路径。日志、失败和Native二进制hash随报告保存；首次加载失败会写明原异常，不能算完成兼容。
