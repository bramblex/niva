use super::file_identity::{FileIdentity, file_id_info_is_unsupported_hresult};

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

    assert_eq!(first, same);
    assert_ne!(first, high_half_differs);
    assert_ne!(first, volume_differs);
    assert_eq!(FileIdentity::extended(7, [0; 16]), None);
}

#[test]
fn extended_id_is_not_reduced_to_a_matching_legacy_low_word() {
    let extended =
        FileIdentity::extended(7, [1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]).unwrap();
    let legacy = FileIdentity::legacy(7, 1).unwrap();

    assert_ne!(extended, legacy);
}

#[test]
fn invalid_legacy_file_indexes_never_become_equal_identities() {
    assert_eq!(FileIdentity::legacy(7, 0), None);
    assert_eq!(FileIdentity::legacy(7, u64::MAX), None);

    let valid = FileIdentity::legacy(7, 1).unwrap();
    assert_eq!(valid, FileIdentity::legacy(7, 1).unwrap());
    assert_ne!(valid, FileIdentity::legacy(8, 1).unwrap());
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
