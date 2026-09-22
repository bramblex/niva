# Origin 权限设计（bridge 授权层）

> 状态：方案（未实现）。解决 `docs/security.md` P0-1（远端 entry 自带全桥 = RCE）。
> 与 `docs/http-auth-plan.md` 正交互补：本篇管“**谁能调 API**”，
> http-auth 管“**谁能读 loopback 字节**”，都要做。

## 1. 决策

- 信任根：**本地包 = 全权**（resource bundle / 本地 server 源的页面）。
  远端 URL = **默认零权限**，除非命中 `niva.json` 的显式授权。
- 授权表（`niva.json` 顶层）：

```jsonc
"permissions": {
  "https://app.example.com": ["window.*", "clipboard.*"],
  "*.trusted-cdn.example": ["resource.read"],
  // 未列出的 origin：零 API 权限（连 window.title 都调不了）
}
```

- 粒度：`namespace.*` 或 `namespace.method`（对标 Tauri capabilities）。
  先给命名空间级，方法级是其子集实现，不加复杂度。

## 2. 为什么不用自由正则

`niva.json` 的 key **只支持两种**：精确 origin（含 scheme+host+port）、
`*.domain` 子域通配。不支持自由正则，原因：

1. 未锚定正则 `example.com` 会匹配 `evil-example.com`——配错即 RCE，
   而配的人是普通开发者，不是安全工程师；
2. 回溯型 ReDoS（鉴权是高频路径，每次 call 都走）；
3. 真有怪需求：exact + 通配已覆盖 99%（SaaS 多租户走 `*.` 即可）。

匹配规则：先精确、后通配（最长后缀胜），`http`/`https` 不同源，
端口参与比对（`:3000` ≠ `:3001`）。

## 3. 执行点（必须在 Rust，不在 JS）

- `ApiManager::dispatch` 按 wid 查**当前 origin** → 查授权表 → 拒绝 theological。
- 当前 origin 来源：建窗 entry URL + 导航记录（现有 `with_navigation_handler`
  处同步更新）。JS 上报的 origin 只做参考，不做依据——依据是 native 侧看到的
  导航 URL。
- iframe：init 脚本 hello 帧加 `origin` 字段（`location.origin`），Rust 校验与
  导航记录一致；子 frame 权限 = **父 grant ∩ 自身 origin grant**（默认 deny）。
  这需要 wire 加字段（`bridge.md` 同步，可选字段，不 bump 大版本）。
- 拒绝码：wire 新增 `code -4 permission denied`（现有 0/-1/-2/-3 不动，
  `bridge.md` 同步）。

## 4. localhost 开发位

`http://localhost:*` / `http://127.0.0.1:*` 的授权**仅 dev 生效**
（`--debug-entry` 场景），发版包不允许配 localhost grant——本机任意进程都
能绑端口，写进校验：发版构建遇到 localhost grant 直接报错。

## 5. 子窗口：open 时的完整配置（主窗口授权）

`window.open(options)` 的 options 即完整窗口配置：除现有几何/主题/菜单外，
新增两类字段（全可选，反序列化兼容，老调用零改动）：

- `permissions`：该子窗口的显式授权（同 §1 形状，`["window.*", …]`）。
- `preload`：资源包内 JS 路径，open 时读出、拼在 bootstrap 初始化脚本之后、
  页面脚本之前执行；另 `injectScripts: string[]` 行内脚本（经 WS JSON 直传，
  小段逻辑用）。

规则（ attenuation：只能收窄，不能放大 ）：

1. 子窗口 effective = open 指定 ∩ 全局表(子 origin) ∩ opener 自身 grant。
   超出的部分静默钳制 + stderr 打一行 warning（不直接炸 open，前端 fail-open
   比 fail-closed 更容易错配权限）。
2. open 指定的 grant **钉在 open 时的 entry origin 上**（origin pinning）：
   子窗口一旦跨域导航，立即回落到全局表判定。防止“带权的窗漂到恶意页”。
3. preload 以子窗口 effective 权限运行；给远端 entry 配 preload + 强权限是
   显式委托——允许，但 devtools 模板给警告注释。
4. 不指定 = 走全局表（旧行为，兼容）。

实现面：`NivaWindowOptions` 加字段 → `WindowManager` 存每窗
`explicit_grant + pinned_origin` → dispatch 查 effective；
`build_webview` 在 bootstrap 之后追加 preload 文本。`packages/types` d.ts 同步。

## 6. 落地步骤

1. `niva.json` 加 `permissions` 结构 + 校验（发版禁 localhost）。
2. dispatch 加 origin→grant 检查 + `-4` 码 + `bridge.md` 同步。
3. hello 加 `origin` + iframe 交集规则。
4. 导航 origin 更新 + 单测（精确/通配/端口/iframe 降级）。
5. 子窗口：`NivaWindowOptions` 加 `permissions/preload/injectScripts` +
   每窗 `explicit_grant + pinned_origin` + effective 钳制 + d.ts。
6. devtools 模板注释 + 文档站：remote entry 默认零权限写死在模板里。
