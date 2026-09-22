# HTTP 静态服务鉴权方案（session-cookie）

> 状态：方案（未实现）。动机：`/ карда/__niva_fs/` 可读任意本地文件，
> loopback 裸奔 = 本机任意进程可读用户文件，属明显安全漏洞。

## 1. 目标

- 只有我们自己的 webview 能读 server 上的静态资源（含 `__niva_fs`）。
- 不破坏页面兼容（不动 UA、不改用户文件、不要求浏览器新能力）。
- 保持现状：WS 走 token 不动；开关关闭的 nodeCompat 等行为不变。

## 2. 设计

### 2.1 密钥（与 WS 统一，单个）

- 不新增密钥：复用启动时生成的 WS `token`（128 位随机，只存内存，
  不接受环境变量、参数或网络传入）。
- 同一密钥的三种形态：WS 用 `?token=` query，HTTP 首验用
  `?niva_token=` query，HTTP 后续用 `niva_session` cookie 存同一值。
  同一信任域（loopback + 页面本就持有 token），不增加暴露面。

### 2.2 认证流程

```text
启动
 ├─ 生成 token（唯一密钥，WS 与 HTTP 共用）
 └─ entry URL 由 native 构造：{base}/{entry}?niva_token={token}
      （注入给网页 bootstrap 的 token 与之一致，单点来源）

首次导航 GET /?niva_token={token}
 ├─ 命中 → Set-Cookie: niva_session={token}; Path=/; HttpOnly; SameSite=Lax
 │         → 200 正常 serve
 └─ 未命中 → 403（复用 error_page，文案"请从应用窗口打开"）

后续一切请求（导航/iframe/子资源/fetch，相对+绝对路径）
 └─ Cookie: niva_session={token} → 比对通过 → serve；否则 403
```

- entry 刷新/重载：URL 自带 token，每次都可重验续 cookie。
- 上次启动的旧 cookie：值对不上 = 无身份（entry 重进即续）。
- 多窗口：同一 profile 共享 cookie jar，天然共享 session；窗口级绑定仍在 WS 层按 wid 做。
- 比较用常量时间比对（顺手，防时序侧信道——loopback 上意义不大，但零成本）。

### 2.3 豁免路径

- `/__niva_compat/*`：公开版本化 JS，无秘密，不鉴权；补
  `Access-Control-Allow-Origin: *`（dev-server 页面跨域加载 module 脚本需要 CORS；
  classic script 本来不需要，module 需要）。
- 其余一切（含 `/__niva_fs/`、资源文件、404 页本身）：必须鉴权。
  注意 404 也要先鉴权再返回，否则成端口存活探针——不，403 与 404 都拒绝未授权，
  统一先鉴权。

### 2.4 明确不做

- UA 标记：三端全替换语义，破坏页面兼容（已决策，见 node-compat-design §7.5）。
- 自定义请求头：wry 只管首请求，无全量通道。
- 一次性 nonce：状态管理复杂，相对固定 sid 零收益（同为 128 位不可猜）。
- Referer/Origin 检查：本地进程随手伪造，不作为依据。

## 3. 改动面（Rust，http_server 内聚）

1. `ServerState` 已有 `token`，零新增字段；`start` 日志只打端口不打 token（现状）。
2. `handle_conn`：解析 query（`niva_token`）与 `Cookie` 头（`niva_session`，
   值均为 token）→ `is_authed()` 判定 → 未授权直接 403（在路由之前，
   含 `/__niva_fs/`）。
3. entry 构造处（builder）：URL 追加 `?niva_token={token}`
   （注意 entry 本身可能带 query → 用 `&` 拼接；`url_join` 后处理；
   与注入给网页的 `__niva_token` 同一来源）。
4. 403 复用 `error_page_html`，文案区分"缺文件 404"与"未授权 403"。
5. compat 响应加 CORS 头。

## 4. 测试

- 单测：cookie 解析（多 cookie/空格/大小写）、token 比对、豁免路径表。
- raw-socket e2e：无 cookie → 403；错 cookie → 403；entry token → 200 + Set-Cookie（token 即 session 值）；
  带 cookie 复访 → 200；compat 无 cookie → 200 + CORS 头；`__niva_fs` 无 cookie → 403。
- 回归：现有 WS 电池、iframe 双标记、devtools 双路径（entry 自动带 token，
  devtools 零改动）。

## 5. 风险

- 用户把带 token 的 entry URL 拷给别人：同 loopback 同 profile 才有效，
  攻击者需已在本机——威胁模型内可接受，文档注明。
- 旧引擎无 Cookie？不存在：所有 WebView 后端 cookie 行为正常。
- devtools 调试时直接 curl 资源：需手动带 cookie——devtools 自身不受影响
  （走 webview）；文档给一条 curl 示例。
