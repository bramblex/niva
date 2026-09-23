# 对话框 dialog

消息框、单选和多选文件/目录对话框。取消选择时 Promise 解析为 `null`。

```ts
interface NivaDialog {
  /**
   * 显示一个独立消息框。
   * @param title 消息框的标题。
   * @param content 消息框的内容；省略时按空字符串显示。
   * @param level 消息框的级别。
   * @returns 一个 Promise，在消息框关闭时解析该 Promise，或在发生错误时拒绝该 Promise。
   */
  showMessage(
    title: string,
    content?: string,
    level?: "info" | "warning" | "error"
  ): Promise<void>;
  /**
   * 在文件系统中选择一个文件，支持过滤器和起始目录。
   * @param filters 文件类型筛选器。
   * @param startDir 文件选择对话框的起始目录。
   * @returns 一个 Promise，在选择文件时解析该 Promise 以返回文件名或文件路径，或解析 `null`（如果没有选择文件）。
   */
  pickFile(filters?: string[], startDir?: string): Promise<string | null>;
  /**
   * 在文件系统中选择多个文件；取消对话框时解析为 null。
   * @param filters 文件类型筛选器。
   * @param startDir 文件选择对话框的起始目录。
   * @returns 选择的文件路径数组，或在取消时返回 null。
   */
  pickFiles(filters?: string[], startDir?: string): Promise<string[] | null>;
  /**
   * 在文件系统中选择一个文件夹，支持起始目录。
   * @param startDir 文件夹选择对话框的起始目录。
   * @returns 一个 Promise，在选择文件夹时解析该 Promise 以返回文件夹路径，或解析 `null`（如果没有选择文件夹）。
   */
  pickDir(startDir?: string): Promise<string | null>;
  /**
   * 在文件系统中选择多个文件夹；取消对话框时解析为 null。
   * @param startDir 文件夹选择对话框的起始目录。
   * @returns 选择的文件夹路径数组，或在取消时返回 null。
   */
  pickDirs(startDir?: string): Promise<string[] | null>;
  /**
   * 在文件系统中保存一个文件，支持过滤器和起始目录。
   * @param filters 文件类型筛选器。
   * @param startDir 文件保存对话框的起始目录。
   * @returns 一个 Promise，在保存文件时解析该 Promise 以返回文件名或文件路径，或解析 `null`（如果没有保存文件）。
   */
  saveFile(filters?: string[], startDir?: string): Promise<string | null>;
}
```
