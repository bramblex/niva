//! Pure identity comparison helpers shared with `win_utils`.
//!
//! This file intentionally has no Windows API dependencies so its policy can
//! be tested on non-Windows hosts with `rustc --test`.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum FileIdentity {
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
    pub(super) fn extended(volume_serial: u64, file_id: [u8; 16]) -> Option<Self> {
        // FILE_ID_INFORMATION specifies zero for file systems that do not
        // provide 128-bit IDs. It is a capability signal, not a usable ID.
        if file_id == [0; 16] {
            return None;
        }

        Some(Self::Extended {
            volume_serial,
            file_id,
        })
    }

    pub(super) fn legacy(volume_serial: u32, file_id: u64) -> Option<Self> {
        // BY_HANDLE_FILE_INFORMATION can report either value when the file
        // system cannot supply a usable 64-bit file index. Never treat such an
        // unknown index as an identity, even when both handles return it.
        if file_id == 0 || file_id == u64::MAX {
            return None;
        }

        Some(Self::Legacy {
            volume_serial,
            file_id,
        })
    }

    pub(super) fn matches(self, other: Self) -> bool {
        // Do not reduce the 128-bit ID to 64 bits or compare an extended ID
        // with a legacy one. A caller that observes different identity
        // capabilities for the two handles must reject the comparison.
        self == other
    }
}

/// Whether a failed FileIdInfo query explicitly indicates an unsupported
/// information class/provider, making the older identity query a reasonable
/// compatibility fallback.
///
/// The caller must pass an HRESULT produced by HRESULT_FROM_WIN32. Other
/// failures, including access denied and sharing violations, must propagate.
pub(super) fn file_id_info_is_unsupported_hresult(hresult: u32) -> bool {
    let Some(error) = hresult_from_win32_error(hresult) else {
        return false;
    };

    matches!(
        error,
        ERROR_INVALID_FUNCTION | ERROR_NOT_SUPPORTED | ERROR_INVALID_PARAMETER
    )
}

fn hresult_from_win32_error(hresult: u32) -> Option<u32> {
    // HRESULT_FROM_WIN32(error) uses facility 7 and the failure bit. Require
    // the full shape so unrelated HRESULTs cannot accidentally select fallback.
    if hresult & 0xffff_0000 != 0x8007_0000 {
        return None;
    }
    Some(hresult & 0xffff)
}

const ERROR_INVALID_FUNCTION: u32 = 1;
const ERROR_NOT_SUPPORTED: u32 = 50;
const ERROR_INVALID_PARAMETER: u32 = 87;

#[cfg(test)]
mod tests {
    use super::{FileIdentity, file_id_info_is_unsupported_hresult};

    fn hresult_from_win32(error: u32) -> u32 {
        if error == 0 { 0 } else { 0x8007_0000 | error }
    }

    #[test]
    fn extended_identity_compares_the_entire_128_bit_file_id() {
        let first = FileIdentity::extended(7, [1; 16]).unwrap();
        let same = FileIdentity::extended(7, [1; 16]).unwrap();
        let mut high_half_id = [1; 16];
        high_half_id[8] = 2;
        let high_half_differs = FileIdentity::extended(7, high_half_id).unwrap();
        let volume_differs = FileIdentity::extended(8, [1; 16]).unwrap();

        assert!(first.matches(same));
        assert!(!first.matches(high_half_differs));
        assert!(!first.matches(volume_differs));
        assert_eq!(FileIdentity::extended(7, [0; 16]), None);
    }

    #[test]
    fn extended_id_is_not_reduced_to_a_matching_legacy_low_word() {
        let extended =
            FileIdentity::extended(7, [1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]).unwrap();
        let legacy = FileIdentity::legacy(7, 1).unwrap();

        assert!(!extended.matches(legacy));
    }

    #[test]
    fn invalid_legacy_file_indexes_never_become_equal_identities() {
        assert_eq!(FileIdentity::legacy(7, 0), None);
        assert_eq!(FileIdentity::legacy(7, u64::MAX), None);

        let valid = FileIdentity::legacy(7, 1).unwrap();
        assert!(valid.matches(FileIdentity::legacy(7, 1).unwrap()));
        assert!(!valid.matches(FileIdentity::legacy(8, 1).unwrap()));
    }

    #[test]
    fn legacy_fallback_is_limited_to_explicit_unsupported_errors() {
        for error in [1, 50, 87] {
            assert!(file_id_info_is_unsupported_hresult(hresult_from_win32(
                error
            )));
        }

        for error in [2, 5, 32, 123] {
            assert!(!file_id_info_is_unsupported_hresult(hresult_from_win32(
                error
            )));
        }
        assert!(!file_id_info_is_unsupported_hresult(0x8000_4005));
    }
}
