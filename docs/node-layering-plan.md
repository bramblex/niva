# NodeCompat：Native 系统基座与 JS 协议层

> 2026-09-24 · 22 个模块 / 179 项 API 的分层重评。此文是目标方案及隔离测量，不代表接口已经完成。总体积仍以全部目标完成后的 3.3 MB release 门禁验收。

## 1. 结论

Native 提供可复用的 `net`、`dgram`、`tls` 系统基座；JS 在其上实现 DNS、HTTP/HTTPS **客户端和应用服务器**。浏览器自身没有 UDP/TCP socket，不妨碍 Niva 通过 Native 向 JS 提供这些能力。无需为每种上层协议再建立一套 Native 后端。

普通 Web 请求仍可独立使用 fetch，但 Node HTTP 主方案基于 Niva socket，才能覆盖服务器监听、原始请求头、双向流等契约。`https` 共用 HTTP/1.1 实现，将底层 TCP 流替换为 TLS 流。HTTP/2 已排除。

## 2. 模块放在哪一层

| 模块/能力 | Native 负责 | JS 负责 | 评价 |
|---|---|---|---|
| `net` | TCP connect/bind/listen/accept、字节读写、关闭和系统错误 | Node Socket/Server、stream/event 形态 | Native 高；是上层协议公共前置 |
| `dgram` | UDP socket、数据报边界、地址、绑定/收发/关闭 | Node dgram 对象与事件 | Native 高；DNS复用，不重复造UDP |
| `tls` | TLS客户端/服务器握手、证书与主机名校验、密钥、SNI及明文字节流 | Node TLS选项/对象/错误映射 | Native 高；HTTPS复用，不重复造TLS |
| `http` 客户端和服务器 | 不另建协议后端，复用 net | HTTP/1.1解析/序列化、请求响应对象、连接池、生命周期 | 整体中；parser部分低 |
| `https` 客户端和服务器 | 不另建协议后端，复用 tls | 共用http实现，绑定TLS流 | 中 |
| `dns.lookup` | 系统 getaddrinfo/等价接口；读取当前系统DNS配置的薄接口 | 回调、选项、结果、错误映射 | 中；保留hosts等系统解析语义 |
| `dns.resolve/resolve4` | 不新增DNS codec；依赖dgram/net及配置读取 | DNS报文、查询事务、重试/超时、TCP回退、记录映射 | codec低；完整接入中 |
| `fs` | 真实文件/目录/元数据/句柄/watch、原生读写 | 编码、Buffer、Promise/callback、Node对象和参数 | Native必要；高层语义JS |
| `child_process` | 真正启动、管道、等待、终止及信号 | ChildProcess、输出流、结果与错误 | Native必要；fork已放弃 |
| `process` | 真实进程信息/退出/stdio等；只向main提供目标对象 | 静态字段直接读；动态值经XHR；对象/调度/事件包装 | 混合；不引入Node宿主 |
| `os` | 系统信息采集；静态启动注入、动态按XHR查询 | Node字段和返回结构 | 混合 |
| `crypto` | 严格timingSafeEqual等JS不能保证的契约另评 | 摘要/HMAC/KDF走noble，随机走Web Crypto | 7项JS低；不预设算法下沉 |
| `stream`、`buffer`、`zlib` | 无额外算法后端；真实I/O仍用上述系统基座 | 现成浏览器库及薄封装 | 低 |
| `path`、`url`、`querystring` | 需要当前cwd时复用系统查询，不单独建路径Native模块 | 字符串/URL逻辑 | JS |
| `events`、`util`、`assert` | 无 | 纯JS语义/库 | JS；未有现成覆盖的边界仍需实现 |
| `string_decoder`、`timers` | 无额外Native模块 | 浏览器解码/调度原语和JS库 | JS；不声称完整Node事件循环 |

低成本协议库不意味着上层 Node 模块已完成。完成依赖关系是：先有可靠 socket/stream，再完成协议对象；不能把 Native 基座尚未实现的前置条件漏掉。

## 3. Native socket 基座要一次做好

- TCP 必须同时覆盖客户端和服务器：连接、监听、接受、二进制读写、半关闭、EOF、destroy、错误及本地/对端地址。
- UDP 保留每个 datagram 的边界及来源地址；有界接收队列、发送结果、取消/关闭与系统选项由Native处理。
- TLS 在Native验证证书链和主机名，处理SNI、服务端证书/密钥与握手错误；向JS暴露统一的明文字节流。
- 读写需要可用的背压和生命周期接口；Native资源绑定窗口/连接，页面断开和窗口关闭时可回收。JS的Node stream对象处理上层状态，不能只靠无限缓存假装背压。
- 身份、权限及目的地址策略仍在Rust连接层执行。现有HTTP客户端的目标校验不能因协议迁往JS而消失；JS提交的“已解析IP”也不能跳过Native校验。系统DNS端点、合法本机/局域网服务需按Native策略处理，不能让JS自行放宽。

跨域仍只提供获准的简单IPC子集，不因为把协议移到JS就向外源开放原始socket或全部Node API。

## 4. 小型 JS HTTP parser 可行，但不是完整 http 模块

