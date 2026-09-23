//! Windows 实现（PE 资源写盘）。本文件只在 Windows 上编译。
//!
//! 输入已经是备好的字节（`lib.rs::PreparedData`）：文件 IO、资源打包、
//! PNG 转 ICO 都在跨平台侧做完了，这里只做 `UpdateResource` 写盘。
//!
use anyhow::{Context, Result, anyhow};
use std::path::Path;
use windows::Win32::Foundation::FreeLibrary;
use windows::Win32::System::LibraryLoader::{
    BeginUpdateResourceW, EndUpdateResourceW, FindResourceExW, LOAD_LIBRARY_AS_DATAFILE,
    LoadLibraryExW, UpdateResourceW,
};
use windows::core::PCWSTR;

use crate::{PackRequest, PreparedData};

pub(crate) fn apply(req: &PackRequest, data: &PreparedData) -> Result<()> {
    let existing_icon_ids = existing_icon_ids(req, data)?;
    let exe_path = to_wide_path(&req.output_exe)?;
    // SAFETY: exe_path is NUL-terminated and remains alive for this call.
    let handle = unsafe { BeginUpdateResourceW(PCWSTR(exe_path.as_ptr()), false) }
        .with_context(|| format!("begin resource update for {}", req.output_exe.display()))?;

    let update_result = (|| {
        update_rcdata(handle, req, data)?;
        replace_icon(handle, req, data, &existing_icon_ids)?;
        apply_version(handle, req, data)?;
        Ok(())
    })();

    if let Err(update_error) = update_result {
        // SAFETY: handle was returned by BeginUpdateResourceW and has not yet
        // been closed. Discard the staged updates after any failed step.
        let rollback = unsafe { EndUpdateResourceW(handle, true) };
        return match rollback {
            Ok(()) => Err(update_error),
            Err(rollback_error) => Err(anyhow!(
                "{update_error:#}; discarding resource update also failed: {rollback_error}"
            )),
        };
    }

    // SAFETY: handle still owns the staged resource updates.
    if let Err(commit_error) = unsafe { EndUpdateResourceW(handle, false) } {
        // A failed commit can leave the update handle open. Try to discard its
        // pending changes so a failed pack never commits a partial resource set.
        // SAFETY: the first EndUpdateResourceW returned an error.
        return match unsafe { EndUpdateResourceW(handle, true) } {
            Ok(()) => Err(commit_error).context("commit PE resource updates"),
            Err(rollback_error) => Err(anyhow!(
                "commit PE resource updates failed: {commit_error}; rollback also failed: {rollback_error}"
            )),
        };
    }
    Ok(())
}

fn update_rcdata(
    handle: windows::Win32::Foundation::HANDLE,
    req: &PackRequest,
    data: &PreparedData,
) -> Result<()> {
    let kind = resource_type(10); // RT_RCDATA
    for (resource_name, bytes) in &data.rcdata {
        let name = to_wide_string(resource_name)?;
        update_resource(handle, kind, PCWSTR(name.as_ptr()), req.lang, Some(bytes))
            .with_context(|| format!("write RCDATA resource {resource_name:?}"))?;
    }
    Ok(())
}

