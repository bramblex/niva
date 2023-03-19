// The following code is from https://github.com/Brooooooklyn/keyring-node/blob/main/src/entry.rs#L343
#[allow(dead_code)]
pub fn to_wstr(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(Some(0)).collect()
}

#[allow(dead_code)]
pub unsafe fn from_wstr(ws: *const u16) -> String {
    // null pointer case, return empty string
    if ws.is_null() {
        return String::new();
    }

    // this code from https://stackoverflow.com/a/48587463/558006
    let len = (0..).take_while(|&i| *ws.offset(i) != 0).count();
    let slice = std::slice::from_raw_parts(ws, len);
    String::from_utf16_lossy(slice)
}