候选 [`http-parser-js` 0.5.10](https://github.com/creationix/http-parser-js) 是无运行时依赖的纯JS请求/响应解析器。本轮browser ESM构建和分片请求、响应、chunked body、trailers合成用例通过。打包实测：

| 测量范围 | minified bytes | raw DEFLATE bytes |
|---|---:|---:|
| HTTP parser独立 | 7,380 | 2,644 |
| 现有stream + DNS + Buffer | 150,148 | 44,390 |
| 上述组合再加HTTP parser | 157,028 | 46,583 |
| HTTP parser共享后的边际 | 6,880 | **2,193（约2.1 KiB）** |

还需实现 ClientRequest、IncomingMessage、Server/响应写入、Agent/连接复用、请求序列化、超时取消、100-continue/upgrade及stream生命周期，因此HTTP/HTTPS整体评为中。上游明确强调宽容解析；用于服务端前必须核对重复Content-Length、TE/CL冲突、非法头、消息边界与关闭策略，不能将两个正常合成用例当作完整服务器验收。[上游说明](https://github.com/creationix/http-parser-js#http-parser)。

## 5. DNS：JS 主方案与小 Rust 库的实测比较

两个 `dns-packet` 是不同项目：npm [`dns-packet` 5.6.1](https://github.com/mafintosh/dns-packet) 是多种RR编解码库；Rust [`dns-packet` 0.1.0](https://docs.rs/dns-packet/0.1.0/dns_packet/) 是更底层的Reader/Writer。Rust版确实轻，`no_std`且只有可关闭的log依赖，不能把它误算成完整的大型resolver。

| 项目 | JS dns-packet | Rust dns_packet |
|---|---|---|
| 体积样本 | 与stream/Buffer共享后的raw DEFLATE边际 **8,541 bytes（8.3 KiB）** | 同一微型release harness从286,160增至302,720，差 **16,560 bytes（16.2 KiB）** |
| 记录值 | 已有A/AAAA/CNAME/MX/TXT/NS/PTR/SOA/SRV/NAPTR/CAA/TLSA等typed data | Resource只返回记录头和RDLENGTH；RDATA需调用方读取/解码 |
| 压缩名样例 | 单指针及指向已有压缩名的指针链均解码成功 | 普通单跳压缩名成功；指向压缩指针的样例被拒绝 |
| 查询网络/重试 | 不提供 | 不提供 |
| 接入当前目标 | 复用已规划dgram/net | 仍需补typed RDATA、边界与查询生命周期 |

Rust尺寸是隔离harness的实际差分，不是Niva内的最终链接差分；两列的覆盖范围不同，不能据此宣称Rust在Niva一定比JS大。Rust的7个测试中有一个是“确认指针链被拒绝”的预期缺口测试，不能说它通过了该兼容用例。JS六个合成报文检查通过；输入需转换成packaged Buffer。没有网络、WebView或全面畸形报文验收。

推荐本轮按JS主方案：`resolve/resolve4`用JS codec + 已规划UDP/TCP；`lookup`保留系统解析。不是因为Rust必然大，而是当前JS库覆盖更完整，共享体积小，符合JS优先。若以后选择Rust，应先把功能补到同一范围再做Niva release A/B，而不是只比较crate源码大小。

DNS通常走UDP，但必须处理TCP回退；Node lookup与resolve的语义也不同。[RFC 7766](https://www.rfc-editor.org/rfc/rfc7766)、[Node DNS文档](https://nodejs.org/api/dns.html)、[DNS名称压缩](https://www.rfc-editor.org/rfc/rfc1035)。查询客户端还要补ID/来源匹配、超时重试、截断回退、系统DNS服务器配置、Node结果/错误格式；不另建从根服务器迭代的递归解析器，也不默认改用固定公共DoH服务。

## 6. 现有 Native HTTP 的迁移与体积

当前 Rust `http.requestStream` 使用 ureq；NodeCompat HTTP、初始化脚本的 `Niva.api.http.get/post/request`、Devtools版本检查及现有示例仍在调用。迁移所有调用后才删除这套旧Native客户端及相关依赖，目标不保留两套重复实现。[Native客户端](../crates/niva/src/app/api/http.rs#L10)、[NodeCompat调用](../packages/node-compat/src/runtime/http.js#L299)、[Devtools调用](../packages/devtools/src/common/utils.ts#L159)。

依赖树中 `ureq-proto`、`utf8-zero`、`der`、`webpki-root-certs` 等是潜在随ureq退出的项；`native-tls`仍供TLS基座使用，`flate2`仍供资源解压使用，url/其他共用依赖不能一起删除。尚未做依赖移除的release A/B，**不提前计入任何节省字节**。

**Niva 内部的 Rust HTTP/WS 服务保留。** 它服务启动资源、鉴权文件路由和Native API WebSocket；它与应用开发者创建的Node http.Server是两个职责，后者在JS上依赖Native net.listen/accept即可。[内部服务](../crates/niva/src/app/http_server/mod.rs#L22)。

重新计费后，Native net/tls/dgram各计一次；http/https不再增加独立Native协议后端，增加JS parser与对象封装预算。当前所有已量化项合算约 **2.52–3.18 MB**，尚未抵扣删除ureq的可能收益。比旧分层更有机会满足3.3 MB，但依然是预算，不是完整179项功能已构建达标。[逐模块预算](node-api-size-estimates.md)。

## 7. 落地顺序和验收

1. 完成Native socket/TLS基础契约和JS stream适配，验证关闭、取消、背压、证书和窗口归属。
2. 完成JS DNS客户端和HTTP/HTTPS客户端/服务器，测试真实UDP/TCP回退、服务端请求响应及严格报文边界。
3. 将现有NodeCompat/Devtools调用迁入统一路径，再删除旧Native HTTP客户端与不再使用的依赖。
4. 对完整目标逐平台构建release并测量包含内嵌JS的主程序；低于3,300,000 bytes才宣布内嵌门禁通过。

本轮仅更新评估。Rust DNS测试、JS DNS/HTTP合成测试、精确版本/入口和Native源码审计保存在 [node-network-layer-evidence.json](node-network-layer-evidence.json)；没有改动应用源码、依赖清单或实际运行时路径。
