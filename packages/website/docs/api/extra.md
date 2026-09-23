# 系统额外 extra

`hideApplication`、`showApplication`、`hideOtherApplications` 和 `setActivationPolicy` 仅适用于 macOS。活动窗口 ID 和焦点 API 的可用性取决于目标平台。

```ts
interface NivaExtra {
  /**
   * 隐藏应用程序，仅适用于 macOS。
   * @returns 一个 Promise，在应用程序成功隐藏时解析该 Promise，如果发生错误则拒绝该 Promise。
   */
  hideApplication(): Promise<void>;
  /**
   * 显示应用程序，仅适用于 macOS。
   * @returns 一个 Promise，在应用程序成功显示时解析该 Promise，如果发生错误则拒绝该 Promise。
   */
  showApplication(): Promise<void>;
  /**
   * 隐藏其他应用程序，仅适用于 macOS。
   * @returns 一个 Promise，在其他应用程序成功隐藏时解析该 Promise，如果发生错误则拒绝该 Promise。
   */
  hideOtherApplications(): Promise<void>;
  /**
   * 设置应用程序的激活策略，仅适用于 macOS。
   * @param policy 要设置的激活策略。
   * @returns 一个 Promise，在激活策略成功设置时解析该 Promise，如果发生错误则拒绝该 Promise。
   */
  setActivationPolicy(
    policy: "regular" | "accessory" | "prohibited"
  ): Promise<void>;
  /**
   * 获取当前活动窗口的 ID，仅适用于 macOS 和 Windows。
   * 对于 macOS，将返回 `process_id_window_id` 的格式，其中 `process_id` 和 `window_id` 为整数。
   * 对于 Windows，将返回窗口句柄的字符串形式。
   * @returns macOS 返回活动窗口 ID 或无活动窗口时的 null；Windows 始终返回前台窗口句柄字符串。
   */
  getActiveWindowId(): Promise<string | null>;
  /**
   * 将焦点设置到特定 ID 的窗口，仅适用于 macOS 和 Windows。
   * 对于 macOS，ID 应该是 `process_id_window_id` 的格式，其中 `process_id` 和 `window_id` 为整数。
   * 对于 Windows，ID 应该是窗口句柄的字符串形式。
   * @param id_string 要设置焦点窗口的 ID 字符串。
   * @returns macOS 返回是否成功激活；Windows 调用 SetForegroundWindow 后解析为 void，但操作系统可能拒绝切换焦点而不使调用失败。格式错误的 ID 会拒绝。
   */
  focusByWindowId(idString: string): Promise<boolean | void>;
}
```
