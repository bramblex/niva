use anyhow::{Ok, Result};

use std::ptr;
use winapi::um::libloaderapi::{
    FindResourceW, FreeResource, GetModuleHandleW, LoadResource, LockResource, SizeofResource,
};
use winapi::um::winuser::RT_RCDATA;

pub fn load_resource(name: &str) -> Result<Vec<u8>> {
    unsafe {
        let h_module = GetModuleHandleW(ptr::null());
        if h_module.is_null() {
            return Err(anyhow::anyhow!("Failed to get module handle."));
        }

        let lp_name = to_wstr(name);
        let lp_name_ptr = lp_name.as_ptr();

        let h_res_info = FindResourceW(h_module, lp_name_ptr, RT_RCDATA);
        if h_res_info.is_null() {
            return Err(anyhow::anyhow!("Failed to find resource."));
        }
        let size = SizeofResource(h_module, h_res_info) as usize;

        let h_res_data = LoadResource(h_module, h_res_info);
        if h_res_data.is_null() {
            return Err(anyhow::anyhow!("Failed to load resource."));
        }

        let lp_res_data = LockResource(h_res_data) as *const u8;
        if lp_res_data.is_null() {
            return Err(anyhow::anyhow!("Failed to lock resource."));
        }

        let mut data: Vec<u8> = vec![0; size];
        std::ptr::copy(lp_res_data, data.as_mut_ptr(), size);

        FreeResource(h_res_data);

        Ok(data)
    }
}

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