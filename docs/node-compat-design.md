# 统一Niva运行时与Node接口设计

> 2026-09-25：统一runtime实施中，完成度与验证见[实施台账](architecture-implementation-plan.md)。旧NodeCompat的模块/API数量、官方测试与二进制大小均为历史快照。

## Native 能力优先复用

文件、进程、网络、TLS及其它已有Native能力以Rust为实现来源；JavaScript层只保留Node接口与Native契约之间必要的适配，不再另造同一能力。Rust接口暂缺Node所需的流、背压、取消或生命周期语义时，应扩展Native桥接，再由JS包装成Node对象，同时保留已声明的行为范围。HTTP/HTTPS的`requestText`和Node `http.request/get`客户端都复用Rust ureq；Node流式客户端通过本地WebSocket逐块上传、逐块读取响应，JS只适配ClientRequest/IncomingMessage对象。`createServer`是独立的服务端能力，继续在JS上复用Native TCP/TLS，不由ureq替代。客户端连接复用、定制socket、upgrade、response trailers及原始自定义reason phrase不在当前支持范围内。进度和验收边界见[实施台账](architecture-implementation-plan.md)。

## 一个实现，多个导出入口

页面实现集中在`packages/runtime`的TypeScript源码中，Native嵌入其构建产物。所有API放在Niva对象：`Niva.fs`、`Niva.os`、`Niva.child_process`、`Niva.http`等是Node风格模块，`Niva.window`、`Niva.dialog`等是应用专用能力。Native RPC传输属于`Niva.bridge`；`Niva.stream`表示Node stream模块。

CommonJS内置注册表与浏览器ESM facade只导出Niva对象上的同一接口，不另造一份模块状态或函数。`require('fs').readFile === Niva.fs.readFile`是身份约定；用户覆盖import map/模块对象自行负责，框架不禁止覆盖。

| 构件 | 职责 |
|---|---|
| TypeScript API模块 | JS逻辑与Native调用适配，维护资源对象行为 |
| bootstrap | 建立基础Niva、选择传输、统一注入Node全局 |
| CommonJS loader | 内置映射、module/exports/cache/循环、页面中执行用户JS |
| Rust module.resolve/load | 同步解析与读取本机JS/JSON文件，拒绝.node及require ESM |
| ESM facade/import map | 浏览器标准import接口；只引用Niva，不重新初始化runtime |
| types | 从统一源码/支持面生成声明；复用Node类型不等于宣称完整Node支持 |

## 环境注入开关

`injectCommonJs`与`injectEsm`是顶层独立布尔字段，默认false。关闭只表示不注入相应环境，基础Niva API始终提供。API文件不能私自写全局require/process/Buffer；统一bootstrap负责Node全局。真实process与标准流仅给可信主窗口。

CommonJS开启时注入require/module等环境，内置模块无需Native查找。用户JS/JSON通过同步XHR加载；缓存、循环依赖、module.parent/children及错误移除缓存由页面loader负责。wrapper具有Node顶层this/arguments语义；不使用Rust反向eval去等待已被同步XHR阻塞的JS。

ESM开启时提供`/__niva_runtime/esm/*.mjs`与import map，执行/解析规则仍归浏览器。不支持自建ESM引擎或Native addon。Devtools通过Vite虚拟模块把Node import直接映射到Niva接口，因此不依赖开发服务器HTML注入，也不必开启全局require。

## 文件与执行边界

顶层require以应用资源根为解析起点，模块内部使用真实parentFilename；标准祖先node_modules、package main/exports等交给固定版本解析库。可信Native文件API与HTTP资源路径沙箱是不同边界：HTTP资源不能逃出资源根，已授权Node文件读取可以使用普通绝对路径及祖先依赖目录。

目录/绿色模式直接使用真实资源目录。内嵌模式首次加载用户CommonJS时将资源树流式解包到进程独占临时目录，保持__filename/__dirname、相邻文件与readdir一致；有磁盘解包成本，业务持久数据应使用完整应用UUID对应的data目录。

本地页面的CommonJS factory使用Native生成、独立于鉴权token的CSP nonce；只为受控factory/importmap增加相应nonce，不开启unsafe-eval或任意unsafe-inline。外部页面自己的HTTP响应CSP不能被这种本地HTML处理绕过，受限调用明确失败。

## 构建与材料

运行`npm run build --workspace=packages/runtime`进行类型检查和资产/声明生成。Cargo核对输入与产物hash，拒绝陈旧dist。bootstrap压缩内嵌一份，经Wry初始化脚本注入，不暴露成网络资源；网络资产只允许manifest中的ESM facade/chunk。许可notice跟随build kit和最终业务产物。

`process.version`为`v22.14.0`，`process.versions.node`和`nodeCompat`为`22.14.0`，供Node依赖选择目标API版本对应的代码路径；实际产品版本在`process.versions.niva`。这些是兼容目标元数据，不表示内置Node/V8，也不表示完整Node 22契约已通过。验收不根据这些字段跳过缺失能力；参考Node只做oracle/构建/外部测试驱动，不能替代Niva缺失的被测API。

## 传输与资源

WS承载异步/流/二进制，同步XHR承载Native同步，IPC只提供明确有界的一次性JSON接口。文本文件、HTTP/HTTPS文本与exec文本可降级；默认Buffer、Native同步、watch、持久文件/socket句柄和流不能伪装成IPC能力。详见[Bridge合约](bridge.md)。

资源的显式close/dispose为主，GC只是兜底。WS断线、IPC租约失联或窗口关闭使所属请求失败并清理Native/JS资源，不重放旧任务。Native清理需要Drop守卫或独立监督者；丢弃异步future并不保证执行await后面的代码。没有明确所有权转移时不得用detached选项逃过清理。

`crypto.timingSafeEqual`按本次明确决定采用纯JS字节比较：长度不同报错，完整遍历并累积差异；调用时警告不保证恒定时间或抵抗时序侧信道。它不能当作Node安全语义完全兼容，也不能替代Rust侧bridge token的常量时间鉴权比较。
