---
sidebar_position: 2
---

# 远端页面权限

远端页面通过原生 IPC 调用 API 时，Niva 按**窗口配置、调用 frame 的来源 URL 和 API 方法名**逐次授权。没有匹配项时拒绝调用。该 grant 只适用于非流式 JSON API；流式 API 和二进制传输始终不能通过 IPC 授权。

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

## 调用时的校验

远端页面没有本地 WebSocket 凭据。macOS IPC 从 WebKit 消息的发送 frame URL 取来源；Windows IPC 从 WebView2 frame 消息取来源，并在回包前检查导航状态。Rust 根据来源 origin、窗口自己的 permissions 和完整 API 名称进行检查，不信任消息体中的 `wid` 或 origin。

- 单次 IPC 请求和响应上限均为 256 KiB；等待回复超时为 60 秒。
- IPC 只支持注册为 unary JSON 的原生 API。`Niva.stream`、二进制帧和流式 handler 不可用。
- `window.open` 与 `webview.baseFileSystemUrl` 即使在 grant 中列出也会被拒绝。
- API 错误会使 `Niva.call()` 返回的 Promise reject；权限拒绝使用 bridge 错误码 `-4`。

```ts
try {
  const title = await Niva.call("window.title", []);
  console.log(title);
} catch (error) {
  // 未配置 origin 或 method 时会在这里拒绝。
  console.error(error);
}
```

`permissions` 控制远端 IPC，不控制本地 WebSocket 页面。同源 frame 按浏览器同源规则可访问顶层页面的本地 bridge 凭据；请只在信任本地包页面及其同源脚本时使用完整本地 API。详见[Bridge 与传输方式](./bridge)。

## 验收边界

这些规则来自当前 Rust handler 和平台 IPC 实现。Windows frame 来源、递归 iframe 处理及导航期间的回包行为尚需在目标 Windows WebView Runtime 上真机确认；代码存在不等同于目标平台验收通过。
