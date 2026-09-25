use anyhow::{Ok, Result};

use windows::{
    Win32::Foundation::{
        ERROR_RESOURCE_LANG_NOT_FOUND, ERROR_RESOURCE_NAME_NOT_FOUND,
        ERROR_RESOURCE_TYPE_NOT_FOUND, GetLastError, HANDLE,
    },
    Win32::Storage::FileSystem::{BY_HANDLE_FILE_INFORMATION, GetFileInformationByHandle},
    Win32::System::LibraryLoader::{
        FindResourceW, FreeResource, GetModuleHandleW, LoadResource, LockResource, SizeofResource,
    },
    core::PCWSTR,
};

// MAKEINTRESOURCEW(10): predefined resource type, stable Win32 ABI.
const RT_RCDATA: PCWSTR = PCWSTR(10 as *const u16);

pub fn opened_file_matches_path(opened: &std::fs::File, path: &std::path::Path) -> Result<bool> {
    use std::os::windows::io::AsRawHandle;

    fn file_identity(file: &std::fs::File) -> Result<(u32, u32, u32)> {
        let mut info = BY_HANDLE_FILE_INFORMATION::default();
        unsafe {
            GetFileInformationByHandle(HANDLE(file.as_raw_handle() as *mut _), &mut info)?;
        }
        Ok((
            info.dwVolumeSerialNumber,
            info.nFileIndexHigh,
            info.nFileIndexLow,
        ))
    }

    let current = std::fs::File::open(path)?;
    Ok(file_identity(opened)? == file_identity(&current)?)
}

pub fn load_resource(name: &str) -> Result<Vec<u8>> {
    load_resource_optional(name)?.ok_or_else(|| anyhow::anyhow!("Failed to find resource."))
}

pub fn load_resource_optional(name: &str) -> Result<Option<Vec<u8>>> {
    unsafe {
        let h_module = GetModuleHandleW(None)?;
        if h_module.is_invalid() {
            return Err(anyhow::anyhow!("Failed to get module handle."));
        }

        let lp_name = to_wstr(name);
        let h_res_info = FindResourceW(Some(h_module), PCWSTR(lp_name.as_ptr()), RT_RCDATA);
        if h_res_info.is_invalid() {
            let error = GetLastError();
            if error == ERROR_RESOURCE_NAME_NOT_FOUND
                || error == ERROR_RESOURCE_TYPE_NOT_FOUND
                || error == ERROR_RESOURCE_LANG_NOT_FOUND
            {
                return Ok(None);
            }
            return Err(anyhow::anyhow!(
                "Failed to find resource (Win32 {}).",
                error.0
            ));
        }
        let size = SizeofResource(Some(h_module), h_res_info) as usize;
        if size == 0 {
            return Ok(Some(Vec::new()));
        }

        let h_res_data = LoadResource(Some(h_module), h_res_info)
            .map_err(|_| anyhow::anyhow!("Failed to load resource."))?;

        let lp_res_data = LockResource(h_res_data) as *const u8;
        if lp_res_data.is_null() {
            return Err(anyhow::anyhow!("Failed to lock resource."));
        }

        let mut data: Vec<u8> = vec![0; size];
        std::ptr::copy(lp_res_data, data.as_mut_ptr(), size);

        let _ = FreeResource(h_res_data);

        Ok(Some(data))
    }
}

// The following code is from https://github.com/Brooooooklyn/keyring-node/blob/main/src/entry.rs#L343
#[allow(dead_code)]
pub fn to_wstr(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(Some(0)).collect()
}

#[allow(dead_code)]
/// Convert a borrowed Windows UTF-16 string into a lossy Rust string.
///
/// # Safety
/// `ws` must point to a readable, NUL-terminated UTF-16 sequence that remains
/// valid for the duration of this call.
pub unsafe fn from_wstr(ws: *const u16) -> String {
    // null pointer case, return empty string
    if ws.is_null() {
        return String::new();
    }

    // this code from https://stackoverflow.com/a/48587463/558006
    let mut len = 0usize;
    // SAFETY: the caller guarantees that `ws` points to a readable
    // NUL-terminated UTF-16 sequence.
    unsafe {
        while *ws.add(len) != 0 {
            len += 1;
        }
        String::from_utf16_lossy(std::slice::from_raw_parts(ws, len))
    }
}
