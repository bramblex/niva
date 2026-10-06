---
sidebar_position: 2
---

# 远端页面权限

远端页面通过原生 IPC 调用 API 时，Niva 按**窗口配置、来源 URL/origin 和 API 方法名**逐次授权。没有匹配项时拒绝调用。远端顶层页面的 grant 只适用于非流式 unary JSON API；流式 API 和二进制传输不能通过 grant 授权。

## 配置

`permissions` 是窗口配置项，key 为完整 origin，value 是允许调用的方法名：

```json
{
  "window": {
    "entry": "https://app.example.com/",
    "permissions": {
      "https://app.example.com": ["window.title", "clipboard.*"],
      "https://tools.example.com:8443": ["dialog.showMessage"]
    }
  }
}
```

origin 必须是精确的 `http` 或 `https` 来源，包括协议、主机和端口；不能包含路径、query、fragment 或用户信息。不同子域或端口要分别配置。方法规则可以是完整的 `namespace.method`，也可以是 `namespace.*`。不存在 origin 通配符。

请只为完全可信的远端页面授予高影响方法。`fs.*`（尤其 `fs.node`）可执行文件系统操作；它受 Native 支持的操作范围约束，但 grant 不按目录隔离。`process.execText` 和 `process.execFileText` 可以运行本机命令。网页脚本供应链若被攻破，这些权限可能转化为本机文件修改或命令执行能力。

## 调用时的校验

远端页面没有本地 WebSocket 凭据，也不能加入本地IPC Channel session。Native IPC 统一走 Wry 消息处理；Wry 不提供可依赖的真实 Native child-frame ID。同源 iframe 的请求经父页 relay 到顶层 IPC host，并按来源 URL/top-origin、session 与完整 API 名称校验；跨源 iframe fail-closed。Rust 不信任消息体中的 `wid` 或 origin。Windows 上的来源与导航行为仍需目标设备验证。

- 单次IPC请求JSON最多256KiB、响应最多8MiB；高层操作还有body/输出与超时限制，在途IPC采用失联租约。
- 远端 IPC只支持明确白名单内的一次性JSON操作；仅注册为Unary并不足以放行。`Niva.bridge.stream`、Native同步、二进制帧和持久handler不可用。
- `window.open` 与 `webview.baseFileSystemUrl` 即使在 grant 中列出也会被拒绝。
- API 错误会使 `Niva.bridge.call()` 返回的 Promise reject；权限拒绝使用 bridge 错误码 `-4`。

```ts
try {
  const title = await Niva.bridge.call("window.title", []);
  console.log(title);
} catch (error) {
  // 未配置 origin 或 method 时会在这里拒绝。
  console.error(error);
}
```

`permissions` 控制远端 IPC，不控制可信本地页面的WS/IPC Channel session。同源 frame 按浏览器同源规则可访问顶层页面的本地 bridge 凭据；请只在信任本地包页面及其同源脚本时使用完整本地 API。详见[Bridge 与传输方式](./bridge)。

## 验收边界

这些规则来自当前 Rust handler 与统一 runtime。嵌套同源 iframe 的顶层 host 路由及跨源祖先拒绝已有 runtime 回归覆盖；Windows WebView Runtime 上的来源、导航、回包和真实 iframe 行为仍需真机确认，单测与 target check 不等于目标平台验收。