fn replace_icon(
    handle: windows::Win32::Foundation::HANDLE,
    req: &PackRequest,
    data: &PreparedData,
    existing_icon_ids: &[u16],
) -> Result<()> {
    let Some((ico, group_id)) = &data.icon else {
        return Ok(());
    };

    let icon_type = resource_type(3); // RT_ICON
    for id in existing_icon_ids {
        update_resource(handle, icon_type, resource_id(*id), req.lang, None)
            .with_context(|| format!("delete old RT_ICON id {id}"))?;
    }

    let entries = crate::icon::split_ico(ico)?;
    let first_id = req
        .delete_icon_ids
        .iter()
        .copied()
        .max()
        .unwrap_or(0)
        .checked_add(1)
        .ok_or_else(|| anyhow!("no RT_ICON IDs remain after deleted IDs"))?
        .max(1);
    let mut group = Vec::with_capacity(6 + entries.len() * 14);
    group.extend_from_slice(&0u16.to_le_bytes());
    group.extend_from_slice(&1u16.to_le_bytes());
    group.extend_from_slice(
        &u16::try_from(entries.len())
            .context("ICO has too many images for an icon group")?
            .to_le_bytes(),
    );

    for (index, entry) in entries.iter().enumerate() {
        let id = first_id
            .checked_add(u16::try_from(index).context("ICO has too many images")?)
            .ok_or_else(|| anyhow!("ICO icon resource ID exceeds 65535"))?;
        update_resource(
            handle,
            icon_type,
            resource_id(id),
            req.lang,
            Some(entry.data),
        )
        .with_context(|| format!("write RT_ICON id {id}"))?;

        // GRPICONDIRENTRY is the ICO entry with its image offset replaced by
        // the 16-bit RT_ICON resource ID.
        group.extend_from_slice(&entry.header);
        group.extend_from_slice(&id.to_le_bytes());
    }

    update_resource(
        handle,
        resource_type(14), // RT_GROUP_ICON
        resource_id(*group_id),
        req.lang,
        Some(&group),
    )
    .with_context(|| format!("write RT_GROUP_ICON id {group_id}"))
}

fn existing_icon_ids(req: &PackRequest, data: &PreparedData) -> Result<Vec<u16>> {
    if data.icon.is_none() || req.delete_icon_ids.is_empty() {
        return Ok(Vec::new());
    }
    let exe_path = to_wide_path(&req.output_exe)?;
    // SAFETY: the path is NUL-terminated and remains alive until the call returns.
    let module =
        unsafe { LoadLibraryExW(PCWSTR(exe_path.as_ptr()), None, LOAD_LIBRARY_AS_DATAFILE) }
            .with_context(|| format!("inspect icon resources in {}", req.output_exe.display()))?;
    let ids = req
        .delete_icon_ids
        .iter()
        .copied()
        .filter(|id| {
            // SAFETY: module is a valid data-file handle and the resource IDs
            // are MAKEINTRESOURCE-compatible integers.
            let resource = unsafe {
                FindResourceExW(Some(module), resource_type(3), resource_id(*id), req.lang)
            };
            !resource.0.is_null()
        })
        .collect();
    // SAFETY: module was returned by LoadLibraryExW above.
    unsafe { FreeLibrary(module) }.context("release icon resource inspection handle")?;
    Ok(ids)
}

fn apply_version(
    handle: windows::Win32::Foundation::HANDLE,
    req: &PackRequest,
    data: &PreparedData,
) -> Result<()> {
    let Some(version_info) = &data.version_info else {
        return Ok(());
    };
    update_resource(
        handle,
        resource_type(16), // RT_VERSION
        resource_id(1),
        req.lang,
        Some(version_info),
    )
    .context("write RT_VERSION resource 1")
}

fn update_resource(
    handle: windows::Win32::Foundation::HANDLE,
    kind: PCWSTR,
    name: PCWSTR,
    lang: u16,
    bytes: Option<&[u8]>,
) -> Result<()> {
    let (data, len) = match bytes {
        Some(bytes) => (
            Some(bytes.as_ptr().cast()),
            u32::try_from(bytes.len()).context("resource data exceeds Win32 size limit")?,
        ),
        None => (None, 0),
    };
    // SAFETY: integer resource identifiers are MAKEINTRESOURCE-compatible;
    // named-resource buffers and data bytes outlive the call.
    unsafe { UpdateResourceW(handle, kind, name, lang, data, len) }.map_err(Into::into)
}

fn resource_type(id: u16) -> PCWSTR {
    PCWSTR(usize::from(id) as *const u16)
}

fn resource_id(id: u16) -> PCWSTR {
    PCWSTR(usize::from(id) as *const u16)
}

fn to_wide_path(path: &Path) -> Result<Vec<u16>> {
    use std::os::windows::ffi::OsStrExt;
    Ok(path.as_os_str().encode_wide().chain(Some(0)).collect())
}

fn to_wide_string(value: &str) -> Result<Vec<u16>> {
    if value.contains('\0') {
        return Err(anyhow!("resource name contains NUL"));
    }
    Ok(value.encode_utf16().chain(Some(0)).collect())
}
