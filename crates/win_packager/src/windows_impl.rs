//! Windows 实现（PE 资源写盘）。本文件只在 Windows 上编译。
//!
//! 输入已经是备好的字节（`lib.rs::PreparedData`）：文件 IO、资源打包、
//! PNG 转 ICO 都在跨平台侧做完了，这里只做 `UpdateResource` 写盘。
//!
//! # Windows 上待实现清单（给实现者）
//!
//! 1. `apply`：`BeginUpdateResourceW(output_exe, false)` 拿句柄，按序调下面三个
//!    子步骤，最后 `EndUpdateResourceW(handle, false)` 提交；任何一步失败用
//!    `EndUpdateResourceW(handle, true)` 丢弃。错误经 `windows::core::Error`
//!    转 `anyhow`，并带上当前步骤名。
//! 2. `update_rcdata`：`data.rcdata` 逐条
//!    `UpdateResourceW(handle, RT_RCDATA(10), name, lang, bytes)`。
//!    `name` 是字符串资源名，先 `encode_utf16 + NUL`（参考
//!    `crates/niva/.../win_utils.rs::to_wstr`），注意 `PCWSTR` 生命周期必须
//!    活到调用结束。
//! 3. `replace_icon`：`data.icon` 为 `None` 时直接返回。否则先删
//!    `req.delete_icon_ids`（`UpdateResourceW(handle, RT_ICON(3),
//!    MAKEINTRESOURCE(id), lang, null, 0)` 逐个删，`GetLastError` 为
//!    `ERROR_RESOURCE_DATA_NOT_FOUND` 时视为已无图标、继续）；再用 `ico` crate
//!    解析 `ico字节` 的 `ICONDIR`，为每个 image 写 `RT_ICON(id=N...)`，最后组
//!    `GRPICONDIR` 写 `RT_GROUP_ICON(14), group_id`。新 `ICON` id 从旧 id 之后
//!    顺延，避免复用残留。语言统一用 `req.lang`。
//! 4. `apply_version`：把 `req.version_info_rc` 的 `VERSION_INFO` 文本
//!    （`1 VERSIONINFO ...` 的 `.rc` 语法）转成 `VS_VERSIONINFO` 二进制再
//!    `UpdateResourceW(handle, RT_VERSION(16), MAKEINTRESOURCE(1), lang, ...)`。
//!    二选一：(a) 调 SDK `rc.exe /fo tmp.res` 再解析 `.res` 提取 VERSION 段
//!    （简单，但要求构建机有 SDK）；(b) 纯 Rust 手拼 `VS_VERSIONINFO`
//!    （无外部依赖，但要小心 DWORD 对齐 + UTF-16 字段，参考 `winres` crate
//!    的拼法）。建议先做 (a)，跑通后再做 (b)。
//! 5. 验证：拿 `currentExe` + 高层模式备好的包跑一遍，与旧 ResourceHacker 产物
//!    逐字节比 `RCDATA`、用资源查看器比图标与版本页；确认 `sign-windows.ts`
//!    仍在打包后签名。

use anyhow::Result;

use crate::{PackRequest, PreparedData};

pub(crate) fn apply(req: &PackRequest, data: &PreparedData) -> Result<()> {
    let _ = (req, data);
    update_rcdata(req, data)?;
    replace_icon(req, data)?;
    apply_version(req, data)?;
    Ok(())
}

fn update_rcdata(_req: &PackRequest, _data: &PreparedData) -> Result<()> {
    // TODO(windows): 见模块文档第 2 条。
    todo!("update_rcdata: BeginUpdateResourceW + UpdateResourceW(RT_RCDATA)")
}

fn replace_icon(_req: &PackRequest, _data: &PreparedData) -> Result<()> {
    // TODO(windows): 见模块文档第 3 条；data.icon 为 None 时直接 Ok。
    todo!("replace_icon: delete RT_ICON + write RT_ICON/RT_GROUP_ICON")
}

fn apply_version(_req: &PackRequest, _data: &PreparedData) -> Result<()> {
    // TODO(windows): 见模块文档第 4 条；req.version_info_rc 为 None 时直接 Ok。
    todo!("apply_version: VERSIONINFO rc -> RT_VERSION")
}
