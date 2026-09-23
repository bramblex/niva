# WebView webview

`loadUrl()` 只接受绝对 HTTP(S) URL；`loadHtml()` 不接收 base URL。`evaluateScript()` 只在当前 WebView 主页面执行，UTF-8 字节上限 64 KiB，且不返回脚本结果。`cookies()` / `cookiesForUrl()` / `setCookie()` / `deleteCookie()` 操作 Wry 共享数据存储；Android 上 Wry 返回空 cookies，写 cookie 会以不支持错误拒绝。

`baseUrl()` 返回本地服务根 URL；`baseFileSystemUrl()` 返回该窗口的 `__niva_fs` 文件服务根 URL，其中含窗口 token。将后者视为 capability URL，不应传给不受信任页面。远端 IPC 路径始终拒绝 `webview.baseFileSystemUrl`，即使 grant 中列出；见[远端页面权限](./permissions)。

Windows WebView2 所用的 Wry 0.57 能打开开发工具，但其 `is_devtools_open()` 固定返回 `false`，`close_devtools()` 不执行关闭操作。因此 Windows 上不能用 `isDevtoolsOpen()` 判断实际状态，也不能依赖 `closeDevtools()` 关闭窗口；这两项仍需受监督的原生 UI 验收。

```ts
interface NivaWebview {
  /** 在当前 WebView 主页面执行脚本。仅接受 UTF-8 字节数不超过 64 KiB 的脚本，不返回脚本结果。 */
  evaluateScript(script: string): Promise<void>;
  /** 将当前 WebView 导航到绝对 HTTP(S) URL。相对 URL 和其他 scheme 会拒绝；可用 baseUrl() 拼接本地资源地址。 */
  loadUrl(url: string): Promise<void>;
  /** 将 HTML 文本加载到当前 WebView；此接口不接收 base URL。 */
  loadHtml(html: string): Promise<void>;
  /** 重新加载当前页面。 */
  reload(): Promise<void>;
  /** 获取当前页面 URL。 */
  url(): Promise<string>;
  /** 打开系统打印流程。 */
  print(): Promise<void>;
  /** 后退到上一条历史记录。 */
  goBack(): Promise<void>;
  /** 前进到下一条历史记录。 */
  goForward(): Promise<void>;
  /** 查询是否可以后退。 */
  canGoBack(): Promise<boolean>;
  /** 查询是否可以前进。 */
  canGoForward(): Promise<boolean>;
  /** 获取当前 Wry WebContext 共享数据存储中的 Cookie，返回 Set-Cookie 格式字符串。Android 上 Wry 返回空数组。 */
  cookies(): Promise<string[]>;
  /** 获取指定 URL 可用的 Cookie，返回 Set-Cookie 格式字符串；筛选遵循各平台 Wry 实现。 */
  cookiesForUrl(url: string): Promise<string[]>;
  /** 设置共享 WebContext 中的 Cookie，参数使用 Set-Cookie 格式字符串；其他窗口可能同时看到此 Cookie。Android 上会以不支持错误拒绝。 */
  setCookie(cookie: string): Promise<void>;
  /** 从共享 WebContext 删除 Cookie，参数传入含 name、domain、path 的 Set-Cookie 格式字符串；其他窗口可能同时看到此变更。Android 上会以不支持错误拒绝。 */
  deleteCookie(cookie: string): Promise<void>;
  /** 请求清理底层 WebView 数据存储的全部浏览数据；同一存储上下文中的其他窗口也可能受影响。 */
  clearAllBrowsingData(): Promise<void>;
  /**
   * 检查开发工具是否打开。Windows WebView2 的 Wry 0.57 固定返回 false，不能据此判断实际状态。
   * @returns 一个 Promise，在检查成功时解析该 Promise，或在发生错误时拒绝该 Promise。成功时返回布尔值，表示开发工具是否打开。
   */
  isDevtoolsOpen(): Promise<boolean>;
  /**
   * 打开开发工具。
   * @returns 一个 Promise，该 Promise 始终解析。
   */
  openDevtools(): Promise<void>;
  /**
   * 关闭开发工具。Windows WebView2 的 Wry 0.57 不执行关闭操作。
   * @returns 一个 Promise，该 Promise 始终解析。
   */
  closeDevtools(): Promise<void>;
  /**
   * 获取应用程序的基本 URL。
   * @returns 一个 Promise，在获取成功时解析该 Promise，或在发生错误时拒绝该 Promise。成功时返回应用程序的基本 URL。
   */
  baseUrl(): Promise<string>;
  /**
   * 获取应用程序文件系统的基本 URL。
   * @returns 一个 Promise，在获取成功时解析该 Promise，或在发生错误时拒绝该 Promise。成功时返回应用程序文件系统的基本 URL。
   */
  baseFileSystemUrl(): Promise<string>;
}
```
