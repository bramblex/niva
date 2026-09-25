# Node模块环境

基础Node风格API始终实现在`Niva`对象上。两项配置只控制页面环境注入，默认都为false：

```json
{ "injectCommonJs": true, "injectEsm": true }
```

CommonJS环境统一注入`require`、`module`及适用的Node全局；真实process只在可信主窗口提供。内置模块直接注册Niva接口，不重复实现：

```js
const fs = require('node:fs');
console.assert(fs.readFile === Niva.fs.readFile);
```

用户JS/JSON模块通过同步XHR请求Native解析与读取，页面执行并维护module/exports、缓存、循环依赖及相对require。不支持.node原生插件，也不自行执行ESM。目录模式按真实文件系统解析；单文件嵌入模式首次加载用户CommonJS时解包资源到进程独占临时目录，保证__filename/__dirname和相邻资源访问一致。持久业务数据应使用应用data目录。

ESM开关提供Node内置API的`.mjs` facade与import map；执行规则仍属于浏览器：

```js
import fs from 'node:fs';
console.assert(fs.readFile === Niva.fs.readFile);
```

用户import map覆盖保留。Vite等构建器可直接把Node import映射到Niva facade，不要求同时注入全局require；Devtools使用这一方式避免与bundler冲突。

这是一套有明确范围的兼容接口，不是内嵌Node运行时。`process.versions.niva`表示实际Niva版本，`versions.nodeCompat`表示目标契约版本；不能从模块名或类型推断所有Node行为已经通过验证。复杂项目验收使用固定版本Cypress Real World App，官方契约、真实WebView和平台测试分别计数。

`crypto.timingSafeEqual`按本次明确决定采用纯JS字节比较：长度不同报错，完整遍历并累积差异；调用时警告不保证恒定时间或抵抗时序侧信道。它不能当作Node安全语义完全兼容，也不能替代Rust侧bridge token的常量时间鉴权比较。
