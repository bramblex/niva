//! Stable Windows file identity shared by path-alias and opened-path checks.
//!
//! ReFS may return 64-bit file indexes that are not unique. Prefer the full
//! 128-bit `FILE_ID_INFO` value and use the older index only when the extended
//! query is explicitly unsupported or the filesystem reports its zero-ID
//! sentinel.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum FileIdentity {
    Extended {
        volume_serial: u64,
        file_id: [u8; 16],
    },
    Legacy {
        volume_serial: u32,
        file_id: u64,
    },
}

impl FileIdentity {
    pub(crate) fn extended(volume_serial: u64, file_id: [u8; 16]) -> Option<Self> {
        (file_id != [0; 16]).then_some(Self::Extended {
            volume_serial,
            file_id,
        })
    }

    pub(crate) fn legacy(volume_serial: u32, file_id: u64) -> Option<Self> {
        (file_id != 0 && file_id != u64::MAX).then_some(Self::Legacy {
            volume_serial,
            file_id,
        })
    }
}

pub(crate) fn file_id_info_is_unsupported_hresult(hresult: u32) -> bool {
    let Some(error) = hresult_from_win32_error(hresult) else {
        return false;
    };

    matches!(
        error,
        ERROR_INVALID_FUNCTION | ERROR_NOT_SUPPORTED | ERROR_INVALID_PARAMETER
    )
}

#[cfg(windows)]
pub(crate) fn from_file(file: &std::fs::File) -> anyhow::Result<Option<FileIdentity>> {
    use std::os::windows::io::AsRawHandle;
    use windows::Win32::{
        Foundation::HANDLE,
        Storage::FileSystem::{
            BY_HANDLE_FILE_INFORMATION, FILE_ID_INFO, FileIdInfo, GetFileInformationByHandle,
            GetFileInformationByHandleEx,
        },
    };

    let handle = HANDLE(file.as_raw_handle() as *mut _);
    let mut extended = FILE_ID_INFO::default();
    // SAFETY: `extended` is a correctly sized writable result buffer and the
    // caller retains an open handle for the duration of this synchronous API.
    match unsafe {
        GetFileInformationByHandleEx(
            handle,
            FileIdInfo,
            (&mut extended as *mut FILE_ID_INFO).cast(),
            std::mem::size_of::<FILE_ID_INFO>() as u32,
        )
    } {
        Ok(()) => {
            if let Some(identity) =
                FileIdentity::extended(extended.VolumeSerialNumber, extended.FileId.Identifier)
            {
                return Ok(Some(identity));
            }
            // The all-zero extended ID is the documented fallback sentinel.
        }
        Err(error) if file_id_info_is_unsupported_hresult(error.code().0 as u32) => {
            // Only explicit unsupported errors allow the legacy query.
        }
        Err(error) => return Err(error.into()),
    }

    let mut legacy = BY_HANDLE_FILE_INFORMATION::default();
    // SAFETY: `legacy` is a correctly sized writable result buffer and `handle`
    // is still open.
    unsafe { GetFileInformationByHandle(handle, &mut legacy)? };
    let file_id = (u64::from(legacy.nFileIndexHigh) << 32) | u64::from(legacy.nFileIndexLow);
    Ok(FileIdentity::legacy(legacy.dwVolumeSerialNumber, file_id))
}

#[cfg(windows)]
pub(crate) fn opened_file_matches_path(
    file: &std::fs::File,
    path: &std::path::Path,
) -> anyhow::Result<bool> {
    let current = std::fs::File::open(path)?;
    let opened = from_file(file)?;
    let current = from_file(&current)?;
    Ok(matches!((opened, current), (Some(left), Some(right)) if left == right))
}

fn hresult_from_win32_error(hresult: u32) -> Option<u32> {
    if hresult & 0xffff_0000 != 0x8007_0000 {
        return None;
    }
    Some(hresult & 0xffff)
}

const ERROR_INVALID_FUNCTION: u32 = 1;
const ERROR_NOT_SUPPORTED: u32 = 50;
const ERROR_INVALID_PARAMETER: u32 = 87;
