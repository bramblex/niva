# Niva 安全评估（2026-09-22，以代码实测为准）

## 0. 信任模型（一句话）

**能跑在桥接 frame 里的 JS = 完全可信 = 持有用户全部权限**
（等价于 Electron `nodeIntegration: true`）。一切结论从此推出：
要防的不是"API 太强"（exec/fs 本来就是 RCE 级别），而是
"**不可信的 JS 如何混进桥接 frame**"。

## 1. 发现（按严重度）

### P0-1. 远端页面/iframe 自带完整桥（含 token）——已实锤

- `builder.rs:266` 初始化脚本 `for_main_frame_only(false)`：**所有 frame
  全部注入**，包括远端 https 页面与跨域 iframe，且 bootstrap 里直接带 token。
- `builder.rs:277` 导航白名单放行 `entry_prefix`：entry 配成远端 URL 即合法。
- 后果：远端页上的任意脚本（CDN 投毒、广告、XSS、恶意 iframe）直接获得
  `fs`/`exec`/`dialog` 全能力 = 以用户身份 RCE。
- 修复方向（二选一，需决策）：entry 信任分级——local entry 全桥，
  remote entry **默认不注入**（或只读子集）；跨域 iframe 同理。
  在修好之前，文档必须写明"entry 配远端 URL = 裸奔"。

### P0-2. 资源目录路径穿越——已实锤

- `resource_manager/mod.rs:54`：`root_dir.join(path)` 无任何规范化，
  `GET /../../etc/passwd` 直接逃出资源目录（hand-rolled server 同样不做
  `%2e` 归一，需一并处理）。
- `__niva_fs` 读任意文件是**设计如此**，但资源目录穿越是**非预期**，
  必须修：canonicalize 后校验前缀，逃逸直接 403。
- http-auth-plan 落地时把这一条带上（同属"读路径" hardening）。

### P1-1. 无 Host 头校验 → DNS rebinding 敞开——已实锤

- `http_server` 不检查 `Host`：`evil.com` 解析到 127.0.0.1 后，
  受害者浏览器里的攻击页可直读 server 资源（含 `__niva_fs`，现状无鉴权）。
- http-auth-plan 落地后 session-cookie 会挡住读路径（攻击者拿不到 token/sid），
  但 Host 校验是零成本纵深，顺手加上：只接受 `127.0.0.1`/`localhost` + 本实例端口。
- WS 层同理可加 `Origin` 校验（浏览器握手必带 Origin），与 token 双保险。

### P1-2. `http.*` API 无 SSRF 防护——设计如此，需文档化

- 页面 JS 可请求任意 URL，包括 `169.254.169.254`（云元数据）、内网服务，
  且顶的是**应用本体**的网络身份。
- 短期：文档 + devtools 模板注释警示；中期：`http` 加可配置 blocklist
  （默认拦 `169.254.0.0/16` + `127.0.0.0/8` 中非本实例端口）。

### P1-3. 跨域现状：默认同源，无 CORS 头——保持即可

- server 当前不发任何 CORS 头：远端页 fetch 读 loopback 资源会被浏览器拦，
  这是好事。`/__niva_compat/*` 豁免时要补的 `Access-Control-Allow-Origin: *`
  只给这一个路径，别扩大。

## 2. eval 专项（用户提问）

结论：**eval 本身不产生新权限，它只是把"谁的代码"这个问题转交出去**。

- `eval(自己写的代码)`：零新增风险（本来就全权）。
- `eval(fetch(远端代码))`：= 把远端代码请进桥接 frame，
  **等价于 P0-1**，远端作者获得全部能力。供应链（CDN 投毒、中间人 http、
  被黑的 npm 包经打包进入）是主要现实路径。
- 给 AI/用户的规矩（三条，写进 devtools 模板注释 + 文档站）：
  1. 永远不要 `eval` 网络拉回的代码；要动态能力走 `Niva.import()` + importmap
    （有版本、有来源，可审计）。
  2. 打包产物建议开 CSP：`script-src 'self'`，不要 `unsafe-eval`
    （注意：这会同时禁掉用户自己的 eval，按需取舍，默认模板给开）。
  3. 真要跑不可信代码：起独立窗口 + remote entry + 等 P0-1 修好后的"无桥模式"，
     而不是在主窗口里 eval。

## 3. 已有mitigations（守住，别退化）

- token/sid 只存内存、不接受外部传入；WS 缺 token 直接 403（已验证）。
- release 2.3MB 无 bundler 生态，供应链面小（反面：旧依赖四件套见 roadmap P1）。
- `is_document_navigation` 只影响改写判定，不参与鉴权（鉴权只看 cookie/token）。

## 4. 行动清单（转 roadmap 跟踪）

- [ ] entry 信任分级（remote 默认无桥）—— P0，方案见 `docs/permission-design.md`
  （本地包全权 + 远端默认零权 + niva.json origin 授权表，`*.` 通配，不用自由正则）- [ ] 资源路径穿越修复（canonicalize + 前缀校验 + `%2e` 归一）—— P0，搭 http-auth-plan 便车
- [ ] Host 头校验 + WS Origin 校验 —— P1，小
- [ ] `http` SSRF blocklist（先文档警示） —— P1
- [ ] CSP 默认模板 + eval 规矩文档化 —— P1
